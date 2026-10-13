#![allow(dead_code)]
//! Independent HTTP crawling, SEO extraction, durable storage and exports.
//! No commercial URL cap is imposed. Resource limits are user configuration.
mod audit;
mod comparison;
mod content_analysis;
mod content_binding;
mod content_capture;
mod control;
mod crawl_policy;
mod crawler;
mod css_discovery;
mod dom_facts;
mod export;
mod extract;
mod extraction_engine;
mod fragments;
mod image_metadata;
mod list_import;
mod metadata;
mod models;
mod onpage;
mod pdf;
mod policy_ledger;
mod recrawl;
mod renderer;
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
mod sitemap_ledger;
pub use sitemap_ledger::*;
mod srcset;
mod store;
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
pub use crawl_policy::*;
pub use crawler::Crawler;
pub use css_discovery::*;
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
pub use models::*;
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
    SITEMAP_XML_ENGINE, SitemapDocumentKind, SitemapEntityDecode, SitemapEntityEncoding,
    SitemapEntries, SitemapEntryObservation, SitemapFailure, SitemapFieldOccurrence,
    SitemapLocationOccurrence, SitemapLocationRole, SitemapNamespaceProfile,
    SitemapParseObservation, SitemapParserContext, SitemapParserLimits, decode_sitemap_entity,
    observe_sitemap_xml, parse_sitemap,
};
pub use store::CrawlStore;
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
