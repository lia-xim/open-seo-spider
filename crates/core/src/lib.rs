#![allow(dead_code)]
//! Independent HTTP crawling, SEO extraction, durable storage and exports.
//! No commercial URL cap is imposed. Resource limits are user configuration.
mod audit;
mod cdn_classification;
mod cdn_directory_reporting;
pub use cdn_directory_reporting::{
    CDN_DIRECTORY_NORMALIZATION, CDN_DIRECTORY_REPORTING_ENGINE, CdnDirectoryReportingConfig,
    CdnDirectoryReportingObservation, CdnDirectoryRuleMatch,
};
mod comparison;
mod content_analysis;
mod content_binding;
mod content_capture;
mod control;
mod crawl_admission;
mod crawl_quota_ledger;
mod crawl_quota_runtime;
mod crawl_quotas;
pub mod spelling;
pub use crawl_quotas::*;
mod crawl_policy;
mod crawler;
mod css_discovery;
mod directive_reporting;
pub use directive_reporting::{
    DirectiveReportingConfig, ReportingVisibility, directive_hidden_issue_urls,
    directive_page_visible, reported_fragments, reported_report, reported_responses, reporting_csv,
    reporting_json, reporting_visibility, source_occurrence_reporting_identity,
};
mod directory_tree;
mod dom_facts;
mod export;
mod extract;
mod extraction_engine;
mod fragments;
pub mod segments;
pub use cdn_classification::{
    CDN_CLASSIFICATION_ENGINE, CdnClassificationConfig, CdnClassificationObservation,
};
pub use segments::{ReportingIdentity, ReportingMode};
mod html_head;
pub use html_head::{
    HTML_HEAD_ENGINE, HTML_HEAD_PROFILE, HtmlHeadObservation, HtmlHeadSpan, HtmlHeadStatus,
};
mod image_metadata;
mod list_import;
mod metadata;
mod migration;
mod models;
mod ngram_corpus;
mod onpage;
mod pdf;
mod policy_ledger;
mod recrawl;
mod renderer;
mod resource_broken_sources;
mod resource_capture;
mod resource_html;
mod resource_ledger;
mod resource_policy;
mod response_observations;
mod robots;
mod robots_simulation;
mod robots_sitemap_source;
mod serp;
mod sitemap;
mod sitemap_coverage;
mod sitemap_findings;
pub use sitemap_findings::*;
mod sitemap_generation;
pub use sitemap_generation::*;
mod sitemap_ledger;
pub use sitemap_coverage::*;
pub use sitemap_ledger::*;
mod sitemap_lastmod;
pub use sitemap_lastmod::*;
mod sitemap_membership_analysis;
pub use sitemap_membership_analysis::*;
mod sitemap_source_measurement;
pub use sitemap_source_measurement::*;
mod sitemap_source_archive;
pub use sitemap_source_archive::{
    MAX_SITEMAP_ARCHIVE_BODY_BYTES, SITEMAP_SOURCE_ARCHIVE_ENGINE, SitemapArchiveOmission,
    SitemapArchivedBody, SitemapArchivedCapture, SitemapBodyExtent, SitemapBodyRepresentation,
    SitemapSourceArchiveConfig, SitemapSourceArchiveInspection, SitemapSourceArchiveSnapshot,
    SitemapSourceArchiveSummary,
};
mod sitemap_source_processing;
pub use sitemap_source_processing::*;
mod sitemap_read_control;
mod sitemap_source_store;
pub use sitemap_read_control::{SitemapReadControl, SitemapReadControlError};
pub use sitemap_source_store::{
    MAX_SOURCE_RECORD_BYTES, MAX_SOURCE_STORE_BYTES, SITEMAP_SOURCE_STORE_ENGINE,
    SITEMAP_SOURCE_STORE_HASH_PROFILE, SOURCE_COMMIT_RESERVE_BYTES,
    SOURCE_METADATA_ALLOCATION_BYTES, SitemapSourceCommitAssociation,
    SitemapSourceStoreInheritedBasis, SitemapSourceStoreLimits, SitemapSourceStoreOmission,
    SitemapSourceStoreSnapshot, SitemapSourceStoreSummary, SitemapSourceStoredMeasurement,
    snapshot_context as sitemap_source_measurement_snapshot_context,
    validate_catalog_binding as validate_sitemap_source_measurement_catalog_binding,
    validate_snapshot as validate_sitemap_source_measurement_snapshot,
};
pub mod api_enrichment;
mod srcset;
mod store;
mod task_owner;
pub use task_owner::{OwnedTask, OwnedTaskError, OwnedTaskStats, TaskAdmission, TaskOwner};
mod tls;
pub use tls::{
    CRAWL_TLS_ENGINE, HstsPolicy, HstsSourceKind, HstsUpgrade, PageTlsObservation, TlsConfig,
    TrustedCertificate,
};
mod text_metrics;
mod urls;

pub use audit::{audit_pages, audit_pages_with_thresholds};
pub use comparison::*;
pub use content_analysis::*;
pub use content_binding::{
    ContentCaptureFailure, PAGE_CONTENT_CAPTURE_ENGINE, PageContentCapture,
    PageContentCaptureStatus, validate_page_content_captures,
};
pub use content_capture::*;
pub use control::{
    ControlledCrawlResult, CrawlCheckpoint, CrawlControl, CrawlOutcome, CrawlRequest,
    CrawlRunState, CrawlStoreInspection,
};
pub use crawl_admission::*;
pub use crawl_policy::*;
pub use crawler::Crawler;
pub use css_discovery::*;
pub use directory_tree::*;
pub use dom_facts::audit_dom_metadata;
pub use export::{
    compare_crawls, csv_text, export_browser_cookies_csv, export_custom_extractions_csv,
    export_custom_javascript_csv, export_dom_observations_csv, export_fragments_csv,
    export_image_resources_csv, export_input_occurrences_csv, export_issues_csv, export_links_csv,
    export_pages_csv, export_pdf_csv, export_sitemap_xml,
};
pub use extract::{extract_custom, extract_html};
pub use extraction_engine::extract_configured;
pub use fragments::validate_fragments;
pub use list_import::{
    LIST_IMPORT_ENGINE, ListCandidateKind, ListColumnSelector, ListCsvDelimiter,
    ListFormulaMetadata, ListFormulaPolicy, ListImportFormat, ListImportInspection,
    ListImportLimits, ListImportOptions, ListImportResult, ListImportRow, ListImportSheet,
    ListSheetSelector, export_url_list_csv, import_url_list, inspect_xlsx_list,
    validate_import_result,
};
pub use metadata::{applicable_robot_directives, parse_link_header, parse_x_robots_tag};
pub use migration::*;
pub use models::*;
pub use ngram_corpus::*;
pub use policy_ledger::{
    InheritedPolicyHistory, LIVE_CRAWL_POLICY_ENGINE, LiveCrawlPolicyLimits, PagePolicyObservation,
    PolicyAdmission, PolicyInputBasis, PolicyLedgerSummary, PolicyObservationBasis,
    PolicyOccurrence, PolicyOccurrencePage, PolicyTaskRole, StoredPolicyOccurrence,
    export_policy_occurrences_csv,
};
pub use recrawl::{
    PreparedSelectedRecrawl, RecrawlParentReference, RecrawlResumeOverride,
    RecrawlSelectionOccurrence, RecrawlSourceKind, RecrawlTarget, RecrawlTargetObservation,
    RecrawlTargetState, RunPlan, SELECTED_RECRAWL_COVERAGE, SELECTED_RECRAWL_ENGINE,
    SelectedRecrawlOperation, SelectedRecrawlParent, SelectedRecrawlPlan, prepare_selected_recrawl,
};
pub use resource_broken_sources::*;
pub use resource_ledger::*;
pub use resource_policy::*;
pub use response_observations::{
    ResponseObservation, export_response_observations_csv, response_observations,
};
pub use robots_simulation::{
    ROBOTS_SIMULATION_ENGINE, RobotsSimulation, RobotsSimulationInput, RobotsSimulationLimits,
    RobotsSimulationRow, export_robots_simulation_csv, simulate_robots,
};
pub use serp::{SerpInputRow, SerpPreview, export_serp_csv, import_serp_csv, preview_serp};
pub use sitemap::{
    SITEMAP_COUNT_LOC_PROFILE, SITEMAP_COUNT_XML_ENGINE, SITEMAP_ENTITY_DECODE_OBSERVATION_ENGINE,
    SITEMAP_XML_ENGINE, SitemapCountLimits, SitemapCountObservation, SitemapDocumentKind,
    SitemapEntityDecode, SitemapEntityDecodeEffectiveLimits, SitemapEntityDecodeIntegrity,
    SitemapEntityDecodeObservation, SitemapEntityDecodeOutcome, SitemapEntityDecodeReadErrorKind,
    SitemapEntityEncoding, SitemapEntries, SitemapEntryObservation, SitemapFailure,
    SitemapFieldOccurrence, SitemapLocationOccurrence, SitemapLocationRole,
    SitemapNamespaceProfile, SitemapParseObservation, SitemapParserContext, SitemapParserLimits,
    SitemapXmlCountFacts, count_sitemap_xml, decode_sitemap_entity, decode_sitemap_entity_observed,
    observe_sitemap_xml, parse_sitemap,
};
pub use store::{
    CrawlStore, SITEMAP_RAW_READ_ENGINE, SITEMAP_READ_WINDOW_ENGINE, SitemapBodyCatalogState,
    SitemapBodyReadContext, SitemapBodyReadError, SitemapBodyWindow, SitemapRawReadContext,
    SitemapRawReadRequest, SitemapRawReadWindow, SitemapRawRowKind, SitemapReadSnapshot,
    SitemapReadState, SitemapReadWindow, SitemapReadWindowError, SitemapReadWindowExpectedContext,
    SitemapStoredRowsFilter, SitemapVerifiedBodyCopy, SitemapVerifiedFindings,
    SitemapVerifiedReadHeader, SitemapVerifiedStoredRows, sitemap_read_state_sha256,
};
pub use urls::normalize_url;

pub use robots_sitemap_source::{
    ROBOTS_BODY_CAP, ROBOTS_SITEMAP_SOURCE_ENGINE, RobotsBodyBuffer, RobotsSitemapDeclaration,
    RobotsSitemapSource,
};

#[cfg(test)]
#[path = "explicit_sitemap_inputs_tests.rs"]
mod explicit_sitemap_inputs;

mod sitemap_discovery;
mod sitemap_initial_roots;
