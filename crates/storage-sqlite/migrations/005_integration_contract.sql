-- Phase 2 integration contract (contract revision 2, storage owner).
--
-- Locator keys: migration 005 does NOT copy v4 normalized_path into the new
-- locator_key columns. A v4 path carries no per-component case-mode evidence,
-- so no v4 value can be trusted as a LocatorKeyV1 (contract §1.1, `v1:`
-- envelope with per-component i:/s: mode tags). Promoting one blindly would
-- risk aliasing two distinct files, or splitting one file, under case-mode
-- ambiguity. Instead every legacy row is quarantined into a disjoint
-- `v0:legacy:` namespace that can never equal a `v1:` key:
--
--   locator_key = 'v0:legacy:' || normalized_path
--
-- Consequences (contract §1.3):
-- - normalized_path is retained untouched as migration-history compatibility
--   data. New code reads and writes locator_key.
-- - Quarantine is a byte-preserving copy inside SQLite TEXT, so non-ASCII
--   legacy paths migrate without failing. The ASCII guard applies only to new
--   v1: observations at the staging boundary; non-ASCII display spellings
--   keep round-tripping through relative_path.
-- - The first new-format scan misses exact-path continuity by construction
--   (disjoint namespaces) and therefore mints new locationIds; the
--   project_file_id association is retained through qualified-identity
--   evidence, and legacy rows unobserved by that scan become missing through
--   the normal unseen-present rule. LocationIds are intentionally not retained
--   across the v4->V1 key-space break; project associations are.
-- - A locator key is supplied by the filesystem boundary; SQLite never derives
--   it by lowercasing a display path.
ALTER TABLE project_file ADD COLUMN modified_at_ns INTEGER NOT NULL DEFAULT 0;
ALTER TABLE file_location ADD COLUMN locator_key TEXT NOT NULL DEFAULT '';
ALTER TABLE file_location ADD COLUMN modified_at_ns INTEGER NOT NULL DEFAULT 0;
ALTER TABLE file_location ADD COLUMN identity_volume_serial TEXT;
ALTER TABLE file_location ADD COLUMN identity_file_id TEXT;
ALTER TABLE scan_stage_observation ADD COLUMN locator_key TEXT NOT NULL DEFAULT '';
ALTER TABLE scan_stage_observation ADD COLUMN modified_at_ns INTEGER NOT NULL DEFAULT 0;
ALTER TABLE scan_stage_observation ADD COLUMN identity_volume_serial TEXT;
ALTER TABLE scan_stage_observation ADD COLUMN identity_file_id TEXT;

-- Converting an old millisecond value to nanoseconds is only valid when the
-- product fits SQLite's signed INTEGER. i64::MAX is 9223372036854775807, so
-- 9223372036854 ms is the largest millisecond value whose nanosecond product
-- still fits; -9223372036854 is the symmetric floor. The guards make an
-- out-of-range legacy value abort the migration instead of silently saturating
-- or losing data.
CREATE TRIGGER migration_005_project_timestamp_guard
BEFORE UPDATE OF modified_at_ns ON project_file
WHEN OLD.modified_at_ms < -9223372036854 OR OLD.modified_at_ms > 9223372036854
BEGIN
    SELECT RAISE(ABORT, 'legacy project timestamp is outside nanosecond bounds');
END;
CREATE TRIGGER migration_005_location_timestamp_guard
BEFORE UPDATE OF modified_at_ns ON file_location
WHEN OLD.modified_at_ms < -9223372036854 OR OLD.modified_at_ms > 9223372036854
BEGIN
    SELECT RAISE(ABORT, 'legacy location timestamp is outside nanosecond bounds');
END;
CREATE TRIGGER migration_005_stage_timestamp_guard
BEFORE UPDATE OF modified_at_ns ON scan_stage_observation
WHEN OLD.modified_at_ms < -9223372036854 OR OLD.modified_at_ms > 9223372036854
BEGIN
    SELECT RAISE(ABORT, 'legacy staging timestamp is outside nanosecond bounds');
END;

UPDATE project_file SET modified_at_ns = modified_at_ms * 1000000;
UPDATE file_location
SET locator_key = 'v0:legacy:' || normalized_path,
    modified_at_ns = modified_at_ms * 1000000;
UPDATE scan_stage_observation
SET locator_key = 'v0:legacy:' || normalized_path,
    modified_at_ns = modified_at_ms * 1000000;

DROP TRIGGER migration_005_project_timestamp_guard;
DROP TRIGGER migration_005_location_timestamp_guard;
DROP TRIGGER migration_005_stage_timestamp_guard;

-- Legacy identities were not bounded or encoded. Preserve only values that can
-- be represented by the new decimal contract; invalid historical values become
-- unavailable identity evidence rather than being guessed at publication time.
-- volume_id is a legacy signed INTEGER, so only 0..=i64::MAX was ever
-- storable; larger historical serials were never representable and correctly
-- stay NULL. filesystem_file_id must already be canonical unsigned decimal
-- (digits only, no leading zeros except "0") within u128 range
-- (max 340282366920938463463374607431768211455). The single UPDATE keeps the
-- pair all-or-none: rows outside the filter keep both columns NULL.
UPDATE file_location
SET identity_volume_serial = CAST(volume_id AS TEXT),
    identity_file_id = filesystem_file_id
WHERE volume_id IS NOT NULL
  AND volume_id >= 0
  AND filesystem_file_id IS NOT NULL
  AND length(filesystem_file_id) BETWEEN 1 AND 39
  AND filesystem_file_id NOT GLOB '*[^0-9]*'
  AND (length(filesystem_file_id) = 1 OR substr(filesystem_file_id, 1, 1) <> '0')
  AND (length(filesystem_file_id) < 39
        OR filesystem_file_id <= '340282366920938463463374607431768211455');

UPDATE scan_stage_observation
SET identity_volume_serial = CAST(volume_id AS TEXT),
    identity_file_id = filesystem_file_id
WHERE volume_id IS NOT NULL
  AND volume_id >= 0
  AND filesystem_file_id IS NOT NULL
  AND length(filesystem_file_id) BETWEEN 1 AND 39
  AND filesystem_file_id NOT GLOB '*[^0-9]*'
  AND (length(filesystem_file_id) = 1 OR substr(filesystem_file_id, 1, 1) <> '0')
  AND (length(filesystem_file_id) < 39
        OR filesystem_file_id <= '340282366920938463463374607431768211455');

-- The v4 path uniqueness/order path is retired. BINARY collation is spelled
-- out explicitly so the on-disk order is the contract order
-- (scan_root_id, locator_key COLLATE BINARY, location_id) regardless of any
-- ambient collation default. The legacy volume/filesystem identity index and
-- the normalized-path stage order index are dropped with them; the bounded
-- decimal identity lookup is file_location_encoded_identity.
CREATE UNIQUE INDEX file_location_active_locator
    ON file_location (scan_root_id, locator_key COLLATE BINARY)
    WHERE scan_root_id IS NOT NULL;
DROP INDEX file_location_active_path;
DROP INDEX file_location_root_presence;
DROP INDEX file_location_identity;
CREATE INDEX file_location_root_presence_locator
    ON file_location (scan_root_id, presence, locator_key COLLATE BINARY);
CREATE INDEX file_location_encoded_identity
    ON file_location (identity_volume_serial, identity_file_id);
CREATE UNIQUE INDEX scan_stage_observation_locator
    ON scan_stage_observation (run_id, locator_key COLLATE BINARY);
DROP INDEX scan_stage_observation_order;
CREATE INDEX scan_stage_observation_locator_order
    ON scan_stage_observation (run_id, locator_key COLLATE BINARY, id);
