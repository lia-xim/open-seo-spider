# Durable source measurement Core interface v1

Owner: generation_postfix_review. New files only; Root533e94e/frozen226 inputs. No build/test/native acceptance. Root integrates all existing-file hooks separately.

Module `sitemap_source_store` internally includes/re-exports `sitemap_source_store_models.rs`, includes `sitemap_source_schema.sql`, and test-includes `sitemap_source_store_tests.rs`. Add this module and its public models to Core exports; no public Store API is added by this handoff.

All functions return anyhow::Result. Caller owns the actual existing Connection/outer Transaction and propagates every error. No function commits independently or starts network work. load must be invoked inside the same existing read transaction as catalog/config/inventory/report/checkpoint. Writable open alone executes SCHEMA; read-only open never migrates.

```
SCHEMA: &str
initialize(tx: &Transaction, generation: u64, limits: &SitemapSourceStoreLimits)
  -> SitemapSourceStoreSummary
load(connection: &Connection) -> Option<SitemapSourceStoreSnapshot>
validate_snapshot(snapshot: &SitemapSourceStoreSnapshot) -> ()
snapshot_context(snapshot: Option<&SitemapSourceStoreSnapshot>) -> String
record_a(tx: &Transaction, staged: &SitemapSourceDocument,
         measurement: &SitemapSourceMeasurement) -> SourceMeasurementReceipt
SourceMeasurementReceipt::after_a_commit(self, connection: &Connection) -> Self
SourceMeasurementReceipt::is_persisted(&self) -> bool
SourceMeasurementReceipt::measurement_id(&self) -> Option<&str>
SourceMeasurementReceipt::omission_reason(&self) -> Option<SitemapSourceStoreOmission>
verify_b_before_admission<'tx>(tx: &'tx Transaction, receipt: &SourceMeasurementReceipt,
                               staged: &SitemapSourceDocument)
  -> SourceMeasurementCommitGuard<'tx>
SourceMeasurementCommitGuard::take_pending(&mut self) -> PendingSitemapSource<'tx>
link_b_after_append(tx: &Transaction, guard: SourceMeasurementCommitGuard)
  -> SitemapSourceStoreSummary
inherit(tx: &Transaction, parent: &SitemapSourceStoreSnapshot,
        parent_snapshot_sha256: &str) -> SitemapSourceStoreSummary
```

Receipt and B-guard fields are private/non-Clone. A must commit before `after_a_commit` observes autocommit and durable receipt state; B refuses an unconfirmed receipt. A guards generation/FullCrawl/actual saved config SHA/initial catalog SHA/pending task/staged document/pure measurement before dedupe or omission. It obtains and drops the actual sitemap_ledger pending token. A never changes frontier/policy/warnings or source done.

B verification is the first admission-sensitive operation and returns the actual fresh pending token via `take_pending`. Existing append_source consumes that token and sets done=1. link_b_after_append then checks the same transaction, token extraction, active/current bindings, actual canonical stored Document payload+sequence and done=1. It never requests pending again. Any B failure must roll back its entire outer transaction; A remains independently durable. For a storage-omitted receipt, B still validates actual source append but creates no fabricated measurement link.

Public DTOs: SitemapSourceStoreLimits, SitemapSourceStoredMeasurement, SitemapSourceCommitAssociation, SitemapSourceStoreOmission, SitemapSourceStoreInheritedBasis, SitemapSourceStoreSummary, SitemapSourceStoreSnapshot. Snapshot fields are schema_version/summary/records/commits. Summary contains optional inherited basis; original captured config/generation/payload remain unchanged. Parser membership_retention never proves a SQL commit. Association matches exact measurement+envelope hash and actual document payload SHA; expectedID alone is insufficient.

Bounds: at most100000 records, complete record envelope at most16384 canonical UTF8 bytes, total charged logical bytes at most33554432. Each unlinked record reserves1024 link bytes. A fixed4096 metadata allocation is charged from initialization, split into actual canonical summary bytes and unused metadata reserve; this allows a fixed omission marker/revision to persist even when payload capacity is exhausted. The actual+reserved byte split is explicit and validated; no SQLite file-size claim. No REPLACE/eviction. Same measurement dedupes only after fresh guards and byte equality. First immutable omission marker advances revision; later identical omission does not turn missing data into measured zero.

Selected inheritance validates the exact existing recrawl_base BLOB/run_plan/config through recrawl::validate_saved_plan, verifies the embedded parent sidecar and actual inherited Legacy catalog, then copies records/associations unchanged under parent snapshot/catalog hashes and current_discovery_executed=false. Missing historical sidecar remains None; Root calls inherit only for an actually present optional parent snapshot.

Root hooks still required: schema registration; typed initialize; A commit/receipt confirmation before ordinary or policy completion; B guard/token/link in both existing whole-transaction paths; same-TX read snapshot; optional report/checkpoint/inspection fields; exact parent snapshot field `sitemap_source_measurements`; selected empty guard and inherit; App state digest via snapshot_context. No source UI/API or S4 body-cap change is included.

Final additive APIs: `pub(crate) ensure_empty(&Connection)->Result<()>`; `pub validate_catalog_binding(&Snapshot,&SitemapCatalog)->Result<()>`; `pub project_inherited_snapshot(&Snapshot,&str)->Result<Snapshot>`; `pub(crate) record_unavailable_a(tx,&SitemapSourceDocument,capture_config_sha256:&str)->Result<SourceMeasurementReceipt>`. snapshot_context/validate_snapshot are public and may be re-exported with a distinct Root alias. Tables are sitemap_source_measurements / sitemap_source_measurement_commits; summary key sitemap_source_store.

Legitimate capture unavailability is separate from storage budget. record_unavailable_a requires a real pre-fetch config SHA, active FullCrawl/generation/pending/staged identity and stores a fixed CaptureUnavailable marker without a measurement. An untrusted/missing pre-fetch context must remain explicit NoMeasurement/Unknown in the compatibility B path; never invent a receipt or call this with a config hash learned only after HTTP. A marker records the first fixed store-wide omission reason/revision, not an individual attempt journal. SQL/inconsistent payload/config/generation errors propagate. record_a also checks captured bounds against actual saved-config body cap and the existing default protocol parser profile. There is no S4 cap increase.

Exact public Summary fields: engine, hash_profile, initial_catalog_config_sha256, limits, revision, last_sequence, record_count, commit_count, record_json_bytes, commit_json_bytes, metadata_json_bytes, logical_json_bytes, reserved_json_bytes, charged_json_bytes, content_sha256, links_sha256, legacy_history_unknown, omission, omission_revision, inherited. Limits: max_catalog_json_bytes/max_record_json_bytes/max_records. Stored record: schema_version/engine/sequence/initial_catalog_config_sha256/staged_document_sha256/measurement. Commit: schema_version/engine/measurement_id/measurement_envelope_sha256/document_id/document_sequence/document_payload_sha256. Omission variants: StorageBudgetExceeded/CaptureUnavailable. Snapshot None is explicitly historical-unknown. No DTO claims source/cohort coverage complete.

Accounting includes actual canonical Snapshot root/array delimiters as well as all row/summary bytes. The1024 per-unjoined-record allocation includes a future link delimiter; actual link JSON cap is1023. Metadata allocation is fixed4096, with actual JSON and unused reserve exposed separately. Reduced total limits must leave enough for root framing plus this metadata allocation; initialization fails explicitly otherwise. These logical/reserved quantities do not include SQLite index/page/WAL size.
