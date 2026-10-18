//! Named portable profiles. IDs and names are SQL values, never filesystem names.
use super::open_connection;
use crate::config_profile::{CONFIG_PROFILE_BYTES, export_config_profile, parse_config_profile};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{Connection, OpenFlags, TransactionBehavior, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fmt, fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub const CONFIG_PROFILE_CATALOG_ENGINE: &str = "open-seo-spider/config-profile-catalog-v1";
pub const CONFIG_PROFILE_CATALOG_ENTRIES: usize = 128;
pub const CONFIG_PROFILE_CATALOG_NAME_BYTES: usize = 256;
pub const CONFIG_PROFILE_CATALOG_TOTAL_BYTES: usize = 256 * 1024 * 1024;
const APPLICATION_ID: i64 = 0x4f535043;
const TABLE: &str = "CREATE TABLE configuration_profiles(
 id TEXT NOT NULL PRIMARY KEY,
 name TEXT NOT NULL COLLATE BINARY UNIQUE,
 revision INTEGER NOT NULL,
 profile_sha256 TEXT NOT NULL,
 profile_bytes INTEGER NOT NULL,
 profile_text TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
)";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigProfileCatalogErrorKind {
    Validation,
    Missing,
    Conflict,
    Limit,
    Unavailable,
}

#[derive(Debug)]
pub struct ConfigProfileCatalogError {
    pub kind: ConfigProfileCatalogErrorKind,
    message: String,
}
impl fmt::Display for ConfigProfileCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ConfigProfileCatalogError {}
type Result<T> = std::result::Result<T, ConfigProfileCatalogError>;
fn error(
    kind: ConfigProfileCatalogErrorKind,
    message: impl Into<String>,
) -> ConfigProfileCatalogError {
    ConfigProfileCatalogError {
        kind,
        message: message.into(),
    }
}
fn unavailable(stage: &str, cause: impl fmt::Display) -> ConfigProfileCatalogError {
    error(
        ConfigProfileCatalogErrorKind::Unavailable,
        format!("Configuration profile catalog {stage}: {cause}"),
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConfigProfileCatalogEntry {
    pub id: String,
    pub name: String,
    /// Positive canonical decimal, never a JSON number.
    pub revision: String,
    pub profile_sha256: String,
    pub profile_bytes: usize,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug)]
pub struct ConfigProfileCatalogDocument {
    pub entry: ConfigProfileCatalogEntry,
    pub profile_text: String,
}
#[derive(Clone, Copy)]
struct Limits {
    entries: usize,
    profile: usize,
    total: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            entries: CONFIG_PROFILE_CATALOG_ENTRIES,
            profile: CONFIG_PROFILE_BYTES,
            total: CONFIG_PROFILE_CATALOG_TOTAL_BYTES,
        }
    }
}

pub struct ConfigProfileCatalog {
    path: PathBuf,
    limits: Limits,
}
impl ConfigProfileCatalog {
    /// Construction performs no I/O. The application chooses the one fixed path.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
            limits: Limits::default(),
        }
    }

    pub fn list(&self) -> Result<Vec<ConfigProfileCatalogEntry>> {
        let Some(mut connection) = self.open_read()? else {
            return Ok(Vec::new());
        };
        let transaction = connection
            .transaction()
            .map_err(|e| unavailable("begin read", e))?;
        let (entries, _) = self.read_catalog(&transaction, None)?;
        transaction
            .commit()
            .map_err(|e| unavailable("finish read", e))?;
        Ok(entries)
    }

    pub fn get(&self, id: &str) -> Result<ConfigProfileCatalogDocument> {
        validate_id(id)?;
        let Some(mut connection) = self.open_read()? else {
            return Err(missing());
        };
        let transaction = connection
            .transaction()
            .map_err(|e| unavailable("begin read", e))?;
        let (entries, text) = self.read_catalog(&transaction, Some(id))?;
        let entry = entries
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(missing)?;
        let profile_text =
            text.ok_or_else(|| unavailable("read", "Missing selected profile bytes"))?;
        transaction
            .commit()
            .map_err(|e| unavailable("finish read", e))?;
        Ok(ConfigProfileCatalogDocument {
            entry,
            profile_text,
        })
    }

    pub fn create(
        &self,
        id: &str,
        name: &str,
        profile_text: &str,
    ) -> Result<ConfigProfileCatalogEntry> {
        validate_id(id)?;
        validate_name(name)?;
        // Reject malformed/secret-bearing material before mkdir or database creation.
        let text = self.canonical(profile_text)?;
        let mut connection = self.open_write(true)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| unavailable("begin create", e))?;
        let (entries, _) = self.read_catalog(&transaction, None)?;
        if entries
            .iter()
            .any(|entry| entry.id == id || entry.name == name)
        {
            return Err(conflict());
        }
        if entries.len() >= self.limits.entries {
            return Err(limit("Configuration profile catalog entry limit reached"));
        }
        self.check_total(&entries, None, text.len())?;
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let entry = ConfigProfileCatalogEntry {
            id: id.into(),
            name: name.into(),
            revision: "1".into(),
            profile_sha256: digest(text.as_bytes()),
            profile_bytes: text.len(),
            created_at: now.clone(),
            updated_at: now,
        };
        transaction.execute("INSERT INTO configuration_profiles(id,name,revision,profile_sha256,profile_bytes,profile_text,created_at,updated_at) VALUES(?1,?2,1,?3,?4,?5,?6,?7)", params![entry.id, entry.name, entry.profile_sha256, entry.profile_bytes as i64, text, entry.created_at, entry.updated_at]).map_err(|e| unavailable("insert profile", e))?;
        transaction
            .commit()
            .map_err(|e| unavailable("commit create", e))?;
        Ok(entry)
    }

    pub fn replace(
        &self,
        id: &str,
        expected_revision: &str,
        expected_sha256: &str,
        name: &str,
        profile_text: &str,
    ) -> Result<ConfigProfileCatalogEntry> {
        self.update(
            id,
            expected_revision,
            expected_sha256,
            name,
            Some(profile_text),
        )
    }
    pub fn rename(
        &self,
        id: &str,
        expected_revision: &str,
        expected_sha256: &str,
        name: &str,
    ) -> Result<ConfigProfileCatalogEntry> {
        self.update(id, expected_revision, expected_sha256, name, None)
    }
    fn update(
        &self,
        id: &str,
        expected_revision: &str,
        expected_sha256: &str,
        name: &str,
        profile_text: Option<&str>,
    ) -> Result<ConfigProfileCatalogEntry> {
        validate_id(id)?;
        validate_name(name)?;
        validate_expected(expected_revision, expected_sha256)?;
        let text = profile_text.map(|text| self.canonical(text)).transpose()?;
        let mut connection = self.open_write(false)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| unavailable("begin update", e))?;
        let (entries, _) = self.read_catalog(&transaction, None)?;
        let mut entry = entries
            .iter()
            .find(|entry| entry.id == id)
            .cloned()
            .ok_or_else(missing)?;
        require_expected(&entry, expected_revision, expected_sha256)?;
        if entries
            .iter()
            .any(|other| other.id != id && other.name == name)
        {
            return Err(conflict());
        }
        if let Some(text) = &text {
            self.check_total(&entries, Some(id), text.len())?;
            entry.profile_sha256 = digest(text.as_bytes());
            entry.profile_bytes = text.len();
        }
        entry.revision = next_revision(&entry.revision)?;
        entry.name = name.into();
        entry.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let changed = if let Some(text) = text {
            transaction.execute("UPDATE configuration_profiles SET name=?1,revision=?2,profile_sha256=?3,profile_bytes=?4,profile_text=?5,updated_at=?6 WHERE id=?7 AND revision=?8 AND profile_sha256=?9",params![entry.name, entry.revision.parse::<i64>().unwrap(), entry.profile_sha256, entry.profile_bytes as i64, text, entry.updated_at, id, expected_revision.parse::<i64>().unwrap(), expected_sha256])
        } else {
            transaction.execute("UPDATE configuration_profiles SET name=?1,revision=?2,updated_at=?3 WHERE id=?4 AND revision=?5 AND profile_sha256=?6",params![entry.name, entry.revision.parse::<i64>().unwrap(), entry.updated_at, id, expected_revision.parse::<i64>().unwrap(), expected_sha256])
        }.map_err(|e| unavailable("update profile", e))?;
        if changed != 1 {
            return Err(conflict());
        }
        transaction
            .commit()
            .map_err(|e| unavailable("commit update", e))?;
        Ok(entry)
    }

    pub fn delete(&self, id: &str, expected_revision: &str, expected_sha256: &str) -> Result<()> {
        validate_id(id)?;
        validate_expected(expected_revision, expected_sha256)?;
        let mut connection = self.open_write(false)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| unavailable("begin delete", e))?;
        let (entries, _) = self.read_catalog(&transaction, None)?;
        let entry = entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(missing)?;
        require_expected(entry, expected_revision, expected_sha256)?;
        let changed = transaction.execute("DELETE FROM configuration_profiles WHERE id=?1 AND revision=?2 AND profile_sha256=?3", params![id, expected_revision.parse::<i64>().unwrap(), expected_sha256]).map_err(|e| unavailable("delete profile", e))?;
        if changed != 1 {
            return Err(conflict());
        }
        transaction
            .commit()
            .map_err(|e| unavailable("commit delete", e))?;
        Ok(())
    }

    fn canonical(&self, text: &str) -> Result<String> {
        if text.len() > self.limits.profile {
            return Err(limit("Configuration profile exceeds its byte limit"));
        }
        let profile = parse_config_profile(text.as_bytes())
            .map_err(|e| error(ConfigProfileCatalogErrorKind::Validation, e.to_string()))?;
        let bytes = export_config_profile(&profile)
            .map_err(|e| error(ConfigProfileCatalogErrorKind::Validation, e.to_string()))?;
        if bytes.len() > self.limits.profile {
            return Err(limit(
                "Canonical configuration profile exceeds its byte limit",
            ));
        }
        String::from_utf8(bytes)
            .map_err(|e| error(ConfigProfileCatalogErrorKind::Validation, e.to_string()))
    }
    fn check_total(
        &self,
        entries: &[ConfigProfileCatalogEntry],
        replaced: Option<&str>,
        incoming: usize,
    ) -> Result<()> {
        let total = entries
            .iter()
            .filter(|entry| Some(entry.id.as_str()) != replaced)
            .try_fold(incoming, |total, entry| {
                total.checked_add(entry.profile_bytes)
            });
        if total.is_none_or(|total| total > self.limits.total) {
            return Err(limit(
                "Configuration profile catalog live-byte limit reached",
            ));
        }
        Ok(())
    }
    fn exists(&self) -> Result<bool> {
        match fs::metadata(&self.path) {
            Ok(metadata) if metadata.is_file() => Ok(true),
            Ok(_) => Err(unavailable("open", "Catalog path is not a regular file")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(unavailable("inspect path", e)),
        }
    }
    fn open_read(&self) -> Result<Option<Connection>> {
        if !self.exists()? {
            return Ok(None);
        }
        let connection = open_connection(&self.path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| unavailable("read-only open", e))?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|e| unavailable("configure read", e))?;
        Ok(Some(connection))
    }
    fn open_write(&self, create: bool) -> Result<Connection> {
        let mut fresh = false;
        if !self.exists()? {
            if !create {
                return Err(missing());
            }
            if let Some(parent) = self.path.parent() {
                fs::create_dir_all(parent).map_err(|e| unavailable("create directory", e))?;
            }
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&self.path)
            {
                Ok(_) => fresh = true,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
                Err(e) => return Err(unavailable("create database", e)),
            }
        }
        let mut connection = open_connection(&self.path, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(|e| unavailable("writable open", e))?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|e| unavailable("configure write", e))?;
        // Deliberately use SQLite's ordinary DELETE journal. Rare profile CAS
        // writes need no persistent WAL and no second filesystem/VFS adapter.
        if fresh {
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| unavailable("begin initialize", e))?;
            transaction
                .execute_batch(TABLE)
                .map_err(|e| unavailable("initialize schema", e))?;
            transaction
                .pragma_update(None, "application_id", APPLICATION_ID)
                .map_err(|e| unavailable("initialize identity", e))?;
            transaction
                .pragma_update(None, "user_version", 1)
                .map_err(|e| unavailable("initialize version", e))?;
            transaction
                .commit()
                .map_err(|e| unavailable("commit initialize", e))?;
        }
        Ok(connection)
    }

    fn read_catalog(
        &self,
        connection: &Connection,
        selected: Option<&str>,
    ) -> Result<(Vec<ConfigProfileCatalogEntry>, Option<String>)> {
        self.check_schema(connection)?;
        let count: i64 = connection
            .query_row("SELECT count(*) FROM configuration_profiles", [], |row| {
                row.get(0)
            })
            .map_err(|e| unavailable("count entries", e))?;
        if count < 0 || count as usize > self.limits.entries {
            return Err(unavailable(
                "integrity",
                "Stored entry count exceeds the catalog limit",
            ));
        }
        // All byte/type guards run in the SAME transaction before any profile
        // String is read. SQLite length(BLOB) does not decode/copy text in Rust.
        let (total, invalid): (i64, i64) = connection.query_row("SELECT COALESCE(SUM(length(CAST(profile_text AS BLOB))),0),COALESCE(SUM(CASE WHEN
          typeof(id)!='text' OR length(CAST(id AS BLOB))!=36 OR typeof(name)!='text' OR length(CAST(name AS BLOB)) NOT BETWEEN 1 AND 256 OR
          typeof(revision)!='integer' OR revision<1 OR typeof(profile_sha256)!='text' OR length(CAST(profile_sha256 AS BLOB))!=64 OR
          typeof(profile_text)!='text' OR length(CAST(profile_text AS BLOB))>?1 OR typeof(profile_bytes)!='integer' OR profile_bytes!=length(CAST(profile_text AS BLOB)) OR
          typeof(created_at)!='text' OR length(CAST(created_at AS BLOB)) NOT BETWEEN 1 AND 64 OR typeof(updated_at)!='text' OR length(CAST(updated_at AS BLOB)) NOT BETWEEN 1 AND 64
          THEN 1 ELSE 0 END),0) FROM configuration_profiles", [self.limits.profile as i64], |row| Ok((row.get(0)?,row.get(1)?))).map_err(|e| unavailable("preflight stored bytes", e))?;
        if invalid != 0 || total < 0 || total as usize > self.limits.total {
            return Err(unavailable(
                "integrity",
                "Stored catalog byte/type limits are invalid",
            ));
        }
        let mut statement = connection.prepare("SELECT id,name,revision,profile_sha256,profile_bytes,created_at,updated_at,profile_text FROM configuration_profiles ORDER BY id COLLATE BINARY").map_err(|e| unavailable("prepare entries", e))?;
        let mut rows = statement
            .query([])
            .map_err(|e| unavailable("read entries", e))?;
        let mut entries = Vec::new();
        let mut names = BTreeSet::new();
        let mut selected_text = None;
        while let Some(row) = rows.next().map_err(|e| unavailable("read entry", e))? {
            let value = (|| -> rusqlite::Result<_> {
                Ok((
                    ConfigProfileCatalogEntry {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        revision: row.get::<_, i64>(2)?.to_string(),
                        profile_sha256: row.get(3)?,
                        profile_bytes: row.get::<_, i64>(4)? as usize,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    },
                    row.get::<_, String>(7)?,
                ))
            })()
            .map_err(|e| unavailable("decode entry", e))?;
            let (entry, text) = value;
            if validate_id(&entry.id).is_err()
                || validate_name(&entry.name).is_err()
                || validate_expected(&entry.revision, &entry.profile_sha256).is_err()
                || !names.insert(entry.name.clone())
                || !valid_timestamp(&entry.created_at)
                || !valid_timestamp(&entry.updated_at)
            {
                return Err(unavailable(
                    "integrity",
                    "Stored profile metadata is invalid",
                ));
            }
            if digest(text.as_bytes()) != entry.profile_sha256 {
                return Err(unavailable("integrity", "Stored profile hash mismatch"));
            }
            let canonical = self
                .canonical(&text)
                .map_err(|_| unavailable("integrity", "Stored profile format is invalid"))?;
            if canonical != text {
                return Err(unavailable("integrity", "Stored profile is not canonical"));
            }
            if selected == Some(entry.id.as_str()) {
                selected_text = Some(text);
            }
            entries.push(entry);
        }
        Ok((entries, selected_text))
    }
    fn check_schema(&self, connection: &Connection) -> Result<()> {
        let application: i64 = connection
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .map_err(|e| unavailable("read identity", e))?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|e| unavailable("read version", e))?;
        if application != APPLICATION_ID || version != 1 {
            return Err(unavailable(
                "schema",
                "Unsupported catalog identity or version",
            ));
        }
        let tables: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| unavailable("read schema", e))?;
        let sql: String = connection.query_row("SELECT sql FROM sqlite_schema WHERE type='table' AND name='configuration_profiles'",[],|row|row.get(0)).map_err(|e|unavailable("read catalog schema",e))?;
        if tables != 1 || sql.trim() != TABLE.trim() {
            return Err(unavailable("schema", "Unexpected catalog table schema"));
        }
        Ok(())
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_timestamp(value: &str) -> bool {
    // The v1 producer writes four-digit UTC years and exactly milliseconds.
    // Reject alternate RFC3339 spellings instead of exposing metadata that the
    // same-version browser cannot faithfully accept.
    value.len() == 24
        && DateTime::parse_from_rfc3339(value).is_ok_and(|timestamp| {
            timestamp
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Millis, true)
                == value
        })
}
fn validate_id(id: &str) -> Result<()> {
    if id.len() != 36
        || !id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
    {
        return Err(error(
            ConfigProfileCatalogErrorKind::Validation,
            "Catalog ID must be a canonical UUID",
        ));
    }
    Ok(())
}
fn validate_name(name: &str) -> Result<()> {
    if name.len() > CONFIG_PROFILE_CATALOG_NAME_BYTES
        || name.trim().is_empty()
        || name.chars().any(char::is_control)
    {
        return Err(error(
            ConfigProfileCatalogErrorKind::Validation,
            "Catalog name must be nonblank, contain no controls and fit256 UTF8 bytes",
        ));
    }
    Ok(())
}
fn validate_expected(revision: &str, sha: &str) -> Result<()> {
    if !revision.bytes().all(|b| b.is_ascii_digit())
        || revision.starts_with('0')
        || revision.parse::<i64>().ok().is_none_or(|n| n < 1)
        || sha.len() != 64
        || !sha
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(error(
            ConfigProfileCatalogErrorKind::Validation,
            "Expected catalog revision/hash must be canonical positive decimal and lowercase SHA256",
        ));
    }
    Ok(())
}
fn next_revision(revision: &str) -> Result<String> {
    revision
        .parse::<i64>()
        .ok()
        .and_then(|n| n.checked_add(1))
        .map(|n| n.to_string())
        .ok_or_else(|| limit("Catalog revision range exhausted"))
}
fn require_expected(entry: &ConfigProfileCatalogEntry, revision: &str, sha: &str) -> Result<()> {
    if entry.revision != revision || entry.profile_sha256 != sha {
        Err(conflict())
    } else {
        Ok(())
    }
}
fn missing() -> ConfigProfileCatalogError {
    error(
        ConfigProfileCatalogErrorKind::Missing,
        "Configuration profile catalog entry was not found",
    )
}
fn conflict() -> ConfigProfileCatalogError {
    error(
        ConfigProfileCatalogErrorKind::Conflict,
        "Configuration profile catalog revision, hash, ID or exact name conflicts",
    )
}
fn limit(message: &str) -> ConfigProfileCatalogError {
    error(ConfigProfileCatalogErrorKind::Limit, message)
}

#[cfg(test)]
#[path = "config_profile_catalog_tests.rs"]
mod tests;
