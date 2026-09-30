-- Immutable validated parser results. Root/location IDs retain historical
-- authority without a cascading foreign key to removable operational rows.
ALTER TABLE project_file ADD COLUMN metadata_revision INTEGER NOT NULL DEFAULT 0
    CHECK (metadata_revision >= 0);
ALTER TABLE project_file ADD COLUMN metadata_publication_revision INTEGER NOT NULL DEFAULT 0
    CHECK (metadata_publication_revision >= 0);
ALTER TABLE file_location ADD COLUMN metadata_revision INTEGER NOT NULL DEFAULT 0
    CHECK (metadata_revision >= 0);

CREATE TABLE metadata_snapshot (
    id TEXT PRIMARY KEY CHECK (length(id) > 0),
    project_file_id TEXT NOT NULL REFERENCES project_file(id),
    input_root_id TEXT NOT NULL CHECK (length(input_root_id) > 0),
    input_location_id TEXT NOT NULL CHECK (length(input_location_id) > 0),
    input_root_revision INTEGER NOT NULL CHECK (input_root_revision >= 0),
    input_file_revision INTEGER NOT NULL CHECK (input_file_revision >= 0),
    input_location_revision INTEGER NOT NULL CHECK (input_location_revision >= 0),
    input_byte_size INTEGER NOT NULL CHECK (input_byte_size >= 0),
    input_modified_at_ns INTEGER NOT NULL CHECK (input_modified_at_ns >= 0),
    input_content_sha256 TEXT CHECK (input_content_sha256 IS NULL OR
        (length(input_content_sha256) = 64 AND input_content_sha256 NOT GLOB '*[^0-9a-f]*')),
    adapter_id TEXT NOT NULL CHECK (adapter_id = 'rust-flp-parser'),
    adapter_version TEXT NOT NULL CHECK (length(adapter_version) BETWEEN 1 AND 64),
    protocol_version INTEGER NOT NULL CHECK (protocol_version > 0),
    parser_schema_version INTEGER NOT NULL CHECK (parser_schema_version > 0),
    projection_version INTEGER NOT NULL CHECK (projection_version = 1),
    outcome TEXT NOT NULL CHECK (outcome IN ('complete', 'partial', 'unsupported', 'failed', 'rejected')),
    parser_code TEXT CHECK (parser_code IS NULL OR length(parser_code) BETWEEN 1 AND 64),
    unsupported_saved_version TEXT CHECK (unsupported_saved_version IS NULL OR length(unsupported_saved_version) BETWEEN 1 AND 64),
    payload_json TEXT CHECK (payload_json IS NULL OR
        (length(CAST(payload_json AS BLOB)) <= 262144 AND json_valid(payload_json))),
    parsed_at_ms INTEGER NOT NULL CHECK (parsed_at_ms >= 0),
    CHECK (
        (outcome IN ('complete', 'partial') AND payload_json IS NOT NULL AND
            input_content_sha256 IS NOT NULL AND parser_code IS NULL AND unsupported_saved_version IS NULL)
        OR (outcome = 'unsupported' AND unsupported_saved_version IS NOT NULL AND
            payload_json IS NULL AND parser_code IS NULL AND input_content_sha256 IS NULL)
        OR (outcome IN ('failed', 'rejected') AND parser_code IS NOT NULL AND
            payload_json IS NULL AND unsupported_saved_version IS NULL AND input_content_sha256 IS NULL)
    )
) STRICT;

CREATE INDEX metadata_snapshot_file_order
    ON metadata_snapshot (project_file_id, parsed_at_ms DESC, id COLLATE BINARY DESC);

ALTER TABLE project_file ADD COLUMN current_metadata_snapshot_id TEXT REFERENCES metadata_snapshot(id);

CREATE TRIGGER metadata_snapshot_no_update BEFORE UPDATE ON metadata_snapshot
BEGIN
    SELECT RAISE(ABORT, 'metadata snapshots are immutable');
END;
CREATE TRIGGER metadata_snapshot_no_delete BEFORE DELETE ON metadata_snapshot
BEGIN
    SELECT RAISE(ABORT, 'metadata snapshots are immutable');
END;

-- Unchanged observations do not invalidate a parse. Every observable change,
-- including a subsequent restoration of old values, advances the fence. The
-- STRICT INTEGER column/check fails closed on arithmetic overflow.
CREATE TRIGGER project_file_metadata_changed
AFTER UPDATE OF byte_size, modified_at_ns ON project_file
WHEN OLD.byte_size IS NOT NEW.byte_size OR OLD.modified_at_ns IS NOT NEW.modified_at_ns
BEGIN
    UPDATE project_file SET metadata_revision = metadata_revision + 1 WHERE id = NEW.id;
END;
CREATE TRIGGER file_location_metadata_changed
AFTER UPDATE OF project_file_id, scan_root_id, detached_scan_root_id, locator_key,
    relative_path, byte_size, modified_at_ns, identity_volume_serial,
    identity_file_id, presence ON file_location
WHEN OLD.project_file_id IS NOT NEW.project_file_id
    OR OLD.scan_root_id IS NOT NEW.scan_root_id
    OR OLD.detached_scan_root_id IS NOT NEW.detached_scan_root_id
    OR OLD.locator_key IS NOT NEW.locator_key
    OR OLD.relative_path IS NOT NEW.relative_path
    OR OLD.byte_size IS NOT NEW.byte_size
    OR OLD.modified_at_ns IS NOT NEW.modified_at_ns
    OR OLD.identity_volume_serial IS NOT NEW.identity_volume_serial
    OR OLD.identity_file_id IS NOT NEW.identity_file_id
    OR OLD.presence IS NOT NEW.presence
BEGIN
    UPDATE file_location SET metadata_revision = metadata_revision + 1 WHERE id = NEW.id;
END;
