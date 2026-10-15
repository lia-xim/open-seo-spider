//! Offline URL-path hierarchy. Grouping nodes are never response observations.
use crate::{PageRecord, normalize_url};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{self, Write};
use url::Url;

pub const DIRECTORY_TREE_ENGINE: &str = "open-seo-spider/saved-directory-tree-v1";
pub const DIRECTORY_TREE_JSON_BYTES: usize = 8 * 1024 * 1024;
pub const DIRECTORY_TREE_CSV_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct DirectoryTreeLimits {
    pub max_pages: usize,
    pub max_nodes: usize,
    pub max_url_bytes: usize,
    pub max_path_segments: usize,
    /// Logical retained key/id/parent/label/address bytes, not a whole-RSS claim.
    pub max_retained_text_bytes: usize,
}
impl Default for DirectoryTreeLimits {
    fn default() -> Self {
        Self {
            max_pages: 100_000,
            max_nodes: 200_000,
            max_url_bytes: 16_384,
            max_path_segments: 128,
            max_retained_text_bytes: 32 * 1024 * 1024,
        }
    }
}
impl DirectoryTreeLimits {
    pub fn validate(&self) -> Result<()> {
        let c = Self::default();
        for (name, n, max) in [
            ("pages", self.max_pages, c.max_pages),
            ("nodes", self.max_nodes, c.max_nodes),
            ("URL bytes", self.max_url_bytes, c.max_url_bytes),
            ("path segments", self.max_path_segments, c.max_path_segments),
            (
                "retained text bytes",
                self.max_retained_text_bytes,
                c.max_retained_text_bytes,
            ),
        ] {
            ensure!(n > 0 && n <= max, "Invalid directory-tree {name} limit");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryNodeKind {
    Root,
    Protocol,
    Origin,
    Directory,
    ObservedUrl,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectoryCounts {
    pub observations: usize,
    pub unique_urls: usize,
    pub duplicate_observations: usize,
    pub conflicting_urls: usize,
    pub status_2xx: usize,
    pub status_3xx: usize,
    pub status_4xx: usize,
    pub status_5xx: usize,
    pub status_other: usize,
    pub status_unobserved: usize,
    pub stored_indexable: usize,
    pub stored_non_indexable: usize,
    pub indexability_unobserved: usize,
    pub errors: usize,
    pub robots_blocked: usize,
    pub internal: usize,
    pub external: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectoryNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: DirectoryNodeKind,
    pub label: String,
    /// A component or normalized requested URL. Group addresses are not fetched URLs.
    pub address: String,
    pub synthetic_group: bool,
    pub counts: DirectoryCounts,
    pub child_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectoryTree {
    pub engine: String,
    pub root_id: String,
    pub input_observations: usize,
    pub invalid_url_observations: usize,
    pub unsupported_url_observations: usize,
    pub credential_url_observations: usize,
    pub retained_text_bytes: usize,
    pub limits: DirectoryTreeLimits,
    pub nodes: Vec<DirectoryNode>,
    pub website_requests: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Observation {
    status: Option<u16>,
    indexable: Option<bool>,
    error: bool,
    blocked: bool,
}
struct WorkingNode {
    node: DirectoryNode,
    first: Option<Observation>,
    conflicting: bool,
}
fn identifier(kind: DirectoryNodeKind, address: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(DIRECTORY_TREE_ENGINE, kind, address))
                .expect("String serialization")
        )
    )
}
fn key(kind: DirectoryNodeKind, address: &str) -> String {
    format!("{kind:?}\0{address}")
}
fn add_node(
    nodes: &mut BTreeMap<String, WorkingNode>,
    retained: &mut usize,
    limits: &DirectoryTreeLimits,
    kind: DirectoryNodeKind,
    address: &str,
    label: &str,
    parent: Option<&str>,
) -> Result<String> {
    let k = key(kind, address);
    if let Some(row) = nodes.get(&k) {
        return Ok(row.node.id.clone());
    }
    ensure!(
        nodes.len() < limits.max_nodes,
        "Directory-tree node budget exceeded"
    );
    let id = identifier(kind, address);
    let cost = [
        k.len(),
        id.len(),
        parent.map_or(0, str::len),
        address.len(),
        label.len(),
    ]
    .into_iter()
    .try_fold(0usize, |a, b| a.checked_add(b))
    .ok_or_else(|| anyhow::anyhow!("Directory text count overflow"))?;
    let next = retained
        .checked_add(cost)
        .ok_or_else(|| anyhow::anyhow!("Directory text count overflow"))?;
    ensure!(
        next <= limits.max_retained_text_bytes,
        "Directory-tree retained text budget exceeded"
    );
    *retained = next;
    nodes.insert(
        k,
        WorkingNode {
            node: DirectoryNode {
                id: id.clone(),
                parent_id: parent.map(str::to_owned),
                kind,
                label: label.into(),
                address: address.into(),
                synthetic_group: kind != DirectoryNodeKind::ObservedUrl,
                counts: Default::default(),
                child_count: 0,
            },
            first: None,
            conflicting: false,
        },
    );
    Ok(id)
}
fn stored_indexability(page: &PageRecord, requested: &str) -> Option<bool> {
    // The stored bool describes the actual document. Incomplete or other-document DOMs stay unknown.
    if page.error.is_some()
        || page.blocked_by_robots
        || page.fetched_at.is_empty()
        || page.renderer_error.is_some()
        || page.truncated
        || (page.rendered && page.rendered_status != Some(200))
        || !matches!(page.status, Some(200..=299))
        || !matches!(page.final_status, Some(200..=299))
    {
        return None;
    }
    let final_url = normalize_url(&page.final_url, None)?;
    let document_url = normalize_url(page.document_url(), None)?;
    (final_url.as_str() == requested && document_url.as_str() == requested)
        .then_some(page.indexable)
}
fn count(
    c: &mut DirectoryCounts,
    page: &PageRecord,
    obs: &Observation,
    unique: bool,
    conflict: bool,
) {
    c.observations += 1;
    c.unique_urls += usize::from(unique);
    c.duplicate_observations += usize::from(!unique);
    c.conflicting_urls += usize::from(conflict);
    match obs.status {
        Some(200..=299) => c.status_2xx += 1,
        Some(300..=399) => c.status_3xx += 1,
        Some(400..=499) => c.status_4xx += 1,
        Some(500..=599) => c.status_5xx += 1,
        Some(100..=199 | 600..=999) => c.status_other += 1,
        _ => c.status_unobserved += 1,
    }
    match obs.indexable {
        Some(true) => c.stored_indexable += 1,
        Some(false) => c.stored_non_indexable += 1,
        None => c.indexability_unobserved += 1,
    }
    c.errors += usize::from(obs.error);
    c.robots_blocked += usize::from(obs.blocked);
    if page.internal {
        c.internal += 1;
    } else {
        c.external += 1;
    }
}
/// Caller supplies saved observations. No fetch, status synthesis or latest-row selection occurs.
pub fn build_directory_tree(
    pages: &[PageRecord],
    limits: &DirectoryTreeLimits,
) -> Result<DirectoryTree> {
    limits.validate()?;
    ensure!(
        pages.len() <= limits.max_pages,
        "Directory-tree page budget exceeded"
    );
    let mut nodes = BTreeMap::new();
    let mut retained = 0;
    let root_id = add_node(
        &mut nodes,
        &mut retained,
        limits,
        DirectoryNodeKind::Root,
        "",
        "Gespeicherte URL-Pfade",
        None,
    )?;
    let mut invalid = 0;
    let mut unsupported = 0;
    let mut credentials = 0;
    for page in pages {
        ensure!(
            page.url.len() <= limits.max_url_bytes
                && page.final_url.len() <= limits.max_url_bytes
                && page.document_url().len() <= limits.max_url_bytes,
            "Directory-tree URL byte budget exceeded"
        );
        let Ok(url) = Url::parse(page.url.trim()) else {
            invalid += 1;
            continue;
        };
        if !matches!(url.scheme(), "http" | "https") {
            unsupported += 1;
            continue;
        }
        if !url.username().is_empty() || url.password().is_some() {
            credentials += 1;
            continue;
        }
        let Some(url) = normalize_url(&page.url, None) else {
            invalid += 1;
            continue;
        };
        let canonical = url.to_string();
        ensure!(
            canonical.len() <= limits.max_url_bytes,
            "Normalized directory URL byte budget exceeded"
        );
        let origin = url.origin().ascii_serialization();
        let tail = url.path().strip_prefix('/').unwrap_or(url.path());
        let segments: Vec<_> = tail.split('/').collect();
        ensure!(
            segments.len() <= limits.max_path_segments,
            "Directory-tree path segment budget exceeded"
        );
        let protocol = add_node(
            &mut nodes,
            &mut retained,
            limits,
            DirectoryNodeKind::Protocol,
            url.scheme(),
            &format!("{}://", url.scheme()),
            Some(&root_id),
        )?;
        let host = origin.split_once("://").map_or(origin.as_str(), |(_, h)| h);
        let host_id = add_node(
            &mut nodes,
            &mut retained,
            limits,
            DirectoryNodeKind::Origin,
            &origin,
            host,
            Some(&protocol),
        )?;
        let mut chain = vec![
            key(DirectoryNodeKind::Root, ""),
            key(DirectoryNodeKind::Protocol, url.scheme()),
            key(DirectoryNodeKind::Origin, &origin),
        ];
        let mut parent = host_id;
        let mut prefix = String::from("/");
        // Keep empty and percent-encoded segments. Never decode %2F into a separator.
        for segment in segments.iter().take(segments.len().saturating_sub(1)) {
            prefix.push_str(segment);
            prefix.push('/');
            let address = format!("{origin}{prefix}");
            parent = add_node(
                &mut nodes,
                &mut retained,
                limits,
                DirectoryNodeKind::Directory,
                &address,
                if segment.is_empty() {
                    "(leeres Pfadsegment)"
                } else {
                    segment
                },
                Some(&parent),
            )?;
            chain.push(key(DirectoryNodeKind::Directory, &address));
        }
        let label = if segments.last().is_none_or(|s| s.is_empty()) {
            url.path().to_owned()
        } else {
            segments.last().unwrap().to_string()
        };
        let label = match url.query() {
            Some(q) => format!("{label}?{q}"),
            None => label,
        };
        add_node(
            &mut nodes,
            &mut retained,
            limits,
            DirectoryNodeKind::ObservedUrl,
            &canonical,
            &label,
            Some(&parent),
        )?;
        let leaf_key = key(DirectoryNodeKind::ObservedUrl, &canonical);
        let obs = Observation {
            status: page.status,
            indexable: stored_indexability(page, &canonical),
            error: page.error.is_some(),
            blocked: page.blocked_by_robots,
        };
        let leaf = nodes.get_mut(&leaf_key).expect("inserted node");
        let unique = leaf.first.is_none();
        let conflict = !leaf.conflicting && leaf.first.as_ref().is_some_and(|first| first != &obs);
        if unique {
            leaf.first = Some(obs.clone());
        }
        leaf.conflicting |= conflict;
        chain.push(leaf_key);
        for k in chain {
            count(
                &mut nodes.get_mut(&k).expect("constructed ancestor").node.counts,
                page,
                &obs,
                unique,
                conflict,
            );
        }
    }
    let parents: Vec<_> = nodes
        .values()
        .filter_map(|n| n.node.parent_id.clone())
        .collect();
    let index: BTreeMap<_, _> = nodes
        .iter()
        .map(|(k, n)| (n.node.id.clone(), k.clone()))
        .collect();
    for p in parents {
        nodes
            .get_mut(index.get(&p).expect("parent present"))
            .expect("parent key present")
            .node
            .child_count += 1;
    }
    Ok(DirectoryTree {
        engine: DIRECTORY_TREE_ENGINE.into(),
        root_id,
        input_observations: pages.len(),
        invalid_url_observations: invalid,
        unsupported_url_observations: unsupported,
        credential_url_observations: credentials,
        retained_text_bytes: retained,
        limits: limits.clone(),
        nodes: nodes.into_values().map(|n| n.node).collect(),
        website_requests: 0,
    })
}
/// Bounded serialization used by this workflow; no partial bytes escape on error.
pub fn directory_tree_json(value: &impl Serialize, max: usize) -> Result<Vec<u8>> {
    ensure!(
        max > 0 && max <= DIRECTORY_TREE_JSON_BYTES,
        "Invalid directory JSON byte limit"
    );
    let mut out = DirectoryBytes {
        bytes: Vec::new(),
        cap: max,
    };
    serde_json::to_writer(&mut out, value)?;
    Ok(out.bytes)
}
struct DirectoryBytes {
    bytes: Vec<u8>,
    cap: usize,
}
impl Write for DirectoryBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.cap.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("Directory export byte budget exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn safe_csv(value: &str) -> String {
    if value
        .trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
        .chars()
        .next()
        .is_some_and(|c| matches!(c, '=' | '+' | '-' | '@'))
    {
        format!("'{value}")
    } else {
        value.into()
    }
}
/// Every synthetic and observed node, with explicit row role and parent ID.
pub fn directory_tree_csv(tree: &DirectoryTree, max: usize) -> Result<Vec<u8>> {
    ensure!(
        max > 0 && max <= DIRECTORY_TREE_CSV_BYTES,
        "Invalid directory CSV byte limit"
    );
    let mut out = DirectoryBytes {
        bytes: Vec::new(),
        cap: max,
    };
    {
        let mut w = csv::WriterBuilder::new()
            .terminator(csv::Terminator::CRLF)
            .from_writer(&mut out);
        w.write_record([
            "kind",
            "synthetic_group",
            "id",
            "parent_id",
            "label",
            "address",
            "observations",
            "unique_urls",
            "duplicate_observations",
            "conflicting_urls",
            "status_2xx",
            "status_3xx",
            "status_4xx",
            "status_5xx",
            "status_other",
            "status_unobserved",
            "stored_indexable",
            "stored_non_indexable",
            "indexability_unobserved",
            "errors",
            "robots_blocked",
            "internal",
            "external",
        ])?;
        for n in &tree.nodes {
            let c = &n.counts;
            w.write_record([
                format!("{:?}", n.kind),
                n.synthetic_group.to_string(),
                n.id.clone(),
                n.parent_id.clone().unwrap_or_default(),
                safe_csv(&n.label),
                safe_csv(&n.address),
                c.observations.to_string(),
                c.unique_urls.to_string(),
                c.duplicate_observations.to_string(),
                c.conflicting_urls.to_string(),
                c.status_2xx.to_string(),
                c.status_3xx.to_string(),
                c.status_4xx.to_string(),
                c.status_5xx.to_string(),
                c.status_other.to_string(),
                c.status_unobserved.to_string(),
                c.stored_indexable.to_string(),
                c.stored_non_indexable.to_string(),
                c.indexability_unobserved.to_string(),
                c.errors.to_string(),
                c.robots_blocked.to_string(),
                c.internal.to_string(),
                c.external.to_string(),
            ])?;
        }
        w.flush()?;
    }
    Ok(out.bytes)
}

#[cfg(test)]
#[path = "directory_tree_tests.rs"]
mod tests;
