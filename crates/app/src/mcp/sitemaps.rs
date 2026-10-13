//! Additional typed query argument validation before API I/O.
use crate::sitemap_membership::{ReadQuery, RowKind};
use anyhow::{Context, Result, bail};
use serde_json::Value;
pub(super) fn query(args: &Value) -> Result<ReadQuery> {
    let text = |key: &str| -> Result<Option<String>> {
        args.get(key)
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .with_context(|| format!("{key} must be a string"))
            })
            .transpose()
    };
    let kind = match text("kind")?.as_deref().unwrap_or("all") {
        "all" => RowKind::All,
        "root" => RowKind::Root,
        "document" => RowKind::Document,
        "entry" => RowKind::Entry,
        "location" => RowKind::Location,
        _ => bail!("Invalid sitemap kind"),
    };
    let limit = args
        .get("limit")
        .map(|value| {
            value
                .as_u64()
                .context("limit must be an integer")
                .and_then(|value| usize::try_from(value).context("limit integer overflow"))
        })
        .transpose()?;
    let query = ReadQuery {
        kind,
        source_observation_id: text("source_observation_id")?,
        membership_url: text("membership_url")?,
        limit,
        cursor: text("cursor")?,
    };
    query
        .validate()
        .map_err(|error| anyhow::anyhow!("{}: {}", error.code, error.error))?;
    Ok(query)
}
