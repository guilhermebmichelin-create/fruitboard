-- One bounded scheduling cell per location. Completed immutable parse history
-- remains in metadata_snapshot; this operational cell may be superseded.
ALTER TABLE app_settings ADD COLUMN analysis_session_id TEXT;
CREATE TABLE analysis_job (
    location_id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL UNIQUE,
    root_id TEXT NOT NULL,
    project_file_id TEXT NOT NULL,
    root_revision INTEGER NOT NULL CHECK(root_revision >= 0),
    file_revision INTEGER NOT NULL CHECK(file_revision >= 0),
    location_revision INTEGER NOT NULL CHECK(location_revision >= 0),
    publication_revision INTEGER NOT NULL CHECK(publication_revision >= 0),
    byte_size INTEGER NOT NULL CHECK(byte_size >= 0),
    modified_at_ns INTEGER NOT NULL CHECK(modified_at_ns >= 0),
    adapter_version TEXT NOT NULL,
    parser_schema_version INTEGER NOT NULL CHECK(parser_schema_version > 0),
    state TEXT NOT NULL CHECK(state IN ('queued','running','complete','unsupported','failed','cancelled','stale')),
    attempt INTEGER NOT NULL DEFAULT 0 CHECK(attempt BETWEEN 0 AND 3),
    due_at_ms INTEGER NOT NULL CHECK(due_at_ms >= 0),
    session_id TEXT,
    lease_token TEXT,
    lease_until_ms INTEGER,
    snapshot_id TEXT REFERENCES metadata_snapshot(id),
    error_code TEXT CHECK(error_code IS NULL OR error_code IN
      ('INTERRUPTED','SOURCE_UNAVAILABLE','SOURCE_CHANGED','PARSER_TRANSPORT','INVALID_REPLY','CANCELLED')),
    CHECK ((state = 'running' AND session_id IS NOT NULL AND lease_token IS NOT NULL AND lease_until_ms IS NOT NULL)
        OR (state != 'running' AND session_id IS NULL AND lease_token IS NULL AND lease_until_ms IS NULL))
) STRICT;
CREATE INDEX analysis_job_due ON analysis_job(state, due_at_ms, job_id COLLATE BINARY);
CREATE UNIQUE INDEX analysis_job_one_running ON analysis_job((1)) WHERE state = 'running';
