//! Independently authored offline discovery policy. No HTTP, file or crawl mutations.
//! One regex search per ordered rewrite rule; observed identities are never overwritten.
use anyhow::{Context, Result, bail, ensure};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use url::Url;

pub const CRAWL_POLICY_ENGINE: &str = "open-seo-spider/crawl-policy-v1";
pub const CRAWL_REPLACEMENT_PROFILE: &str = "rust-first-match-literal-or-braced-captures-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CrawlPolicyLimits {
    pub max_inputs: usize,
    pub max_input_bytes: usize,
    pub max_url_bytes: usize,
    pub max_roots: usize,
    pub max_scope_paths: usize,
    pub max_query_keys: usize,
    pub max_query_tokens: usize,
    pub max_rewrite_rules: usize,
    pub max_filters: usize,
    pub max_pattern_bytes: usize,
    pub max_total_pattern_bytes: usize,
    pub max_replacement_bytes: usize,
    pub max_capture_groups: usize,
    pub max_trace_steps: usize,
    pub max_output_bytes: usize,
    pub regex_size_limit: usize,
    pub dfa_size_limit: usize,
}
impl Default for CrawlPolicyLimits {
    fn default() -> Self {
        Self {
            max_inputs: 10_000,
            max_input_bytes: 8 * 1024 * 1024,
            max_url_bytes: 16_384,
            max_roots: 32,
            max_scope_paths: 128,
            max_query_keys: 128,
            max_query_tokens: 1024,
            max_rewrite_rules: 64,
            max_filters: 128,
            max_pattern_bytes: 8192,
            max_total_pattern_bytes: 65_536,
            max_replacement_bytes: 4096,
            max_capture_groups: 32,
            max_trace_steps: 256,
            max_output_bytes: 16 * 1024 * 1024,
            regex_size_limit: 1024 * 1024,
            dfa_size_limit: 1024 * 1024,
        }
    }
}
impl CrawlPolicyLimits {
    pub fn validate(&self) -> Result<()> {
        let actual = serde_json::to_value(self)?;
        let ceilings = serde_json::to_value(Self::default())?;
        for (name, value) in actual.as_object().context("limit object")? {
            let value = value.as_u64().context("numeric limit")?;
            ensure!(
                value > 0 && value <= ceilings[name].as_u64().context("limit ceiling")?,
                "{name} must be positive and within its hard ceiling"
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutsideScopePolicy {
    #[default]
    Deny,
    CheckOnce,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrawlScopeRoot {
    pub origin: String,
    #[serde(default)]
    pub subdomains: bool,
    pub folders: Vec<String>,
    #[serde(default)]
    pub exact_paths: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CrawlScopeConfig {
    pub roots: Vec<CrawlScopeRoot>,
    pub outside_folder: OutsideScopePolicy,
    pub external: OutsideScopePolicy,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryKeyMatch {
    EncodedBytes,
    #[default]
    PercentDecodedUtf8,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryOrder {
    #[default]
    Preserve,
    StableEncodedKey,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct QueryPolicy {
    pub remove_keys: Vec<String>,
    pub key_match: QueryKeyMatch,
    pub order: QueryOrder,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RewriteReplacement {
    Literal { text: String },
    Captures { template: String },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewriteRule {
    pub id: String,
    pub pattern: String,
    pub replacement: RewriteReplacement,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterIdentity {
    #[default]
    Fetch,
    Discovered,
    Both,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InitialRewritePolicy {
    #[default]
    Preserve,
    Apply,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InitialFilterPolicy {
    #[default]
    Exempt,
    Apply,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InitialInputPolicy {
    pub rewrite: InitialRewritePolicy,
    pub filters: InitialFilterPolicy,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CrawlPolicyConfig {
    pub engine: String,
    pub scope: CrawlScopeConfig,
    pub query: QueryPolicy,
    pub rewrites: Vec<RewriteRule>,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub filter_identity: FilterIdentity,
    pub initial_inputs: InitialInputPolicy,
    pub limits: CrawlPolicyLimits,
}
impl Default for CrawlPolicyConfig {
    fn default() -> Self {
        Self {
            engine: CRAWL_POLICY_ENGINE.into(),
            scope: CrawlScopeConfig::default(),
            query: QueryPolicy::default(),
            rewrites: vec![],
            include_patterns: vec![],
            exclude_patterns: vec![],
            filter_identity: FilterIdentity::default(),
            initial_inputs: InitialInputPolicy::default(),
            limits: CrawlPolicyLimits::default(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateSourceKind {
    Seed,
    ListPage,
    SitemapDocument,
    SitemapLocation,
    Anchor,
    Resource,
    Relationship,
    PdfAnnotation,
    Redirect,
    SavedFetch,
}
impl CandidateSourceKind {
    fn initial(self) -> bool {
        matches!(self, Self::Seed | Self::ListPage)
    }
    fn transformable(self) -> bool {
        !matches!(
            self,
            Self::Redirect | Self::SavedFetch | Self::SitemapDocument
        )
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateSourceLocation {
    pub document_source: String,
    #[serde(default)]
    pub dom_path: Option<String>,
    #[serde(default)]
    pub source_index: Option<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateInput {
    pub occurrence_id: String,
    pub source_kind: CandidateSourceKind,
    pub raw_value: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub source_location: Option<CandidateSourceLocation>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlPolicyDecisionKind {
    FetchAndTraverse,
    CheckOnly,
    Deny,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlPolicyStepKind {
    QueryRemoval,
    QueryOrder,
    Rewrite,
    Filters,
    Scope,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrawlPolicyStep {
    pub kind: CrawlPolicyStepKind,
    pub rule_id: Option<String>,
    pub matched: Option<bool>,
    pub changed: bool,
    pub before: Option<String>,
    pub after: Option<String>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateDecision {
    pub occurrence_id: String,
    pub source_kind: CandidateSourceKind,
    pub source_url: Option<String>,
    pub source_location: Option<CandidateSourceLocation>,
    pub raw_value: String,
    pub resolved_discovered_url: Option<String>,
    pub normalized_discovered_url: Option<String>,
    pub fetch_candidate_url: Option<String>,
    pub fragment: Option<String>,
    pub policy_steps: Vec<CrawlPolicyStep>,
    pub decision: CrawlPolicyDecisionKind,
    pub reason_code: Option<String>,
    pub error: Option<String>,
    pub matched_rule_ids: Vec<String>,
    pub host_internal: bool,
    pub folder_internal: bool,
    pub traverse: bool,
    pub coalesced_into: Option<String>,
    pub policy_engine: String,
    pub policy_sha256: String,
    pub warnings: Vec<String>,
    pub policy_complete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrawlPolicyPreviewInput {
    pub config: CrawlPolicyConfig,
    pub inputs: Vec<CandidateInput>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrawlPolicyPreview {
    pub engine: String,
    pub replacement_profile: String,
    pub policy_sha256: String,
    pub config: CrawlPolicyConfig,
    pub decisions: Vec<CandidateDecision>,
    pub website_requests: usize,
    pub limitations: Vec<String>,
}

enum TemplatePart {
    Text(String),
    Group(usize),
}
struct CompiledRule {
    id: String,
    regex: Regex,
    replacement: Vec<TemplatePart>,
}
struct CompiledRoot {
    url: Url,
    subdomains: bool,
    folders: Vec<String>,
    exact_paths: Vec<String>,
}
pub struct CompiledCrawlPolicy {
    config: CrawlPolicyConfig,
    hash: String,
    roots: Vec<CompiledRoot>,
    rewrites: Vec<CompiledRule>,
    includes: Vec<Regex>,
    excludes: Vec<Regex>,
}
impl CompiledCrawlPolicy {
    pub fn new(config: &CrawlPolicyConfig) -> Result<Self> {
        config.limits.validate()?;
        ensure!(
            config.engine == CRAWL_POLICY_ENGINE,
            "unsupported crawl policy engine"
        );
        let limits = &config.limits;
        ensure!(
            !config.scope.roots.is_empty() && config.scope.roots.len() <= limits.max_roots,
            "scope requires 1..=max_roots explicit roots"
        );
        ensure!(
            config.query.remove_keys.len() <= limits.max_query_keys,
            "too many removal keys"
        );
        ensure!(
            config.rewrites.len() <= limits.max_rewrite_rules,
            "too many rewrite rules"
        );
        ensure!(
            config.include_patterns.len() + config.exclude_patterns.len() <= limits.max_filters,
            "too many URL filters"
        );
        ensure_serialized_budget(config, limits.max_input_bytes)?;
        let mut effective = config.clone();
        let mut roots = Vec::new();
        let mut root_ids = BTreeSet::new();
        let mut path_count = 0usize;
        for root in &mut effective.scope.roots {
            let parsed = safe_url(&root.origin, None, limits.max_url_bytes)?;
            ensure!(
                !root.subdomains || matches!(parsed.host(), Some(url::Host::Domain(_))),
                "subdomain expansion requires an explicit domain host, not an IP address"
            );
            ensure!(
                parsed.path() == "/" && parsed.query().is_none() && parsed.fragment().is_none(),
                "scope origin must have no non-root path, query or fragment"
            );
            root.origin = parsed.origin().ascii_serialization();
            ensure!(
                root.folders.len() + root.exact_paths.len() > 0,
                "scope root requires folder or exact path"
            );
            path_count = path_count
                .checked_add(root.folders.len() + root.exact_paths.len())
                .context("path count overflow")?;
            ensure!(path_count <= limits.max_scope_paths, "too many scope paths");
            let mut unique = BTreeSet::new();
            for (path, folder) in root
                .folders
                .iter()
                .map(|p| (p, true))
                .chain(root.exact_paths.iter().map(|p| (p, false)))
            {
                ensure!(
                    path.starts_with('/') && (!folder || path.ends_with('/')),
                    "folder prefixes must begin and end with slash"
                );
                ensure!(
                    !path.contains(['?', '#', '\\']) && valid_percent(path),
                    "ambiguous encoded scope path"
                );
                let url = safe_url(path, Some(&parsed), limits.max_url_bytes)?;
                ensure!(
                    url.path() == path && url.origin() == parsed.origin(),
                    "scope path must be explicit encoded normalized path"
                );
                ensure!(unique.insert(path), "duplicate scope path");
            }
            ensure!(
                root_ids.insert(serde_json::to_string(root)?),
                "duplicate scope root"
            );
            roots.push(CompiledRoot {
                url: parsed,
                subdomains: root.subdomains,
                folders: root.folders.clone(),
                exact_paths: root.exact_paths.clone(),
            });
        }
        let mut keys = BTreeSet::new();
        for key in &config.query.remove_keys {
            ensure!(
                !key.is_empty() && key.len() <= 512 && !key.chars().any(char::is_control),
                "query removal key requires 1..=512 safe bytes"
            );
            ensure!(keys.insert(key), "duplicate query removal key");
        }
        let mut total = 0usize;
        let mut compile = |pattern: &str| -> Result<Regex> {
            ensure!(
                !pattern.is_empty() && pattern.len() <= limits.max_pattern_bytes,
                "regex requires 1..=max_pattern_bytes"
            );
            total = total
                .checked_add(pattern.len())
                .context("pattern size overflow")?;
            ensure!(
                total <= limits.max_total_pattern_bytes,
                "total patterns exceed byte limit"
            );
            let regex = RegexBuilder::new(pattern)
                .size_limit(limits.regex_size_limit)
                .dfa_size_limit(limits.dfa_size_limit)
                .build()
                .context("invalid/over-budget Rust regex")?;
            ensure!(
                regex.captures_len().saturating_sub(1) <= limits.max_capture_groups,
                "too many capture groups"
            );
            Ok(regex)
        };
        let mut ids = BTreeSet::new();
        let mut rewrites = Vec::new();
        for rule in &config.rewrites {
            ensure!(
                !rule.id.is_empty()
                    && rule.id.len() <= 64
                    && rule
                        .id
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')),
                "rule id requires 1..=64 ASCII token bytes"
            );
            ensure!(ids.insert(&rule.id), "duplicate rewrite id");
            let regex = compile(&rule.pattern)?;
            let replacement = compile_replacement(&rule.replacement, &regex, limits)?;
            rewrites.push(CompiledRule {
                id: rule.id.clone(),
                regex,
                replacement,
            });
        }
        let includes = config
            .include_patterns
            .iter()
            .map(|p| compile(p))
            .collect::<Result<_>>()?;
        let excludes = config
            .exclude_patterns
            .iter()
            .map(|p| compile(p))
            .collect::<Result<_>>()?;
        ensure_serialized_budget(&effective, limits.max_input_bytes)?;
        let hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&effective)?));
        Ok(Self {
            config: effective,
            hash,
            roots,
            rewrites,
            includes,
            excludes,
        })
    }
    pub fn config(&self) -> &CrawlPolicyConfig {
        &self.config
    }
    pub fn policy_sha256(&self) -> &str {
        &self.hash
    }
    pub fn evaluate(&self, input: &CandidateInput) -> CandidateDecision {
        let mut out = CandidateDecision {
            occurrence_id: input.occurrence_id.clone(),
            source_kind: input.source_kind,
            source_url: input.source_url.clone(),
            source_location: input.source_location.clone(),
            raw_value: input.raw_value.clone(),
            resolved_discovered_url: None,
            normalized_discovered_url: None,
            fetch_candidate_url: None,
            fragment: None,
            policy_steps: vec![],
            decision: CrawlPolicyDecisionKind::Deny,
            reason_code: None,
            error: None,
            matched_rule_ids: vec![],
            host_internal: false,
            folder_internal: false,
            traverse: false,
            coalesced_into: None,
            policy_engine: CRAWL_POLICY_ENGINE.into(),
            policy_sha256: self.hash.clone(),
            warnings: vec![],
            policy_complete: true,
        };
        if let Err(error) = self.evaluate_into(input, &mut out) {
            out.decision = CrawlPolicyDecisionKind::Deny;
            out.traverse = false;
            if out.reason_code.is_none() {
                out.reason_code = Some("policy_limit".into());
                out.policy_complete = false;
            }
            out.error = Some(error.to_string());
        }
        out
    }
    fn evaluate_into(&self, input: &CandidateInput, out: &mut CandidateDecision) -> Result<()> {
        let limits = &self.config.limits;
        validate_input(input, limits).map_err(|e| fail(out, "invalid_input", e))?;
        let base = match &input.base_url {
            Some(value) => Some(
                safe_url(value, None, limits.max_url_bytes)
                    .map_err(|e| fail(out, "unsafe_url", e))?,
            ),
            None => None,
        };
        let resolved = safe_url(&input.raw_value, base.as_ref(), limits.max_url_bytes)
            .map_err(|e| fail(out, "unsafe_url", e))?;
        out.resolved_discovered_url = Some(resolved.to_string());
        out.fragment = resolved.fragment().map(str::to_owned);
        let mut current = resolved;
        current.set_fragment(None);
        out.normalized_discovered_url = Some(current.to_string());
        out.fetch_candidate_url = Some(current.to_string());
        let transform = input.source_kind.transformable()
            && (!input.source_kind.initial()
                || self.config.initial_inputs.rewrite == InitialRewritePolicy::Apply);
        let mut visited = BTreeSet::new();
        visited.insert(current.to_string());
        if transform {
            self.transform_query(&mut current, out)?;
            visited.insert(current.to_string());
            for rule in &self.rewrites {
                let before = current.to_string();
                let Some(captures) = rule.regex.captures(&before) else {
                    push_step(
                        out,
                        limits,
                        CrawlPolicyStep {
                            kind: CrawlPolicyStepKind::Rewrite,
                            rule_id: Some(rule.id.clone()),
                            matched: Some(false),
                            changed: false,
                            before: None,
                            after: None,
                            error: None,
                        },
                    )?;
                    continue;
                };
                let matched = captures.get(0).context("regex match missing")?;
                let mut after = String::new();
                bounded_append(&mut after, &before[..matched.start()], limits.max_url_bytes)?;
                for part in &rule.replacement {
                    let text = match part {
                        TemplatePart::Text(text) => text.as_str(),
                        TemplatePart::Group(group) => {
                            captures.get(*group).map(|v| v.as_str()).unwrap_or("")
                        }
                    };
                    bounded_append(&mut after, text, limits.max_url_bytes)?;
                }
                bounded_append(&mut after, &before[matched.end()..], limits.max_url_bytes)?;
                let candidate = safe_url(&after, None, limits.max_url_bytes).and_then(|url| {
                    ensure!(
                        url.fragment().is_none(),
                        "rewrite must not create a fragment"
                    );
                    ensure!(
                        url.query().is_none_or(|query| query
                            .split('&')
                            .take(limits.max_query_tokens + 1)
                            .count()
                            <= limits.max_query_tokens),
                        "rewritten query exceeds token limit"
                    );
                    Ok(url)
                });
                let error = candidate.as_ref().err().map(ToString::to_string);
                push_step(
                    out,
                    limits,
                    CrawlPolicyStep {
                        kind: CrawlPolicyStepKind::Rewrite,
                        rule_id: Some(rule.id.clone()),
                        matched: Some(true),
                        changed: before != after,
                        before: Some(before.clone()),
                        after: Some(after),
                        error,
                    },
                )?;
                let candidate = candidate.map_err(|e| {
                    out.fetch_candidate_url = None;
                    fail(out, "unsafe_rewrite", e)
                })?;
                let identity = candidate.to_string();
                out.fetch_candidate_url = Some(identity.clone());
                if identity != before && !visited.insert(identity) {
                    bail!(
                        "{}",
                        fail(
                            out,
                            "rewrite_cycle",
                            anyhow::anyhow!("changed rewrite revisits an earlier identity")
                        )
                    );
                }
                current = candidate;
            }
        }
        out.fetch_candidate_url = Some(current.to_string());
        let (host, folder) = self.scope(&current);
        out.host_internal = host;
        out.folder_internal = folder;
        let apply_filters = !input.source_kind.initial()
            || self.config.initial_inputs.filters == InitialFilterPolicy::Apply;
        if apply_filters {
            let discovered = out
                .normalized_discovered_url
                .as_deref()
                .context("discovered identity")?;
            let fetch = current.as_str();
            let targets = match self.config.filter_identity {
                FilterIdentity::Fetch => vec![fetch],
                FilterIdentity::Discovered => vec![discovered],
                FilterIdentity::Both => vec![discovered, fetch],
            };
            let include_ok = self.includes.is_empty()
                || targets
                    .iter()
                    .all(|target| self.includes.iter().any(|r| r.is_match(target)));
            for (i, regex) in self.includes.iter().enumerate() {
                if targets.iter().any(|target| regex.is_match(target)) {
                    out.matched_rule_ids.push(format!("include-{}", i + 1));
                }
            }
            let mut excluded = false;
            for (i, regex) in self.excludes.iter().enumerate() {
                if targets.iter().any(|target| regex.is_match(target)) {
                    excluded = true;
                    out.matched_rule_ids.push(format!("exclude-{}", i + 1));
                }
            }
            push_step(
                out,
                limits,
                CrawlPolicyStep {
                    kind: CrawlPolicyStepKind::Filters,
                    rule_id: None,
                    matched: Some(excluded || !include_ok),
                    changed: false,
                    before: None,
                    after: None,
                    error: None,
                },
            )?;
            if excluded || !include_ok {
                out.reason_code = Some(if excluded { "excluded" } else { "not_included" }.into());
                return Ok(());
            }
        }
        push_step(
            out,
            limits,
            CrawlPolicyStep {
                kind: CrawlPolicyStepKind::Scope,
                rule_id: None,
                matched: Some(folder),
                changed: false,
                before: None,
                after: None,
                error: None,
            },
        )?;
        out.decision = if folder {
            CrawlPolicyDecisionKind::FetchAndTraverse
        } else if host && self.config.scope.outside_folder == OutsideScopePolicy::CheckOnce
            || !host && self.config.scope.external == OutsideScopePolicy::CheckOnce
        {
            CrawlPolicyDecisionKind::CheckOnly
        } else {
            out.reason_code = Some(
                if host {
                    "outside_folder"
                } else {
                    "outside_origin"
                }
                .into(),
            );
            CrawlPolicyDecisionKind::Deny
        };
        out.traverse = out.decision == CrawlPolicyDecisionKind::FetchAndTraverse
            && input.source_kind != CandidateSourceKind::SitemapDocument;
        if input.source_kind == CandidateSourceKind::SitemapDocument
            && out.decision == CrawlPolicyDecisionKind::FetchAndTraverse
        {
            out.decision = CrawlPolicyDecisionKind::CheckOnly;
        }
        Ok(())
    }
    fn scope(&self, candidate: &Url) -> (bool, bool) {
        let mut host = false;
        let mut folder = false;
        for root in &self.roots {
            let c = candidate.host_str().unwrap_or("");
            let r = root.url.host_str().unwrap_or("");
            let matched = candidate.scheme() == root.url.scheme()
                && candidate.port_or_known_default() == root.url.port_or_known_default()
                && (c == r || root.subdomains && c.ends_with(&format!(".{r}")));
            if matched {
                host = true;
                folder |= root
                    .folders
                    .iter()
                    .any(|prefix| candidate.path().starts_with(prefix))
                    || root.exact_paths.iter().any(|path| candidate.path() == path);
            }
        }
        (host, folder)
    }
    fn transform_query(&self, current: &mut Url, out: &mut CandidateDecision) -> Result<()> {
        let limits = &self.config.limits;
        let Some(query) = current.query().map(str::to_owned) else {
            return Ok(());
        };
        ensure!(
            query.split('&').take(limits.max_query_tokens + 1).count() <= limits.max_query_tokens,
            "query token limit exceeded"
        );
        let mut tokens = query.split('&').map(str::to_owned).collect::<Vec<_>>();
        if !self.config.query.remove_keys.is_empty() {
            let before = current.to_string();
            let mut undecodable = false;
            tokens.retain(|token| {
                let raw = token.split_once('=').map(|p| p.0).unwrap_or(token);
                let key = match self.config.query.key_match {
                    QueryKeyMatch::EncodedBytes => Some(raw.to_owned()),
                    QueryKeyMatch::PercentDecodedUtf8 => decode_key(raw),
                };
                let Some(key) = key else {
                    undecodable = true;
                    return true;
                };
                !self
                    .config
                    .query
                    .remove_keys
                    .iter()
                    .any(|remove| remove == &key)
            });
            if undecodable {
                out.warnings
                    .push("query_key_undecodable: invalid percent/UTF-8 key retained".into());
            }
            let joined_query = if tokens.is_empty() {
                None
            } else {
                Some(tokens.join("&"))
            };
            current.set_query(joined_query.as_deref());
            let after = current.to_string();
            push_step(
                out,
                limits,
                CrawlPolicyStep {
                    kind: CrawlPolicyStepKind::QueryRemoval,
                    rule_id: None,
                    matched: Some(before != after),
                    changed: before != after,
                    before: Some(before),
                    after: Some(after),
                    error: None,
                },
            )?;
        }
        if self.config.query.order == QueryOrder::StableEncodedKey {
            let before = current.to_string();
            tokens.sort_by(|a, b| query_key(a).cmp(query_key(b)));
            let joined_query = if tokens.is_empty() {
                None
            } else {
                Some(tokens.join("&"))
            };
            current.set_query(joined_query.as_deref());
            let after = current.to_string();
            push_step(
                out,
                limits,
                CrawlPolicyStep {
                    kind: CrawlPolicyStepKind::QueryOrder,
                    rule_id: None,
                    matched: Some(before != after),
                    changed: before != after,
                    before: Some(before),
                    after: Some(after),
                    error: None,
                },
            )?;
        }
        ensure!(
            current.as_str().len() <= limits.max_url_bytes,
            "query result exceeds URL budget"
        );
        Ok(())
    }
}

fn query_key(token: &str) -> &str {
    token.split_once('=').map(|p| p.0).unwrap_or(token)
}
fn bounded_append(output: &mut String, text: &str, cap: usize) -> Result<()> {
    ensure!(
        output
            .len()
            .checked_add(text.len())
            .is_some_and(|n| n <= cap),
        "rewrite result exceeds URL budget"
    );
    output.push_str(text);
    Ok(())
}
fn fail(out: &mut CandidateDecision, code: &str, error: anyhow::Error) -> anyhow::Error {
    out.reason_code = Some(code.into());
    error
}
fn push_step(
    out: &mut CandidateDecision,
    limits: &CrawlPolicyLimits,
    step: CrawlPolicyStep,
) -> Result<()> {
    ensure!(
        out.policy_steps.len() < limits.max_trace_steps,
        "policy trace step limit exceeded"
    );
    out.policy_steps.push(step);
    Ok(())
}
fn safe_url(input: &str, base: Option<&Url>, max: usize) -> Result<Url> {
    ensure!(
        input.len() <= max && !input.chars().any(char::is_control),
        "URL exceeds budget or contains raw controls"
    );
    let url = match base {
        Some(base) => base.join(input.trim()),
        None => Url::parse(input.trim()),
    }
    .context("invalid URL")?;
    ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none(),
        "only HTTP(S) host URLs without userinfo are supported"
    );
    ensure!(
        url.as_str().len() <= max,
        "normalized URL exceeds byte limit"
    );
    Ok(url)
}
fn valid_percent(input: &str) -> bool {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit()
            {
                return false;
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    true
}
fn decode_key(input: &str) -> Option<String> {
    if !valid_percent(input) {
        return None;
    }
    let mut output = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            output.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            output.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(output).ok()
}
fn compile_replacement(
    value: &RewriteReplacement,
    regex: &Regex,
    limits: &CrawlPolicyLimits,
) -> Result<Vec<TemplatePart>> {
    let (text, captures) = match value {
        RewriteReplacement::Literal { text } => (text, false),
        RewriteReplacement::Captures { template } => (template, true),
    };
    ensure!(
        text.len() <= limits.max_replacement_bytes && !text.chars().any(char::is_control),
        "replacement exceeds budget or contains raw controls"
    );
    if !captures {
        return Ok(vec![TemplatePart::Text(text.clone())]);
    }
    let names = regex
        .capture_names()
        .enumerate()
        .filter_map(|(i, n)| n.map(|n| (n, i)))
        .collect::<BTreeMap<_, _>>();
    let mut parts = Vec::new();
    let mut literal = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            literal.push(ch);
            continue;
        }
        match chars.next() {
            Some('$') => literal.push('$'),
            Some('{') => {
                let mut name = String::new();
                let mut closed = false;
                for ch in chars.by_ref() {
                    if ch == '}' {
                        closed = true;
                        break;
                    }
                    name.push(ch);
                }
                ensure!(
                    closed && !name.is_empty(),
                    "capture placeholder requires closed nonempty braces"
                );
                let group = if name.bytes().all(|b| b.is_ascii_digit()) {
                    name.parse::<usize>().context("invalid numeric capture")?
                } else {
                    *names.get(name.as_str()).context("unknown named capture")?
                };
                ensure!(group < regex.captures_len(), "unknown numeric capture");
                if !literal.is_empty() {
                    parts.push(TemplatePart::Text(std::mem::take(&mut literal)));
                }
                parts.push(TemplatePart::Group(group));
            }
            _ => bail!("capture replacement dollar syntax requires $$ or ${{group}}"),
        }
    }
    if !literal.is_empty() {
        parts.push(TemplatePart::Text(literal));
    }
    Ok(parts)
}
fn validate_input(input: &CandidateInput, limits: &CrawlPolicyLimits) -> Result<()> {
    ensure!(
        !input.occurrence_id.is_empty()
            && input.occurrence_id.len() <= 128
            && !input.occurrence_id.chars().any(char::is_control),
        "invalid occurrence id"
    );
    ensure!(
        input.raw_value.len() <= limits.max_url_bytes,
        "raw candidate exceeds URL limit"
    );
    for value in [&input.base_url, &input.source_url].into_iter().flatten() {
        safe_url(value, None, limits.max_url_bytes)?;
    }
    if let Some(location) = &input.source_location {
        ensure!(
            !location.document_source.is_empty()
                && location.document_source.len() <= 128
                && !location.document_source.chars().any(char::is_control),
            "invalid source name"
        );
        if let Some(path) = &location.dom_path {
            ensure!(
                path.len() <= 8192 && !path.chars().any(char::is_control),
                "invalid source path"
            );
        }
        ensure!(
            location.source_index.is_none_or(|n| n > 0),
            "source index must be positive"
        );
    }
    Ok(())
}
struct BudgetWriter {
    count: usize,
    cap: usize,
}
impl std::io::Write for BudgetWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let n = self
            .count
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("JSON budget overflow"))?;
        if n > self.cap {
            return Err(std::io::Error::other("serialized JSON exceeds byte budget"));
        }
        self.count = n;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn ensure_serialized_budget<T: Serialize>(value: &T, cap: usize) -> Result<usize> {
    let mut writer = BudgetWriter { count: 0, cap };
    serde_json::to_writer(&mut writer, value).context("serialized JSON exceeds byte budget")?;
    Ok(writer.count)
}
pub fn preview_crawl_policy(input: &CrawlPolicyPreviewInput) -> Result<CrawlPolicyPreview> {
    let policy = CompiledCrawlPolicy::new(&input.config)?;
    let limits = &policy.config.limits;
    ensure!(
        !input.inputs.is_empty() && input.inputs.len() <= limits.max_inputs,
        "preview requires 1..=max_inputs occurrences"
    );
    ensure_serialized_budget(input, limits.max_input_bytes)?;
    let mut ids = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut decisions = Vec::new();
    let mut output = 0usize;
    for row in &input.inputs {
        ensure!(ids.insert(&row.occurrence_id), "duplicate occurrence id");
        // Invalid URL values remain per-row decisions; malformed metadata is also explicit.
        let mut decision = policy.evaluate(row);
        if decision.decision != CrawlPolicyDecisionKind::Deny
            && let Some(url) = &decision.fetch_candidate_url
            && !seen.insert(url.clone())
        {
            decision.coalesced_into = Some(url.clone());
        }
        output = output
            .checked_add(ensure_serialized_budget(
                &decision,
                limits.max_output_bytes,
            )?)
            .context("preview output budget overflow")?;
        ensure!(
            output <= limits.max_output_bytes,
            "preview decisions exceed output byte budget"
        );
        decisions.push(decision);
    }
    let result=CrawlPolicyPreview {engine:CRAWL_POLICY_ENGINE.into(),replacement_profile:CRAWL_REPLACEMENT_PROFILE.into(),policy_sha256:policy.hash.clone(),config:policy.config.clone(),decisions,website_requests:0,
        limitations:vec!["Offline decisions do not fetch URLs or attest document provenance, robots permission, server responses or original-engine parity.".into(),"First-match rewrite profile differs from all-match/Java replacement semantics; no dynamic mid-crawl rule updates.".into(),"check_only forbids onward graph traversal; actual native/renderer/robots integration requires separate gates.".into()]};
    ensure_serialized_budget(&result, limits.max_output_bytes)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> CrawlPolicyConfig {
        CrawlPolicyConfig {
            scope: CrawlScopeConfig {
                roots: vec![CrawlScopeRoot {
                    origin: "https://example.test".into(),
                    subdomains: false,
                    folders: vec!["/shop/".into()],
                    exact_paths: vec![],
                }],
                ..Default::default()
            },
            ..Default::default()
        }
    }
    fn candidate(raw: &str, source_kind: CandidateSourceKind) -> CandidateInput {
        CandidateInput {
            occurrence_id: "one".into(),
            source_kind,
            raw_value: raw.into(),
            base_url: Some("https://example.test/shop/index".into()),
            source_url: None,
            source_location: None,
        }
    }
    fn rule(id: &str, pattern: &str, text: &str) -> RewriteRule {
        RewriteRule {
            id: id.into(),
            pattern: pattern.into(),
            replacement: RewriteReplacement::Literal { text: text.into() },
        }
    }
    #[test]
    fn forty_seven_independently_authored_policy_contracts_are_executed() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/crawl-policy/authored-policy-cases.json"
        ))
        .unwrap();
        let cases = fixture["candidate_cases"].as_array().unwrap();
        assert_eq!(cases.len(), 47);
        for case in cases {
            let config: CrawlPolicyConfig = serde_json::from_value(case["policy"].clone()).unwrap();
            let policy = CompiledCrawlPolicy::new(&config).unwrap();
            let mut input = case["input"].clone();
            input["occurrence_id"] = case["id"].clone();
            let input: CandidateInput = serde_json::from_value(input).unwrap();
            let actual = serde_json::to_value(policy.evaluate(&input)).unwrap();
            let expected = case["expected"].as_object().unwrap();
            for (field, value) in expected {
                match field.as_str() {
                    "changed_steps" => assert_eq!(
                        actual["policy_steps"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter(|step| step["changed"] == true)
                            .count(),
                        value.as_u64().unwrap() as usize,
                        "{}",
                        case["name"]
                    ),
                    "query_key_warning" => assert_eq!(
                        !actual["warnings"].as_array().unwrap().is_empty(),
                        value.as_bool().unwrap(),
                        "{}",
                        case["name"]
                    ),
                    "discovered_descendants" => {
                        assert_eq!(actual["traverse"], false);
                        assert!(value.as_array().unwrap().is_empty());
                    }
                    _ => assert_eq!(&actual[field], value, "{} field {field}", case["name"]),
                }
            }
            assert_eq!(actual["raw_value"], case["input"]["raw_value"]);
            assert_eq!(actual["policy_sha256"], policy.policy_sha256());
        }
    }
    #[test]
    fn preview_preserves_order_duplicates_aliases_invalid_rows_and_zero_http() {
        let mut config = config();
        config.query.remove_keys = vec!["utm_source".into()];
        let mut inputs = vec![
            candidate("/shop/a?utm_source=one", CandidateSourceKind::Anchor),
            candidate("/shop/a?utm_source=two", CandidateSourceKind::Anchor),
            candidate("mailto:user@example.test", CandidateSourceKind::Anchor),
        ];
        for (i, input) in inputs.iter_mut().enumerate() {
            input.occurrence_id = (i + 1).to_string();
        }
        let result = preview_crawl_policy(&CrawlPolicyPreviewInput {
            config,
            inputs: inputs.clone(),
        })
        .unwrap();
        assert_eq!(result.website_requests, 0);
        assert_eq!(result.decisions.len(), 3);
        assert_eq!(result.decisions[0].raw_value, inputs[0].raw_value);
        assert!(result.decisions[0].coalesced_into.is_none());
        assert_eq!(
            result.decisions[1].coalesced_into.as_deref(),
            Some("https://example.test/shop/a")
        );
        assert_eq!(result.decisions[2].decision, CrawlPolicyDecisionKind::Deny);
        assert_eq!(
            result.decisions[2].reason_code.as_deref(),
            Some("unsafe_url")
        );
        assert_eq!(
            serde_json::from_value::<CrawlPolicyPreview>(serde_json::to_value(&result).unwrap())
                .unwrap(),
            result
        );
    }
    #[test]
    fn query_byte_identity_is_preserved_and_ordering_is_opt_in_stable() {
        let mut config = config();
        let input = candidate(
            "/shop/?b=%2f&a=+&a=%20&&flag&empty=&",
            CandidateSourceKind::Anchor,
        );
        let plain = CompiledCrawlPolicy::new(&config).unwrap().evaluate(&input);
        assert_eq!(
            plain.fetch_candidate_url.as_deref(),
            Some("https://example.test/shop/?b=%2f&a=+&a=%20&&flag&empty=&")
        );
        config.query.order = QueryOrder::StableEncodedKey;
        let sorted = CompiledCrawlPolicy::new(&config).unwrap().evaluate(&input);
        assert_eq!(
            sorted.fetch_candidate_url.as_deref(),
            Some("https://example.test/shop/?&&a=+&a=%20&b=%2f&empty=&flag")
        );
        assert_ne!(plain.policy_sha256, sorted.policy_sha256);
        assert_eq!(
            plain.normalized_discovered_url,
            sorted.normalized_discovered_url
        );
        let empty = CompiledCrawlPolicy::new(&config)
            .unwrap()
            .evaluate(&candidate("/shop/?", CandidateSourceKind::Anchor));
        assert_eq!(
            empty.fetch_candidate_url.as_deref(),
            Some("https://example.test/shop/?")
        );
    }
    #[test]
    fn capture_replacement_is_explicit_first_match_with_optional_group_and_literal_dollar() {
        let mut config = config();
        config.rewrites = vec![RewriteRule {
            id: "capture".into(),
            pattern: "(old)(x)?".into(),
            replacement: RewriteReplacement::Captures {
                template: "new${2}$$${1}".into(),
            },
        }];
        let result = CompiledCrawlPolicy::new(&config)
            .unwrap()
            .evaluate(&candidate("/shop/old/old", CandidateSourceKind::Anchor));
        assert_eq!(
            result.fetch_candidate_url.as_deref(),
            Some("https://example.test/shop/new$old/old")
        );
        assert_eq!(result.policy_steps[0].rule_id.as_deref(), Some("capture"));
        assert_eq!(result.policy_steps[0].matched, Some(true));
        assert!(result.policy_steps[0].changed);
    }
    #[test]
    fn compiled_config_rejects_unsafe_ambiguous_unsupported_rules_before_evaluation() {
        let mut configs = vec![];
        let mut change = config();
        change.engine = "unknown".into();
        configs.push(change);
        let mut change = config();
        change.scope.roots[0].origin = "https://user:password@example.test".into();
        configs.push(change);
        let mut change = config();
        change.scope.roots[0].origin = "https://example.test/path".into();
        configs.push(change);
        let mut change = config();
        change.scope.roots[0].folders = vec!["/shop".into()];
        configs.push(change);
        let mut change = config();
        change.scope.roots[0].folders = vec!["/shop/../other/".into()];
        configs.push(change);
        let mut change = config();
        change.scope.roots[0].folders = vec!["/shop/%2e%2e/".into()];
        configs.push(change);
        let mut change = config();
        change.scope.roots[0].folders = vec!["/shop/%ZZ/".into()];
        configs.push(change);
        let mut change = config();
        change.scope.roots[0].folders = vec!["//other.test/".into()];
        configs.push(change);
        let mut change = config();
        change.query.remove_keys = vec!["a".into(), "a".into()];
        configs.push(change);
        for pattern in ["(?<=old)new", r"(old)\1", "["] {
            let mut change = config();
            change.rewrites = vec![rule("bad", pattern, "new")];
            configs.push(change);
        }
        for template in ["$1", "${missing}", "${99}", "${1", "$", "${}"] {
            let mut change = config();
            change.rewrites = vec![RewriteRule {
                id: "bad".into(),
                pattern: "(old)".into(),
                replacement: RewriteReplacement::Captures {
                    template: template.into(),
                },
            }];
            configs.push(change);
        }
        let mut change = config();
        change.rewrites = vec![rule("same", "a", "b"), rule("same", "b", "c")];
        configs.push(change);
        let mut change = config();
        change.scope.roots.clear();
        configs.push(change);
        for config in configs {
            assert!(
                CompiledCrawlPolicy::new(&config).is_err(),
                "unexpected valid: {config:?}"
            );
        }
    }
    #[test]
    fn every_resource_limit_is_positive_bounded_and_enforced_without_truncation() {
        let object = serde_json::to_value(CrawlPolicyLimits::default()).unwrap();
        for key in object.as_object().unwrap().keys() {
            for value in [0, object[key].as_u64().unwrap() + 1] {
                let mut altered = object.clone();
                altered[key] = serde_json::json!(value);
                assert!(
                    serde_json::from_value::<CrawlPolicyLimits>(altered)
                        .unwrap()
                        .validate()
                        .is_err(),
                    "{key}"
                );
            }
        }
        let mut config = config();
        config.limits.max_query_tokens = 1;
        let limited = CompiledCrawlPolicy::new(&config)
            .unwrap()
            .evaluate(&candidate("/shop/a?a=1&b=2", CandidateSourceKind::Anchor));
        assert_eq!(limited.decision, CrawlPolicyDecisionKind::Deny);
        assert!(!limited.policy_complete);
        let mut config = super::tests::config();
        config.limits.max_trace_steps = 1;
        config.rewrites = vec![rule("one", "old", "new"), rule("two", "new", "last")];
        let limited = CompiledCrawlPolicy::new(&config)
            .unwrap()
            .evaluate(&candidate("/shop/old", CandidateSourceKind::Anchor));
        assert_eq!(limited.reason_code.as_deref(), Some("policy_limit"));
        assert!(!limited.policy_complete);
        assert_eq!(limited.policy_steps.len(), 1);
        let mut config = super::tests::config();
        config.limits.max_replacement_bytes = 1;
        config.rewrites = vec![rule("too-long", "old", "new")];
        assert!(CompiledCrawlPolicy::new(&config).is_err());
        let mut config = super::tests::config();
        config.limits.max_output_bytes = 100;
        assert!(
            preview_crawl_policy(&CrawlPolicyPreviewInput {
                config,
                inputs: vec![candidate("/shop/a", CandidateSourceKind::Anchor)]
            })
            .is_err()
        );
    }
    #[test]
    fn redirects_saved_fetch_and_sitemap_documents_skip_discovery_transformations() {
        let mut config = config();
        config.query.remove_keys = vec!["track".into()];
        config.rewrites = vec![rule("path", "old", "new")];
        let policy = CompiledCrawlPolicy::new(&config).unwrap();
        for kind in [
            CandidateSourceKind::Redirect,
            CandidateSourceKind::SavedFetch,
            CandidateSourceKind::SitemapDocument,
        ] {
            let result = policy.evaluate(&candidate("/shop/old?track=1", kind));
            assert_eq!(
                result.fetch_candidate_url.as_deref(),
                Some("https://example.test/shop/old?track=1")
            );
            assert!(!result.policy_steps.iter().any(|step| matches!(
                step.kind,
                CrawlPolicyStepKind::QueryRemoval | CrawlPolicyStepKind::Rewrite
            )));
            if kind == CandidateSourceKind::SitemapDocument {
                assert_eq!(result.decision, CrawlPolicyDecisionKind::CheckOnly);
                assert!(!result.traverse);
            }
        }
    }
    #[test]
    fn scopes_use_actual_effective_origin_and_do_not_infer_sibling_hosts() {
        let mut config = config();
        config.scope.roots[0].origin = "https://WWW.Example.test:443/".into();
        config.scope.roots[0].subdomains = true;
        let policy = CompiledCrawlPolicy::new(&config).unwrap();
        assert_eq!(
            policy.config().scope.roots[0].origin,
            "https://www.example.test"
        );
        assert_eq!(
            policy
                .evaluate(&candidate(
                    "https://www.example.test:443/shop/a",
                    CandidateSourceKind::Anchor
                ))
                .decision,
            CrawlPolicyDecisionKind::FetchAndTraverse
        );
        assert_eq!(
            policy
                .evaluate(&candidate(
                    "https://cdn.example.test/shop/a",
                    CandidateSourceKind::Anchor
                ))
                .reason_code
                .as_deref(),
            Some("outside_origin")
        );
        assert_eq!(
            policy
                .evaluate(&candidate(
                    "http://www.example.test/shop/a",
                    CandidateSourceKind::Anchor
                ))
                .reason_code
                .as_deref(),
            Some("outside_origin")
        );
        assert_eq!(
            policy
                .evaluate(&candidate(
                    "https://a.www.example.test/shop/a",
                    CandidateSourceKind::Anchor
                ))
                .decision,
            CrawlPolicyDecisionKind::FetchAndTraverse
        );
    }
    #[test]
    fn ip_origins_are_exact_and_cannot_enable_domain_suffix_expansion() {
        for origin in ["http://127.0.0.1:8080", "http://[::1]:8080"] {
            let mut config = config();
            config.scope.roots[0].origin = origin.into();
            let policy = CompiledCrawlPolicy::new(&config).unwrap();
            let result = policy.evaluate(&candidate(
                &format!("{origin}/shop/a"),
                CandidateSourceKind::Anchor,
            ));
            assert_eq!(result.decision, CrawlPolicyDecisionKind::FetchAndTraverse);
            config.scope.roots[0].subdomains = true;
            assert!(CompiledCrawlPolicy::new(&config).is_err());
        }
    }
    #[test]
    fn denied_filter_keeps_scope_classification_and_both_identity_is_conservative() {
        let mut config = config();
        config.rewrites = vec![rule("path", "old", "new")];
        config.include_patterns = vec!["/old".into()];
        config.filter_identity = FilterIdentity::Both;
        let result = CompiledCrawlPolicy::new(&config)
            .unwrap()
            .evaluate(&candidate("/shop/old", CandidateSourceKind::Anchor));
        assert_eq!(result.reason_code.as_deref(), Some("not_included"));
        assert!(result.host_internal && result.folder_internal);
        assert!(!result.traverse);
        assert_eq!(result.matched_rule_ids, vec!["include-1"]);
    }
    #[test]
    fn unknown_json_shapes_duplicate_occurrence_ids_and_malformed_metadata_fail_explicitly() {
        let mut value = serde_json::to_value(config()).unwrap();
        value["surprise"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CrawlPolicyConfig>(value).is_err());
        let mut value =
            serde_json::to_value(candidate("/shop/a", CandidateSourceKind::Anchor)).unwrap();
        value["source_location"] = serde_json::json!({"document_source":"raw_html","hidden":true});
        assert!(serde_json::from_value::<CandidateInput>(value).is_err());
        let input = candidate("/shop/a", CandidateSourceKind::Anchor);
        assert!(
            preview_crawl_policy(&CrawlPolicyPreviewInput {
                config: config(),
                inputs: vec![input.clone(), input]
            })
            .is_err()
        );
        let mut input = candidate("/shop/a", CandidateSourceKind::Anchor);
        input.source_location = Some(CandidateSourceLocation {
            document_source: "raw_html".into(),
            dom_path: None,
            source_index: Some(0),
        });
        let result = CompiledCrawlPolicy::new(&config())
            .unwrap()
            .evaluate(&input);
        assert_eq!(result.reason_code.as_deref(), Some("invalid_input"));
        assert_eq!(result.decision, CrawlPolicyDecisionKind::Deny);
    }
}
