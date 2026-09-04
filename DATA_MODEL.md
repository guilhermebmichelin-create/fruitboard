# Data model

Status: **Proposed logical model; no migration has been implemented**

## Modeling principles

- A logical song (`project`) is not a file (`project_file`), and a file is not a
  device path (`file_location`).
- Every domain entity has a stable UUIDv7 string ID. Paths, filenames, Drive
  IDs, and hashes are attributes, not primary keys.
- Extracted facts live in immutable parser snapshots. User-authored values live
  on domain entities or explicit overrides. Inferences live in suggestions with
  evidence. The three are never collapsed into one unexplained value.
- Missing, archived, abandoned, and deleted are distinct states.
- Syncable and device-local data are classified at the table/field level.
- Application mutations and their sync operation are committed atomically.
- Timestamps are UTC Unix milliseconds. User-entered calendar dates remain ISO
  local dates when time zone semantics would be misleading.
- Deletions of syncable records use tombstones long enough for offline replicas.

## Entity overview

```mermaid
erDiagram
    PROJECT ||--o{ PROJECT_FILE : groups
    PROJECT_FILE ||--o{ FILE_LOCATION : located_on_device
    PROJECT_FILE ||--o{ METADATA_SNAPSHOT : parsed_as
    METADATA_SNAPSHOT ||--o{ SNAPSHOT_PLUGIN : contains
    PLUGIN ||--o{ SNAPSHOT_PLUGIN : identified_as
    METADATA_SNAPSHOT ||--o{ SAMPLE_REFERENCE : contains
    PROJECT ||--o{ PROJECT_TAG : tagged
    TAG ||--o{ PROJECT_TAG : applies
    PROJECT ||--o{ TASK : has
    PROJECT ||--o| PROJECT_NOTE : has_main_note
    PROJECT ||--o{ JOURNAL_ENTRY : journals
    PROJECT ||--o{ ACTIVITY_EVENT : records
    PROJECT ||--o{ WORK_SESSION : tracks
    PROJECT ||--o{ RELEASE_TRACK : included_as
    RELEASE ||--o{ RELEASE_TRACK : orders
    PROJECT_FILE ||--o{ EXPORT_ASSOCIATION : may_export
    EXPORTED_AUDIO ||--o{ EXPORT_ASSOCIATION : linked_by
    EXPORTED_AUDIO ||--o{ EXPORT_LOCATION : located_on_device
    PROJECT_FILE ||--o{ FILE_RELATIONSHIP : source
    PROJECT_FILE ||--o{ FILE_RELATIONSHIP : target
    WORKFLOW_STAGE ||--o{ PROJECT : contains
```

The diagram omits operational, artwork, custom-field, and sync tables for
readability.

## Provenance and effective values

Do not build a generic entity-attribute-value store for core fields. Instead:

- `project` stores authoritative manual/product fields such as display name,
  stage, progress, rating, priority, favorite, genre override, key override, and
  release intent.
- `metadata_snapshot` stores extracted title, BPM, FL version, embedded project
  time, arrangement estimates, and structural counts.
- suggestion tables store inferred genre/key/version/export candidates with
  algorithm version, score, evidence, and review state.
- the query/application layer returns an effective value envelope where useful:

```json
{
  "value": 122.0,
  "provenance": "extracted",
  "confidence": "high",
  "sourceId": "snapshot UUID",
  "observedAt": "2026-09-04T16:00:00Z",
  "status": "available"
}
```

Allowed provenance values are `manual`, `extracted`, and `inferred`. Availability
is separate: `available`, `unavailable`, `unsupported`, or `failed`. A manual
override wins presentation but does not delete the extracted suggestion.

## Core tables

Fields below are a logical starting point, not executable SQL. All syncable
tables also carry `created_at_ms`, `updated_hlc`, `updated_by_device_id`, and
nullable `deleted_at_ms` unless explicitly immutable.

### Identity and workflow

`project`

- `id` primary key
- `display_name` required, user-authoritative
- `workflow_stage_id` foreign key
- `progress_percent` integer, nullable, check 0–100
- `rating` integer, nullable, check 1–5
- `priority` check `none|low|medium|high`
- `is_favorite` boolean
- `disposition` check `normal|abandoned|deprecated`
- `archived_at_ms` nullable; archive is not deletion
- manual musical fields: `genre`, `subgenre`, `musical_key`, `scale`, `mood`,
  `artist_alias`, `label_name`, `target_release_date`, `release_date`
- `current_file_id` nullable foreign key chosen by user/application policy

`workflow_stage`

- `id`, `name`, `rank_key`, `built_in_key` nullable
- `is_terminal`, `is_archived_stage`
- deletion is rejected while referenced unless a replacement stage is supplied

`project_file`

- `id`, `project_id`
- `display_filename`, `extension` (initially always `.flp`)
- `byte_size`, `filesystem_created_at_ms`, `filesystem_modified_at_ms`
- `availability` summarized as `available|missing|offline|permission_denied`
- `full_content_hash` nullable and `quick_fingerprint` nullable
- `current_snapshot_id` nullable
- no absolute path in the sync projection

`file_location` — device-local

- `id`, `project_file_id`, `device_id`, `scan_root_id`
- `absolute_path`, `normalized_path`, `relative_path`
- `volume_id`, `filesystem_file_id` nullable
- `presence` check `present|missing|offline|unknown`
- `cloud_presence` check `local|placeholder|partial|unknown`
- `last_seen_scan_run_id`, `last_seen_at_ms`
- unique `(device_id, normalized_path)` and, when trustworthy, unique
  `(device_id, volume_id, filesystem_file_id)`

`file_relationship`

- `id`, `from_project_file_id`, `to_project_file_id`
- `kind` check `version_of|duplicate_of|derived_from`
- `source` check `manual|confirmed_suggestion`
- `created_at_ms`, `reversed_at_ms` nullable

`version_suggestion` — local/recomputable except review decisions

- candidate file/project IDs, `ruleset_version`, `score`
- `band` check `unlikely|review|strong`
- `state` check `pending|confirmed|rejected|expired`
- JSON `evidence` using a versioned schema
- a rejection fingerprint prevents the same suggestion from recurring

### Parser snapshots and dependencies

`metadata_snapshot` — immutable after completion

- `id`, `project_file_id`
- input fingerprint, parser adapter ID/version, protocol version, schema version
- `outcome` check `success|partial|unsupported|failed`
- parsed timestamps and duration; failure category and safe diagnostic code
- extracted project title/comments/author/genre where available
- FL Studio version/build, BPM, PPQ, time signature
- duration estimate milliseconds, bar estimate, estimate confidence/method
- FL embedded time value and explicit confidence; never tracked work
- counts for arrangements, patterns, channels, playlist/mixer tracks,
  automation, markers, MIDI notes/controllers
- `diagnostic_summary_json`; raw traceback/log stays device-local

`arrangement_summary`

- `id`, `metadata_snapshot_id`, parser-local stable ordinal, name
- selected flag, raw maximum end ticks, estimated duration/bar count,
  uncertainty reason

`plugin`

- canonical `id`, normalized name, display name, vendor nullable
- kind `instrument|effect|unknown`
- format `native|vst2|vst3|other|unknown`
- normalized plugin identifier nullable
- uniqueness is conservative; aliases remain separate until evidence supports a
  merge

`plugin_alias`

- `id`, `plugin_id`, observed name/identifier, normalization version

`snapshot_plugin`

- `metadata_snapshot_id`, `plugin_id`, role, instance count
- `missing_state` check `present|missing|unknown` (only when detectable)
- source locations/counts; never stores or loads plugin DLL content

`sample_reference`

- `id`, `metadata_snapshot_id`, display basename, normalized extension
- reference form `absolute|relative|data_path|unknown`
- missing state and optional non-sensitive relative hint
- absolute resolved location belongs in device-local `dependency_location` and is
  excluded from sync by default

### Project management

`project_note`

- `project_id` unique
- versioned structured-document JSON and derived plain text for search
- sanitization schema version and base revision for conflict detection

`journal_entry`

- `id`, `project_id`, `occurred_at_ms`, structured body, derived plain text
- editable with revision history; timestamp and created time are separate

`task`

- `id`, `project_id`, title, notes, `completed_at_ms`, `due_date`, `rank_key`
- category is deferred; stable IDs make it additive

`tag`, `project_tag`

- tag `id`, display name, normalized name, optional color later
- unique normalized live name; join records have their own sync metadata so
  concurrent add/remove behavior is defined

`project_collaborator`

- `id`, `project_id`, display name only; it is creative metadata, not an account
  or permission principal

`custom_field_definition`, `project_custom_field_value`

- definitions specify name, type, validation, and order
- values use typed nullable columns or validated JSON; definitions are not
  allowed to shadow core fields

### Releases and artwork

`release`

- `id`, title, kind `album|ep|soundtrack|collection`, description, notes
- target and actual release dates; optional artwork ID

`release_track`

- `id`, `release_id`, `project_id`, `rank_key`, optional display track number
- unique live `(release_id, project_id)`; one project may belong to many releases

`artwork`

- `id`, media type, pixel dimensions, byte size, content hash
- user-visible ownership and crop metadata
- original local source path is never required and remains device-local

`asset_location` — device-local plus optional sync reference

- `artwork_id`, device cache path, Drive blob file ID nullable, availability
- synchronized artwork uses an optimized derivative by explicit policy; the
  original is not automatically uploaded

### Exports and playback

`exported_audio`

- `id`, display filename, format, size, duration, sample rate/channels nullable,
  content hash nullable
- represents an audio asset independent of a device path

`export_location` — device-local

- `id`, `exported_audio_id`, `device_id`, path and presence fields analogous to
  `file_location`

`export_association`

- `id`, `project_file_id`, `exported_audio_id`
- `state` check `suggested|confirmed|rejected`
- score, algorithm version, evidence JSON, manual override timestamp

Mobile preview is a different asset/derivative, not a flag that uploads the
desktop master.

### Activity and time

`activity_event`

- `id`, `project_id`, semantic event type, `occurred_at_ms`
- actor/device ID, concise versioned payload, optional source entity ID
- dedupe key for repeatable scanner events
- only meaningful user/history events; scanner diagnostics are not stored here

`work_session`

- `id`, `project_id`, start/end, active seconds
- source `automatic|manual`, confidence, heuristic version
- user-adjusted flag and optional note
- supporting evidence summary; no keystroke/content capture

`historical_activity_signal`

- `id`, `project_id`/`project_file_id`, timestamp, type such as
  `filesystem_modified` or `version_observed`
- supports a history visualization, never synthesized duration

### Scan operations — device-local

`scan_root`

- `id`, `device_id`, display name, absolute/canonical path, enabled
- watch/reconciliation state, cloud hydration policy, last successful scan
- root availability and last safe error code

`scan_run`

- `id`, `scan_root_id`, kind `initial|manual|periodic|recovery`
- start/end, outcome, counters, generation number

`scan_job`

- `id`, root/location, kind, state, priority, attempt count, not-before time
- expected fingerprint and correlation ID
- unique active job key prevents event-storm duplication

Detailed logs are rolling structured files, not unbounded database rows.

### Devices and sync

`device`

- stable installation ID, user-assigned name, platform, first/last seen
- contains no hardware fingerprint

`sync_operation` — append-only/outbox

- `operation_id`, origin device, monotonic origin sequence, HLC timestamp
- entity type/ID, operation kind, base revision/hash, versioned payload
- applied/uploaded timestamps and payload hash

`sync_cursor`

- backend ID, remote file/change token, last successful attempt, safe error state

`sync_conflict`

- `id`, entity/field, local and remote operation IDs, conflict type
- preserved values, state `unresolved|resolved`, chosen/merged operation

`sync_asset`

- local asset ID, Drive file ID/version/etag, content hash, state

OAuth tokens are not database rows. They live in platform secure storage on
desktop and memory/session-managed browser authorization on the PWA.

## Sync classification

| Data | Default |
| --- | --- |
| Projects, stages, ratings, tasks, tags, notes, journal, releases | Syncable |
| Selected normalized parser metadata and plugin/sample display metadata | Syncable |
| Absolute roots and file/export/dependency paths | Device-local |
| Scanner jobs/runs/logs and parser tracebacks | Device-local |
| Work sessions and meaningful activity events | Syncable |
| OAuth tokens | Secure store only, never sync payload |
| Artwork optimized derivative | Opt-in syncable |
| FLPs and original audio exports | Never metadata-synced |

Phase 0 confirms that work sessions and meaningful activity events are
syncable after the user explicitly enables Google Drive sync; they do not
require a second, category-specific opt-in by default. This supports consistent
PWA history and analytics. Raw process/window observations and low-level
tracking evidence remain device-local. The Phase 11 encryption/security ADR
must re-evaluate this sensitivity, disclose the category in the pre-sync
preview, and decide whether a granular opt-out is warranted before shipping.

## Index and query plan

Initial indexes should cover:

- projects by stage/rank, modified/activity date, rating, priority, progress,
  disposition, archived state, and favorite;
- project files by project, hash, modified time, current snapshot;
- locations by root/presence/normalized path and trustworthy filesystem ID;
- jobs by state/not-before/priority and active dedupe key;
- snapshot plugins by plugin/snapshot and samples by normalized basename;
- tasks by project/completion/rank/due date;
- activities and work sessions by project/time;
- release tracks by release/rank;
- sync operations by origin sequence, upload/applied state, and entity ID.

Use SQLite FTS5 for a materialized `project_search` document containing project
name, safe filenames/relative folder labels, tags, notes, journal, tasks,
release names, musical metadata, plugin names, and non-sensitive sample names.
Update the search document in the same application transaction as source
changes. Structured filters remain ordinary indexed SQL; advanced query syntax
is parsed to a typed query AST, never interpolated SQL.

## SQLite ownership and configuration

- Store the database under the platform application-data directory, never in a
  scanned or Drive-synchronized folder.
- Rust owns connections and migrations. The webview cannot execute arbitrary
  SQL.
- Enable foreign keys and a busy timeout. Use WAL only after verifying the
  embedded SQLite is 3.51.3+ or an officially fixed backport; prefer a single
  writer and short transactions.
- Select an explicit durability setting after crash/power-loss tests; user
  metadata protection takes precedence over benchmark scores.
- Back up with SQLite's online backup API before consequential migrations and
  provide tested restore diagnostics.
- Run `integrity_check` only at safe lifecycle points, not every startup.

SQLite documents that WAL permits concurrent readers with a writer but still
has one writer, and that WAL state includes companion files. Never copy a live
database with ordinary file copy or place it on a network filesystem.

## Migration rules

1. Forward-only, monotonically numbered SQL migrations are committed with code.
2. Every migration is transactional where SQLite permits and tested from every
   supported prior schema snapshot.
3. Destructive transformations use create/copy/validate/swap and a pre-migration
   backup; “down” migrations are not the recovery plan for user data.
4. Application startup refuses an unknown newer schema with a clear diagnostic.
5. Parser schema and sync protocol versions are independent of DB schema.
6. Seeded workflow stage IDs are stable and idempotent.
7. Migration tests include Unicode, tombstones, foreign keys, large notes, and
   interrupted/failed migration recovery.

## Questions deferred to schema spike

- Whether normalized snapshot children should be fully relational or keep a
  validated JSON detail blob plus relational search/analytics projections.
- The exact rich-note document format; it must support deterministic
  sanitization and conflict copies before adoption.
- Whether UUID strings or 16-byte blobs materially affect realistic query
  performance. Prefer strings unless benchmarks show a real need.
- Retention limits for immutable parser snapshots, rejected suggestions,
  tombstones, and sync operations.
- Which sample path components are useful enough to sync without exposing
  private directory structure.

## References

- [SQLite foreign key support](https://www.sqlite.org/foreignkeys.html)
- [SQLite write-ahead logging](https://www.sqlite.org/wal.html)
- [SQLite FTS5](https://www.sqlite.org/fts5.html)
- [SQLite online backup API](https://www.sqlite.org/backup.html)
