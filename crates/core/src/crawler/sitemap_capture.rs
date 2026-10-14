//! Private in-flight S3 collection/binding. No writes, admission or durability claim.
//! Raw helper facts stay separate from the unchanged legacy failure and row hashes.
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CaptureFailure {
    NoDurableContext,
    MissingCurrentConfig,
    GenerationMismatch,
    DirectCallerUnverified,
    ReceiveCounterUnavailable,
    MissingTransport,
    UnsupportedSource,
    UnsupportedHelperProfile,
    UnsupportedFailureCode,
    InconsistentHelperFacts,
    Measurement(SitemapSourceMeasurementFailure),
}
#[derive(Debug, Clone)]
pub(super) enum RunBasis {
    Verified {
        config_sha256: String,
        generation: u64,
    },
    Unavailable(CaptureFailure),
}
/// One existing read-TX binds ACTUAL saved config bytes/current generation/catalog.
/// Initial catalog config SHA is historical and is deliberately never substituted.
/// The durable writer must propagate snapshot SQL/integrity errors. This
/// supersedes the former capture-only optional fallback; absent Store stays unknown.
pub(super) fn discovery_basis(
    store: Option<&CrawlStore>,
    generation: Option<u64>,
) -> anyhow::Result<(Option<SitemapCatalog>, RunBasis)> {
    let Some(store) = store else {
        return Ok((
            None,
            RunBasis::Unavailable(CaptureFailure::NoDurableContext),
        ));
    };
    let snapshot = store.sitemap_read_snapshot()?;
    let basis = match (
        snapshot.crawl_config_sha256,
        snapshot.generation,
        generation,
    ) {
        (Some(config_sha256), Some(observed), Some(expected)) if observed == expected => {
            RunBasis::Verified {
                config_sha256,
                generation: expected,
            }
        }
        (None, _, _) => RunBasis::Unavailable(CaptureFailure::MissingCurrentConfig),
        _ => RunBasis::Unavailable(CaptureFailure::GenerationMismatch),
    };
    Ok((snapshot.catalog, basis))
}
#[derive(Debug, Clone)]
pub(super) struct ReceiveProgress {
    pub(super) received_bytes: Option<u64>,
    pub(super) counter_unavailable: bool,
    pub(super) body_limit_bytes: u64,
}
impl ReceiveProgress {
    pub(super) fn new(body_limit: usize) -> Self {
        Self {
            received_bytes: None,
            counter_unavailable: false,
            body_limit_bytes: u64::try_from(body_limit).expect("bounded protocol cap fits u64"),
        }
    }
    pub(super) fn begin_body(&mut self) {
        self.received_bytes = Some(0);
    }
    pub(super) fn returned(&mut self, length: usize) {
        let next = u64::try_from(length)
            .ok()
            .and_then(|n| self.received_bytes.and_then(|old| old.checked_add(n)));
        self.counter_unavailable |= next.is_none();
        self.received_bytes = next;
    }
}
#[derive(Debug, Clone)]
pub(super) enum CountCapture {
    NotInvoked(SitemapSourceMeasurementStop),
    Observed(Result<SitemapCountObservation, SitemapFailure>),
}
#[derive(Debug, Clone)]
pub(super) struct SourceCapture {
    pub(super) received: ReceiveProgress,
    pub(super) retained_prefix_sha256: Option<String>,
    pub(super) decoder: Option<SitemapEntityDecodeObservation>,
    pub(super) count: CountCapture,
    pub(super) limits: SitemapParserLimits,
    pub(super) unobserved_stop: SitemapSourceMeasurementStop,
}
#[derive(Debug, Clone)]
pub(super) struct CaptureOutcome {
    /// Compact raw facts, including exact legacy stop/read-error/EOF diagnostics.
    /// No XML Vec or per-entry rows are added here.
    pub(super) captured: SourceCapture,
    /// Expected source ID is not a committed-document or persisted-receipt claim.
    pub(super) measurement: Result<SitemapSourceMeasurement, CaptureFailure>,
}
impl SourceCapture {
    pub(super) fn not_invoked(
        received: ReceiveProgress,
        stop: SitemapSourceMeasurementStop,
    ) -> Self {
        Self {
            received,
            retained_prefix_sha256: None,
            decoder: None,
            count: CountCapture::NotInvoked(stop),
            limits: SitemapParserLimits::default(),
            unobserved_stop: stop,
        }
    }
    pub(super) fn count_limits(&self) -> SitemapCountLimits {
        SitemapCountLimits {
            max_xml_bytes: self.limits.max_xml_bytes,
            max_xml_events: self.limits.max_xml_events,
            max_xml_depth: self.limits.max_xml_depth,
            max_field_utf8_bytes: self.limits.max_field_utf8_bytes,
            allow_legacy_unqualified: self.limits.allow_legacy_unqualified,
        }
    }
    pub(super) fn into_outcome(
        self,
        document: &SitemapSourceDocument,
        basis: &RunBasis,
    ) -> CaptureOutcome {
        let measurement = self.bind(document, basis);
        CaptureOutcome {
            captured: self,
            measurement,
        }
    }
    fn bind(
        &self,
        document: &SitemapSourceDocument,
        basis: &RunBasis,
    ) -> Result<SitemapSourceMeasurement, CaptureFailure> {
        let (config_sha256, generation) = match basis {
            RunBasis::Verified {
                config_sha256,
                generation,
            } => (config_sha256, *generation),
            RunBasis::Unavailable(reason) => return Err(reason.clone()),
        };
        if document.kind != SitemapSourceKind::SitemapXml {
            return Err(CaptureFailure::UnsupportedSource);
        }
        if document.generation != generation {
            return Err(CaptureFailure::GenerationMismatch);
        }
        if self.received.counter_unavailable {
            return Err(CaptureFailure::ReceiveCounterUnavailable);
        }
        let transport = document
            .transport
            .as_ref()
            .ok_or(CaptureFailure::MissingTransport)?;
        let received = if !transport.request_started {
            SitemapMeasuredValue::unavailable()
        } else {
            match self.received.received_bytes {
                Some(bytes) if transport.body_complete => SitemapMeasuredValue::exact(bytes),
                Some(bytes) => SitemapMeasuredValue::at_least(bytes),
                None => SitemapMeasuredValue::unavailable(),
            }
        };
        let decode = self.decode_measurement(transport)?;
        let scan = self.scan_measurement()?;
        // Complete means the legacy source parser's results are retained in this
        // private batch. A later catalog transaction may still reject/roll back.
        let retention = match &document.parser {
            None => SitemapMembershipRetention::Unobserved,
            Some(parser) if parser.complete && document.retained_rows_complete => {
                SitemapMembershipRetention::Complete
            }
            Some(_) => SitemapMembershipRetention::Incomplete,
        };
        let payload = SitemapSourceMeasurementPayload {
            schema_version: 1,
            engine: SITEMAP_SOURCE_MEASUREMENT_ENGINE.into(),
            binding: SitemapSourceMeasurementBinding {
                crawl_config_sha256: config_sha256.clone(),
                run_generation: generation,
                source_kind: document.kind,
                requested_task_url: document.task_url.clone(),
                expected_source_observation_id: document.observation_id.clone(),
                source_revision: document.source_revision.clone(),
            },
            transport: SitemapSourceTransportMeasurement {
                http_decode_profile: SITEMAP_SOURCE_HTTP_PROFILE.into(),
                request_started: transport.request_started,
                requested_url: transport.requested_url.clone(),
                final_url: transport.final_url.clone(),
                initial_status: transport.initial_status,
                final_status: transport.final_status,
                blocked_by_robots: transport.blocked_by_robots,
                body_complete: transport.body_complete,
                post_http_received_bytes: received,
                retained_post_http_prefix_bytes: transport.post_http_decoding_bytes,
                retained_post_http_prefix_sha256: self.retained_prefix_sha256.clone(),
                complete_post_http_entity_sha256: transport.post_http_decoding_sha256.clone(),
            },
            decode,
            scan,
            membership_retention: retention,
            bounds: SitemapSourceCaptureBounds {
                max_retained_post_http_bytes: self.received.body_limit_bytes,
                max_retained_xml_bytes: self.limits.max_xml_bytes as u64,
                max_xml_events: self.limits.max_xml_events as u64,
                max_xml_depth: self.limits.max_xml_depth as u64,
                max_field_utf8_bytes: self.limits.max_field_utf8_bytes as u64,
                max_retained_entries: self.limits.max_entries as u64,
                max_retained_location_occurrences: self.limits.max_location_occurrences as u64,
                max_retained_text_utf8_bytes: self.limits.max_retained_text_utf8_bytes as u64,
            },
        };
        SitemapSourceMeasurement::new(payload, &SitemapSourceMeasurementLimits::default())
            .map_err(CaptureFailure::Measurement)
    }
    fn decode_measurement(
        &self,
        transport: &SitemapTransportFacts,
    ) -> Result<SitemapSourceDecodeMeasurement, CaptureFailure> {
        let Some(observation) = &self.decoder else {
            return Ok(SitemapSourceDecodeMeasurement {
                encoding: None,
                decoded_xml_bytes: SitemapMeasuredValue::unavailable(),
                decoded_prefix_sha256: None,
                decoded_complete_sha256: None,
                integrity: SitemapSourceDecodeIntegrity::NotObserved,
                stop_code: Some(self.unobserved_stop),
            });
        };
        if observation.schema_version != 1
            || observation.engine != SITEMAP_ENTITY_DECODE_OBSERVATION_ENGINE
            || observation.supplied_post_http_entity_slice_bytes
                != transport.post_http_decoding_bytes
        {
            return Err(CaptureFailure::UnsupportedHelperProfile);
        }
        let effective = observation
            .effective_limits
            .as_ref()
            .ok_or(CaptureFailure::UnsupportedHelperProfile)?;
        if effective.max_entity_bytes != self.limits.max_entity_bytes as u64
            || effective.max_xml_bytes != self.limits.max_xml_bytes as u64
        {
            return Err(CaptureFailure::InconsistentHelperFacts);
        }
        let integrity = match observation.integrity {
            SitemapEntityDecodeIntegrity::Complete => SitemapSourceDecodeIntegrity::Complete,
            SitemapEntityDecodeIntegrity::Failed => SitemapSourceDecodeIntegrity::Failed,
            SitemapEntityDecodeIntegrity::Incomplete => SitemapSourceDecodeIntegrity::Incomplete,
            SitemapEntityDecodeIntegrity::NotObserved => SitemapSourceDecodeIntegrity::NotObserved,
        };
        let bytes = match observation.decoded_bytes_returned {
            Some(bytes) if integrity == SitemapSourceDecodeIntegrity::Complete => {
                SitemapMeasuredValue::exact(bytes)
            }
            Some(bytes) => SitemapMeasuredValue::at_least(bytes),
            None => SitemapMeasuredValue::unavailable(),
        };
        let stop = observation
            .stop_code
            .as_deref()
            .map(decode_stop)
            .transpose()?;
        Ok(SitemapSourceDecodeMeasurement {
            encoding: observation.encoding,
            decoded_xml_bytes: bytes,
            decoded_prefix_sha256: observation.decoded_prefix_sha256.clone(),
            decoded_complete_sha256: observation.decoded_complete_sha256.clone(),
            integrity,
            stop_code: stop,
        })
    }
    fn scan_measurement(&self) -> Result<SitemapSourceScanMeasurement, CaptureFailure> {
        let unavailable = |stop| SitemapSourceScanMeasurement {
            xml_profile: SITEMAP_SOURCE_XML_PROFILE.into(),
            loc_lexical_profile: SITEMAP_SOURCE_LOC_PROFILE.into(),
            document_kind: None,
            namespace_profile: None,
            status: SitemapSourceScanStatus::NotObserved,
            stop_code: Some(stop),
            events_observed: SitemapMeasuredValue::unavailable(),
            counts: SitemapSourceEntryMeasurements {
                immediate_entry_starts: SitemapMeasuredValue::unavailable(),
                completed_immediate_entries: SitemapMeasuredValue::unavailable(),
                actual_document_namespace_loc_occurrences: SitemapMeasuredValue::unavailable(),
                protocol_eligible_completed_entries: SitemapMeasuredValue::unavailable(),
                missing_loc_entries: 0,
                multiple_loc_entries: 0,
                invalid_single_loc_entries: 0,
            },
        };
        let count = match &self.count {
            CountCapture::NotInvoked(stop) => return Ok(unavailable(*stop)),
            CountCapture::Observed(Err(failure)) => {
                return Ok(unavailable(scan_stop(&failure.code)?));
            }
            CountCapture::Observed(Ok(count)) => count,
        };
        if count.schema_version != 1
            || count.engine != SITEMAP_COUNT_XML_ENGINE
            || count.syntax_profile != SITEMAP_SOURCE_XML_PROFILE
            || count.loc_lexical_profile != SITEMAP_SOURCE_LOC_PROFILE
            || count.loc_whitespace_profile != "xml-s-edge-trim-v1"
            || count.loc_character_limit_exclusive != 2048
            || count.effective_limits != self.count_limits()
        {
            return Err(CaptureFailure::UnsupportedHelperProfile);
        }
        let decoder = self
            .decoder
            .as_ref()
            .ok_or(CaptureFailure::InconsistentHelperFacts)?;
        if decoder.integrity != SitemapEntityDecodeIntegrity::Complete
            || decoder.decoded_bytes_returned != Some(count.input_xml_bytes as u64)
            || decoder.decoded_complete_sha256.as_deref() != Some(count.input_xml_sha256.as_str())
            || count.input_xml_sha_profile != "decoded-sitemap-xml-bytes-v1"
            || count.xml_scan_complete != count.failure.is_none()
        {
            return Err(CaptureFailure::InconsistentHelperFacts);
        }
        let value = |n| {
            if count.xml_scan_complete {
                SitemapMeasuredValue::exact(n)
            } else {
                SitemapMeasuredValue::at_least(n)
            }
        };
        let c = &count.counts;
        Ok(SitemapSourceScanMeasurement {
            xml_profile: count.syntax_profile.clone(),
            loc_lexical_profile: count.loc_lexical_profile.clone(),
            document_kind: count.kind,
            namespace_profile: count.namespace_profile.clone(),
            status: if count.xml_scan_complete {
                SitemapSourceScanStatus::CompleteProfile
            } else {
                SitemapSourceScanStatus::Failed
            },
            stop_code: count
                .failure
                .as_ref()
                .map(|f| scan_stop(&f.code))
                .transpose()?,
            events_observed: value(count.events_observed as u64),
            counts: SitemapSourceEntryMeasurements {
                immediate_entry_starts: value(c.immediate_entry_starts),
                completed_immediate_entries: value(c.completed_immediate_entries),
                actual_document_namespace_loc_occurrences: value(
                    c.actual_document_namespace_loc_occurrences,
                ),
                protocol_eligible_completed_entries: value(c.protocol_eligible_completed_entries),
                missing_loc_entries: c.missing_location_entries,
                multiple_loc_entries: c.multiple_location_entries,
                invalid_single_loc_entries: c.invalid_single_location_entries,
            },
        })
    }
}
pub(super) fn decode_stop(code: &str) -> Result<SitemapSourceMeasurementStop, CaptureFailure> {
    use SitemapSourceMeasurementStop as S;
    Ok(match code {
        "sitemap_entity_byte_budget" => S::EntityByteLimit,
        "sitemap_xml_byte_budget" | "sitemap_xml_inflation_byte_budget" => S::DecodedByteLimit,
        "sitemap_invalid_gzip_entity" => S::InvalidGzip,
        "sitemap_nested_gzip_entity" => S::UnsupportedNestedGzip,
        _ => return Err(CaptureFailure::UnsupportedFailureCode),
    })
}
fn scan_stop(code: &str) -> Result<SitemapSourceMeasurementStop, CaptureFailure> {
    use SitemapSourceMeasurementStop as S;
    Ok(match code {
        "sitemap_invalid_utf8" => S::InvalidUtf8,
        "sitemap_illegal_xml_character" => S::IllegalXmlCharacter,
        "sitemap_dtd_unsupported" => S::DtdDenied,
        "sitemap_xml_event_budget" => S::EventLimit,
        "sitemap_xml_depth_budget" => S::DepthLimit,
        "sitemap_field_byte_budget" => S::FieldLimit,
        "sitemap_xml_byte_budget" => S::DecodedByteLimit,
        "sitemap_cdata_outside_root"
        | "sitemap_duplicate_attribute"
        | "sitemap_duplicate_expanded_attribute"
        | "sitemap_entity_outside_root"
        | "sitemap_incomplete_xml"
        | "sitemap_invalid_attribute"
        | "sitemap_invalid_character_data"
        | "sitemap_invalid_element_parent"
        | "sitemap_invalid_entity"
        | "sitemap_invalid_namespace"
        | "sitemap_invalid_processing_instruction"
        | "sitemap_invalid_root_namespace"
        | "sitemap_invalid_root"
        | "sitemap_invalid_xml_comment"
        | "sitemap_invalid_xml_declaration"
        | "sitemap_invalid_xml_span"
        | "sitemap_invalid_xml"
        | "sitemap_mismatched_end"
        | "sitemap_multiple_roots"
        | "sitemap_nested_field_markup"
        | "sitemap_text_outside_root"
        | "sitemap_unbalanced_end"
        | "sitemap_unbound_namespace_prefix"
        | "sitemap_unsupported_xml_name" => S::XmlSyntax,
        _ => return Err(CaptureFailure::UnsupportedFailureCode),
    })
}
