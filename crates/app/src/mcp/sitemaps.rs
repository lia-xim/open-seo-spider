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
    let integer = |key: &str| -> Result<Option<usize>> {
        args.get(key)
            .map(|value| {
                value
                    .as_u64()
                    .with_context(|| format!("{key} must be an integer"))
                    .and_then(|value| {
                        usize::try_from(value).with_context(|| format!("{key} integer overflow"))
                    })
            })
            .transpose()
    };
    let query = ReadQuery {
        kind,
        source_observation_id: text("source_observation_id")?,
        membership_url: text("membership_url")?,
        limit: integer("limit")?,
        cursor: text("cursor")?,
        page_limit: integer("page_limit")?,
        page_cursor: text("page_cursor")?,
    };
    query
        .validate()
        .map_err(|error| anyhow::anyhow!("{}: {}", error.code, error.error))?;
    Ok(query)
}
pub(super) fn query_pairs(query: ReadQuery) -> Vec<(String, String)> {
    let mut values = vec![
        ("kind".into(), query.kind.name().into()),
        ("limit".into(), query.limit.unwrap_or(100).to_string()),
        (
            "page_limit".into(),
            query.page_limit.unwrap_or(100).to_string(),
        ),
    ];
    for (key, value) in [
        ("source_observation_id", query.source_observation_id),
        ("membership_url", query.membership_url),
        ("cursor", query.cursor),
        ("page_cursor", query.page_cursor),
    ] {
        if let Some(value) = value {
            values.push((key.into(), value));
        }
    }
    values
}
