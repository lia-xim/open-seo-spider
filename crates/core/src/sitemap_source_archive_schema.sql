-- Separate from immutable measurement A and membership/admission B.
-- Tables/marker are created only for an explicit writable archive profile.
CREATE TABLE sitemap_body_archive_state (
    id INTEGER PRIMARY KEY CHECK(id=1),
    data TEXT NOT NULL, data_sha256 TEXT NOT NULL
);
CREATE TABLE sitemap_body_archive_captures (
    sequence INTEGER PRIMARY KEY CHECK(sequence>0),
    measurement_id TEXT NOT NULL UNIQUE REFERENCES sitemap_source_measurements(measurement_id),
    data TEXT NOT NULL, data_sha256 TEXT NOT NULL
);
