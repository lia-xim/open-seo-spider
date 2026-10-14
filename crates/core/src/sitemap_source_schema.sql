-- Creation is not initialization. Historical absent sidecar remains unknown.
CREATE TABLE IF NOT EXISTS sitemap_source_measurements (
    sequence INTEGER PRIMARY KEY CHECK(sequence>0),
    measurement_id TEXT NOT NULL UNIQUE,
    source_kind TEXT NOT NULL CHECK(source_kind='sitemap'),
    task_url TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK(generation>=0),
    source_revision TEXT NOT NULL,
    expected_document_id TEXT NOT NULL,
    capture_config_sha256 TEXT NOT NULL,
    initial_catalog_config_sha256 TEXT NOT NULL,
    data TEXT NOT NULL,
    data_sha256 TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sitemap_source_measurement_commits (
    document_id TEXT PRIMARY KEY REFERENCES sitemap_documents(row_id),
    measurement_id TEXT NOT NULL UNIQUE REFERENCES sitemap_source_measurements(measurement_id),
    document_sequence INTEGER NOT NULL UNIQUE CHECK(document_sequence>0),
    data TEXT NOT NULL,
    data_sha256 TEXT NOT NULL
);
