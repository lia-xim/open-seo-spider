//! Own immutable local bytes. No URL/report path is filesystem authority.
use super::*;
use cap_fs_ext::{DirExt, FollowSymlinks, MetadataExt, OpenOptionsFollowExt, OpenOptionsSyncExt};
use cap_std::fs::{Dir, OpenOptions};
use std::{
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub(crate) fn archive_path(database: &Path) -> Result<PathBuf> {
    let absolute = if database.is_absolute() {
        database.to_owned()
    } else {
        std::env::current_dir()?.join(database)
    };
    let mut filename = absolute
        .file_name()
        .context("Archive database filename missing")?
        .to_os_string();
    filename.push(".sitemap-bodies");
    Ok(absolute.with_file_name(filename))
}
fn ordinary_root(path: &Path, create: bool) -> Result<Dir> {
    let mut anchor = PathBuf::new();
    let mut parts = Vec::new();
    for part in path.components() {
        match part {
            Component::Prefix(p) => anchor.push(p.as_os_str()),
            Component::RootDir => anchor.push(part.as_os_str()),
            Component::Normal(p) => parts.push(p.to_os_string()),
            Component::CurDir => (),
            Component::ParentDir => bail!("Archive traversal forbidden"),
        }
    }
    ensure!(
        path.is_absolute() && !parts.is_empty(),
        "Archive root must be absolute with a leaf"
    );
    let mut dir = Dir::open_ambient_dir(anchor, cap_std::ambient_authority())?;
    for (i, part) in parts.iter().enumerate() {
        if create && i + 1 == parts.len() {
            match dir.create_dir(part) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
                Err(e) => return Err(e.into()),
            }
        }
        dir = dir
            .open_dir_nofollow(part)
            .context("Archive root ancestor/leaf must be an ordinary nofollow directory")?;
    }
    Ok(dir)
}
fn regular(dir: &Dir, name: &str) -> Result<cap_std::fs::File> {
    let mut o = OpenOptions::new();
    o.read(true).follow(FollowSymlinks::No).nonblock(true);
    let file = dir.open_with(name, &o)?;
    ensure!(
        file.metadata()?.is_file(),
        "Archive object must be a regular nofollow file"
    );
    Ok(file)
}
fn identity(file: &cap_std::fs::File) -> Result<(u64, u64)> {
    let m = file.metadata()?;
    Ok((m.dev(), m.ino()))
}
fn verify(dir: &Dir, name: &str, expected: &SitemapArchivedBody) -> Result<()> {
    let mut file = regular(dir, name)?;
    ensure!(
        file.metadata()?.len() == expected.bytes,
        "Archive object length differs"
    );
    let before = identity(&file)?;
    let mut hash = Sha256::new();
    let mut count = 0u64;
    let mut buffer = [0u8; 65_536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        count = count
            .checked_add(n as u64)
            .context("Archive read length overflow")?;
        ensure!(
            count <= expected.bytes && count <= MAX_SITEMAP_ARCHIVE_BODY_BYTES,
            "Archive object grew beyond declared cap"
        );
        hash.update(&buffer[..n]);
    }
    ensure!(
        count == expected.bytes && format!("{:x}", hash.finalize()) == expected.sha256,
        "Archive object content/EOF differs"
    );
    ensure!(
        before == identity(&file)? && file.metadata()?.len() == count,
        "Archive object identity/size changed while reading"
    );
    Ok(())
}
fn name(id: &str) -> Result<String> {
    valid_sha(id)?;
    Ok(format!("{id}.bin"))
}
struct OwnedName {
    name: String,
    identity: (u64, u64),
}
pub(crate) struct PublishedBodies {
    dir: Dir,
    created: Vec<OwnedName>,
    retained: bool,
}
impl PublishedBodies {
    pub(crate) fn retain(&mut self) {
        self.retained = true;
    }
    pub(crate) fn cleanup(&mut self) -> Result<()> {
        if self.retained {
            return Ok(());
        }
        let mut failures = Vec::new();
        for entry in self.created.drain(..).rev() {
            match regular(&self.dir, &entry.name) {
                Ok(file) => match identity(&file) {
                    Ok(id) if id == entry.identity => {
                        drop(file);
                        if let Err(e) = self.dir.remove_file(&entry.name) {
                            failures.push(e.to_string())
                        }
                    }
                    _ => failures.push(format!(
                        "Owned archive cleanup identity uncertain; retained {}",
                        entry.name
                    )),
                },
                Err(e) => failures.push(format!(
                    "Owned archive cleanup refused; retained {}: {e}",
                    entry.name
                )),
            }
        }
        ensure!(failures.is_empty(), "{}", failures.join("; "));
        Ok(())
    }
}
impl Drop for PublishedBodies {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
static NEXT: AtomicU64 = AtomicU64::new(1);
pub(crate) struct ArchiveWriter {
    dir: Dir,
    lock: Option<cap_std::fs::File>,
    identity: (u64, u64),
    bytes: Vec<u8>,
    released: bool,
}
impl ArchiveWriter {
    pub(crate) fn acquire(path: &Path) -> Result<Self> {
        let dir = ordinary_root(path, true)?;
        let mut o = OpenOptions::new();
        o.write(true)
            .create_new(true)
            .follow(FollowSymlinks::No)
            .nonblock(true);
        let mut file = dir
            .open_with(".__oss_body_active", &o)
            .context("Archive already has a writer/retained crash lease; no automatic takeover")?;
        let id = identity(&file)?;
        let bytes = format!(
            "{}:{}\n",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )
        .into_bytes();
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(Self {
            dir,
            lock: Some(file),
            identity: id,
            bytes,
            released: false,
        })
    }
    pub(crate) fn release(&mut self) -> Result<()> {
        if self.released {
            return Ok(());
        }
        let mut current = regular(&self.dir, ".__oss_body_active")?;
        ensure!(
            identity(&current)? == self.identity,
            "Archive writer lease identity changed; retain"
        );
        let mut bytes = Vec::new();
        Read::by_ref(&mut current)
            .take(128)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes == self.bytes,
            "Archive writer lease bytes changed; retain"
        );
        drop(current);
        drop(self.lock.take());
        self.dir.remove_file(".__oss_body_active")?;
        self.released = true;
        Ok(())
    }
    pub(crate) fn publish(
        &self,
        bodies: &[(SitemapArchivedBody, &[u8])],
        config: &SitemapSourceArchiveConfig,
    ) -> Result<PublishedBodies> {
        config.validate()?;
        ensure!(
            bodies
                .iter()
                .all(|(b, data)| b.bytes <= config.max_bytes_per_body
                    && b.bytes <= MAX_SITEMAP_ARCHIVE_BODY_BYTES
                    && b.bytes == data.len() as u64),
            "Archive binary publication exceeds explicit body cap"
        );
        // Uncommitted published orphans consume real quota too. No generic GC.
        let mut physical = 0u64;
        let mut count = 0u64;
        for entry in self.dir.entries()? {
            let entry = entry?;
            let filename = entry.file_name();
            let s = filename
                .to_str()
                .context("Unexpected non-UTF8 archive entry")?;
            if s == ".__oss_body_active" {
                continue;
            }
            let id = s.strip_suffix(".bin").context(
                "Unverified archive staging/foreign entry retained; offline recovery required",
            )?;
            valid_sha(id)?;
            count = count
                .checked_add(1)
                .context("Archive object count overflow")?;
            ensure!(
                count <= config.max_capture_records * 2,
                "Archive physical object count exceeds configured bound"
            );
            let f = regular(&self.dir, s)?;
            let n = f.metadata()?.len();
            ensure!(
                n <= MAX_SITEMAP_ARCHIVE_BODY_BYTES,
                "Archive physical object exceeds body hard cap"
            );
            physical = physical
                .checked_add(n)
                .context("Archive physical bytes overflow")?;
        }
        for (body, _) in bodies {
            let n = name(&body.body_id)?;
            match self.dir.symlink_metadata(n) {
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    physical = physical
                        .checked_add(body.bytes)
                        .context("Archive physical quota overflow")?;
                    count = count
                        .checked_add(1)
                        .context("Archive physical count overflow")?;
                    ensure!(
                        count <= config.max_capture_records * 2,
                        "Archive physical/orphan object count exhausts quota"
                    );
                }
                Err(e) => return Err(e.into()),
            }
        }
        ensure!(
            physical <= config.max_total_body_bytes,
            "Published/orphan archive payload exhausts disk quota; offline recovery required"
        );
        publish(&self.dir, bodies)
    }
}
impl Drop for ArchiveWriter {
    fn drop(&mut self) {
        let _ = self.release();
    }
}
fn publish(root: &Dir, bodies: &[(SitemapArchivedBody, &[u8])]) -> Result<PublishedBodies> {
    let dir = root.try_clone()?;
    let mut batch = PublishedBodies {
        dir,
        created: Vec::new(),
        retained: false,
    };
    let result = (|| -> Result<()> {
        for (body, bytes) in bodies {
            ensure!(
                bytes.len() as u64 == body.bytes && digest(bytes) == body.sha256,
                "Archive candidate content differs"
            );
            let final_name = name(&body.body_id)?;
            match batch.dir.symlink_metadata(&final_name) {
                Ok(_) => {
                    verify(&batch.dir, &final_name, body)?;
                    continue;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.into()),
            }
            let mut staged = None;
            for _ in 0..16 {
                let temporary = format!(
                    ".__oss_body_{}_{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                );
                let mut o = OpenOptions::new();
                o.write(true)
                    .create_new(true)
                    .follow(FollowSymlinks::No)
                    .nonblock(true);
                match batch.dir.open_with(&temporary, &o) {
                    Ok(file) => {
                        staged = Some((temporary, file));
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
                    Err(e) => return Err(e.into()),
                }
            }
            let (temporary, mut file) =
                staged.context("Could not reserve own archive staging file")?;
            let temp_id = identity(&file)?;
            batch.created.push(OwnedName {
                name: temporary.clone(),
                identity: temp_id,
            });
            for chunk in bytes.chunks(65_536) {
                file.write_all(chunk)?;
            }
            file.sync_all()?;
            drop(file);
            verify(&batch.dir, &temporary, body)?;
            let check = regular(&batch.dir, &temporary)?;
            ensure!(
                identity(&check)? == temp_id,
                "Archive staging identity changed"
            );
            drop(check);
            // New-name hard link: no rename fallback or overwrite of another entry.
            batch.dir.hard_link(&temporary, &batch.dir, &final_name)?;
            let published = regular(&batch.dir, &final_name)?;
            ensure!(
                identity(&published)? == temp_id,
                "Archive publication identity changed"
            );
            drop(published);
            batch.created.push(OwnedName {
                name: final_name.clone(),
                identity: temp_id,
            });
            verify(&batch.dir, &final_name, body)?;
            let check = regular(&batch.dir, &temporary)?;
            ensure!(
                identity(&check)? == temp_id,
                "Archive temp identity changed before unlink"
            );
            drop(check);
            batch.dir.remove_file(&temporary)?;
            batch.created.retain(|e| e.name != temporary);
        }
        Ok(())
    })();
    if let Err(error) = result {
        return match batch.cleanup() {
            Ok(()) => Err(error),
            Err(cleanup) => Err(error).context(format!("Archive cleanup incomplete: {cleanup:#}")),
        };
    }
    Ok(batch)
}
pub(crate) fn verify_snapshot(path: &Path, snapshot: &SitemapSourceArchiveSnapshot) -> Result<()> {
    if snapshot.captures.iter().all(|c| c.bodies.is_empty()) {
        return Ok(());
    }
    let dir = ordinary_root(path, false)?;
    for capture in &snapshot.captures {
        for body in &capture.bodies {
            verify(&dir, &name(&body.body_id)?, body)?
        }
    }
    Ok(())
}

/// Selected immutable object -> unadvertised private sink. No path escapes.
pub(crate) fn copy_verified(
    path: &Path,
    expected: &SitemapArchivedBody,
    sink: &mut (impl Write + ?Sized),
    checkpoint: &mut impl FnMut() -> Result<()>,
) -> Result<()> {
    valid_sha(&expected.body_id)?;
    valid_sha(&expected.sha256)?;
    ensure!(
        expected.bytes <= MAX_SITEMAP_ARCHIVE_BODY_BYTES,
        "Source body exceeds independent64MiB cap"
    );
    let dir = ordinary_root(path, false)?;
    let mut file = regular(&dir, &format!("{}.bin", expected.body_id))?;
    let before = identity(&file)?;
    ensure!(
        file.metadata()?.len() == expected.bytes,
        "Source body object length differs"
    );
    let mut hash = Sha256::new();
    let mut count = 0u64;
    let mut buffer = [0u8; 65_536];
    loop {
        checkpoint()?;
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        count = count
            .checked_add(n as u64)
            .context("Source body copy length overflow")?;
        ensure!(
            count <= expected.bytes && count <= MAX_SITEMAP_ARCHIVE_BODY_BYTES,
            "Source body grew beyond declared cap"
        );
        hash.update(&buffer[..n]);
        sink.write_all(&buffer[..n])?;
    }
    ensure!(
        count == expected.bytes && format!("{:x}", hash.finalize()) == expected.sha256,
        "Source body content/EOF differs"
    );
    ensure!(
        before == identity(&file)? && file.metadata()?.len() == count,
        "Source body identity changed while copying"
    );
    checkpoint()?;
    Ok(())
}
