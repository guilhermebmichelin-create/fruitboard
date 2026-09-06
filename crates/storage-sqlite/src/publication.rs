use super::{Database, Result, StorageError};
use rusqlite::{OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A single worker batch is deliberately bounded. The eventual enumerator can
/// stream many batches, while each transaction remains small and retryable.
pub const MAX_STAGED_BATCH_RECORDS: usize = 512;
pub const MAX_STAGED_RECORDS: i64 = 10_000;
pub const MAX_STAGED_PATH_BYTES: i64 = 4 * 1024 * 1024;
const MAX_OBSERVATION_PATH_BYTES: usize = 32 * 1024;
const MAX_FILE_ID_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FilePresence {
    Present,
    Missing,
}

impl FilePresence {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "present" => Ok(Self::Present),
            "missing" => Ok(Self::Missing),
            _ => Err(StorageError::InvalidSchema),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStageState {
    Open,
    Published,
    Discarded,
}

impl ScanStageState {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "open" => Ok(Self::Open),
            "published" => Ok(Self::Published),
            "discarded" => Ok(Self::Discarded),
            _ => Err(StorageError::InvalidSchema),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanObservation {
    pub normalized_path: String,
    pub relative_path: String,
    pub byte_size: u64,
    pub modified_at_ms: i64,
    pub volume_id: Option<i64>,
    pub filesystem_file_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStaging {
    pub run_id: String,
    pub scan_root_id: String,
    pub generation: i64,
    pub configuration_revision: i64,
    pub session_id: String,
    pub state: ScanStageState,
    pub record_count: i64,
    pub path_bytes: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub published_at_ms: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedLocation {
    pub id: String,
    pub project_file_id: String,
    pub scan_root_id: Option<String>,
    pub detached_scan_root_id: Option<String>,
    pub normalized_path: String,
    pub relative_path: String,
    pub byte_size: u64,
    pub modified_at_ms: i64,
    pub volume_id: Option<i64>,
    pub filesystem_file_id: Option<String>,
    pub presence: FilePresence,
    pub last_seen_scan_run_id: Option<String>,
    pub last_seen_at_ms: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanPublication {
    pub run_id: String,
    pub scan_root_id: String,
    pub generation: i64,
    pub location_count: i64,
    pub published_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRootPublication {
    pub scan_root_id: String,
    pub last_successful_run_id: Option<String>,
    pub last_successful_generation: Option<i64>,
    pub last_successful_at_ms: Option<i64>,
}

#[derive(Clone, Debug)]
struct PublicationContext {
    run_id: String,
    job_id: String,
    root_id: String,
    generation: i64,
    configuration_revision: i64,
    session_id: String,
    lease_token: String,
}

#[derive(Clone, Debug)]
struct StagedObservation {
    normalized_path: String,
    relative_path: String,
    byte_size: i64,
    modified_at_ms: i64,
    volume_id: Option<i64>,
    filesystem_file_id: Option<String>,
}

#[derive(Clone, Debug)]
struct ExistingLocation {
    id: String,
    project_file_id: String,
    volume_id: Option<i64>,
    filesystem_file_id: Option<String>,
}

fn map_not_found(error: rusqlite::Error) -> StorageError {
    match error {
        rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound,
        other => other.into(),
    }
}

fn map_u64(value: i64) -> Result<u64> {
    u64::try_from(value).map_err(|_| StorageError::InvalidSchema)
}

fn observation_path_bytes(observation: &ScanObservation) -> Result<i64> {
    if observation.normalized_path.is_empty()
        || observation.relative_path.is_empty()
        || observation.normalized_path.len() > MAX_OBSERVATION_PATH_BYTES
        || observation.relative_path.len() > MAX_OBSERVATION_PATH_BYTES
        || observation.normalized_path.contains(['\0', ':'])
        || observation.relative_path.contains(['\0', ':'])
        || observation.normalized_path.starts_with(['/', '\\'])
        || observation.relative_path.starts_with(['/', '\\'])
        || observation
            .normalized_path
            .split(['/', '\\'])
            .any(|part| matches!(part, "" | "." | ".."))
        || observation
            .relative_path
            .split(['/', '\\'])
            .any(|part| matches!(part, "" | "." | ".."))
        || observation
            .filesystem_file_id
            .as_deref()
            .is_some_and(|id| id.is_empty() || id.len() > MAX_FILE_ID_BYTES || id.contains('\0'))
        || observation.volume_id.is_some_and(|id| id < 0)
    {
        return Err(StorageError::Conflict);
    }
    let bytes = observation
        .normalized_path
        .len()
        .checked_add(observation.relative_path.len())
        .ok_or(StorageError::Conflict)?;
    i64::try_from(bytes).map_err(|_| StorageError::Conflict)
}

fn staged_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StagedObservation> {
    Ok(StagedObservation {
        normalized_path: row.get(0)?,
        relative_path: row.get(1)?,
        byte_size: row.get(2)?,
        modified_at_ms: row.get(3)?,
        volume_id: row.get(4)?,
        filesystem_file_id: row.get(5)?,
    })
}

fn staged_observation_path_bytes(observation: &StagedObservation) -> Result<i64> {
    observation_path_bytes(&ScanObservation {
        normalized_path: observation.normalized_path.clone(),
        relative_path: observation.relative_path.clone(),
        byte_size: map_u64(observation.byte_size)?,
        modified_at_ms: observation.modified_at_ms,
        volume_id: observation.volume_id,
        filesystem_file_id: observation.filesystem_file_id.clone(),
    })
}

fn select_staging(transaction: &Transaction<'_>, run_id: &str) -> Result<ScanStaging> {
    transaction
        .query_row(
            "SELECT run_id, scan_root_id, generation, configuration_revision,
                    session_id, state, record_count, path_bytes, created_at_ms,
                    updated_at_ms, published_at_ms
             FROM scan_stage WHERE run_id = ?1",
            [run_id],
            |row| {
                Ok(ScanStaging {
                    run_id: row.get(0)?,
                    scan_root_id: row.get(1)?,
                    generation: row.get(2)?,
                    configuration_revision: row.get(3)?,
                    session_id: row.get(4)?,
                    state: ScanStageState::parse(&row.get::<_, String>(5)?).map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            5,
                            "state".into(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    record_count: row.get(6)?,
                    path_bytes: row.get(7)?,
                    created_at_ms: row.get(8)?,
                    updated_at_ms: row.get(9)?,
                    published_at_ms: row.get(10)?,
                })
            },
        )
        .map_err(map_not_found)
}

fn select_staging_optional(
    transaction: &Transaction<'_>,
    run_id: &str,
) -> Result<Option<ScanStaging>> {
    transaction
        .query_row(
            "SELECT run_id, scan_root_id, generation, configuration_revision,
                    session_id, state, record_count, path_bytes, created_at_ms,
                    updated_at_ms, published_at_ms
             FROM scan_stage WHERE run_id = ?1",
            [run_id],
            |row| {
                let state = row.get::<_, String>(5)?;
                Ok(ScanStaging {
                    run_id: row.get(0)?,
                    scan_root_id: row.get(1)?,
                    generation: row.get(2)?,
                    configuration_revision: row.get(3)?,
                    session_id: row.get(4)?,
                    state: ScanStageState::parse(&state).map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            5,
                            "state".into(),
                            rusqlite::types::Type::Text,
                        )
                    })?,
                    record_count: row.get(6)?,
                    path_bytes: row.get(7)?,
                    created_at_ms: row.get(8)?,
                    updated_at_ms: row.get(9)?,
                    published_at_ms: row.get(10)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn validate_owner(
    transaction: &Transaction<'_>,
    run_id: &str,
    session_id: &str,
    lease_token: &str,
    now_ms: i64,
) -> Result<PublicationContext> {
    let raw = transaction
        .query_row(
            "SELECT r.id, r.scan_job_id, r.scan_root_id, r.generation,
                    r.configuration_revision, r.session_id, r.lease_token,
                    r.state, r.cancellation_requested, r.lease_expires_at_ms,
                    j.state, j.cancellation_requested, j.follow_up_requested,
                    s.ended_at_ms, root.enabled, root.generation,
                    root.configuration_revision, j.scan_root_id
             FROM scan_run AS r
             JOIN scan_job AS j ON j.id = r.scan_job_id
             JOIN scan_session AS s ON s.id = r.session_id
             JOIN scan_root AS root ON root.id = r.scan_root_id
             WHERE r.id = ?1",
            [run_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, Option<i64>>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, i64>(15)?,
                    row.get::<_, i64>(16)?,
                    row.get::<_, String>(17)?,
                ))
            },
        )
        .map_err(map_not_found)?;
    let (
        actual_run_id,
        job_id,
        root_id,
        generation,
        configuration_revision,
        actual_session_id,
        actual_lease_token,
        run_state,
        run_cancel,
        lease_expires_at_ms,
        job_state,
        job_cancel,
        follow_up,
        ended_at_ms,
        enabled,
        root_generation,
        root_revision,
        job_root_id,
    ) = raw;
    if actual_run_id != run_id
        || actual_session_id != session_id
        || actual_lease_token != lease_token
        || run_state != "running"
        || job_state != "running"
        || run_cancel != 0
        || job_cancel != 0
        || follow_up != 0
        || ended_at_ms.is_some()
        || enabled != 1
        || job_root_id != root_id
        || lease_expires_at_ms <= now_ms
        || root_generation != generation
        || root_revision != configuration_revision
    {
        return Err(StorageError::Conflict);
    }
    Ok(PublicationContext {
        run_id: actual_run_id,
        job_id,
        root_id,
        generation,
        configuration_revision,
        session_id: actual_session_id,
        lease_token: actual_lease_token,
    })
}

fn ensure_open_staging(
    transaction: &Transaction<'_>,
    context: &PublicationContext,
    now_ms: i64,
) -> Result<ScanStaging> {
    if let Some(existing) = select_staging_optional(transaction, &context.run_id)? {
        let lease_matches: bool = transaction.query_row(
            "SELECT lease_token = ?1 FROM scan_stage WHERE run_id = ?2",
            params![&context.lease_token, &context.run_id],
            |row| row.get(0),
        )?;
        if existing.state != ScanStageState::Open
            || existing.scan_root_id != context.root_id
            || existing.generation != context.generation
            || existing.configuration_revision != context.configuration_revision
            || existing.session_id != context.session_id
            || !lease_matches
        {
            return Err(StorageError::Conflict);
        }
        return Ok(existing);
    }
    transaction.execute(
        "INSERT INTO scan_stage
         (run_id, scan_root_id, generation, configuration_revision, session_id,
          lease_token, state, record_count, path_bytes, created_at_ms, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'open', 0, 0, ?7, ?7)",
        params![
            &context.run_id,
            &context.root_id,
            context.generation,
            context.configuration_revision,
            &context.session_id,
            &context.lease_token,
            now_ms,
        ],
    )?;
    select_staging(transaction, &context.run_id)
}

fn select_staged_observations(
    transaction: &Transaction<'_>,
    run_id: &str,
) -> Result<Vec<StagedObservation>> {
    let mut statement = transaction.prepare(
        "SELECT normalized_path, relative_path, byte_size, modified_at_ms,
                volume_id, filesystem_file_id
         FROM scan_stage_observation
         WHERE run_id = ?1
         ORDER BY normalized_path, id",
    )?;
    statement
        .query_map([run_id], staged_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn create_project_file(
    transaction: &Transaction<'_>,
    observation: &StagedObservation,
    now_ms: i64,
) -> Result<String> {
    let id = uuid::Uuid::now_v7().to_string();
    let (display_filename, extension) = filename_parts(&observation.relative_path);
    transaction.execute(
        "INSERT INTO project_file
         (id, display_filename, extension, byte_size, modified_at_ms,
          created_at_ms, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            &id,
            display_filename,
            extension,
            observation.byte_size,
            observation.modified_at_ms,
            now_ms,
        ],
    )?;
    Ok(id)
}

fn update_project_file(
    transaction: &Transaction<'_>,
    project_file_id: &str,
    observation: &StagedObservation,
    now_ms: i64,
) -> Result<()> {
    let (display_filename, extension) = filename_parts(&observation.relative_path);
    let changed = transaction.execute(
        "UPDATE project_file
         SET display_filename = ?1, extension = ?2, byte_size = ?3,
             modified_at_ms = ?4, updated_at_ms = ?5
         WHERE id = ?6",
        params![
            display_filename,
            extension,
            observation.byte_size,
            observation.modified_at_ms,
            now_ms,
            project_file_id,
        ],
    )?;
    if changed != 1 {
        return Err(StorageError::Conflict);
    }
    Ok(())
}

fn filename_parts(path: &str) -> (String, String) {
    let display = path
        .rsplit(['/', '\\'])
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or(path)
        .to_owned();
    let extension = display
        .rsplit_once('.')
        .filter(|(stem, suffix)| !stem.is_empty() && !suffix.is_empty())
        .map(|(_, suffix)| format!(".{}", suffix.to_ascii_lowercase()))
        .unwrap_or_default();
    (display, extension)
}

fn select_location_by_path(
    transaction: &Transaction<'_>,
    root_id: &str,
    normalized_path: &str,
) -> Result<Option<ExistingLocation>> {
    transaction
        .query_row(
            "SELECT id, project_file_id, byte_size, modified_at_ms,
                    volume_id, filesystem_file_id
             FROM file_location
             WHERE scan_root_id = ?1 AND normalized_path = ?2",
            params![root_id, normalized_path],
            |row| {
                Ok(ExistingLocation {
                    id: row.get(0)?,
                    project_file_id: row.get(1)?,
                    volume_id: row.get(4)?,
                    filesystem_file_id: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn select_location_by_identity(
    transaction: &Transaction<'_>,
    root_id: &str,
    volume_id: Option<i64>,
    filesystem_file_id: Option<&str>,
) -> Result<Option<String>> {
    let (Some(volume_id), Some(filesystem_file_id)) = (volume_id, filesystem_file_id) else {
        return Ok(None);
    };
    transaction
        .query_row(
            "SELECT project_file_id
             FROM file_location
             WHERE scan_root_id = ?1 AND volume_id = ?2
               AND filesystem_file_id = ?3
             ORDER BY id
             LIMIT 1",
            params![root_id, volume_id, filesystem_file_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
}

fn publish_observation(
    transaction: &Transaction<'_>,
    context: &PublicationContext,
    observation: &StagedObservation,
    now_ms: i64,
) -> Result<()> {
    let existing =
        select_location_by_path(transaction, &context.root_id, &observation.normalized_path)?;
    let project_file_id = if let Some(location) = &existing {
        let identity_changed = location.volume_id.is_some()
            && observation.volume_id.is_some()
            && location.filesystem_file_id.is_some()
            && observation.filesystem_file_id.is_some()
            && (location.volume_id != observation.volume_id
                || location.filesystem_file_id != observation.filesystem_file_id);
        if identity_changed {
            match select_location_by_identity(
                transaction,
                &context.root_id,
                observation.volume_id,
                observation.filesystem_file_id.as_deref(),
            )? {
                Some(project_file_id) => project_file_id,
                None => create_project_file(transaction, observation, now_ms)?,
            }
        } else {
            location.project_file_id.clone()
        }
    } else {
        match select_location_by_identity(
            transaction,
            &context.root_id,
            observation.volume_id,
            observation.filesystem_file_id.as_deref(),
        )? {
            Some(project_file_id) => project_file_id,
            None => create_project_file(transaction, observation, now_ms)?,
        }
    };
    update_project_file(transaction, &project_file_id, observation, now_ms)?;

    if let Some(location) = existing {
        let changed = transaction.execute(
            "UPDATE file_location
             SET project_file_id = ?1, relative_path = ?2, byte_size = ?3,
                 modified_at_ms = ?4, volume_id = ?5, filesystem_file_id = ?6,
                 presence = 'present', last_seen_scan_run_id = ?7,
                 last_seen_at_ms = ?8, updated_at_ms = ?8
             WHERE id = ?9 AND scan_root_id = ?10",
            params![
                &project_file_id,
                &observation.relative_path,
                observation.byte_size,
                observation.modified_at_ms,
                observation.volume_id,
                observation.filesystem_file_id.as_deref(),
                &context.run_id,
                now_ms,
                &location.id,
                &context.root_id,
            ],
        )?;
        if changed != 1 {
            return Err(StorageError::Conflict);
        }
    } else {
        let location_id = uuid::Uuid::now_v7().to_string();
        transaction.execute(
            "INSERT INTO file_location
             (id, project_file_id, scan_root_id, detached_scan_root_id,
              normalized_path, relative_path, byte_size, modified_at_ms,
              volume_id, filesystem_file_id, presence, last_seen_scan_run_id,
              last_seen_at_ms, created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, ?9, 'present',
                     ?10, ?11, ?11, ?11)",
            params![
                &location_id,
                &project_file_id,
                &context.root_id,
                &observation.normalized_path,
                &observation.relative_path,
                observation.byte_size,
                observation.modified_at_ms,
                observation.volume_id,
                observation.filesystem_file_id.as_deref(),
                &context.run_id,
                now_ms,
            ],
        )?;
    }
    Ok(())
}

pub(crate) fn publish_scan_run_tx(
    transaction: &Transaction<'_>,
    run_id: &str,
    session_id: &str,
    lease_token: &str,
    now_ms: i64,
) -> Result<ScanPublication> {
    let context = validate_owner(transaction, run_id, session_id, lease_token, now_ms)?;
    let staging = select_staging(transaction, run_id)?;
    if staging.state != ScanStageState::Open
        || staging.scan_root_id != context.root_id
        || staging.generation != context.generation
        || staging.configuration_revision != context.configuration_revision
        || staging.session_id != context.session_id
    {
        return Err(StorageError::Conflict);
    }
    let lease_matches: bool = transaction.query_row(
        "SELECT lease_token = ?1 FROM scan_stage WHERE run_id = ?2",
        params![&context.lease_token, &context.run_id],
        |row| row.get(0),
    )?;
    if !lease_matches {
        return Err(StorageError::Conflict);
    }
    let observations = select_staged_observations(transaction, run_id)?;
    if observations.len() as i64 != staging.record_count
        || staging.record_count > MAX_STAGED_RECORDS
        || staging.path_bytes > MAX_STAGED_PATH_BYTES
    {
        return Err(StorageError::Conflict);
    }
    let path_bytes = observations.iter().try_fold(0_i64, |total, observation| {
        let bytes = staged_observation_path_bytes(observation)?;
        total.checked_add(bytes).ok_or(StorageError::Conflict)
    })?;
    if path_bytes != staging.path_bytes || path_bytes > MAX_STAGED_PATH_BYTES {
        return Err(StorageError::Conflict);
    }
    let mut seen = BTreeSet::new();
    for observation in &observations {
        if !seen.insert(&observation.normalized_path) {
            return Err(StorageError::Conflict);
        }
        publish_observation(transaction, &context, observation, now_ms)?;
    }
    transaction.execute(
        "UPDATE file_location
         SET presence = 'missing', updated_at_ms = ?1
         WHERE scan_root_id = ?2 AND presence = 'present'
           AND (last_seen_scan_run_id IS NULL OR last_seen_scan_run_id <> ?3)",
        params![now_ms, &context.root_id, &context.run_id],
    )?;
    let marker_changed = transaction.execute(
        "UPDATE scan_root
         SET last_successful_run_id = ?1,
             last_successful_generation = ?2,
             last_successful_at_ms = ?3
         WHERE id = ?4 AND enabled = 1 AND generation = ?2
           AND configuration_revision = ?5",
        params![
            &context.run_id,
            context.generation,
            now_ms,
            &context.root_id,
            context.configuration_revision,
        ],
    )?;
    if marker_changed != 1 {
        return Err(StorageError::Conflict);
    }
    let run_changed = transaction.execute(
        "UPDATE scan_run
         SET state = 'completed', finished_at_ms = ?1, outcome = 'completed',
             error_code = NULL
         WHERE id = ?2 AND state = 'running' AND session_id = ?3
           AND lease_token = ?4",
        params![
            now_ms,
            &context.run_id,
            &context.session_id,
            &context.lease_token
        ],
    )?;
    if run_changed != 1 {
        return Err(StorageError::Conflict);
    }
    let job_changed = transaction.execute(
        "UPDATE scan_job
         SET state = 'completed', updated_at_ms = ?1, last_error_code = NULL
         WHERE id = ?2 AND state = 'running' AND cancellation_requested = 0
           AND follow_up_requested = 0",
        params![now_ms, &context.job_id],
    )?;
    if job_changed != 1 {
        return Err(StorageError::Conflict);
    }
    let stage_changed = transaction.execute(
        "UPDATE scan_stage
         SET state = 'published', updated_at_ms = ?1, published_at_ms = ?1
         WHERE run_id = ?2 AND state = 'open'",
        params![now_ms, &context.run_id],
    )?;
    if stage_changed != 1 {
        return Err(StorageError::Conflict);
    }
    transaction.execute(
        "DELETE FROM scan_stage_observation WHERE run_id = ?1",
        [&context.run_id],
    )?;
    let location_count = transaction.query_row(
        "SELECT count(*) FROM file_location WHERE scan_root_id = ?1",
        [&context.root_id],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(ScanPublication {
        run_id: context.run_id,
        scan_root_id: context.root_id,
        generation: context.generation,
        location_count,
        published_at_ms: now_ms,
    })
}

fn published_location_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PublishedLocation> {
    Ok(PublishedLocation {
        id: row.get(0)?,
        project_file_id: row.get(1)?,
        scan_root_id: row.get(2)?,
        detached_scan_root_id: row.get(3)?,
        normalized_path: row.get(4)?,
        relative_path: row.get(5)?,
        byte_size: map_u64(row.get(6)?).map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                6,
                "byte_size".into(),
                rusqlite::types::Type::Integer,
            )
        })?,
        modified_at_ms: row.get(7)?,
        volume_id: row.get(8)?,
        filesystem_file_id: row.get(9)?,
        presence: FilePresence::parse(&row.get::<_, String>(10)?).map_err(|_| {
            rusqlite::Error::InvalidColumnType(10, "presence".into(), rusqlite::types::Type::Text)
        })?,
        last_seen_scan_run_id: row.get(11)?,
        last_seen_at_ms: row.get(12)?,
    })
}

const LOCATION_COLUMNS: &str = "SELECT id, project_file_id, scan_root_id, detached_scan_root_id,
            normalized_path, relative_path, byte_size, modified_at_ms,
            volume_id, filesystem_file_id, presence,
            last_seen_scan_run_id, last_seen_at_ms
     FROM file_location";

/// Mark staging ineligible before a running run is cancelled or invalidated.
/// The caller owns the surrounding execution transaction.
pub(crate) fn discard_staging_for_run_tx(
    transaction: &Transaction<'_>,
    run_id: &str,
    now_ms: i64,
) -> Result<()> {
    transaction.execute(
        "UPDATE scan_stage SET state = 'discarded', updated_at_ms = ?1
         WHERE run_id = ?2 AND state = 'open'",
        params![now_ms, run_id],
    )?;
    transaction.execute(
        "DELETE FROM scan_stage_observation WHERE run_id = ?1",
        [run_id],
    )?;
    Ok(())
}

pub(crate) fn discard_staging_for_job_tx(
    transaction: &Transaction<'_>,
    job_id: &str,
    now_ms: i64,
) -> Result<()> {
    transaction.execute(
        "UPDATE scan_stage SET state = 'discarded', updated_at_ms = ?1
         WHERE run_id IN (SELECT id FROM scan_run WHERE scan_job_id = ?2)
           AND state = 'open'",
        params![now_ms, job_id],
    )?;
    transaction.execute(
        "DELETE FROM scan_stage_observation
         WHERE run_id IN (SELECT id FROM scan_run WHERE scan_job_id = ?1)",
        [job_id],
    )?;
    Ok(())
}

pub(crate) fn discard_staging_for_root_tx(
    transaction: &Transaction<'_>,
    root_id: &str,
    now_ms: i64,
) -> Result<()> {
    transaction.execute(
        "UPDATE scan_stage SET state = 'discarded', updated_at_ms = ?1
         WHERE run_id IN (SELECT id FROM scan_run WHERE scan_root_id = ?2)
           AND state = 'open'",
        params![now_ms, root_id],
    )?;
    transaction.execute(
        "DELETE FROM scan_stage_observation
         WHERE run_id IN (SELECT id FROM scan_run WHERE scan_root_id = ?1)",
        [root_id],
    )?;
    Ok(())
}

pub(crate) fn discard_all_open_staging_tx(
    transaction: &Transaction<'_>,
    now_ms: i64,
) -> Result<()> {
    transaction.execute(
        "UPDATE scan_stage SET state = 'discarded', updated_at_ms = ?1
         WHERE state = 'open'",
        [now_ms],
    )?;
    transaction.execute("DELETE FROM scan_stage_observation", [])?;
    Ok(())
}

pub(crate) fn detach_locations_for_root_tx(
    transaction: &Transaction<'_>,
    root_id: &str,
    now_ms: i64,
) -> Result<()> {
    transaction.execute(
        "UPDATE file_location
         SET scan_root_id = NULL, detached_scan_root_id = ?1,
             updated_at_ms = ?2
         WHERE scan_root_id = ?1",
        params![root_id, now_ms],
    )?;
    Ok(())
}

impl Database {
    /// Begin an authoritative staging session for a leased run. An empty stage
    /// is meaningful: it represents a completed enumeration of an empty root.
    pub fn begin_scan_staging(
        &mut self,
        run_id: &str,
        session_id: &str,
        lease_token: &str,
        now_ms: i64,
    ) -> Result<ScanStaging> {
        if run_id.is_empty() || session_id.is_empty() || lease_token.is_empty() {
            return Err(StorageError::InvalidSchema);
        }
        self.transaction(|transaction| {
            let context = validate_owner(transaction, run_id, session_id, lease_token, now_ms)?;
            ensure_open_staging(transaction, &context, now_ms)
        })
    }

    /// Append one bounded, idempotent observation batch outside the committed
    /// Library dataset. Every batch validates the same run fences as publish.
    pub fn stage_scan_observations(
        &mut self,
        run_id: &str,
        session_id: &str,
        lease_token: &str,
        now_ms: i64,
        observations: &[ScanObservation],
    ) -> Result<ScanStaging> {
        if run_id.is_empty() || session_id.is_empty() || lease_token.is_empty() {
            return Err(StorageError::InvalidSchema);
        }
        if observations.len() > MAX_STAGED_BATCH_RECORDS {
            return Err(StorageError::Conflict);
        }
        self.transaction(|transaction| {
            let context = validate_owner(transaction, run_id, session_id, lease_token, now_ms)?;
            let staging = ensure_open_staging(transaction, &context, now_ms)?;
            let mut batch_paths = BTreeSet::new();
            let mut new_count = 0_i64;
            let mut new_path_bytes = 0_i64;
            for observation in observations {
                let path_bytes = observation_path_bytes(observation)?;
                if !batch_paths.insert(&observation.normalized_path) {
                    return Err(StorageError::Conflict);
                }
                let byte_size =
                    i64::try_from(observation.byte_size).map_err(|_| StorageError::Conflict)?;
                let existing = transaction
                    .query_row(
                        "SELECT relative_path, byte_size, modified_at_ms, volume_id,
                                filesystem_file_id
                         FROM scan_stage_observation
                         WHERE run_id = ?1 AND normalized_path = ?2",
                        params![&context.run_id, &observation.normalized_path],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, i64>(1)?,
                                row.get::<_, i64>(2)?,
                                row.get::<_, Option<i64>>(3)?,
                                row.get::<_, Option<String>>(4)?,
                            ))
                        },
                    )
                    .optional()?;
                if let Some(existing) = existing {
                    if existing
                        != (
                            observation.relative_path.clone(),
                            byte_size,
                            observation.modified_at_ms,
                            observation.volume_id,
                            observation.filesystem_file_id.clone(),
                        )
                    {
                        return Err(StorageError::Conflict);
                    }
                    continue;
                }
                new_count = new_count.checked_add(1).ok_or(StorageError::Conflict)?;
                new_path_bytes = new_path_bytes
                    .checked_add(path_bytes)
                    .ok_or(StorageError::Conflict)?;
                transaction.execute(
                    "INSERT INTO scan_stage_observation
                     (run_id, normalized_path, relative_path, byte_size,
                      modified_at_ms, volume_id, filesystem_file_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        &context.run_id,
                        &observation.normalized_path,
                        &observation.relative_path,
                        byte_size,
                        observation.modified_at_ms,
                        observation.volume_id,
                        observation.filesystem_file_id.as_deref(),
                    ],
                )?;
            }
            if staging
                .record_count
                .checked_add(new_count)
                .is_none_or(|count| count > MAX_STAGED_RECORDS)
                || staging
                    .path_bytes
                    .checked_add(new_path_bytes)
                    .is_none_or(|bytes| bytes > MAX_STAGED_PATH_BYTES)
            {
                return Err(StorageError::Conflict);
            }
            transaction.execute(
                "UPDATE scan_stage
                 SET record_count = record_count + ?1,
                     path_bytes = path_bytes + ?2, updated_at_ms = ?3
                 WHERE run_id = ?4 AND state = 'open'",
                params![new_count, new_path_bytes, now_ms, &context.run_id],
            )?;
            select_staging(transaction, &context.run_id)
        })
    }

    /// Publish all staged observations and complete the run in one SQLite
    /// transaction. No completed ledger state is written before this succeeds.
    pub fn publish_scan_run(
        &mut self,
        run_id: &str,
        session_id: &str,
        lease_token: &str,
        now_ms: i64,
    ) -> Result<ScanPublication> {
        if run_id.is_empty() || session_id.is_empty() || lease_token.is_empty() {
            return Err(StorageError::InvalidSchema);
        }
        self.transaction(|transaction| {
            publish_scan_run_tx(transaction, run_id, session_id, lease_token, now_ms)
        })
    }

    pub fn scan_staging(&self, run_id: &str) -> Result<ScanStaging> {
        let transaction = self.connection.unchecked_transaction()?;
        let result = select_staging(&transaction, run_id);
        transaction.rollback()?;
        result
    }

    pub fn list_published_locations(&self, root_id: &str) -> Result<Vec<PublishedLocation>> {
        let exists: i64 = self.connection.query_row(
            "SELECT count(*) FROM scan_root WHERE id = ?1",
            [root_id],
            |row| row.get(0),
        )?;
        if exists != 1 {
            return Err(StorageError::NotFound);
        }
        let mut statement = self.connection.prepare(&format!(
            "{LOCATION_COLUMNS} WHERE scan_root_id = ?1 ORDER BY normalized_path, id"
        ))?;
        statement
            .query_map([root_id], published_location_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    /// Detached locations are retained after a root is removed. They are
    /// history only and cannot be selected by a newly added root.
    pub fn list_detached_locations(&self) -> Result<Vec<PublishedLocation>> {
        let mut statement = self.connection.prepare(&format!(
            "{LOCATION_COLUMNS}
             WHERE scan_root_id IS NULL AND detached_scan_root_id IS NOT NULL
             ORDER BY detached_scan_root_id, normalized_path, id"
        ))?;
        statement
            .query_map([], published_location_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn scan_root_publication(&self, root_id: &str) -> Result<ScanRootPublication> {
        self.connection
            .query_row(
                "SELECT id, last_successful_run_id, last_successful_generation,
                        last_successful_at_ms
                 FROM scan_root WHERE id = ?1",
                [root_id],
                |row| {
                    Ok(ScanRootPublication {
                        scan_root_id: row.get(0)?,
                        last_successful_run_id: row.get(1)?,
                        last_successful_generation: row.get(2)?,
                        last_successful_at_ms: row.get(3)?,
                    })
                },
            )
            .map_err(map_not_found)
    }
}
