-- Scan staging and publication. Staging is disposable by contract: the
-- removal/invalidation transactions discard it before any committed state
-- changes. scan_stage.scan_root_id deliberately carries REFERENCES
-- scan_root(id) ON DELETE CASCADE rather than no foreign key: root removal
-- invalidates staging first (cancel_root_work discards every open stage and
-- deletes its observations) and only then deletes the root row, so the
-- cascade can only remove already-discarded stage rows. It can never remove
-- file history: file_location rows are detached (ON DELETE SET NULL) before
-- the root delete, and neither table cascades into project_file.
ALTER TABLE scan_root ADD COLUMN last_successful_run_id TEXT;
ALTER TABLE scan_root ADD COLUMN last_successful_generation INTEGER
    CHECK (last_successful_generation IS NULL OR last_successful_generation >= 0);
ALTER TABLE scan_root ADD COLUMN last_successful_at_ms INTEGER;

CREATE TABLE project_file (
    id TEXT PRIMARY KEY CHECK (length(id) > 0),
    display_filename TEXT NOT NULL CHECK (length(display_filename) > 0),
    extension TEXT NOT NULL,
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    modified_at_ms INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
) STRICT;

CREATE TABLE file_location (
    id TEXT PRIMARY KEY CHECK (length(id) > 0),
    project_file_id TEXT NOT NULL REFERENCES project_file(id),
    scan_root_id TEXT REFERENCES scan_root(id) ON DELETE SET NULL,
    detached_scan_root_id TEXT,
    normalized_path TEXT NOT NULL CHECK (length(normalized_path) > 0),
    relative_path TEXT NOT NULL CHECK (length(relative_path) > 0),
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    modified_at_ms INTEGER NOT NULL,
    volume_id INTEGER,
    filesystem_file_id TEXT,
    presence TEXT NOT NULL DEFAULT 'present'
        CHECK (presence IN ('present', 'missing')),
    last_seen_scan_run_id TEXT REFERENCES scan_run(id) ON DELETE SET NULL,
    last_seen_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    CHECK (
        (scan_root_id IS NOT NULL AND detached_scan_root_id IS NULL)
        OR (scan_root_id IS NULL AND detached_scan_root_id IS NOT NULL)
    )
) STRICT;

CREATE UNIQUE INDEX file_location_active_path
    ON file_location (scan_root_id, normalized_path)
    WHERE scan_root_id IS NOT NULL;
CREATE INDEX file_location_identity
    ON file_location (volume_id, filesystem_file_id);
CREATE INDEX file_location_root_presence
    ON file_location (scan_root_id, presence, normalized_path);

CREATE TABLE scan_stage (
    run_id TEXT PRIMARY KEY REFERENCES scan_run(id) ON DELETE CASCADE,
    scan_root_id TEXT NOT NULL REFERENCES scan_root(id) ON DELETE CASCADE,
    generation INTEGER NOT NULL CHECK (generation >= 0),
    configuration_revision INTEGER NOT NULL CHECK (configuration_revision >= 0),
    session_id TEXT NOT NULL CHECK (length(session_id) > 0),
    lease_token TEXT NOT NULL CHECK (length(lease_token) > 0),
    state TEXT NOT NULL DEFAULT 'open'
        CHECK (state IN ('open', 'published', 'discarded')),
    record_count INTEGER NOT NULL DEFAULT 0 CHECK (record_count >= 0),
    path_bytes INTEGER NOT NULL DEFAULT 0 CHECK (path_bytes >= 0),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    published_at_ms INTEGER
) STRICT;

CREATE TABLE scan_stage_observation (
    id INTEGER PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES scan_stage(run_id) ON DELETE CASCADE,
    normalized_path TEXT NOT NULL CHECK (length(normalized_path) > 0),
    relative_path TEXT NOT NULL CHECK (length(relative_path) > 0),
    byte_size INTEGER NOT NULL CHECK (byte_size >= 0),
    modified_at_ms INTEGER NOT NULL,
    volume_id INTEGER,
    filesystem_file_id TEXT,
    UNIQUE (run_id, normalized_path)
) STRICT;

CREATE INDEX scan_stage_observation_order
    ON scan_stage_observation (run_id, normalized_path, id);
