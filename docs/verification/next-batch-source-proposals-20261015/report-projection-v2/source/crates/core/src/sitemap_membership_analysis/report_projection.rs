//! Request-local optimization of two report guards, never a snapshot proof.
//! Original report SHA/checkpoint/JSON/source checks precede this private call.
//! The full catalog was independently verified; these two guards never read its
//! embedded report copy. All other report fields retain SQLite JSON semantics.
use super::*;

// Optional retained projection only. This does not reduce the old 256MiB input
// cap: unsupported/large forms use the unchanged original queries below.
const COMPACT_REPORT_BYTES: usize = 2 * 1024 * 1024;
const IDENTITY_SQL: &str = "SELECT count(DISTINCT json_extract(p.value,'$.url')),coalesce(sum(CASE WHEN json_type(p.value,'$.url')='text' AND json_type(p.value,'$.links')='array' THEN 0 ELSE 1 END),0),coalesce(sum(json_array_length(p.value,'$.links')),0) FROM metadata m,json_each(m.value,'$.pages') p WHERE m.key='report'";
const COMPACT_IDENTITY_SQL: &str = "SELECT count(DISTINCT json_extract(p.value,'$.url')),coalesce(sum(CASE WHEN json_type(p.value,'$.url')='text' AND json_type(p.value,'$.links')='array' THEN 0 ELSE 1 END),0),coalesce(sum(json_array_length(p.value,'$.links')),0) FROM (SELECT ?1 AS value) m,json_each(m.value,'$.pages') p";
const SELECTED_SQL: &str = "SELECT json_type(value,'$.operation')='object' FROM metadata WHERE key='report'";
const COMPACT_SELECTED_SQL: &str = "SELECT json_type(?1,'$.operation')='object'";

pub(super) fn compact_report(
    connection: &Connection,
    control: &SitemapReadControl,
) -> Result<Option<String>> {
    compact_with_limit(connection, control, COMPACT_REPORT_BYTES)
}
fn compact_with_limit(
    connection: &Connection,
    control: &SitemapReadControl,
    limit: usize,
) -> Result<Option<String>> {
    control.checkpoint()?;
    // The historical materialized reader can be called without a retained TX.
    // Never create a reusable optimization value from that unsupported scope.
    if connection.is_autocommit() {
        return Ok(None);
    }
    let mut statement = connection.prepare(
        "SELECT CASE WHEN typeof(value)='text' AND json_type(value,'$.sitemap_catalog')='object' THEN json_remove(value,'$.sitemap_catalog') END FROM metadata WHERE key='report'",
    )?;
    let mut rows = statement.query([])?;
    let Some(row) = rows.next()? else { return Ok(None); };
    #[cfg(test)]
    change_work(|work| work.projection_selects += 1);
    let value = row.get_ref(0)?;
    let rusqlite::types::ValueRef::Text(bytes) = value else { return Ok(None); };
    if bytes.len() > limit.min(COMPACT_REPORT_BYTES) {
        return Ok(None);
    }
    // A Rust UTF8 projection is optional: the old SQL guards may accept TEXT
    // containing uninterpreted non-UTF8 bytes in irrelevant fields. Preserve
    // those historical semantics through the original-query fallback. SQL,
    // row/type and cancellation failures above are still real failures.
    let Ok(text) = std::str::from_utf8(bytes) else { return Ok(None); };
    control.checkpoint()?;
    #[cfg(test)]
    change_work(|work| { work.accepted_projections += 1; work.retained_projection_bytes += text.len(); });
    Ok(Some(text.to_owned()))
}
pub(super) fn identity_counts(
    connection: &Connection,
    compact: Option<&str>,
) -> Result<(i64, i64, i64)> {
    let get = |row: &rusqlite::Row<'_>| -> rusqlite::Result<(i64, i64, i64)> { Ok((row.get(0)?, row.get(1)?, row.get(2)?)) };
    let value = match compact {
        Some(text) => connection.query_row(COMPACT_IDENTITY_SQL, [text], get)?,
        None => connection.query_row(IDENTITY_SQL, [], get)?,
    };
    #[cfg(test)]
    change_work(|work| if compact.is_some() { work.compact_guard_queries += 1; } else { work.original_guard_queries += 1; });
    Ok(value)
}
pub(super) fn selected(connection: &Connection, compact: Option<&str>) -> Result<bool> {
    let get = |row: &rusqlite::Row<'_>| row.get::<_, Option<bool>>(0);
    let value = match compact {
        Some(text) => connection.query_row(COMPACT_SELECTED_SQL, [text], get)?,
        None => connection.query_row(SELECTED_SQL, [], get)?,
    };
    #[cfg(test)]
    change_work(|work| if compact.is_some() { work.compact_guard_queries += 1; } else { work.original_guard_queries += 1; });
    Ok(value.unwrap_or(false))
}

// Counters observe actual successful hash/SQL calls, not guessed JSON parser
// internals, elapsed time or bytes SQLite might read through its own cache.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Work {
    original_report_hash_passes: usize,
    original_report_hashed_bytes: usize,
    projection_selects: usize,
    accepted_projections: usize,
    retained_projection_bytes: usize,
    compact_guard_queries: usize,
    original_guard_queries: usize,
}
#[cfg(test)]
thread_local! { static WORK: std::cell::Cell<Work> = std::cell::Cell::new(Work::default()); }
#[cfg(test)]
fn change_work(change: impl FnOnce(&mut Work)) {
    WORK.with(|cell| { let mut value = cell.get(); change(&mut value); cell.set(value); });
}
#[cfg(test)]
pub(super) fn record_original_hash(bytes: usize) {
    change_work(|work| { work.original_report_hash_passes += 1; work.original_report_hashed_bytes += bytes; });
}
#[cfg(test)]
#[path = "report_projection_tests.rs"]
mod tests;
