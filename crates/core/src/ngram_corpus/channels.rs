//! Explicit saved-element profile. No HTML parsing or element concatenation.
use super::*;
use crate::{ContentAnalysisLimits, ContentCaptureSourceKind};
use sha2::{Digest, Sha256};

pub const NGRAM_ELEMENT_ENGINE: &str = "open-seo-spider/saved-element-ngram-corpus-v1";
pub const NGRAM_ELEMENT_PROFILE_VERSION: u32 = 1;
pub const MAX_NGRAM_ELEMENTS_PER_PAGE: usize = 10_000;
pub const MAX_NGRAM_ELEMENTS: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NgramHtmlTextChannel {
    Title,
    MetaDescription,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}
impl NgramHtmlTextChannel {
    pub fn heading_level(self) -> Option<u8> {
        match self {
            Self::H1 => Some(1),
            Self::H2 => Some(2),
            Self::H3 => Some(3),
            Self::H4 => Some(4),
            Self::H5 => Some(5),
            Self::H6 => Some(6),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NgramHtmlTextProfile {
    pub version: u32,
    pub channel: NgramHtmlTextChannel,
}
impl NgramHtmlTextProfile {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == NGRAM_ELEMENT_PROFILE_VERSION,
            "Unsupported saved HTML text channel version"
        );
        Ok(())
    }
}
impl<'de> Deserialize<'de> for NgramHtmlTextProfile {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            version: u32,
            channel: NgramHtmlTextChannel,
        }
        struct Object;
        impl<'de> serde::de::Visitor<'de> for Object {
            type Value = NgramHtmlTextProfile;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a versioned saved HTML text channel object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let raw = Fields::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                let p = NgramHtmlTextProfile {
                    version: raw.version,
                    channel: raw.channel,
                };
                p.validate().map_err(serde::de::Error::custom)?;
                Ok(p)
            }
        }
        d.deserialize_map(Object)
    }
}

pub const NGRAM_RETAINED_METADATA_ENGINE: &str =
    "open-seo-spider/saved-retained-metadata-ngram-corpus-v1";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NgramRetainedMetadataProfile {
    pub version: u32,
    pub channel: NgramHtmlTextChannel,
}
impl NgramRetainedMetadataProfile {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1,
            "Unsupported retained metadata channel version"
        );
        Ok(())
    }
}
impl<'de> Deserialize<'de> for NgramRetainedMetadataProfile {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let p = NgramHtmlTextProfile::deserialize(d)?;
        Ok(Self {
            version: p.version,
            channel: p.channel,
        })
    }
}
#[derive(Debug, Clone, Copy, Serialize)]
pub struct NgramRetainedMetadataIdentity<'a> {
    pub fetched_at: &'a str,
    pub rendered_at: Option<&'a str>,
    pub engine: &'a str,
    pub source: &'a str,
}
#[derive(Debug, Clone, Serialize)]
pub struct NgramRetainedMetadataSource {
    pub authority_profile: &'static str,
    pub revision: String,
    pub fetched_at: String,
    pub rendered_at: Option<String>,
    pub engine: String,
    pub source: String,
}
/// Each entry is one retained element, including an observed empty element.
/// None is unknown; Some(empty) is a complete observed set with no elements.
pub struct NgramChannelDocument<'a> {
    pub source: NgramCorpusDocument<'a>,
    pub source_kind: ContentCaptureSourceKind,
    pub source_document_url: Option<&'a str>,
    pub capture_html_sha256: Option<&'a str>,
    pub capture_status: Option<crate::PageContentCaptureStatus>,
    pub metadata_observation: Option<NgramRetainedMetadataIdentity<'a>>,
    pub elements: Option<Vec<&'a str>>,
    pub unavailable_reason: Option<&'static str>,
}
#[derive(Debug, Clone, Serialize)]
pub struct NgramElementSpan {
    pub retained_element_ordinal: usize,
    pub token_start: usize,
    pub tokens: usize,
    pub utf8_bytes: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct NgramChannelSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<NgramHtmlTextProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retained_metadata_profile: Option<NgramRetainedMetadataProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_source: Option<NgramRetainedMetadataSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_status: Option<crate::PageContentCaptureStatus>,
    pub source_kind: ContentCaptureSourceKind,
    pub source_document_url: Option<String>,
    pub capture_html_sha256: Option<String>,
    pub channel_revision: String,
    pub element_set_sha256: Option<String>,
    pub hash_profile: &'static str,
    pub position_unit: &'static str,
    pub elements: Option<Vec<NgramElementSpan>>,
    pub unavailable_reason: Option<&'static str>,
}
fn digest(value: &impl Serialize) -> Result<String> {
    struct Hash {
        sha: Sha256,
        bytes: usize,
    }
    impl Write for Hash {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.bytes = self
                .bytes
                .checked_add(b.len())
                .filter(|n| *n <= 64 * 1024 * 1024)
                .ok_or_else(|| io::Error::other("Element identity exceeds64MiB"))?;
            self.sha.update(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut out = Hash {
        sha: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut out, value)?;
    Ok(format!("{:x}", out.sha.finalize()))
}

pub fn aggregate_saved_channel_ngrams(
    documents: &[NgramChannelDocument<'_>],
    options: &NgramCorpusOptions,
    profile: NgramHtmlTextProfile,
) -> Result<NgramCorpusReport> {
    aggregate_elements(documents, options, profile, None)
}
pub fn aggregate_saved_metadata_ngrams(
    documents: &[NgramChannelDocument<'_>],
    options: &NgramCorpusOptions,
    profile: NgramRetainedMetadataProfile,
) -> Result<NgramCorpusReport> {
    profile.validate()?;
    aggregate_elements(
        documents,
        options,
        NgramHtmlTextProfile {
            version: profile.version,
            channel: profile.channel,
        },
        Some(profile),
    )
}
fn aggregate_elements(
    documents: &[NgramChannelDocument<'_>],
    options: &NgramCorpusOptions,
    profile: NgramHtmlTextProfile,
    metadata_profile: Option<NgramRetainedMetadataProfile>,
) -> Result<NgramCorpusReport> {
    profile.validate()?;
    options.validate()?;
    ensure!(
        documents.len() <= options.limits.max_documents,
        "Saved document count exceeds ngram corpus limit"
    );
    // Preserve the existing option/stopword/identity/duplicate-URL validation.
    // These temporary declarations contain no text and cannot bridge elements.
    let declarations = documents
        .iter()
        .map(|d| NgramCorpusDocument {
            requested_url: d.source.requested_url,
            source_revision: d.source.source_revision,
            capture_config_sha256: d.source.capture_config_sha256,
            text_sha256: None,
            text: None,
            complete: false,
            basis: d.source.basis,
            observed_generation: d.source.observed_generation,
        })
        .collect::<Vec<_>>();
    let mut report = aggregate_saved_ngrams(&declarations, options)?;
    report.engine = if metadata_profile.is_some() {
        NGRAM_RETAINED_METADATA_ENGINE
    } else {
        NGRAM_ELEMENT_ENGINE
    };
    report.channel_profile = if metadata_profile.is_some() {
        None
    } else {
        Some(profile)
    };
    report.retained_metadata_profile = metadata_profile;
    report.source_states.clear();
    report.all_supplied_sources_complete = true;
    report.limitations = vec![
        "Version1 retained title/meta-description or one H1-H6 channel only; no raw/DOM fallback, HTML reparsing, computed visibility or image-alt substitution.",
        "Every retained element is independently tokenized with the unchanged case/stopword profile. Windows never cross elements, documents, or excluded stopwords.",
        "Each requested page is one source and one document-frequency identity, even with duplicate or empty retained elements. Token positions are one-based across successive element token spans; spans retain the element boundary.",
        "Missing/incomplete/legacy source observations are excluded, never an observed zero. Only complete matching captured HTML plus complete matching DOM facts may establish a known empty element set.",
        "The element-array hash/revision is a versioned derived analysis identity; original PageRecord/capture/policy/resource observations are unchanged. No SQL or whole-current-website attestation.",
        "At most10000 elements per page/100000 across the supplied corpus; existing text/token/key/membership/result budgets remain fail-closed. Position examples remain bounded; full counts and complete filtered CSV are separate.",
    ];
    if metadata_profile.is_some() {
        report.limitations = vec![
            "retained_html_metadata_v1: one complete saved title/meta-description/H1-H6 channel from the selected raw or rendered metadata observation; optional body capture is a separate original attachment, never a prerequisite or archived HTML file.",
            "Metadata revision binds requested/document URL, fetch/render observation, metadata engine/source, basis/generation and ordered element-array digest; imported records remain declarations, not native parser or SQL attestation.",
            "No raw/DOM fallback; pending, missing/legacy/incomplete/truncated/wrong-source observations stay unknown. A complete matching empty set is known empty; empty and duplicate elements are retained.",
            "Unchanged token/case/stopword profile; no cross-element/document/removed-stopword windows. Document frequency counts each requested page once; explicit token spans retain element boundaries.",
            "At most10000 elements/page and100000 overall with unchanged text/token/key/membership/identity/output caps. Complete filtered CSV is independent of bounded examples or UI topN.",
        ]
    }
    let mut element_count = 0usize;
    let mut extra_identities = 0usize;
    for d in documents {
        ensure!(
            documents
                .first()
                .is_none_or(|first| first.source_kind == d.source_kind),
            "A channel aggregate cannot mix raw and DOM sources"
        );
        for identity in [
            Some(d.source.requested_url),
            d.source.source_revision,
            d.source.capture_config_sha256,
            d.source_document_url,
            d.capture_html_sha256,
        ]
        .into_iter()
        .flatten()
        {
            ensure!(
                identity.len() <= 16 * 1024,
                "Channel source identity exceeds16384 bytes"
            );
            extra_identities = add(extra_identities, identity.len())?;
        }
        ensure!(
            extra_identities <= options.limits.max_identity_bytes,
            "Channel identity budget exceeded"
        );
        if let Some(identity) = &d.metadata_observation {
            for value in [
                Some(identity.fetched_at),
                identity.rendered_at,
                Some(identity.engine),
                Some(identity.source),
            ]
            .into_iter()
            .flatten()
            {
                ensure!(
                    value.len() <= 256,
                    "Metadata identity field exceeds256 UTF8 bytes"
                );
                extra_identities = add(extra_identities, value.len())?;
            }
            ensure!(
                extra_identities <= options.limits.max_identity_bytes,
                "Metadata identity budget exceeded"
            );
        }
        if metadata_profile.is_some() {
            ensure!(
                d.capture_html_sha256
                    .is_none_or(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())),
                "Invalid optional original capture HTML SHA256"
            );
            if d.source.complete {
                let identity = d
                    .metadata_observation
                    .context("Complete retained metadata has no observation identity")?;
                ensure!(
                    !identity.fetched_at.is_empty()
                        && d.source_document_url.is_some_and(|u| !u.is_empty()),
                    "Complete retained metadata requires fetch/document identity"
                );
                ensure!(
                    identity.engine == "independent_html5_dom_v1",
                    "Unknown retained metadata engine"
                );
                match d.source_kind {
                    ContentCaptureSourceKind::RawHttpHtml => ensure!(
                        identity.source == "raw_html" && identity.rendered_at.is_none(),
                        "Raw metadata identity differs"
                    ),
                    ContentCaptureSourceKind::RenderedDomHtml => ensure!(
                        identity.source == "rendered_html"
                            && identity.rendered_at.is_some_and(|v| !v.is_empty()),
                        "DOM metadata identity differs"
                    ),
                }
            } else {
                ensure!(
                    d.metadata_observation.is_none(),
                    "Unknown metadata must not declare a complete observation identity"
                );
            }
        }
        if d.source.complete {
            if metadata_profile.is_none() {
                ensure!(
                    d.source_document_url.is_some()
                        && d.capture_html_sha256
                            .is_some_and(|hash| hash.len() == 64
                                && hash.bytes().all(|b| b.is_ascii_hexdigit())),
                    "Complete channel requires a captured document identity and HTML SHA256"
                );
            }
            let elements = d
                .elements
                .as_ref()
                .context("Complete channel has no observed element set")?;
            ensure!(
                d.unavailable_reason.is_none(),
                "Complete channel has an unavailable reason"
            );
            ensure!(
                elements.len() <= MAX_NGRAM_ELEMENTS_PER_PAGE,
                "Channel per-page element limit exceeded"
            );
            element_count = add(element_count, elements.len())?;
            ensure!(
                element_count <= MAX_NGRAM_ELEMENTS,
                "Channel element limit exceeded"
            );
            let bytes = elements.iter().try_fold(0, |n, s| add(n, s.len()))?;
            ensure!(
                bytes <= ContentAnalysisLimits::default().max_text_bytes,
                "Channel per-page text limit exceeded"
            );
            report.total_text_bytes = add(report.total_text_bytes, bytes)?;
            ensure!(
                report.total_text_bytes <= options.limits.max_text_bytes,
                "Channel total text budget exceeded"
            );
        } else {
            ensure!(
                d.elements.is_none(),
                "Unknown channel must not supply countable prefix elements"
            );
        }
    }
    let extended = if options.max_n > 3 {
        Some(ExtendedPreparation::new(options)?)
    } else {
        None
    };
    let mut rows = BTreeMap::new();
    let mut key_bytes = 0usize;
    for (ordinal, d) in documents.iter().enumerate() {
        let mut source = report.sources[ordinal].clone();
        let mut spans = None;
        let mut element_hash = None;
        source.counted = d.source.complete;
        source.text_bytes = 0;
        source.tokens = 0;
        if let Some(elements) = &d.elements {
            element_hash = Some(digest(elements)?);
            source.text_sha256 = element_hash.clone();
            let mut observed = Vec::with_capacity(elements.len());
            for (index, text) in elements.iter().enumerate() {
                let result = analysis(
                    options,
                    &format!("source-{ordinal}-element-{index}"),
                    Some(text),
                )?;
                let doc = &result.documents[0];
                let offset = source.tokens;
                source.tokens = add(source.tokens, doc.token_count)?;
                ensure!(
                    source.tokens <= ContentAnalysisLimits::default().max_tokens_per_document,
                    "Channel per-page token limit exceeded"
                );
                report.total_tokens = add(report.total_tokens, doc.token_count)?;
                ensure!(
                    report.total_tokens <= options.limits.max_total_tokens,
                    "Channel corpus token budget exceeded"
                );
                source.text_bytes = add(source.text_bytes, text.len())?;
                observed.push(NgramElementSpan {
                    retained_element_ordinal: index + 1,
                    token_start: add(offset, 1)?,
                    tokens: doc.token_count,
                    utf8_bytes: text.len(),
                });
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
                    merge(
                        &mut report,
                        &mut rows,
                        &mut key_bytes,
                        distribution,
                        ordinal,
                        offset,
                        options,
                    )?;
                }
                if let Some(preparation) = &extended {
                    let tokens = preparation.tokens(text, options.case_behavior)?;
                    ensure!(
                        tokens.len() == doc.token_count,
                        "Element token preparation differs from existing profile"
                    );
                    let mut unique = 0;
                    let mut keys = 0;
                    for n in 4..=options.max_n {
                        let distribution = preparation.distribution(
                            &tokens,
                            n,
                            options,
                            &mut unique,
                            &mut keys,
                        )?;
                        let c = &mut report.extended_windows[n - 4];
                        c.candidate_windows =
                            add(c.candidate_windows, distribution.candidate_windows)?;
                        c.excluded_stopword_windows = add(
                            c.excluded_stopword_windows,
                            distribution.excluded_stopword_windows,
                        )?;
                        c.included_windows =
                            add(c.included_windows, distribution.included_windows)?;
                        merge(
                            &mut report,
                            &mut rows,
                            &mut key_bytes,
                            &distribution,
                            ordinal,
                            offset,
                            options,
                        )?;
                    }
                }
            }
            spans = Some(observed);
        }
        source.state = if source.counted {
            if source.tokens == 0 {
                "complete_empty"
            } else {
                "complete"
            }
        } else {
            d.unavailable_reason.unwrap_or("missing_or_unavailable")
        };
        *report.source_states.entry(source.state).or_default() += 1;
        report.all_supplied_sources_complete &= source.counted;
        let metadata_source = if metadata_profile.is_some() && d.source.complete {
            let identity = d
                .metadata_observation
                .context("Metadata identity disappeared")?;
            Some(NgramRetainedMetadataSource {
                authority_profile: "retained_html_metadata_v1",
                revision: digest(&(
                    "retained-html-metadata-revision-v1",
                    metadata_profile,
                    d.source_kind,
                    d.source.requested_url,
                    d.source_document_url,
                    identity,
                    d.source.basis,
                    d.source.observed_generation,
                    &element_hash,
                ))?,
                fetched_at: identity.fetched_at.to_owned(),
                rendered_at: identity.rendered_at.map(str::to_owned),
                engine: identity.engine.to_owned(),
                source: identity.source.to_owned(),
            })
        } else {
            None
        };
        let revision = if metadata_profile.is_some() {
            digest(&(
                "saved-retained-metadata-channel-revision-v1",
                metadata_profile,
                metadata_source.as_ref().map(|m| m.revision.as_str()),
                d.source.source_revision,
                d.capture_html_sha256,
                d.capture_status,
                d.unavailable_reason,
            ))?
        } else {
            digest(&(
                "saved-html-element-channel-revision-v1",
                profile,
                d.source_kind,
                d.source.source_revision,
                d.source_document_url,
                d.capture_html_sha256,
                &element_hash,
                d.unavailable_reason,
            ))?
        };
        source.channel = Some(NgramChannelSource {
            profile: if metadata_profile.is_some() {
                None
            } else {
                Some(profile)
            },
            retained_metadata_profile: metadata_profile,
            metadata_source,
            capture_status: if metadata_profile.is_some() {
                d.capture_status
            } else {
                None
            },
            source_kind: d.source_kind,
            source_document_url: d.source_document_url.map(str::to_owned),
            capture_html_sha256: d.capture_html_sha256.map(str::to_owned),
            channel_revision: revision,
            element_set_sha256: element_hash,
            hash_profile: "serde-json-retained-element-array-v1",
            position_unit: "one_based_channel_token_offset_with_explicit_element_spans_v1",
            elements: spans,
            unavailable_reason: d.unavailable_reason,
        });
        report.sources[ordinal] = source;
    }
    report.rows = rows.into_values().collect();
    report.rows.sort_by(|a, b| {
        b.occurrences
            .cmp(&a.occurrences)
            .then_with(|| a.n.cmp(&b.n))
            .then_with(|| a.tokens.cmp(&b.tokens))
    });
    let mut sink = ByteBudget {
        bytes: 0,
        cap: 64 * 1024 * 1024,
    };
    serde_json::to_writer(&mut sink, &report)?;
    Ok(report)
}
fn merge(
    report: &mut NgramCorpusReport,
    rows: &mut BTreeMap<Vec<String>, NgramCorpusRow>,
    key_bytes: &mut usize,
    distribution: &crate::ContentNgramDistribution,
    ordinal: usize,
    offset: usize,
    options: &NgramCorpusOptions,
) -> Result<()> {
    for gram in &distribution.rows {
        if !rows.contains_key(&gram.tokens) {
            ensure!(
                rows.len() < options.limits.max_unique_ngrams,
                "Unique channel ngram budget exceeded"
            );
            *key_bytes = add(*key_bytes, gram.tokens.iter().map(String::len).sum())?;
            ensure!(
                *key_bytes <= options.limits.max_key_bytes,
                "Channel ngram key budget exceeded"
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
            .context("Channel gram disappeared")?;
        row.occurrences = add(row.occurrences, gram.occurrences)?;
        if row
            .sources
            .last()
            .is_none_or(|s| s.source_ordinal != ordinal)
        {
            report.memberships = add(report.memberships, 1)?;
            ensure!(
                report.memberships <= options.limits.max_memberships,
                "Channel membership budget exceeded"
            );
            row.document_frequency = add(row.document_frequency, 1)?;
            row.sources.push(NgramCorpusMembership {
                source_ordinal: ordinal,
                occurrences: 0,
                first_positions: vec![],
                positions_sampled: false,
            });
        }
        let member = row
            .sources
            .last_mut()
            .context("Channel membership disappeared")?;
        member.occurrences = add(member.occurrences, gram.occurrences)?;
        for position in &gram.first_positions {
            if member.first_positions.len() < options.limits.position_samples {
                member.first_positions.push(add(offset, *position)?);
            }
        }
        member.positions_sampled = member.occurrences > member.first_positions.len();
    }
    Ok(())
}

#[cfg(test)]
#[path = "channels_tests.rs"]
mod tests;
