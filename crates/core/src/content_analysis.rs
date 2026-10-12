//! Independent, deterministic analysis of explicitly supplied text. No I/O or fetching.
use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

pub const CONTENT_ANALYSIS_ENGINE: &str = "open-seo-spider/content-analysis-v1";
pub const CONTENT_TOKEN_PROFILE: &str = "unicode-alphanumeric-mark-runs-no-normalization-v1";
pub const CONTENT_SIMILARITY_PROFILE: &str = "exact-token-shingle-set-jaccard-v1";
pub const CONTENT_READABILITY_PROFILE: &str = "declared-en-de-vowel-groups-terminal-e-v1";

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentCaseBehavior {
    Preserve,
    #[default]
    UnicodeLowercase,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContentAnalysisConfig {
    pub case_behavior: ContentCaseBehavior,
    /// Exact single-token stopwords. A window containing one is excluded, not bridged.
    pub stopwords: Vec<String>,
    pub similarity: ContentSimilarityOptions,
    pub readability: ContentReadabilityOptions,
    pub limits: ContentAnalysisLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContentSimilarityOptions {
    pub shingle_size: usize,
    pub threshold: f64,
    pub min_tokens: usize,
}
impl Default for ContentSimilarityOptions {
    fn default() -> Self {
        Self {
            shingle_size: 3,
            threshold: 0.9,
            min_tokens: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContentReadabilityOptions {
    pub min_words: usize,
}
impl Default for ContentReadabilityOptions {
    fn default() -> Self {
        Self { min_words: 30 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContentAnalysisLimits {
    pub max_documents: usize,
    pub max_input_bytes: usize,
    pub max_text_bytes: usize,
    pub max_tokens_per_document: usize,
    pub max_token_bytes: usize,
    pub max_unique_ngrams_per_document: usize,
    pub max_ngram_key_bytes: usize,
    pub max_pairs: usize,
    pub max_pair_feature_visits: usize,
    pub max_position_samples: usize,
    pub max_shared_shingle_samples: usize,
    pub max_output_bytes: usize,
}
impl Default for ContentAnalysisLimits {
    fn default() -> Self {
        Self {
            max_documents: 128,
            max_input_bytes: 8 * 1024 * 1024,
            max_text_bytes: 512 * 1024,
            max_tokens_per_document: 50_000,
            max_token_bytes: 4096,
            max_unique_ngrams_per_document: 100_000,
            max_ngram_key_bytes: 16 * 1024 * 1024,
            max_pairs: 8192,
            max_pair_feature_visits: 20_000_000,
            max_position_samples: 16,
            max_shared_shingle_samples: 16,
            max_output_bytes: 16 * 1024 * 1024,
        }
    }
}
impl ContentAnalysisLimits {
    pub fn validate(&self) -> Result<()> {
        let ceiling = Self::default();
        macro_rules! bounded {
            ($($field:ident),+ $(,)?) => { $(
                ensure!(self.$field > 0 && self.$field <= ceiling.$field,
                    "{} must be positive and at most {}", stringify!($field), ceiling.$field);
            )+ };
        }
        bounded!(
            max_documents,
            max_input_bytes,
            max_text_bytes,
            max_tokens_per_document,
            max_token_bytes,
            max_unique_ngrams_per_document,
            max_ngram_key_bytes,
            max_pairs,
            max_pair_feature_visits,
            max_position_samples,
            max_shared_shingle_samples,
            max_output_bytes
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentTextInput {
    pub id: String,
    /// Caller-declared source, e.g. manual_text or selected_saved_extraction. Not attested.
    pub source: String,
    pub text: Option<String>,
    pub language: Option<String>,
    /// Completeness of this supplied text in its declared source scope, not a website claim.
    pub complete: bool,
    #[serde(default)]
    pub truncation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentAnalysisInput {
    pub corpus_id: String,
    pub corpus_scope: String,
    pub corpus_complete: bool,
    #[serde(default)]
    pub config: ContentAnalysisConfig,
    pub documents: Vec<ContentTextInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentTextStatus {
    Missing,
    Empty,
    Complete,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentNgram {
    pub tokens: Vec<String>,
    pub occurrences: usize,
    /// One-based positions in the original token stream; overlapping windows count.
    pub first_positions: Vec<usize>,
    pub positions_sampled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentNgramDistribution {
    pub n: usize,
    pub candidate_windows: usize,
    pub excluded_stopword_windows: usize,
    pub included_windows: usize,
    pub rows: Vec<ContentNgram>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentReadability {
    pub profile: String,
    pub declared_language: Option<String>,
    pub formula: Option<String>,
    pub words: usize,
    pub sentences: usize,
    pub syllables: Option<usize>,
    /// Unclamped formula result for the supplied sample; never a school-grade label.
    pub score: Option<f64>,
    pub reason: Option<String>,
    pub unsupported_tokens: usize,
    pub unsupported_token_samples: Vec<String>,
    pub sample_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDocumentAnalysis {
    pub id: String,
    pub source: String,
    pub status: ContentTextStatus,
    pub text_bytes: Option<usize>,
    pub text_sha256: Option<String>,
    pub declared_complete: bool,
    pub truncation_reason: Option<String>,
    pub token_count: usize,
    pub stopword_tokens: usize,
    pub normalized_token_stream_sha256: Option<String>,
    pub unique_similarity_shingles: usize,
    pub ngrams: Vec<ContentNgramDistribution>,
    pub readability: ContentReadability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSimilarityPair {
    pub left_id: String,
    pub right_id: String,
    pub shingle_size: usize,
    pub threshold: f64,
    pub left_unique_shingles: usize,
    pub right_unique_shingles: usize,
    pub intersection: Option<usize>,
    pub union: Option<usize>,
    pub observed_jaccard: Option<f64>,
    /// Unknown for missing, short/empty-feature or incomplete samples.
    pub near_duplicate: Option<bool>,
    pub identical_supplied_text: Option<bool>,
    pub sample_complete: bool,
    pub reason: Option<String>,
    pub shared_shingle_samples: Vec<Vec<String>>,
    pub shared_samples_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentCorpusEvidence {
    pub id: String,
    pub scope: String,
    pub declared_complete: bool,
    pub all_supplied_text_complete: bool,
    pub documents: usize,
    pub missing_text_documents: usize,
    pub incomplete_text_documents: usize,
    pub possible_pairs: usize,
    pub pairs_emitted: usize,
    pub pairs_scored: usize,
    pub pair_feature_visits: usize,
    pub analysis_complete_for_supplied_inputs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAnalysisResult {
    pub engine: String,
    pub token_profile: String,
    pub similarity_profile: String,
    pub lowercase_unicode_version: (u8, u8, u8),
    pub token_character_classes: String,
    pub config: ContentAnalysisConfig,
    pub config_sha256: String,
    /// Hash of this typed serialized request, not an original uploaded file attestation.
    pub input_sha256: String,
    pub corpus: ContentCorpusEvidence,
    pub documents: Vec<ContentDocumentAnalysis>,
    pub pairs: Vec<ContentSimilarityPair>,
    pub website_requests: usize,
    pub limitations: Vec<String>,
}

struct PreparedDocument {
    tokens: Vec<String>,
    shingles: BTreeSet<Vec<String>>,
}

/// Analyze only explicit saved/manual text inputs. No URL fetching or body inference.
pub fn analyze_content(input: &ContentAnalysisInput) -> Result<ContentAnalysisResult> {
    let config = &input.config;
    let limits = &config.limits;
    limits.validate()?;
    label(&input.corpus_id, "corpus_id", 128)?;
    label(&input.corpus_scope, "corpus_scope", 256)?;
    ensure!(
        !input.documents.is_empty() && input.documents.len() <= limits.max_documents,
        "documents must contain 1..=max_documents rows"
    );
    serialized_bytes(input, limits.max_input_bytes)?;
    ensure!(
        (1..=3).contains(&config.similarity.shingle_size),
        "shingle_size must be 1, 2 or 3"
    );
    ensure!(
        config.similarity.threshold.is_finite()
            && (0.0..=1.0).contains(&config.similarity.threshold),
        "similarity threshold must be finite and in 0..=1"
    );
    ensure!(
        config.similarity.min_tokens >= config.similarity.shingle_size
            && config.similarity.min_tokens <= limits.max_tokens_per_document,
        "min_tokens must be at least shingle_size and within the token budget"
    );
    ensure!(
        config.readability.min_words > 0
            && config.readability.min_words <= limits.max_tokens_per_document,
        "readability min_words must be positive and within the token budget"
    );
    ensure!(
        config.stopwords.len() <= 1024,
        "at most 1024 explicit stopwords are allowed"
    );
    // A scalar's General_Category Mark allows combining marks only after a token start.
    let mark = Regex::new(r"^\p{M}$")?;
    let mut stopwords = BTreeSet::new();
    for value in &config.stopwords {
        let words = tokenize(value, &mark, limits)?;
        ensure!(
            words.len() == 1 && words[0] == *value,
            "stopword must be exactly one unpadded token"
        );
        let normalized = apply_case(value, config.case_behavior);
        ensure!(
            normalized.len() <= limits.max_token_bytes,
            "case-normalized stopword exceeds byte budget"
        );
        ensure!(
            stopwords.insert(normalized),
            "duplicate stopword after configured case handling"
        );
    }
    let possible_pairs = pair_count(input.documents.len())?;
    ensure!(
        possible_pairs <= limits.max_pairs,
        "corpus exceeds pair budget"
    );
    let mut ids = BTreeSet::new();
    let mut prepared = Vec::new();
    let mut documents = Vec::new();
    let mut key_bytes = 0usize;
    for document in &input.documents {
        label(&document.id, "document id", 128)?;
        label(&document.source, "text source", 128)?;
        ensure!(ids.insert(&document.id), "duplicate document id");
        if let Some(language) = &document.language {
            label(language, "language", 128)?;
        }
        if let Some(reason) = &document.truncation_reason {
            label(reason, "truncation_reason", 1024)?;
            ensure!(
                !document.complete,
                "complete text cannot declare truncation"
            );
        }
        ensure!(
            document.text.is_some() || !document.complete,
            "missing text cannot be declared complete"
        );
        if let Some(text) = &document.text {
            ensure!(
                text.len() <= limits.max_text_bytes,
                "text exceeds byte budget"
            );
        }
        let observed_tokens = tokenize(document.text.as_deref().unwrap_or(""), &mark, limits)?;
        let mut tokens = Vec::with_capacity(observed_tokens.len());
        for token in &observed_tokens {
            let token = apply_case(token, config.case_behavior);
            ensure!(
                token.len() <= limits.max_token_bytes,
                "case-normalized token exceeds byte budget"
            );
            tokens.push(token);
        }
        let mut distributions = Vec::new();
        let mut shingles = BTreeSet::new();
        let mut unique_count = 0usize;
        for n in 1..=3 {
            let mut counts: BTreeMap<Vec<String>, (usize, Vec<usize>)> = BTreeMap::new();
            let candidate_windows = tokens.len().checked_sub(n).map_or(0, |count| count + 1);
            let mut excluded = 0usize;
            for (index, window) in tokens.windows(n).enumerate() {
                if window.iter().any(|token| stopwords.contains(token)) {
                    excluded = checked_add(excluded, 1, "stopword windows")?;
                    continue;
                }
                if let Some((count, positions)) = counts.get_mut(window) {
                    *count = checked_add(*count, 1, "ngram occurrences")?;
                    if positions.len() < limits.max_position_samples {
                        positions.push(index + 1);
                    }
                } else {
                    unique_count = checked_add(unique_count, 1, "unique ngrams")?;
                    ensure!(
                        unique_count <= limits.max_unique_ngrams_per_document,
                        "unique ngram budget exceeded"
                    );
                    let size = window.iter().try_fold(0usize, |sum, token| {
                        checked_add(sum, token.len(), "ngram key bytes")
                    })?;
                    key_bytes = checked_add(key_bytes, size, "corpus ngram key bytes")?;
                    ensure!(
                        key_bytes <= limits.max_ngram_key_bytes,
                        "corpus ngram key byte budget exceeded"
                    );
                    counts.insert(window.to_vec(), (1, vec![index + 1]));
                }
            }
            if n == config.similarity.shingle_size {
                shingles.extend(counts.keys().cloned());
            }
            distributions.push(ContentNgramDistribution {
                n,
                candidate_windows,
                excluded_stopword_windows: excluded,
                included_windows: candidate_windows - excluded,
                rows: counts
                    .into_iter()
                    .map(|(tokens, (occurrences, first_positions))| ContentNgram {
                        tokens,
                        occurrences,
                        positions_sampled: occurrences > first_positions.len(),
                        first_positions,
                    })
                    .collect(),
            });
        }
        let status = match document.text.as_deref() {
            None => ContentTextStatus::Missing,
            Some(_) if !document.complete => ContentTextStatus::Incomplete,
            Some(_) if tokens.is_empty() => ContentTextStatus::Empty,
            Some(_) => ContentTextStatus::Complete,
        };
        documents.push(ContentDocumentAnalysis {
            id: document.id.clone(),
            source: document.source.clone(),
            status,
            text_bytes: document.text.as_ref().map(String::len),
            text_sha256: document.text.as_ref().map(|text| sha(text.as_bytes())),
            declared_complete: document.complete,
            truncation_reason: document.truncation_reason.clone(),
            token_count: tokens.len(),
            stopword_tokens: tokens
                .iter()
                .filter(|token| stopwords.contains(*token))
                .count(),
            normalized_token_stream_sha256: document.text.as_ref().map(|_| token_hash(&tokens)),
            unique_similarity_shingles: shingles.len(),
            ngrams: distributions,
            readability: readability(document, &observed_tokens, &config.readability)?,
        });
        prepared.push(PreparedDocument { tokens, shingles });
    }
    let mut visits = 0usize;
    let mut pairs = Vec::with_capacity(possible_pairs);
    for left in 0..input.documents.len() {
        for right in left + 1..input.documents.len() {
            let left_input = &input.documents[left];
            let right_input = &input.documents[right];
            let a = &prepared[left];
            let b = &prepared[right];
            let missing = left_input.text.is_none() || right_input.text.is_none();
            let short = a.tokens.len() < config.similarity.min_tokens
                || b.tokens.len() < config.similarity.min_tokens;
            let no_features = a.shingles.is_empty() || b.shingles.is_empty();
            let sample_complete = !missing && left_input.complete && right_input.complete;
            let mut intersection = 0usize;
            let mut union = 0usize;
            let mut shared = Vec::new();
            let score = if missing || short || no_features {
                None
            } else {
                visits = checked_add(
                    visits,
                    checked_add(a.shingles.len(), b.shingles.len(), "pair feature visits")?,
                    "corpus pair feature visits",
                )?;
                ensure!(
                    visits <= limits.max_pair_feature_visits,
                    "pair feature work budget exceeded"
                );
                for shingle in a.shingles.intersection(&b.shingles) {
                    intersection = checked_add(intersection, 1, "intersection")?;
                    if shared.len() < limits.max_shared_shingle_samples {
                        shared.push(shingle.clone());
                    }
                }
                union = checked_add(a.shingles.len(), b.shingles.len(), "union")? - intersection;
                Some(intersection as f64 / union as f64)
            };
            pairs.push(ContentSimilarityPair {
                left_id: left_input.id.clone(),
                right_id: right_input.id.clone(),
                shingle_size: config.similarity.shingle_size,
                threshold: config.similarity.threshold,
                left_unique_shingles: a.shingles.len(),
                right_unique_shingles: b.shingles.len(),
                intersection: score.map(|_| intersection),
                union: score.map(|_| union),
                observed_jaccard: score,
                near_duplicate: score
                    .filter(|_| sample_complete)
                    .map(|score| score >= config.similarity.threshold),
                identical_supplied_text: (!missing).then(|| left_input.text == right_input.text),
                sample_complete,
                reason: if missing {
                    Some("missing_text")
                } else if short {
                    Some("too_few_tokens")
                } else if no_features {
                    Some("no_shingles_after_stopword_filter")
                } else if !sample_complete {
                    Some("incomplete_sample_score_only")
                } else {
                    None
                }
                .map(str::to_owned),
                shared_samples_truncated: intersection > shared.len(),
                shared_shingle_samples: shared,
            });
        }
    }
    let result = ContentAnalysisResult {
        engine: CONTENT_ANALYSIS_ENGINE.into(), token_profile: CONTENT_TOKEN_PROFILE.into(),
        similarity_profile: CONTENT_SIMILARITY_PROFILE.into(), lowercase_unicode_version: char::UNICODE_VERSION,
        token_character_classes: "Rust std Unicode Alphabetic/Numeric; regex General_Category Mark after a token start; locked dependency tables".into(),
        config: config.clone(), config_sha256: sha(&serde_json::to_vec(config)?),
        input_sha256: sha(&serde_json::to_vec(input)?),
        corpus: ContentCorpusEvidence {
            id: input.corpus_id.clone(), scope: input.corpus_scope.clone(), declared_complete: input.corpus_complete,
            all_supplied_text_complete: input.documents.iter().all(|document| document.text.is_some() && document.complete),
            documents: documents.len(), missing_text_documents: input.documents.iter().filter(|document| document.text.is_none()).count(),
            incomplete_text_documents: input.documents.iter().filter(|document| document.text.is_some() && !document.complete).count(),
            possible_pairs, pairs_emitted: pairs.len(), pairs_scored: pairs.iter().filter(|pair| pair.observed_jaccard.is_some()).count(),
            pair_feature_visits: visits, analysis_complete_for_supplied_inputs: true,
        }, documents, pairs, website_requests: 0,
        limitations: vec![
            "Only supplied text is analyzed; no HTML body, boilerplate removal, website completeness or vendor parity is inferred.".into(),
            "Token profile has no Unicode normalization, stemming, dictionary segmentation or full case folding; punctuation breaks tokens and ngrams span sentence boundaries.".into(),
            "Jaccard uses exact unique token shingles, not counts or MinHash; score 1 does not prove byte-identical text.".into(),
            "Readability uses declared language and a versioned syllable/sentence heuristic, not phonetic lookup, language detection, comprehension or Google ranking prediction.".into(),
        ],
    };
    serialized_bytes(&result, limits.max_output_bytes)?;
    Ok(result)
}

fn label(value: &str, name: &str, max: usize) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control),
        "invalid {name}"
    );
    Ok(())
}

fn tokenize(text: &str, mark: &Regex, limits: &ContentAnalysisLimits) -> Result<Vec<String>> {
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
                "token byte budget exceeded"
            );
        } else if let Some(first) = start.take() {
            ensure!(
                result.len() < limits.max_tokens_per_document,
                "token count budget exceeded"
            );
            result.push(text[first..offset].to_owned());
        }
    }
    if let Some(first) = start {
        ensure!(
            result.len() < limits.max_tokens_per_document,
            "token count budget exceeded"
        );
        result.push(text[first..].to_owned());
    }
    Ok(result)
}

fn apply_case(value: &str, behavior: ContentCaseBehavior) -> String {
    match behavior {
        ContentCaseBehavior::Preserve => value.to_owned(),
        ContentCaseBehavior::UnicodeLowercase => value.to_lowercase(),
    }
}

fn sentence_count(text: &str) -> usize {
    let mut sentences = 0usize;
    let mut pending_word = false;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            pending_word = true;
        } else if matches!(ch, '.' | '!' | '?') && pending_word {
            sentences += 1;
            pending_word = false;
        }
    }
    sentences + usize::from(pending_word)
}

fn readability(
    document: &ContentTextInput,
    words: &[String],
    options: &ContentReadabilityOptions,
) -> Result<ContentReadability> {
    let language = document.language.as_deref().and_then(supported_language);
    let mut out = ContentReadability {
        profile: CONTENT_READABILITY_PROFILE.into(),
        declared_language: document.language.clone(),
        formula: language.map(|language| {
            if language == "en" {
                "206.835 - 1.015*(words/sentences) - 84.6*(syllables/words)"
            } else {
                "180 - (words/sentences) - 58.5*(syllables/words)"
            }
            .into()
        }),
        words: words.len(),
        sentences: sentence_count(document.text.as_deref().unwrap_or("")),
        syllables: None,
        score: None,
        reason: None,
        unsupported_tokens: 0,
        unsupported_token_samples: Vec::new(),
        sample_complete: document.text.is_some() && document.complete,
    };
    let Some(language) = language else {
        out.reason = Some("unsupported_or_missing_declared_language".into());
        return Ok(out);
    };
    let mut syllables = 0usize;
    for word in words {
        match estimate_syllables(word, language) {
            Some(count) => syllables = checked_add(syllables, count, "syllables")?,
            None => {
                out.unsupported_tokens += 1;
                if out.unsupported_token_samples.len() < 8 {
                    out.unsupported_token_samples.push(word.clone());
                }
            }
        }
    }
    if out.unsupported_tokens == 0 && !words.is_empty() {
        out.syllables = Some(syllables);
    }
    out.reason = if document.text.is_none() {
        Some("missing_text")
    } else if words.is_empty() {
        Some("empty_text")
    } else if !document.complete {
        Some("incomplete_text")
    } else if out.unsupported_tokens > 0 {
        Some("unsupported_syllable_tokens")
    } else if words.len() < options.min_words {
        Some("too_few_words")
    } else if out.sentences == 0 {
        Some("no_sentence")
    } else {
        None
    }
    .map(str::to_owned);
    if out.reason.is_none() {
        let words = words.len() as f64;
        let sentence_length = words / out.sentences as f64;
        let word_syllables = syllables as f64 / words;
        out.score = Some(if language == "en" {
            206.835 - 1.015 * sentence_length - 84.6 * word_syllables
        } else {
            180.0 - sentence_length - 58.5 * word_syllables
        });
    }
    Ok(out)
}

fn supported_language(value: &str) -> Option<&'static str> {
    let mut parts = value.split('-');
    let primary = parts.next()?;
    if !parts.all(|part| {
        !part.is_empty() && part.len() <= 8 && part.bytes().all(|ch| ch.is_ascii_alphanumeric())
    }) {
        return None;
    }
    if primary.eq_ignore_ascii_case("en") {
        Some("en")
    } else if primary.eq_ignore_ascii_case("de") {
        Some("de")
    } else {
        None
    }
}

fn estimate_syllables(word: &str, language: &str) -> Option<usize> {
    let lower = word.to_lowercase();
    if !lower.chars().all(|ch| {
        ch.is_ascii_lowercase() || (language == "de" && matches!(ch, 'ä' | 'ö' | 'ü' | 'ß'))
    }) {
        return None;
    }
    let mut groups = 0usize;
    let mut previous_vowel = false;
    for ch in lower.chars() {
        let vowel = matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
            || (language == "de" && matches!(ch, 'ä' | 'ö' | 'ü'));
        if vowel && !previous_vowel {
            groups += 1;
        }
        previous_vowel = vowel;
    }
    if language == "en" && groups > 1 && lower.ends_with('e') {
        let consonant_le = lower.ends_with("le")
            && lower
                .as_bytes()
                .get(lower.len().saturating_sub(3))
                .is_some_and(|ch| !matches!(*ch, b'a' | b'e' | b'i' | b'o' | b'u' | b'y'));
        if !consonant_le {
            groups -= 1;
        }
    }
    Some(groups.max(1))
}

fn token_hash(tokens: &[String]) -> String {
    let mut hash = Sha256::new();
    for token in tokens {
        hash.update((token.len() as u64).to_le_bytes());
        hash.update(token.as_bytes());
    }
    format!("{hash:x}", hash = hash.finalize())
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn checked_add(a: usize, b: usize, name: &str) -> Result<usize> {
    a.checked_add(b).with_context(|| format!("{name} overflow"))
}
fn pair_count(documents: usize) -> Result<usize> {
    documents
        .checked_mul(documents.saturating_sub(1))
        .map(|count| count / 2)
        .context("pair count overflow")
}
struct BoundedWriter {
    bytes: usize,
    limit: usize,
}
impl Write for BoundedWriter {
    fn write(&mut self, value: &[u8]) -> std::io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(value.len())
            .filter(|count| *count <= self.limit)
            .ok_or_else(|| std::io::Error::other("serialized byte budget exceeded"))?;
        Ok(value.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn serialized_bytes<T: Serialize>(value: &T, limit: usize) -> Result<usize> {
    let mut writer = BoundedWriter { bytes: 0, limit };
    serde_json::to_writer(&mut writer, value)?;
    Ok(writer.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document(id: &str, text: &str, language: Option<&str>) -> ContentTextInput {
        ContentTextInput {
            id: id.into(),
            source: "authored_fixture".into(),
            text: Some(text.into()),
            language: language.map(str::to_owned),
            complete: true,
            truncation_reason: None,
        }
    }
    fn input(documents: Vec<ContentTextInput>) -> ContentAnalysisInput {
        ContentAnalysisInput {
            corpus_id: "authored".into(),
            corpus_scope: "explicit_test_texts".into(),
            corpus_complete: true,
            config: ContentAnalysisConfig {
                similarity: ContentSimilarityOptions {
                    min_tokens: 3,
                    ..Default::default()
                },
                readability: ContentReadabilityOptions { min_words: 1 },
                ..Default::default()
            },
            documents,
        }
    }
    fn row<'a>(doc: &'a ContentDocumentAnalysis, n: usize, tokens: &[&str]) -> &'a ContentNgram {
        doc.ngrams[n - 1]
            .rows
            .iter()
            .find(|row| {
                row.tokens
                    .iter()
                    .map(String::as_str)
                    .eq(tokens.iter().copied())
            })
            .unwrap()
    }
    #[test]
    fn overlapping_windows_keep_occurrences_positions_and_no_hash_collisions() {
        let result = analyze_content(&input(vec![document(
            "a",
            "alpha beta alpha beta alpha",
            Some("en"),
        )]))
        .unwrap();
        let doc = &result.documents[0];
        assert_eq!(row(doc, 1, &["alpha"]).occurrences, 3);
        assert_eq!(row(doc, 2, &["alpha", "beta"]).first_positions, [1, 3]);
        assert_eq!(row(doc, 3, &["alpha", "beta", "alpha"]).occurrences, 2);
        assert_eq!(
            doc.ngrams
                .iter()
                .map(|n| n.included_windows)
                .collect::<Vec<_>>(),
            [5, 4, 3]
        );
        assert_eq!(result.website_requests, 0);
        assert_eq!(result.corpus.pairs_emitted, 0);
    }
    #[test]
    fn unicode_marks_case_and_punctuation_follow_an_explicit_profile() {
        let mut request = input(vec![document(
            "u",
            "CAFÉ cafe\u{301} Straße STRASSE 東京 🙂 l'amour １２",
            None,
        )]);
        let result = analyze_content(&request).unwrap();
        let tokens: Vec<_> = result.documents[0].ngrams[0]
            .rows
            .iter()
            .map(|row| row.tokens[0].as_str())
            .collect();
        assert_eq!(
            tokens,
            [
                "amour",
                "cafe\u{301}",
                "café",
                "l",
                "strasse",
                "straße",
                "東京",
                "１２"
            ]
        );
        assert_eq!(result.documents[0].readability.score, None);
        request.config.case_behavior = ContentCaseBehavior::Preserve;
        let preserve = analyze_content(&request).unwrap();
        assert_eq!(row(&preserve.documents[0], 1, &["CAFÉ"]).occurrences, 1);
        assert_ne!(preserve.config_sha256, result.config_sha256);
    }
    #[test]
    fn stopwords_exclude_windows_without_inventing_adjacency_and_do_not_change_readability() {
        let mut request = input(vec![document(
            "a",
            "alpha THE beta alpha the beta",
            Some("en"),
        )]);
        let full = analyze_content(&request).unwrap();
        request.config.stopwords = vec!["the".into()];
        let filtered = analyze_content(&request).unwrap();
        let doc = &filtered.documents[0];
        assert_eq!(doc.token_count, 6);
        assert_eq!(doc.stopword_tokens, 2);
        assert_eq!(doc.ngrams[1].included_windows, 1);
        assert_eq!(doc.ngrams[1].rows[0].tokens, ["beta", "alpha"]);
        assert!(doc.ngrams[2].rows.is_empty());
        assert_eq!(doc.readability.score, full.documents[0].readability.score);
        request.config.stopwords.push("THE".into());
        assert!(analyze_content(&request).is_err());
    }
    #[test]
    fn exact_pairs_explain_identical_controlled_changes_and_threshold_boundaries() {
        let mut request = input(vec![
            document(
                "base",
                "one two three four five six seven eight nine ten",
                None,
            ),
            document(
                "identical",
                "one two three four five six seven eight nine ten",
                None,
            ),
            document(
                "changed",
                "one two three four five different seven eight nine ten",
                None,
            ),
        ]);
        request.config.similarity.threshold = 5.0 / 11.0;
        let result = analyze_content(&request).unwrap();
        assert_eq!(result.pairs.len(), 3);
        assert_eq!(result.pairs[0].observed_jaccard, Some(1.0));
        assert_eq!(result.pairs[0].identical_supplied_text, Some(true));
        let changed = &result.pairs[1];
        assert_eq!((changed.intersection, changed.union), (Some(5), Some(11)));
        assert_eq!(changed.observed_jaccard, Some(5.0 / 11.0));
        assert_eq!(changed.near_duplicate, Some(true));
        assert_eq!(changed.shared_shingle_samples.len(), 5);
        request.config.similarity.threshold = 0.5;
        assert_eq!(
            analyze_content(&request).unwrap().pairs[1].near_duplicate,
            Some(false)
        );
    }
    #[test]
    fn distinct_text_can_have_equal_shingle_sets_without_byte_identity() {
        let mut request = input(vec![
            document("a", "ONE two three", None),
            document("b", "one TWO three!", None),
        ]);
        request.config.similarity.shingle_size = 1;
        let result = analyze_content(&request).unwrap();
        assert_eq!(result.pairs[0].observed_jaccard, Some(1.0));
        assert_eq!(result.pairs[0].identical_supplied_text, Some(false));
        assert_ne!(
            result.documents[0].text_sha256,
            result.documents[1].text_sha256
        );
    }
    #[test]
    fn missing_empty_short_and_all_stopword_documents_are_unknown_not_false_matches() {
        let mut missing = document("missing", "", None);
        missing.text = None;
        missing.complete = false;
        let request = input(vec![
            missing,
            document("empty", "!!!", Some("en")),
            document("short", "cat dog", Some("en")),
        ]);
        let result = analyze_content(&request).unwrap();
        assert_eq!(result.documents[0].status, ContentTextStatus::Missing);
        assert_eq!(result.documents[1].status, ContentTextStatus::Empty);
        assert!(
            result
                .pairs
                .iter()
                .all(|pair| pair.observed_jaccard.is_none() && pair.near_duplicate.is_none())
        );
        assert!(
            result
                .pairs
                .iter()
                .all(|pair| pair.intersection.is_none() && pair.union.is_none())
        );
        let mut stops = input(vec![
            document("a", "the the the", None),
            document("b", "the the the", None),
        ]);
        stops.config.stopwords = vec!["the".into()];
        assert_eq!(
            analyze_content(&stops).unwrap().pairs[0].reason.as_deref(),
            Some("no_shingles_after_stopword_filter")
        );
    }
    #[test]
    fn incomplete_samples_preserve_scores_but_cannot_establish_duplicate_or_readability_classification()
     {
        let mut a = document("a", "The cat sits on the mat.", Some("en"));
        a.complete = false;
        a.truncation_reason = Some("saved_excerpt_only".into());
        let mut request = input(vec![
            a,
            document("b", "The cat sits on the mat.", Some("en")),
        ]);
        request.corpus_complete = false;
        let result = analyze_content(&request).unwrap();
        assert_eq!(result.pairs[0].observed_jaccard, Some(1.0));
        assert_eq!(result.pairs[0].near_duplicate, None);
        assert_eq!(
            result.documents[0].readability.reason.as_deref(),
            Some("incomplete_text")
        );
        assert_eq!(result.documents[0].readability.score, None);
        assert_eq!(result.corpus.incomplete_text_documents, 1);
        assert!(!result.corpus.declared_complete && !result.corpus.all_supplied_text_complete);
    }
    #[test]
    fn supported_readability_formulas_use_all_original_words_and_unclamped_results() {
        let result = analyze_content(&input(vec![
            document("en", "The cat sits on the mat.", Some("en-US")),
            document("de", "Der Hund ist im Haus.", Some("de-DE")),
        ]))
        .unwrap();
        assert_eq!(
            (
                result.documents[0].readability.words,
                result.documents[0].readability.sentences,
                result.documents[0].readability.syllables
            ),
            (6, 1, Some(6))
        );
        assert!((result.documents[0].readability.score.unwrap() - 116.145).abs() < 1e-9);
        assert_eq!(result.documents[1].readability.syllables, Some(5));
        assert!((result.documents[1].readability.score.unwrap() - 116.5).abs() < 1e-9);
        assert_eq!(estimate_syllables("table", "en"), Some(2));
        assert_eq!(estimate_syllables("hope", "en"), Some(1));
        assert_eq!(estimate_syllables("gehen", "de"), Some(2));
    }
    #[test]
    fn unsupported_languages_scripts_numbers_and_short_readability_remain_unknown() {
        let mut request = input(vec![
            document("fr", "Le chat dort.", Some("fr")),
            document("script", "The cat 東京 sleeps.", Some("en")),
            document("digits", "Der Hund hat 2 Beine.", Some("de")),
            document("short", "The cat sleeps.", Some("en")),
        ]);
        request.config.readability.min_words = 30;
        let result = analyze_content(&request).unwrap();
        assert!(
            result
                .documents
                .iter()
                .all(|document| document.readability.score.is_none())
        );
        assert_eq!(
            result.documents[1].readability.unsupported_token_samples,
            ["東京"]
        );
        assert_eq!(result.documents[2].readability.unsupported_tokens, 1);
        assert_eq!(
            result.documents[3].readability.reason.as_deref(),
            Some("too_few_words")
        );
    }
    #[test]
    fn deterministic_pair_order_and_full_serialized_roundtrip_are_reproducible() {
        let request = input(vec![
            document("z", "alpha beta gamma", None),
            document("a", "alpha beta gamma", None),
            document("m", "different more text", None),
        ]);
        let first = analyze_content(&request).unwrap();
        let second = analyze_content(&request).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(
            first
                .pairs
                .iter()
                .map(|pair| (pair.left_id.as_str(), pair.right_id.as_str()))
                .collect::<Vec<_>>(),
            [("z", "a"), ("z", "m"), ("a", "m")]
        );
        let recovered: ContentAnalysisResult =
            serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
        assert_eq!(recovered.input_sha256, first.input_sha256);
    }
    #[test]
    fn configuration_metadata_and_unknown_shapes_fail_before_results() {
        let base = input(vec![document("a", "one two three", None)]);
        let mut request = base.clone();
        request.documents.push(request.documents[0].clone());
        assert!(analyze_content(&request).is_err());
        let mut request = base.clone();
        request.documents[0].source = "bad\nsource".into();
        assert!(analyze_content(&request).is_err());
        let mut request = base.clone();
        request.documents[0].text = None;
        assert!(analyze_content(&request).is_err());
        let mut request = base.clone();
        request.documents[0].truncation_reason = Some("cut".into());
        assert!(analyze_content(&request).is_err());
        for threshold in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
            let mut request = base.clone();
            request.config.similarity.threshold = threshold;
            assert!(analyze_content(&request).is_err());
        }
        let mut request = base.clone();
        request.config.stopwords = vec!["two words".into()];
        assert!(analyze_content(&request).is_err());
        let mut value = serde_json::to_value(base).unwrap();
        value["config"]["guessed_field"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ContentAnalysisInput>(value).is_err());
    }
    #[test]
    fn all_positive_budgets_are_capped_and_real_work_bounds_fail_without_silent_truncation() {
        let fields = [
            "max_documents",
            "max_input_bytes",
            "max_text_bytes",
            "max_tokens_per_document",
            "max_token_bytes",
            "max_unique_ngrams_per_document",
            "max_ngram_key_bytes",
            "max_pairs",
            "max_pair_feature_visits",
            "max_position_samples",
            "max_shared_shingle_samples",
            "max_output_bytes",
        ];
        for field in fields {
            let mut value = serde_json::to_value(ContentAnalysisLimits::default()).unwrap();
            value[field] = serde_json::json!(0);
            assert!(
                serde_json::from_value::<ContentAnalysisLimits>(value.clone())
                    .unwrap()
                    .validate()
                    .is_err()
            );
            value[field] = serde_json::json!(usize::MAX);
            assert!(
                serde_json::from_value::<ContentAnalysisLimits>(value)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        let base = input(vec![
            document("a", "one two three four five", None),
            document("b", "one two three four five", None),
            document("c", "six seven eight nine ten", None),
        ]);
        for field in [
            "max_documents",
            "max_input_bytes",
            "max_text_bytes",
            "max_tokens_per_document",
            "max_token_bytes",
            "max_unique_ngrams_per_document",
            "max_ngram_key_bytes",
            "max_pairs",
            "max_pair_feature_visits",
            "max_output_bytes",
        ] {
            let mut value = serde_json::to_value(&base).unwrap();
            value["config"]["limits"][field] = serde_json::json!(1);
            assert!(
                analyze_content(&serde_json::from_value::<ContentAnalysisInput>(value).unwrap())
                    .is_err(),
                "{field}"
            );
        }
        assert!(pair_count(usize::MAX).is_err());
        assert!(checked_add(usize::MAX, 1, "fixture").is_err());
        let mut token_bound = base;
        token_bound.config.limits.max_tokens_per_document = 3;
        assert!(
            analyze_content(&token_bound)
                .unwrap_err()
                .to_string()
                .contains("token count budget")
        );
    }
    #[test]
    fn sample_caps_are_explicit_but_counts_and_scores_remain_complete() {
        let mut request = input(vec![
            document("a", "cat cat cat cat cat", None),
            document("b", "cat cat cat cat cat", None),
        ]);
        request.config.limits.max_position_samples = 1;
        request.config.limits.max_shared_shingle_samples = 1;
        let result = analyze_content(&request).unwrap();
        let unigram = row(&result.documents[0], 1, &["cat"]);
        assert_eq!(unigram.occurrences, 5);
        assert_eq!(unigram.first_positions, [1]);
        assert!(unigram.positions_sampled);
        assert_eq!(result.pairs[0].observed_jaccard, Some(1.0));
        assert_eq!(result.corpus.pairs_scored, 1);
        request.documents = vec![
            document("a", "one two three four", None),
            document("b", "one two three four", None),
        ];
        let varied = analyze_content(&request).unwrap();
        assert_eq!(varied.pairs[0].intersection, Some(2));
        assert_eq!(varied.pairs[0].shared_shingle_samples.len(), 1);
        assert!(varied.pairs[0].shared_samples_truncated);
    }

    #[test]
    fn unicode_start_marks_joiners_and_expanding_lowercase_have_exact_boundaries() {
        let mut request = input(vec![document(
            "u",
            "\u{301}A a\u{301} a\u{200d}b 中文词 İ I ı",
            None,
        )]);
        let result = analyze_content(&request).unwrap();
        let doc = &result.documents[0];
        assert_eq!(doc.token_count, 8);
        assert_eq!(row(doc, 1, &["a"]).occurrences, 2);
        assert_eq!(row(doc, 1, &["a\u{301}"]).occurrences, 1);
        assert_eq!(row(doc, 1, &["中文词"]).occurrences, 1);
        assert_eq!(row(doc, 1, &["i\u{307}"]).occurrences, 1);
        assert_eq!(row(doc, 1, &["ı"]).occurrences, 1);
        request.documents[0].text = Some("İ".into());
        request.config.limits.max_token_bytes = 2;
        assert!(
            analyze_content(&request)
                .unwrap_err()
                .to_string()
                .contains("case-normalized token")
        );
        request.documents[0].text = Some("a".into());
        request.config.stopwords = vec!["İ".into()];
        assert!(
            analyze_content(&request)
                .unwrap_err()
                .to_string()
                .contains("case-normalized stopword")
        );
        request.config.limits.max_token_bytes = 3;
        request.documents[0].text = Some("İ i\u{307}".into());
        let filtered = analyze_content(&request).unwrap();
        assert_eq!(filtered.documents[0].stopword_tokens, 2);
        assert_eq!(filtered.documents[0].ngrams[0].included_windows, 0);
    }

    #[test]
    fn shingle_sizes_distinguish_order_and_zero_threshold_is_explicit() {
        let mut request = input(vec![
            document("a", "cat dog cat dog", None),
            document("b", "cat cat dog dog", None),
        ]);
        let expected = [(1, 2, 2, 1.0), (2, 1, 4, 0.25), (3, 0, 4, 0.0)];
        for (size, intersection, union, score) in expected {
            request.config.similarity.shingle_size = size;
            let result = analyze_content(&request).unwrap();
            let pair = &result.pairs[0];
            assert_eq!(pair.intersection, Some(intersection));
            assert_eq!(pair.union, Some(union));
            assert_eq!(pair.observed_jaccard, Some(score));
            assert_eq!(pair.identical_supplied_text, Some(false));
        }
        request.config.similarity.threshold = 0.0;
        assert_eq!(
            analyze_content(&request).unwrap().pairs[0].near_duplicate,
            Some(true)
        );
        request.config.similarity.threshold = 1.0;
        assert_eq!(
            analyze_content(&request).unwrap().pairs[0].near_duplicate,
            Some(false)
        );
    }

    #[test]
    fn punctuation_splits_sentences_but_does_not_break_the_ngram_stream() {
        let result = analyze_content(&input(vec![document(
            "a",
            "cat... dog!? cat\n\ndog",
            Some("en"),
        )]))
        .unwrap();
        let doc = &result.documents[0];
        assert_eq!(doc.token_count, 4);
        assert_eq!(doc.readability.sentences, 3);
        assert_eq!(row(doc, 2, &["cat", "dog"]).first_positions, [1, 3]);
        assert_eq!(row(doc, 3, &["cat", "dog", "cat"]).first_positions, [1]);
        assert_eq!(doc.ngrams[1].candidate_windows, 3);
    }

    #[test]
    fn partial_corpus_preserves_complete_pair_evidence_and_missing_row_unknowns() {
        let mut missing = document("missing", "", None);
        missing.text = None;
        missing.complete = false;
        missing.truncation_reason = Some("body_not_retained".into());
        let mut request = input(vec![
            document("a", "one two three four", None),
            document("b", "one two three four", None),
            missing,
        ]);
        request.corpus_complete = false;
        let result = analyze_content(&request).unwrap();
        assert_eq!(result.pairs[0].near_duplicate, Some(true));
        assert!(result.pairs[0].sample_complete);
        assert_eq!(result.corpus.possible_pairs, 3);
        assert_eq!(result.corpus.pairs_scored, 1);
        assert_eq!(result.corpus.missing_text_documents, 1);
        assert!(!result.corpus.declared_complete);
        assert!(!result.corpus.all_supplied_text_complete);
        assert!(result.corpus.analysis_complete_for_supplied_inputs);
        for pair in &result.pairs[1..] {
            assert_eq!(pair.reason.as_deref(), Some("missing_text"));
            assert_eq!(pair.identical_supplied_text, None);
            assert_eq!(pair.near_duplicate, None);
            assert_eq!(pair.intersection, None);
        }
    }

    #[test]
    fn readability_language_heuristics_and_negative_scores_remain_observable() {
        for language in ["en_Us", "en-", "de--DE", "en-abcdefghi", " en", "fr-FR"] {
            assert_eq!(supported_language(language), None);
        }
        assert_eq!(supported_language("EN-us"), Some("en"));
        assert_eq!(supported_language("de-Latn-DE"), Some("de"));
        assert_eq!(estimate_syllables("FÜNF", "de"), Some(1));
        assert_eq!(estimate_syllables("größer", "de"), Some(2));
        assert_eq!(estimate_syllables("bauen", "de"), Some(1));
        assert_eq!(estimate_syllables("cafe\u{301}", "en"), None);
        let text = format!("{}.", "cat ".repeat(200));
        let result = analyze_content(&input(vec![document("a", &text, Some("en"))])).unwrap();
        let metric = &result.documents[0].readability;
        assert_eq!(
            (metric.words, metric.sentences, metric.syllables),
            (200, 1, Some(200))
        );
        assert!((metric.score.unwrap() - -80.765).abs() < 1e-9);
    }

    #[test]
    fn pair_work_budget_counts_each_eligible_pair_and_uses_inclusive_limit() {
        let mut request = input(vec![
            document("a", "one two three four", None),
            document("b", "one two three four", None),
            document("c", "five six seven eight", None),
        ]);
        request.config.limits.max_pair_feature_visits = 12;
        let result = analyze_content(&request).unwrap();
        assert_eq!(result.corpus.pair_feature_visits, 12);
        assert_eq!(result.corpus.pairs_scored, 3);
        request.config.limits.max_pair_feature_visits = 11;
        assert!(
            analyze_content(&request)
                .unwrap_err()
                .to_string()
                .contains("pair feature work")
        );
        assert_eq!(pair_count(128).unwrap(), 8128);
    }

    #[test]
    fn actual_serialized_bytes_include_json_escaping_and_all_unknown_input_shapes_fail() {
        let value = serde_json::json!({"text": "\n\t\"\\🙂"});
        let actual = serde_json::to_vec(&value).unwrap().len();
        assert_eq!(serialized_bytes(&value, actual).unwrap(), actual);
        assert!(serialized_bytes(&value, actual - 1).is_err());
        let base = serde_json::to_value(input(vec![document("a", "one two three", None)])).unwrap();
        for object in ["top", "document", "similarity", "readability", "limits"] {
            let mut value = base.clone();
            let target = match object {
                "top" => &mut value,
                "document" => &mut value["documents"][0],
                name => &mut value["config"][name],
            };
            target["unexpected"] = serde_json::json!(true);
            assert!(
                serde_json::from_value::<ContentAnalysisInput>(value).is_err(),
                "{object}"
            );
        }
    }
}
