//! Connection-local Windows filesystem names; no process-wide VFS or OS change.
use anyhow::Context;
use rusqlite::{Connection, OpenFlags};
use std::path::{Component, Path, PathBuf, Prefix};

pub(super) fn open(path: &Path, flags: OpenFlags) -> anyhow::Result<Connection> {
    match filesystem_path(path)? {
        Some(path) => Ok(Connection::open_with_flags_and_vfs(
            path,
            flags,
            "win32-longpath",
        )?),
        None => Ok(Connection::open_with_flags(path, flags)?),
    }
}

/// `None` delegates names whose SQLite/Win32 interpretation must stay intact.
/// In particular, URI query parameters may select another VFS or memory mode.
/// Prefixing those strings would silently create an unrelated filesystem file.
fn filesystem_path(path: &Path) -> anyhow::Result<Option<PathBuf>> {
    if path.as_os_str().is_empty()
        || path
            .to_str()
            .is_some_and(|name| name == ":memory:" || name.starts_with("file:"))
    {
        return Ok(None);
    }
    if let Some(Component::Prefix(prefix)) = path.components().next() {
        match prefix.kind() {
            // Explicit verbatim names already express the caller's namespace;
            // retain every byte, but avoid the default VFS's 1040-byte ceiling.
            Prefix::VerbatimDisk(_) | Prefix::VerbatimUNC(_, _) => {
                return Ok(Some(path.to_owned()));
            }
            Prefix::DeviceNS(_) | Prefix::Verbatim(_) => return Ok(None),
            Prefix::UNC(server, share) if legacy_name(server) || legacy_name(share) => {
                return Ok(None);
            }
            Prefix::Disk(_) | Prefix::UNC(_, _) => (),
        }
    }
    // Extended syntax suppresses legacy Win32 name parsing. Do not turn a DOS
    // device, trailing-dot/space alias or alternate stream into a different file.
    if path
        .components()
        .any(|part| matches!(part, Component::Normal(name) if legacy_name(name)))
    {
        return Ok(None);
    }
    // This is GetFullPathNameW on Windows, matching SQLite's own full-path step.
    // Unlike canonicalize it works for new files and does not follow symlinks.
    let absolute = std::path::absolute(path).context("resolve Windows SQLite filesystem path")?;
    let mut parts = absolute.components();
    let Some(Component::Prefix(prefix)) = parts.next() else {
        anyhow::bail!("Windows SQLite absolute filesystem path has no prefix");
    };
    let mut extended = match prefix.kind() {
        Prefix::Disk(drive) => PathBuf::from(format!(r"\\?\{}:\", char::from(drive))),
        Prefix::UNC(server, share) => {
            // An incomplete UNC prefix is not a PathBuf directory: pushing a
            // server onto it can add a second separator. Assemble the prefix as
            // native string units before resuming ordinary component pushes.
            let mut name = std::ffi::OsString::from(r"\\?\UNC\");
            name.push(server);
            name.push(r"\");
            name.push(share);
            PathBuf::from(name)
        }
        // Preserve platform-resolved namespaces rather than guessing aliases.
        _ => return Ok(None),
    };
    for part in parts {
        match part {
            Component::RootDir => (),
            Component::Normal(name) => extended.push(name),
            _ => anyhow::bail!("Windows SQLite absolute filesystem path was not normalized"),
        }
    }
    if absolute
        .as_os_str()
        .to_str()
        .is_some_and(|name| name.ends_with(['\\', '/']))
    {
        // A trailing separator requires a directory. Never turn that old error
        // into creation of a database at the same name without the separator.
        extended.push("");
    }
    Ok(Some(extended))
}

fn legacy_name(name: &std::ffi::OsStr) -> bool {
    let Some(name) = name.to_str() else {
        // Rusqlite already refuses non-UTF8 Windows paths; leave that error intact.
        return true;
    };
    if name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*'))
    {
        return true;
    }
    let stem = name.split('.').next().unwrap_or("").trim_end_matches(' ');
    let upper = stem.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" | "CLOCK$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        upper.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    })
}

#[cfg(test)]
#[path = "windows_path_tests.rs"]
mod tests;
