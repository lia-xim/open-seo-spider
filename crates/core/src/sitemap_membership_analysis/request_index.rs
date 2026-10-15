//! Bounded disposable same-request index, never durable or cross-request authority.
//! The original hash, complete report/source/ledger/checkpoint checks run first.
//! This adds a separate logical64MiB/262144-row budget and a private128MiB
//! SQLite page ceiling; neither is the old2MiB projection limit or an RSS claim.
use super::*;
use crate::sitemap_ledger::verified_read::VerifiedRawRead;
use rusqlite::types::{ToSqlOutput, ValueRef};

fn borrowed(value: ValueRef<'_>) -> ToSqlOutput<'_> {
    ToSqlOutput::Borrowed(value)
}

const MAX_LOGICAL_BYTES: usize = 64 * 1024 * 1024;
const MAX_ROWS: usize = 262_144;
const MAX_ROW_UTF8_BYTES: usize = 2 * 1024 * 1024;
const PAGE_SIZE: usize = 4096;
const MAX_DB_PAGES: usize = 32768;
const ROW_CHARGE: usize = 256;

#[derive(Debug)]
enum Declined {
    Budget,
    Unsupported,
}
impl std::fmt::Display for Declined {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Optional request index declined")
    }
}
impl std::error::Error for Declined {}
#[derive(Clone, Copy)]
struct Limits {
    logical_bytes: usize,
    rows: usize,
    db_pages: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            logical_bytes: MAX_LOGICAL_BYTES,
            rows: MAX_ROWS,
            db_pages: MAX_DB_PAGES,
        }
    }
}
struct Charge {
    bytes: usize,
    utf8: usize,
    rows: usize,
    limits: Limits,
}
impl Charge {
    fn row(&mut self, fields: &[ValueRef<'_>]) -> Result<()> {
        let mut bytes = 0usize;
        for field in fields {
            if let ValueRef::Text(raw) | ValueRef::Blob(raw) = field {
                if raw.len() > MAX_ROW_UTF8_BYTES {
                    return Err(Declined::Budget.into());
                }
                if matches!(field, ValueRef::Text(_)) && std::str::from_utf8(raw).is_err() {
                    return Err(Declined::Unsupported.into());
                }
                bytes = bytes
                    .checked_add(raw.len())
                    .context("Request index byte overflow")?;
            }
        }
        if bytes > MAX_ROW_UTF8_BYTES {
            return Err(Declined::Budget.into());
        }
        let next = self
            .bytes
            .checked_add(bytes)
            .and_then(|n| n.checked_add(ROW_CHARGE))
            .context("Request index charge overflow")?;
        let rows = self
            .rows
            .checked_add(1)
            .context("Request index row overflow")?;
        if next > self.limits.logical_bytes.min(MAX_LOGICAL_BYTES)
            || rows > self.limits.rows.min(MAX_ROWS)
        {
            return Err(Declined::Budget.into());
        }
        let utf8 = self.utf8.checked_add(bytes).ok_or(Declined::Budget)?;
        self.bytes = next;
        self.rows = rows;
        self.utf8 = utf8;
        Ok(())
    }
}

pub(super) struct RequestIndex {
    connection: Connection,
    // Same-connection binding only. No source handle or snapshot is fabricated.
    source_address: usize,
    source_total_changes: u64,
}
impl RequestIndex {
    pub(super) fn prepare(
        source: &Connection,
        raw: &VerifiedRawRead<'_>,
        control: &SitemapReadControl,
    ) -> Result<Option<Self>> {
        ensure!(
            !source.is_autocommit() && std::ptr::eq(source, raw.connection()),
            "Request index has no same-TX verified raw basis"
        );
        prepare_with_limits(source, control, Limits::default())
    }
    pub(super) fn connection<'a>(&'a self, source: &Connection) -> Result<&'a Connection> {
        ensure!(
            !source.is_autocommit() && source as *const Connection as usize == self.source_address,
            "Request index source transaction changed"
        );
        ensure!(
            source.total_changes() == self.source_total_changes,
            "Request index source was written after preparation"
        );
        Ok(&self.connection)
    }
    fn columns() -> String {
        // Retain the existing SQLite JSON projection and the unchanged decoder.
        // First matching operation target is indexed by its actual traversal
        // ordinal; duplicate/scalar generation values retain SQLite types.
        page_columns().replace(
            "(SELECT json_extract(t.value,'$.committed_generation') FROM json_each(m.operation_targets_json) t WHERE json_extract(t.value,'$.requested_url')=json_extract(p.value,'$.url'))",
            "(SELECT generation FROM targets WHERE url COLLATE BINARY=p.url COLLATE BINARY ORDER BY ordinal LIMIT 1)",
        ).replace("m.configured_renderer", "(SELECT configured_renderer FROM context)")
    }
    pub(super) fn candidate_sql(&self, tail: &str) -> String {
        format!(
            "SELECT {},{} FROM pages p WHERE (?1 IS NULL OR p.key>?1) ORDER BY p.key",
            Self::columns(),
            tail
        )
    }
    pub(super) fn window(
        &self,
        source: &Connection,
        request: &SitemapMembershipAnalysisRequest,
        selected: bool,
        generation: u64,
        control: &SitemapReadControl,
    ) -> Result<Vec<SitemapTargetObservation>> {
        if request.page_limit == 0 {
            return Ok(vec![]);
        }
        let connection = self.connection(source)?;
        let _binding = control.bind_connection(connection)?;
        let sql = format!(
            "SELECT {} FROM pages p WHERE (?1 IS NULL OR p.key>?1) ORDER BY p.key LIMIT ?2",
            Self::columns()
        );
        let mut statement = connection.prepare(&sql)?;
        let mut rows = statement.query(params![
            request.after_page_ordinal.map(i64::try_from).transpose()?,
            i64::try_from(request.page_limit + 1)?
        ])?;
        #[cfg(test)]
        change_work(|w| w.indexed_queries += 1);
        let mut out = vec![];
        while let Some(row) = rows.next()? {
            control.checkpoint()?;
            out.push(page_from_row(row, selected, generation)?);
        }
        Ok(out)
    }
    pub(super) fn targets(
        &self,
        source: &Connection,
        wanted: &BTreeSet<String>,
        selected: bool,
        generation: u64,
        control: &SitemapReadControl,
    ) -> Result<BTreeMap<String, SitemapTargetObservation>> {
        if wanted.is_empty() {
            return Ok(BTreeMap::new());
        }
        let connection = self.connection(source)?;
        let _binding = control.bind_connection(connection)?;
        let sql = format!(
            "SELECT {} FROM pages p WHERE p.url COLLATE BINARY IN (SELECT value FROM json_each(?1)) ORDER BY p.key",
            Self::columns()
        );
        let mut statement = connection.prepare(&sql)?;
        let mut rows = statement.query([serde_json::to_string(wanted)?])?;
        #[cfg(test)]
        change_work(|w| w.indexed_queries += 1);
        let mut out = BTreeMap::new();
        while let Some(row) = rows.next()? {
            control.checkpoint()?;
            let page = page_from_row(row, selected, generation)?;
            out.insert(page.requested_url.clone(), page);
        }
        Ok(out)
    }
    pub(super) fn inlinks(
        &self,
        source: &Connection,
        wanted: &BTreeSet<String>,
        control: &SitemapReadControl,
    ) -> Result<BTreeMap<String, SitemapObservedInlinks>> {
        let connection = self.connection(source)?;
        let _binding = control.bind_connection(connection)?;
        #[cfg(test)]
        change_work(|w| w.indexed_queries += usize::from(!wanted.is_empty()));
        read_inlinks_from(
            connection,
            wanted,
            control,
            "FROM anchors l JOIN pages p ON p.key=l.page_ordinal WHERE l.target COLLATE BINARY IN (SELECT value FROM json_each(?1))",
        )
    }
}

fn prepare_with_limits(
    source: &Connection,
    control: &SitemapReadControl,
    limits: Limits,
) -> Result<Option<RequestIndex>> {
    control.checkpoint()?;
    if source.is_autocommit() {
        return Ok(None);
    }
    #[cfg(test)]
    change_work(|w| w.attempts += 1);
    let result = build(source, control, limits);
    // Cancellation always wins, including when it interrupted the auxiliary DB.
    control.checkpoint()?;
    match result {
        Ok(index) => {
            #[cfg(test)]
            change_work(|w| w.committed += 1);
            Ok(Some(index))
        }
        Err(error) => {
            let declined = error.downcast_ref::<Declined>();
            let sqlite = error
                .chain()
                .find_map(|cause| cause.downcast_ref::<rusqlite::Error>())
                .and_then(|e| match e {
                    rusqlite::Error::SqliteFailure(code, message) => {
                        Some((code.code, message.as_deref()))
                    }
                    _ => None,
                });
            let fallback = declined.is_some()
                || matches!(
                    sqlite,
                    Some((
                        rusqlite::ErrorCode::OutOfMemory | rusqlite::ErrorCode::DiskFull,
                        _
                    )) | Some((rusqlite::ErrorCode::Unknown, Some("malformed JSON")))
                );
            if fallback {
                #[cfg(test)]
                change_work(|w| match (declined, sqlite) {
                    (Some(Declined::Budget), _) => w.fallback_budget += 1,
                    (_, Some((rusqlite::ErrorCode::OutOfMemory, _))) => w.fallback_oom += 1,
                    (_, Some((rusqlite::ErrorCode::DiskFull, _))) => w.fallback_full += 1,
                    _ => w.fallback_unsupported += 1,
                });
                // Dropping build's private connection/transaction freed its
                // incomplete index; callers run the original SQL path exactly.
                Ok(None)
            } else {
                control.map_result(Err(error))
            }
        }
    }
}
fn build(
    source: &Connection,
    control: &SitemapReadControl,
    limits: Limits,
) -> Result<RequestIndex> {
    let mut indexed = Connection::open_in_memory()?;
    let _binding = control.bind_connection(&indexed)?;
    let mut charge = Charge {
        bytes: 0,
        utf8: 0,
        rows: 0,
        limits,
    };
    indexed.execute_batch("PRAGMA page_size=4096;PRAGMA temp_store=MEMORY;PRAGMA cache_size=-2048;PRAGMA max_page_count=32768;
        CREATE TABLE context(configured_renderer INTEGER NOT NULL);
        CREATE TABLE pages(key INTEGER PRIMARY KEY,url TEXT COLLATE BINARY NOT NULL,value TEXT NOT NULL);
        CREATE INDEX pages_url ON pages(url COLLATE BINARY);
        CREATE TABLE targets(ordinal INTEGER PRIMARY KEY,url TEXT COLLATE BINARY NOT NULL,generation);
        CREATE INDEX targets_url ON targets(url COLLATE BINARY,ordinal);
        CREATE TABLE anchors(page_ordinal INTEGER NOT NULL,key INTEGER NOT NULL,target TEXT COLLATE BINARY NOT NULL,value TEXT NOT NULL);
        CREATE INDEX anchors_target ON anchors(target COLLATE BINARY);
        CREATE TABLE members(original_url TEXT COLLATE BINARY,normalized_url TEXT,alias_url TEXT COLLATE BINARY,task TEXT COLLATE BINARY,observation TEXT COLLATE BINARY);
        CREATE INDEX members_original ON members(original_url COLLATE BINARY);
        CREATE INDEX members_alias ON members(alias_url COLLATE BINARY);")?;
    // The page ceiling is independent of logical/UTF8 row admission. Reduce it
    // for deterministic FULL controls; never raise the production128MiB bound.
    let _: i64 = indexed.query_row("PRAGMA max_page_count", [], |r| r.get(0))?;
    let db_pages =
        i64::try_from(limits.db_pages.min(MAX_DB_PAGES)).map_err(|_| Declined::Budget)?;
    indexed.pragma_update(None, "max_page_count", db_pages)?;
    let tx = indexed.transaction()?;
    {
        let mut statement=source.prepare("SELECT typeof(value),json_quote(json_extract(value,'$.operation.targets')),coalesce(json_type(value,'$.config.renderer')='object',0),json_type(value,'$.operation.targets') FROM metadata WHERE key='report'")?;
        let mut rows = statement.query([])?;
        #[cfg(test)]
        change_work(|w| w.original_report_selects += 1);
        let row = rows.next()?.context("Verified report missing")?;
        if row.get::<_, String>(0)? != "text" {
            return Err(Declined::Unsupported.into());
        }
        let targets = row.get_ref(1)?;
        charge.row(&[targets])?;
        let kind: Option<String> = row.get(3)?;
        if !matches!(kind.as_deref(), None | Some("null" | "array")) {
            return Err(Declined::Unsupported.into());
        }
        tx.execute("INSERT INTO context VALUES(?1)", [row.get::<_, bool>(2)?])?;
        let mut target_statement=tx.prepare("SELECT json_extract(value,'$.requested_url'),json_extract(value,'$.committed_generation') FROM json_each(?1)")?;
        let mut insert = tx.prepare("INSERT INTO targets VALUES(?1,?2,?3)")?;
        let mut targets = target_statement.query([borrowed(targets)])?;
        let mut ordinal = 0i64;
        while let Some(row) = targets.next()? {
            control.checkpoint()?;
            let url = row.get_ref(0)?;
            let generation = row.get_ref(1)?;
            charge.row(&[url, generation])?;
            if let ValueRef::Text(url) = url {
                let url = std::str::from_utf8(url).map_err(|_| Declined::Unsupported)?;
                insert.execute(params![ordinal, url, borrowed(generation)])?;
            }
            ordinal = ordinal
                .checked_add(1)
                .context("Operation index ordinal overflow")?;
            #[cfg(test)]
            change_work(|w| w.operation_rows += 1);
        }
    }
    {
        let mut statement=source.prepare("SELECT CAST(p.key AS INTEGER),json_extract(p.value,'$.url'),p.value FROM metadata m,json_each(m.value,'$.pages') p WHERE m.key='report'")?;
        let mut rows = statement.query([])?;
        #[cfg(test)]
        change_work(|w| w.original_report_selects += 1);
        let mut insert = tx.prepare("INSERT INTO pages VALUES(?1,?2,?3)")?;
        while let Some(row) = rows.next()? {
            control.checkpoint()?;
            let url = row.get_ref(1)?;
            let value = row.get_ref(2)?;
            charge.row(&[url, value])?;
            if !matches!((url, value), (ValueRef::Text(_), ValueRef::Text(_))) {
                return Err(Declined::Unsupported.into());
            }
            insert.execute(params![
                row.get::<_, i64>(0)?,
                borrowed(url),
                borrowed(value)
            ])?;
            #[cfg(test)]
            change_work(|w| w.original_page_rows += 1);
        }
    }
    {
        let mut statement=tx.prepare("SELECT p.key,CAST(l.key AS INTEGER),json_extract(l.value,'$.url'),l.value FROM pages p,json_each(p.value,'$.links') l WHERE json_extract(l.value,'$.kind')='anchor'")?;
        let mut rows = statement.query([])?;
        let mut insert = tx.prepare("INSERT INTO anchors VALUES(?1,?2,?3,?4)")?;
        while let Some(row) = rows.next()? {
            control.checkpoint()?;
            let target = row.get_ref(2)?;
            let value = row.get_ref(3)?;
            charge.row(&[target, value])?;
            if !matches!((target, value), (ValueRef::Text(_), ValueRef::Text(_))) {
                return Err(Declined::Unsupported.into());
            }
            insert.execute(params![
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                borrowed(target),
                borrowed(value)
            ])?;
            #[cfg(test)]
            change_work(|w| w.private_anchor_rows += 1);
        }
    }
    {
        let mut statement=source.prepare("SELECT CASE WHEN json_extract(l.data,'$.row.observed.validation_error') IS NULL AND json_type(l.data,'$.row.observed.loc_ordinal')='integer' THEN json_extract(l.data,'$.row.observed.normalized_fetch_url') END,json_extract(l.data,'$.row.observed.normalized_fetch_url'),json_extract(l.data,'$.row.admission.admitted_fetch_url'),d.task_url,d.row_id FROM sitemap_locations l JOIN sitemap_documents d ON d.row_id=l.observation_id WHERE json_extract(l.data,'$.row.observed.role')='page_location'")?;
        let mut rows = statement.query([])?;
        #[cfg(test)]
        change_work(|w| w.original_membership_selects += 1);
        let mut insert = tx.prepare("INSERT INTO members VALUES(?1,?2,?3,?4,?5)")?;
        while let Some(row) = rows.next()? {
            control.checkpoint()?;
            let values = [
                row.get_ref(0)?,
                row.get_ref(1)?,
                row.get_ref(2)?,
                row.get_ref(3)?,
                row.get_ref(4)?,
            ];
            charge.row(&values)?;
            if values
                .iter()
                .any(|v| !matches!(v, ValueRef::Text(_) | ValueRef::Null))
            {
                return Err(Declined::Unsupported.into());
            }
            insert.execute(params![
                borrowed(values[0]),
                borrowed(values[1]),
                borrowed(values[2]),
                borrowed(values[3]),
                borrowed(values[4])
            ])?;
            #[cfg(test)]
            change_work(|w| w.original_membership_rows += 1);
        }
    }
    control.checkpoint()?;
    let pages: i64 = tx.query_row("PRAGMA page_count", [], |r| r.get(0))?;
    let page_size: i64 = tx.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    let pages = usize::try_from(pages).map_err(|_| Declined::Budget)?;
    let page_size = usize::try_from(page_size).map_err(|_| Declined::Budget)?;
    ensure!(
        page_size == PAGE_SIZE,
        "Private request-index page size differs"
    );
    let db_bytes = pages.checked_mul(page_size).ok_or(Declined::Budget)?;
    let max_db_bytes = limits
        .db_pages
        .min(MAX_DB_PAGES)
        .checked_mul(PAGE_SIZE)
        .ok_or(Declined::Budget)?;
    if charge.bytes > limits.logical_bytes.min(MAX_LOGICAL_BYTES)
        || charge.rows > limits.rows.min(MAX_ROWS)
        || pages > limits.db_pages.min(MAX_DB_PAGES)
        || db_bytes > max_db_bytes
    {
        return Err(Declined::Budget.into());
    }
    tx.commit()?;
    indexed.pragma_update(None, "query_only", true)?;
    control.checkpoint()?;
    #[cfg(test)]
    change_work(|w| {
        w.committed_rows += charge.rows;
        w.committed_logical_bytes += charge.bytes;
        w.committed_utf8_bytes += charge.utf8;
        w.committed_db_pages += pages;
    });
    Ok(RequestIndex {
        connection: indexed,
        source_address: source as *const Connection as usize,
        source_total_changes: source.total_changes(),
    })
}

#[cfg(test)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Work {
    attempts: usize,
    committed: usize,
    fallback_budget: usize,
    fallback_unsupported: usize,
    fallback_full: usize,
    fallback_oom: usize,
    original_report_selects: usize,
    original_page_rows: usize,
    operation_rows: usize,
    private_anchor_rows: usize,
    original_membership_selects: usize,
    original_membership_rows: usize,
    indexed_queries: usize,
    committed_rows: usize,
    committed_logical_bytes: usize,
    committed_utf8_bytes: usize,
    committed_db_pages: usize,
}
#[cfg(test)]
thread_local! {static WORK:std::cell::Cell<Work>=std::cell::Cell::new(Work::default());}
#[cfg(test)]
fn change_work(change: impl FnOnce(&mut Work)) {
    WORK.with(|cell| {
        let mut work = cell.get();
        change(&mut work);
        cell.set(work);
    });
}
#[cfg(test)]
#[path = "request_index_tests.rs"]
mod tests;
