-- WORK-only candidate. Writable open may create tables; that is not initialization evidence.
CREATE TABLE IF NOT EXISTS sitemap_documents (
    sequence INTEGER PRIMARY KEY CHECK(sequence>0),
    row_id TEXT NOT NULL UNIQUE,
    source_kind TEXT NOT NULL CHECK(source_kind IN ('robots','sitemap')),
    task_url TEXT NOT NULL,
    generation INTEGER NOT NULL CHECK(generation>=0),
    source_revision TEXT NOT NULL,
    data TEXT NOT NULL,
    data_sha256 TEXT NOT NULL,
    UNIQUE(source_kind,task_url)
);
CREATE TABLE IF NOT EXISTS sitemap_roots (
    sequence INTEGER PRIMARY KEY CHECK(sequence>0),
    row_id TEXT NOT NULL UNIQUE,
    observation_id TEXT REFERENCES sitemap_documents(row_id),
    ordinal INTEGER NOT NULL CHECK(ordinal>=0),
    data TEXT NOT NULL,
    data_sha256 TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sitemap_entries (
    sequence INTEGER PRIMARY KEY CHECK(sequence>0),
    row_id TEXT NOT NULL UNIQUE,
    observation_id TEXT NOT NULL REFERENCES sitemap_documents(row_id),
    ordinal INTEGER NOT NULL CHECK(ordinal>=0),
    data TEXT NOT NULL,
    data_sha256 TEXT NOT NULL,
    UNIQUE(observation_id,ordinal)
);
CREATE TABLE IF NOT EXISTS sitemap_locations (
    sequence INTEGER PRIMARY KEY CHECK(sequence>0),
    row_id TEXT NOT NULL UNIQUE,
    observation_id TEXT NOT NULL REFERENCES sitemap_documents(row_id),
    entry_id TEXT NOT NULL REFERENCES sitemap_entries(row_id),
    ordinal INTEGER NOT NULL CHECK(ordinal>=0),
    data TEXT NOT NULL,
    data_sha256 TEXT NOT NULL,
    UNIQUE(observation_id,ordinal)
);
CREATE INDEX IF NOT EXISTS sitemap_roots_source ON sitemap_roots(observation_id,ordinal);
CREATE INDEX IF NOT EXISTS sitemap_locations_source ON sitemap_locations(observation_id,ordinal);
