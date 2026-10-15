//! Bounded saved-text corpus: unchanged own 1/2/3 engine plus opt-in 4..6 windows.
//! Supplied selected text only. No HTML parsing, fetching, similarity scheduling,
//! title/heading/inlink merging, inferred missing text or reference parity.
use crate::{
    CONTENT_TOKEN_PROFILE, ContentAnalysisConfig, ContentAnalysisInput, ContentCaseBehavior,
    ContentTextInput, analyze_content,
};
use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
};
pub const NGRAM_CORPUS_ENGINE: &str = "open-seo-spider/saved-ngram-corpus-v2";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NgramCorpusLimits {
    pub max_documents: usize,
    pub max_text_bytes: usize,
    pub max_total_tokens: usize,
    pub max_unique_ngrams: usize,
    pub max_key_bytes: usize,
    pub max_memberships: usize,
    pub max_identity_bytes: usize,
    pub position_samples: usize,
}
impl Default for NgramCorpusLimits {
    fn default() -> Self {
        Self {
            max_documents: 50_000,
            max_text_bytes: 64 * 1024 * 1024,
            max_total_tokens: 2_000_000,
            max_unique_ngrams: 100_000,
            max_key_bytes: 16 * 1024 * 1024,
            max_memberships: 500_000,
            max_identity_bytes: 16 * 1024 * 1024,
            position_samples: 16,
        }
    }
}
impl NgramCorpusLimits {
    pub fn validate(&self) -> Result<()> {
        let ceiling = Self::default();
        macro_rules! bound{($($f:ident),+)=>{$(ensure!(self.$f>0&&self.$f<=ceiling.$f,concat!(stringify!($f)," limit outside supported range"));)+}}
        bound!(
            max_documents,
            max_text_bytes,
            max_total_tokens,
            max_unique_ngrams,
            max_key_bytes,
            max_memberships,
            max_identity_bytes,
            position_samples
        );
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NgramCorpusOptions {
    pub case_behavior: ContentCaseBehavior,
    pub stopwords: Vec<String>,
    pub limits: NgramCorpusLimits,
    pub max_n: usize,
}
impl Default for NgramCorpusOptions {
    fn default() -> Self {
        Self {
            case_behavior: Default::default(),
            stopwords: vec![],
            limits: Default::default(),
            max_n: 3,
        }
    }
}
impl NgramCorpusOptions {
    pub fn validate(&self) -> Result<()> {
        self.limits.validate()?;
        ensure!(
            (3..=6).contains(&self.max_n),
            "Ngram max_n must be3..=6; the existing1..3 profile is always retained"
        );
        Ok(())
    }
}
/// Counters for each requested extended size. The legacy arrays below remain n1..3.
#[derive(Debug, Serialize)]
pub struct NgramCorpusExtendedWindows {
    pub n: usize,
    pub candidate_windows: usize,
    pub excluded_stopword_windows: usize,
    pub included_windows: usize,
}
/// Caller declarations. The App adapter separately binds actual saved report,
/// selected-recrawl source selection and capture validation before supplying them.
pub struct NgramCorpusDocument<'a> {
    pub requested_url: &'a str,
    pub source_revision: Option<&'a str>,
    pub capture_config_sha256: Option<&'a str>,
    pub text_sha256: Option<&'a str>,
    pub text: Option<&'a str>,
    pub complete: bool,
    pub basis: &'static str,
    pub observed_generation: Option<u64>,
}
#[derive(Debug, Clone, Serialize)]
pub struct NgramCorpusSource {
    pub ordinal: usize,
    pub requested_url: String,
    pub source_revision: Option<String>,
    pub capture_config_sha256: Option<String>,
    pub text_sha256: Option<String>,
    pub basis: &'static str,
    pub observed_generation: Option<u64>,
    pub state: &'static str,
    pub counted: bool,
    pub text_bytes: usize,
    pub tokens: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct NgramCorpusMembership {
    pub source_ordinal: usize,
    pub occurrences: usize,
    pub first_positions: Vec<usize>,
    pub positions_sampled: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct NgramCorpusRow {
    pub n: usize,
    pub tokens: Vec<String>,
    pub occurrences: usize,
    pub document_frequency: usize,
    pub sources: Vec<NgramCorpusMembership>,
}
#[derive(Debug, Serialize)]
pub struct NgramCorpusReport {
    pub engine: &'static str,
    pub token_profile: &'static str,
    pub lowercase_unicode_version: (u8, u8, u8),
    pub options: NgramCorpusOptions,
    pub sources: Vec<NgramCorpusSource>,
    pub rows: Vec<NgramCorpusRow>,
    pub source_states: BTreeMap<&'static str, usize>,
    pub candidate_windows: [usize; 3],
    pub excluded_stopword_windows: [usize; 3],
    pub included_windows: [usize; 3],
    pub extended_windows: Vec<NgramCorpusExtendedWindows>,
    pub total_tokens: usize,
    pub total_text_bytes: usize,
    pub memberships: usize,
    pub complete_for_supplied_document_rows: bool,
    pub all_supplied_sources_complete: bool,
    pub website_requests: usize,
    pub whole_website_coverage_attested: bool,
    pub limitations: Vec<&'static str>,
}
fn add(left: usize, right: usize) -> Result<usize> {
    left.checked_add(right)
        .context("Ngram corpus count overflow")
}
fn profile(options: &NgramCorpusOptions) -> ContentAnalysisConfig {
    let mut config = ContentAnalysisConfig {
        case_behavior: options.case_behavior,
        stopwords: options.stopwords.clone(),
        ..Default::default()
    };
    config.limits.max_position_samples = options.limits.position_samples;
    config
}
fn analysis(
    options: &NgramCorpusOptions,
    id: &str,
    text: Option<&str>,
) -> Result<crate::ContentAnalysisResult> {
    analyze_content(&ContentAnalysisInput {
        corpus_id: "saved-ngram-corpus".into(),
        corpus_scope: "one validated saved text observation; aggregation never bridges documents"
            .into(),
        corpus_complete: true,
        config: profile(options),
        documents: vec![ContentTextInput {
            id: id.into(),
            source: "saved-selected-text".into(),
            text: text.map(str::to_owned),
            language: None,
            complete: text.is_some(),
            truncation_reason: None,
        }],
    })
}
pub fn aggregate_saved_ngrams(
    documents: &[NgramCorpusDocument<'_>],
    options: &NgramCorpusOptions,
) -> Result<NgramCorpusReport> {
    options.validate()?;
    ensure!(
        documents.len() <= options.limits.max_documents,
        "Saved document count exceeds ngram corpus limit"
    );
    ensure!(
        options.stopwords.len() <= 1024 && options.stopwords.iter().all(|s| s.len() <= 4096),
        "Stopword input exceeds profile budget"
    );
    // Validate unchanged stopword/case profile even for zero complete sources.
    analysis(options, "profile-validation", None)?;
    let extended = if options.max_n > 3 {
        Some(ExtendedPreparation::new(options)?)
    } else {
        None
    };
    let mut urls = BTreeSet::new();
    let mut identities = 0usize;
    let mut total_text = 0usize;
    for document in documents {
        ensure!(
            document.requested_url.len() <= 16 * 1024,
            "Source URL exceeds16384 bytes"
        );
        ensure!(
            urls.insert(document.requested_url),
            "Duplicate saved requested URL; aggregate identity is ambiguous"
        );
        identities = add(identities, document.requested_url.len())?;
        for value in [
            document.source_revision,
            document.capture_config_sha256,
            document.text_sha256,
        ]
        .into_iter()
        .flatten()
        {
            ensure!(value.len() <= 256, "Saved source identity exceeds256 bytes");
            identities = add(identities, value.len())?;
        }
        ensure!(
            identities <= options.limits.max_identity_bytes,
            "Source identity budget exceeded"
        );
        if document.complete {
            let text = document.text.context("Complete saved source has no text")?;
            ensure!(
                text.len() <= crate::ContentAnalysisLimits::default().max_text_bytes,
                "Saved text exceeds per-document engine limit"
            );
            total_text = add(total_text, text.len())?;
            ensure!(
                total_text <= options.limits.max_text_bytes,
                "Saved text total budget exceeded"
            );
        }
    }
    let mut report = NgramCorpusReport {
        engine: NGRAM_CORPUS_ENGINE,
        token_profile: CONTENT_TOKEN_PROFILE,
        lowercase_unicode_version: char::UNICODE_VERSION,
        options: options.clone(),
        sources: vec![],
        rows: vec![],
        source_states: BTreeMap::new(),
        candidate_windows: [0; 3],
        excluded_stopword_windows: [0; 3],
        included_windows: [0; 3],
        extended_windows: (4..=options.max_n)
            .map(|n| NgramCorpusExtendedWindows {
                n,
                candidate_windows: 0,
                excluded_stopword_windows: 0,
                included_windows: 0,
            })
            .collect(),
        total_tokens: 0,
        total_text_bytes: total_text,
        memberships: 0,
        complete_for_supplied_document_rows: true,
        all_supplied_sources_complete: true,
        website_requests: 0,
        whole_website_coverage_attested: false,
        limitations: vec![
            "Only caller-supplied saved selected text; missing/incomplete sources are excluded and never treated as zero-frequency successful captures.",
            "All requested source rows are visited; failed budgets return an error rather than a first-N corpus.",
            "Exact own Unicode/token/case/stopword profile; overlapping1..max_n windows never bridge documents or removed stopwords. Legacy counter arrays describe1..3; extended_windows identifies every requested4..6 size.",
            "Counts and source membership are complete within this accepted input; positions are explicitly bounded examples per ngram/source.",
            "No new HTML selection, title/headings/unlinked-body/inlink/alt channels, word cloud or reference tokenization parity. API/CLI default max_n3 is retained; the saved-text UI explicitly requests max_n6.",
        ],
    };
    let mut rows: BTreeMap<Vec<String>, NgramCorpusRow> = BTreeMap::new();
    let mut key_bytes = 0usize;
    for (ordinal, document) in documents.iter().enumerate() {
        let state = if document.complete {
            if document.text == Some("") {
                "complete_empty"
            } else {
                "complete"
            }
        } else if document.text.is_some() {
            "incomplete_excluded"
        } else {
            "missing_or_unavailable"
        };
        *report.source_states.entry(state).or_default() += 1;
        report.all_supplied_sources_complete &= document.complete;
        let mut source = NgramCorpusSource {
            ordinal,
            requested_url: document.requested_url.into(),
            source_revision: document.source_revision.map(str::to_owned),
            capture_config_sha256: document.capture_config_sha256.map(str::to_owned),
            text_sha256: document.text_sha256.map(str::to_owned),
            basis: document.basis,
            observed_generation: document.observed_generation,
            state,
            counted: document.complete,
            text_bytes: document.text.map_or(0, str::len),
            tokens: 0,
        };
        if document.complete {
            let result = analysis(options, &format!("source-{ordinal}"), document.text)?;
            let doc = &result.documents[0];
            source.tokens = doc.token_count;
            report.total_tokens = add(report.total_tokens, doc.token_count)?;
            ensure!(
                report.total_tokens <= options.limits.max_total_tokens,
                "Corpus token budget exceeded"
            );
            for distribution in &doc.ngrams {
                let i = distribution.n - 1;
                report.candidate_windows[i] =
                    add(report.candidate_windows[i], distribution.candidate_windows)?;
                report.excluded_stopword_windows[i] = add(
                    report.excluded_stopword_windows[i],
                    distribution.excluded_stopword_windows,
                )?;
                report.included_windows[i] =
                    add(report.included_windows[i], distribution.included_windows)?;
                merge_distribution(
                    &mut report,
                    &mut rows,
                    &mut key_bytes,
                    distribution,
                    ordinal,
                    options,
                )?;
            }
            if let Some(preparation) = &extended {
                let tokens = preparation.tokens(
                    document.text.context("Complete source text disappeared")?,
                    options.case_behavior,
                )?;
                ensure!(
                    tokens.len() == doc.token_count,
                    "Extended token preparation disagrees with the existing profile"
                );
                let mut extended_unique = 0usize;
                let mut extended_keys = 0usize;
                for n in 4..=options.max_n {
                    let distribution = preparation.distribution(
                        &tokens,
                        n,
                        options,
                        &mut extended_unique,
                        &mut extended_keys,
                    )?;
                    let counter = &mut report.extended_windows[n - 4];
                    counter.candidate_windows =
                        add(counter.candidate_windows, distribution.candidate_windows)?;
                    counter.excluded_stopword_windows = add(
                        counter.excluded_stopword_windows,
                        distribution.excluded_stopword_windows,
                    )?;
                    counter.included_windows =
                        add(counter.included_windows, distribution.included_windows)?;
                    merge_distribution(
                        &mut report,
                        &mut rows,
                        &mut key_bytes,
                        &distribution,
                        ordinal,
                        options,
                    )?;
                }
            }
        }
        report.sources.push(source);
    }
    report.rows = rows.into_values().collect();
    report.rows.sort_by(|a, b| {
        b.occurrences
            .cmp(&a.occurrences)
            .then_with(|| a.n.cmp(&b.n))
            .then_with(|| a.tokens.cmp(&b.tokens))
    });
    // Independent complete retained result bound before any successful caller output.
    let mut sink = ByteBudget {
        bytes: 0,
        cap: 64 * 1024 * 1024,
    };
    serde_json::to_writer(&mut sink, &report)?;
    Ok(report)
}

fn merge_distribution(
    report: &mut NgramCorpusReport,
    rows: &mut BTreeMap<Vec<String>, NgramCorpusRow>,
    key_bytes: &mut usize,
    distribution: &crate::ContentNgramDistribution,
    ordinal: usize,
    options: &NgramCorpusOptions,
) -> Result<()> {
    for gram in &distribution.rows {
        report.memberships = add(report.memberships, 1)?;
        ensure!(
            report.memberships <= options.limits.max_memberships,
            "Ngram/source membership budget exceeded"
        );
        if !rows.contains_key(&gram.tokens) {
            ensure!(
                rows.len() < options.limits.max_unique_ngrams,
                "Unique corpus ngram budget exceeded"
            );
            *key_bytes = add(*key_bytes, gram.tokens.iter().map(String::len).sum())?;
            ensure!(
                *key_bytes <= options.limits.max_key_bytes,
                "Corpus ngram key budget exceeded"
            );
            rows.insert(
                gram.tokens.clone(),
                NgramCorpusRow {
                    n: distribution.n,
                    tokens: gram.tokens.clone(),
                    occurrences: 0,
                    document_frequency: 0,
                    sources: vec![],
                },
            );
        }
        let row = rows
            .get_mut(&gram.tokens)
            .context("Accepted gram disappeared")?;
        row.occurrences = add(row.occurrences, gram.occurrences)?;
        row.document_frequency = add(row.document_frequency, 1)?;
        row.sources.push(NgramCorpusMembership {
            source_ordinal: ordinal,
            occurrences: gram.occurrences,
            first_positions: gram.first_positions.clone(),
            positions_sampled: gram.positions_sampled,
        });
    }
    Ok(())
}
/// Separate preparation for pure4..6 windows. The token predicate and normalization
/// deliberately match CONTENT_TOKEN_PROFILE; no similarity/readability settings change.
struct ExtendedPreparation {
    mark: Regex,
    stopwords: BTreeSet<String>,
    limits: crate::ContentAnalysisLimits,
}
impl ExtendedPreparation {
    fn new(options: &NgramCorpusOptions) -> Result<Self> {
        let mark = Regex::new(r"^\p{M}$")?;
        let limits = crate::ContentAnalysisLimits::default();
        let mut stopwords = BTreeSet::new();
        for value in &options.stopwords {
            let words = extended_tokenize(value, &mark, &limits)?;
            ensure!(
                words.len() == 1 && words[0] == *value,
                "Stopword must be exactly one unpadded token"
            );
            let token = extended_case(value, options.case_behavior);
            ensure!(
                token.len() <= limits.max_token_bytes,
                "Case-normalized stopword exceeds byte budget"
            );
            ensure!(
                stopwords.insert(token),
                "Duplicate stopword after configured case handling"
            );
        }
        Ok(Self {
            mark,
            stopwords,
            limits,
        })
    }
    fn tokens(&self, text: &str, behavior: ContentCaseBehavior) -> Result<Vec<String>> {
        let observed = extended_tokenize(text, &self.mark, &self.limits)?;
        let mut tokens = Vec::with_capacity(observed.len());
        for value in observed {
            let value = extended_case(&value, behavior);
            ensure!(
                value.len() <= self.limits.max_token_bytes,
                "Case-normalized token exceeds byte budget"
            );
            tokens.push(value);
        }
        Ok(tokens)
    }
    fn distribution(
        &self,
        tokens: &[String],
        n: usize,
        options: &NgramCorpusOptions,
        unique: &mut usize,
        keys: &mut usize,
    ) -> Result<crate::ContentNgramDistribution> {
        ensure!((4..=6).contains(&n), "Extended ngram size must be4..=6");
        let candidate_windows = tokens.len().checked_sub(n).map_or(0, |value| value + 1);
        let mut excluded = 0usize;
        let mut counts: BTreeMap<Vec<String>, (usize, Vec<usize>)> = BTreeMap::new();
        for (index, window) in tokens.windows(n).enumerate() {
            if window.iter().any(|token| self.stopwords.contains(token)) {
                excluded = add(excluded, 1)?;
                continue;
            }
            if let Some((count, positions)) = counts.get_mut(window) {
                *count = add(*count, 1)?;
                if positions.len() < options.limits.position_samples {
                    positions.push(index + 1);
                }
            } else {
                *unique = add(*unique, 1)?;
                ensure!(
                    *unique <= options.limits.max_unique_ngrams,
                    "Extended per-document unique ngram budget exceeded"
                );
                let size = window
                    .iter()
                    .try_fold(0usize, |sum, token| add(sum, token.len()))?;
                *keys = add(*keys, size)?;
                ensure!(
                    *keys <= options.limits.max_key_bytes,
                    "Extended per-document key byte budget exceeded"
                );
                counts.insert(window.to_vec(), (1, vec![index + 1]));
            }
        }
        Ok(crate::ContentNgramDistribution {
            n,
            candidate_windows,
            excluded_stopword_windows: excluded,
            included_windows: candidate_windows - excluded,
            rows: counts
                .into_iter()
                .map(
                    |(tokens, (occurrences, first_positions))| crate::ContentNgram {
                        tokens,
                        occurrences,
                        positions_sampled: occurrences > first_positions.len(),
                        first_positions,
                    },
                )
                .collect(),
        })
    }
}
fn extended_tokenize(
    text: &str,
    mark: &Regex,
    limits: &crate::ContentAnalysisLimits,
) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut start = None;
    for (offset, ch) in text.char_indices() {
        let mut encoded = [0u8; 4];
        let included = ch.is_alphanumeric()
            || (start.is_some() && mark.is_match(ch.encode_utf8(&mut encoded)));
        if included {
            let first = *start.get_or_insert(offset);
            ensure!(
                offset + ch.len_utf8() - first <= limits.max_token_bytes,
                "Token byte budget exceeded"
            );
        } else if let Some(first) = start.take() {
            ensure!(
                result.len() < limits.max_tokens_per_document,
                "Token count budget exceeded"
            );
            result.push(text[first..offset].to_owned());
        }
    }
    if let Some(first) = start {
        ensure!(
            result.len() < limits.max_tokens_per_document,
            "Token count budget exceeded"
        );
        result.push(text[first..].to_owned());
    }
    Ok(result)
}
fn extended_case(value: &str, behavior: ContentCaseBehavior) -> String {
    match behavior {
        ContentCaseBehavior::Preserve => value.to_owned(),
        ContentCaseBehavior::UnicodeLowercase => value.to_lowercase(),
    }
}

struct ByteBudget {
    bytes: usize,
    cap: usize,
}
impl Write for ByteBudget {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(b.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("Ngram retained result exceeds64MiB"))?;
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "ngram_corpus_tests.rs"]
mod tests;
