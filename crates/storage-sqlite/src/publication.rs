use super::{Database, Result, StorageError};
use rusqlite::{OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod serde_decimal {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    /// Canonical unsigned decimal (§2.1 of the integration contract): "0" or a
    /// non-zero digit followed by digits. Rejects leading zeros, signs,
    /// whitespace, and non-decimal input before any range check.
    fn is_canonical_unsigned(value: &str) -> bool {
        !value.is_empty()
            && (value == "0" || !value.starts_with('0'))
            && value.bytes().all(|byte| byte.is_ascii_digit())
    }

    /// Canonical signed decimal: an optional leading '-' over a canonical
    /// unsigned magnitude. "-0" and "+" are never canonical.
    fn is_canonical_signed(value: &str) -> bool {
        if let Some(magnitude) = value.strip_prefix('-') {
            magnitude != "0" && is_canonical_unsigned(magnitude)
        } else {
            is_canonical_unsigned(value)
        }
    }

    pub mod u64_string {
        use super::*;

        pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&value.to_string())
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
        where
            D: Deserializer<'de>,
        {
            let value = String::deserialize(deserializer)?;
            if !is_canonical_unsigned(&value) {
                return Err(D::Error::custom(
                    "byte_size must be a canonical decimal string",
                ));
            }
            value
                .parse::<u64>()
                .map_err(|_| D::Error::custom("byte_size is outside u64 range"))
        }
    }

    pub mod i64_string {
        use super::*;

        pub fn serialize<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&value.to_string())
        }
    }

    pub mod i128_string {
        use super::*;

        pub fn serialize<S>(value: &i128, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&value.to_string())
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<i128, D::Error>
        where
            D: Deserializer<'de>,
        {
            let value = String::deserialize(deserializer)?;
            if !is_canonical_signed(&value) {
                return Err(D::Error::custom(
                    "modified_at_ns must be a canonical decimal string",
                ));
            }
            value
                .parse::<i128>()
                .map_err(|_| D::Error::custom("modified_at_ns is outside i128 range"))
        }
    }
}

/// A single worker batch is deliberately bounded. The eventual enumerator can
/// stream many batches, while each transaction remains small and retryable.
pub const MAX_STAGED_BATCH_RECORDS: usize = 512;
pub const MAX_STAGED_RECORDS: i64 = 10_000;
pub const MAX_STAGED_PATH_BYTES: i64 = 4 * 1024 * 1024;
const MAX_OBSERVATION_PATH_BYTES: usize = 32 * 1024;
pub const MAX_LIBRARY_PAGE_SIZE: usize = 200;

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedIdentity {
    /// Canonical unsigned decimal encoding of a Rust `u64` volume serial.
    pub volume_serial: String,
    /// Canonical unsigned decimal encoding of a Rust `u128` file ID.
    pub file_id: String,
}

impl<'de> Deserialize<'de> for EncodedIdentity {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawIdentity {
            volume_serial: String,
            file_id: String,
        }
        let raw = RawIdentity::deserialize(deserializer)?;
        // Fixed safe error: never echo the offending value.
        if canonical_u64(&raw.volume_serial) && canonical_u128(&raw.file_id) {
            Ok(EncodedIdentity {
                volume_serial: raw.volume_serial,
                file_id: raw.file_id,
            })
        } else {
            Err(serde::de::Error::custom(
                "identity must be a canonical u64/u128 decimal pair",
            ))
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanObservation {
    /// Boundary-owned comparison key: `LocatorKeyV1` (§1.1 of the integration
    /// contract). It is not a display path and is never derived by lowercasing
    /// `relative_path` in this crate.
    pub locator_key: String,
    pub relative_path: String,
    #[serde(with = "serde_decimal::u64_string")]
    pub byte_size: u64,
    /// The filesystem boundary supplies i128; staging performs a checked
    /// conversion to SQLite's signed 64-bit nanosecond representation.
    #[serde(with = "serde_decimal::i128_string")]
    pub modified_at_ns: i128,
    pub identity: Option<EncodedIdentity>,
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
    pub locator_key: String,
    pub relative_path: String,
    #[serde(with = "serde_decimal::u64_string")]
    pub byte_size: u64,
    #[serde(with = "serde_decimal::i64_string")]
    pub modified_at_ns: i64,
    pub identity: Option<EncodedIdentity>,
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

/// Snapshot-bound position in the stable, root-scoped Library order. The IPC
/// adapter serializes this as an opaque cursor string. A cursor expires when
/// its committed root snapshot changes; clients restart with `cursor: null`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryCursor {
    pub scan_root_id: String,
    pub snapshot: LibrarySnapshot,
    pub locator_key: String,
    pub location_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub last_successful_run_id: Option<String>,
    pub last_successful_generation: Option<i64>,
    pub last_successful_at_ms: Option<i64>,
}

/// Bounded read-only Library query. Pagination is deliberately per root: a
/// global page would need a cross-root snapshot that this slice does not own.
/// This is the storage-side contract for a typed IPC adapter; it is
/// intentionally not a generic SQL or list API.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryQuery {
    pub scan_root_id: String,
    pub page_size: usize,
    pub cursor: Option<LibraryCursor>,
    pub snapshot: Option<LibrarySnapshot>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPage {
    pub scan_root_id: String,
    pub locations: Vec<PublishedLocation>,
    pub next_cursor: Option<LibraryCursor>,
    pub snapshot: LibrarySnapshot,
    pub has_more: bool,
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
    locator_key: String,
    relative_path: String,
    byte_size: i64,
    modified_at_ns: i64,
    identity: Option<EncodedIdentity>,
}

#[derive(Clone, Debug)]
struct ExistingLocation {
    id: String,
    project_file_id: String,
    locator_key: String,
    identity: Option<EncodedIdentity>,
    presence: FilePresence,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct QualifiedIdentity {
    volume_serial: String,
    file_id: String,
}

#[derive(Clone, Debug)]
struct ProjectFileAssignment {
    id: String,
    is_new: bool,
}

#[derive(Clone, Debug)]
struct PlannedObservation {
    observation: StagedObservation,
    location_id: Option<String>,
    project_file: ProjectFileAssignment,
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

fn canonical_u64(value: &str) -> bool {
    !value.is_empty()
        && (value == "0" || !value.starts_with('0'))
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u64>().is_ok()
}

fn canonical_u128(value: &str) -> bool {
    !value.is_empty()
        && (value == "0" || !value.starts_with('0'))
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u128>().is_ok()
}

fn validate_identity(identity: &EncodedIdentity) -> Result<()> {
    if canonical_u64(&identity.volume_serial) && canonical_u128(&identity.file_id) {
        Ok(())
    } else {
        Err(StorageError::Conflict)
    }
}

fn identity_from_columns(
    volume_serial: Option<String>,
    file_id: Option<String>,
) -> Result<Option<EncodedIdentity>> {
    match (volume_serial, file_id) {
        (None, None) => Ok(None),
        (Some(volume_serial), Some(file_id)) => {
            let identity = EncodedIdentity {
                volume_serial,
                file_id,
            };
            validate_identity(&identity)?;
            Ok(Some(identity))
        }
        _ => Err(StorageError::InvalidSchema),
    }
}

fn sqlite_invalid_column(column: usize, name: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(column, name.into(), rusqlite::types::Type::Text)
}

/// Version prefix of the storage integration contract's locator key (§1.1).
pub const LOCATOR_KEY_V1_PREFIX: &str = "v1:";

/// Every '%' must open exactly two hexadecimal digits. Storage accepts either
/// hex case on read; the enumerator emits uppercase per §1.1.
fn valid_pct_encoding(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = bytes.get(index + 1).is_some_and(u8::is_ascii_hexdigit);
            let low = bytes.get(index + 2).is_some_and(u8::is_ascii_hexdigit);
            if !high || !low {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

/// Syntactic validation for `LocatorKeyV1` (§1.1 of the integration contract).
///
/// Storage treats a valid key as opaque bytes afterwards: no folding,
/// lowercasing, or NOCASE comparison happens here. Structural validation
/// exists so a raw display path or an unversioned v4 path fails fast with
/// `staging_rejected` instead of silently aliasing an unrelated location.
fn is_valid_locator_key_v1(key: &str) -> bool {
    if key.len() > MAX_OBSERVATION_PATH_BYTES || !key.is_ascii() || key.contains('\0') {
        return false;
    }
    let Some(body) = key.strip_prefix(LOCATOR_KEY_V1_PREFIX) else {
        return false;
    };
    if body.is_empty() {
        return false;
    }
    body.split('/').all(|segment| {
        let Some((mode, encoded)) = segment.split_once(':') else {
            return false;
        };
        (mode == "i" || mode == "s")
            && !encoded.is_empty()
            && encoded.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'%')
            })
            && valid_pct_encoding(encoded)
    })
}

fn observation_path_bytes(observation: &ScanObservation) -> Result<i64> {
    if !is_valid_locator_key_v1(&observation.locator_key)
        || observation.relative_path.is_empty()
        || observation.relative_path.len() > MAX_OBSERVATION_PATH_BYTES
        || observation.relative_path.contains(['\0', ':'])
        || observation.relative_path.starts_with(['/', '\\'])
        || observation
            .relative_path
            .split(['/', '\\'])
            .any(|part| matches!(part, "" | "." | ".."))
        || observation
            .identity
            .as_ref()
            .is_some_and(|identity| validate_identity(identity).is_err())
    {
        return Err(StorageError::Conflict);
    }
    let bytes = observation
        .locator_key
        .len()
        .checked_add(observation.relative_path.len())
        .ok_or(StorageError::Conflict)?;
    i64::try_from(bytes).map_err(|_| StorageError::Conflict)
}

fn valid_snapshot(snapshot: &LibrarySnapshot) -> bool {
    snapshot
        .last_successful_run_id
        .as_deref()
        .is_none_or(|value| !value.is_empty() && !value.contains('\0'))
        && snapshot
            .last_successful_generation
            .is_none_or(|generation| generation >= 0)
}

fn valid_cursor(cursor: &LibraryCursor) -> bool {
    !cursor.scan_root_id.is_empty()
        && !cursor.scan_root_id.contains('\0')
        && is_valid_locator_key_v1(&cursor.locator_key)
        && !cursor.location_id.is_empty()
        && !cursor.location_id.contains('\0')
        && valid_snapshot(&cursor.snapshot)
}

fn staged_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StagedObservation> {
    let identity = identity_from_columns(row.get(4)?, row.get(5)?)
        .map_err(|_| sqlite_invalid_column(4, "encoded_identity"))?;
    Ok(StagedObservation {
        locator_key: row.get(0)?,
        relative_path: row.get(1)?,
        byte_size: row.get(2)?,
        modified_at_ns: row.get(3)?,
        identity,
    })
}

fn staged_observation_path_bytes(observation: &StagedObservation) -> Result<i64> {
    observation_path_bytes(&ScanObservation {
        locator_key: observation.locator_key.clone(),
        relative_path: observation.relative_path.clone(),
        byte_size: map_u64(observation.byte_size)?,
        modified_at_ns: i128::from(observation.modified_at_ns),
        identity: observation.identity.clone(),
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
        "SELECT locator_key, relative_path, byte_size, modified_at_ns,
                identity_volume_serial, identity_file_id
         FROM scan_stage_observation
         WHERE run_id = ?1
         ORDER BY locator_key COLLATE BINARY, id",
    )?;
    statement
        .query_map([run_id], staged_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn insert_project_file(
    transaction: &Transaction<'_>,
    project_file_id: &str,
    observation: &StagedObservation,
    now_ms: i64,
) -> Result<()> {
    let (display_filename, extension) = filename_parts(&observation.relative_path);
    transaction.execute(
        "INSERT INTO project_file
         (id, display_filename, extension, byte_size, modified_at_ms,
          modified_at_ns, created_at_ms, updated_at_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            project_file_id,
            display_filename,
            extension,
            observation.byte_size,
            legacy_modified_at_ms(observation.modified_at_ns),
            observation.modified_at_ns,
            now_ms,
        ],
    )?;
    Ok(())
}

fn new_project_file_assignment() -> ProjectFileAssignment {
    ProjectFileAssignment {
        id: uuid::Uuid::now_v7().to_string(),
        is_new: true,
    }
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
              modified_at_ms = ?4, modified_at_ns = ?5, updated_at_ms = ?6
         WHERE id = ?7",
        params![
            display_filename,
            extension,
            observation.byte_size,
            legacy_modified_at_ms(observation.modified_at_ns),
            observation.modified_at_ns,
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

fn legacy_modified_at_ms(modified_at_ns: i64) -> i64 {
    modified_at_ns / 1_000_000
}

fn legacy_volume_id(identity: Option<&EncodedIdentity>) -> Option<i64> {
    identity
        .and_then(|identity| identity.volume_serial.parse::<u64>().ok())
        .and_then(|value| i64::try_from(value).ok())
}

fn qualified_identity(identity: Option<&EncodedIdentity>) -> Option<QualifiedIdentity> {
    identity.map(|identity| QualifiedIdentity {
        volume_serial: identity.volume_serial.clone(),
        file_id: identity.file_id.clone(),
    })
}

fn select_root_locations(
    transaction: &Transaction<'_>,
    root_id: &str,
) -> Result<Vec<ExistingLocation>> {
    let mut statement = transaction.prepare(
        "SELECT id, project_file_id, locator_key, identity_volume_serial,
                identity_file_id, presence
         FROM file_location
         WHERE scan_root_id = ?1
         ORDER BY locator_key COLLATE BINARY, id",
    )?;
    statement
        .query_map([root_id], |row| {
            let presence = FilePresence::parse(&row.get::<_, String>(5)?).map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    5,
                    "presence".into(),
                    rusqlite::types::Type::Text,
                )
            })?;
            let identity = identity_from_columns(row.get(3)?, row.get(4)?)
                .map_err(|_| sqlite_invalid_column(3, "encoded_identity"))?;
            Ok(ExistingLocation {
                id: row.get(0)?,
                project_file_id: row.get(1)?,
                locator_key: row.get(2)?,
                identity,
                presence,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn identities_differ(previous: &ExistingLocation, current: Option<&QualifiedIdentity>) -> bool {
    matches!(
        (
            qualified_identity(previous.identity.as_ref()),
            current
        ),
        (Some(previous), Some(current)) if previous != *current
    )
}

/// Build all physical associations before mutating the committed dataset.
///
/// A qualified identity is reusable only when it has current evidence in this
/// complete enumeration: either an already-present location is observed at the
/// same path with the same identity, or an already-present location at an
/// unobserved path supplies the unambiguous rename source. Missing locations
/// are deliberately excluded from this lookup. Exact-path continuity remains
/// separate so a missing path can be restored without making a different new
/// path inherit its historical physical record.
fn plan_observations(
    transaction: &Transaction<'_>,
    root_id: &str,
    observations: &[StagedObservation],
) -> Result<Vec<PlannedObservation>> {
    let previous = select_root_locations(transaction, root_id)?;
    let previous_by_path: BTreeMap<_, _> = previous
        .iter()
        .map(|location| (location.locator_key.as_str(), location))
        .collect();
    let observed_by_path: BTreeMap<_, _> = observations
        .iter()
        .map(|observation| (observation.locator_key.as_str(), observation))
        .collect();

    let observed_identities: BTreeSet<_> = observations
        .iter()
        .filter_map(|observation| qualified_identity(observation.identity.as_ref()))
        .collect();

    // Candidates from the prior committed set are qualified by the complete
    // current observation set. A prior present path observed with a different
    // identity is a replacement, not evidence for the old identity.
    let mut identity_candidates: BTreeMap<QualifiedIdentity, BTreeSet<String>> = BTreeMap::new();
    for location in previous.iter().filter(|location| {
        location.presence == FilePresence::Present && location.identity.is_some()
    }) {
        let identity =
            qualified_identity(location.identity.as_ref()).ok_or(StorageError::Conflict)?;
        let current_supports_identity = observed_by_path
            .get(location.locator_key.as_str())
            .map(|observation| {
                qualified_identity(observation.identity.as_ref()) == Some(identity.clone())
            })
            .unwrap_or(true);
        if current_supports_identity {
            identity_candidates
                .entry(identity)
                .or_default()
                .insert(location.project_file_id.clone());
        }
    }

    // Exact-path continuity is allowed for an existing row, including a
    // missing row being restored. It is not used as a historical identity
    // lookup for an unrelated path.
    let mut path_continuity = Vec::with_capacity(observations.len());
    let mut continuity_candidates: BTreeMap<QualifiedIdentity, BTreeSet<String>> = BTreeMap::new();
    for observation in observations {
        let current_identity = qualified_identity(observation.identity.as_ref());
        let continuity = previous_by_path
            .get(observation.locator_key.as_str())
            .filter(|location| !identities_differ(location, current_identity.as_ref()))
            .map(|location| location.project_file_id.clone());
        if let (Some(identity), Some(project_file_id)) = (current_identity, continuity.as_ref()) {
            continuity_candidates
                .entry(identity)
                .or_default()
                .insert(project_file_id.clone());
        }
        path_continuity.push(continuity);
    }

    let mut identity_assignments = BTreeMap::new();
    for identity in observed_identities {
        let mut candidates = identity_candidates.remove(&identity).unwrap_or_default();
        if let Some(path_candidates) = continuity_candidates.remove(&identity) {
            candidates.extend(path_candidates);
        }
        let assignment = if candidates.len() == 1 {
            ProjectFileAssignment {
                id: candidates.into_iter().next().expect("one candidate"),
                is_new: false,
            }
        } else {
            // No evidence, or conflicting evidence, means a conservative new
            // physical record. Every current alias in this identity group then
            // shares that one new record, independent of stage input order.
            new_project_file_assignment()
        };
        identity_assignments.insert(identity, assignment);
    }

    let mut planned = Vec::with_capacity(observations.len());
    for (index, observation) in observations.iter().enumerate() {
        let identity = qualified_identity(observation.identity.as_ref());
        let project_file = if let Some(identity) = identity {
            identity_assignments
                .get(&identity)
                .cloned()
                .ok_or(StorageError::Conflict)?
        } else if let Some(project_file_id) = &path_continuity[index] {
            ProjectFileAssignment {
                id: project_file_id.clone(),
                is_new: false,
            }
        } else {
            new_project_file_assignment()
        };
        planned.push(PlannedObservation {
            observation: observation.clone(),
            location_id: previous_by_path
                .get(observation.locator_key.as_str())
                .map(|location| location.id.clone()),
            project_file,
        });
    }
    Ok(planned)
}

fn apply_planned_observation(
    transaction: &Transaction<'_>,
    context: &PublicationContext,
    planned: &PlannedObservation,
    now_ms: i64,
) -> Result<()> {
    update_project_file(
        transaction,
        &planned.project_file.id,
        &planned.observation,
        now_ms,
    )?;

    if let Some(location_id) = &planned.location_id {
        let changed = transaction.execute(
            "UPDATE file_location
             SET project_file_id = ?1, relative_path = ?2, byte_size = ?3,
                 normalized_path = ?4, locator_key = ?4,
                 modified_at_ms = ?5, modified_at_ns = ?6,
                 volume_id = ?7, filesystem_file_id = ?8,
                 identity_volume_serial = ?9, identity_file_id = ?10,
                 presence = 'present', last_seen_scan_run_id = ?11,
                 last_seen_at_ms = ?12, updated_at_ms = ?12
             WHERE id = ?13 AND scan_root_id = ?14",
            params![
                &planned.project_file.id,
                &planned.observation.relative_path,
                planned.observation.byte_size,
                &planned.observation.locator_key,
                legacy_modified_at_ms(planned.observation.modified_at_ns),
                planned.observation.modified_at_ns,
                legacy_volume_id(planned.observation.identity.as_ref()),
                planned
                    .observation
                    .identity
                    .as_ref()
                    .map(|identity| identity.file_id.as_str()),
                planned
                    .observation
                    .identity
                    .as_ref()
                    .map(|identity| identity.volume_serial.as_str()),
                planned
                    .observation
                    .identity
                    .as_ref()
                    .map(|identity| identity.file_id.as_str()),
                &context.run_id,
                now_ms,
                location_id,
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
              normalized_path, locator_key, relative_path, byte_size,
              modified_at_ms, modified_at_ns, volume_id, filesystem_file_id,
              identity_volume_serial, identity_file_id, presence,
              last_seen_scan_run_id, last_seen_at_ms, created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, NULL, ?4, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                     ?11, ?12, 'present', ?13, ?14, ?14, ?14)",
            params![
                &location_id,
                &planned.project_file.id,
                &context.root_id,
                &planned.observation.locator_key,
                &planned.observation.relative_path,
                planned.observation.byte_size,
                legacy_modified_at_ms(planned.observation.modified_at_ns),
                planned.observation.modified_at_ns,
                legacy_volume_id(planned.observation.identity.as_ref()),
                planned
                    .observation
                    .identity
                    .as_ref()
                    .map(|identity| identity.file_id.as_str()),
                planned
                    .observation
                    .identity
                    .as_ref()
                    .map(|identity| identity.volume_serial.as_str()),
                planned
                    .observation
                    .identity
                    .as_ref()
                    .map(|identity| identity.file_id.as_str()),
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
    if observations
        .iter()
        .any(|observation| !seen.insert(&observation.locator_key))
    {
        return Err(StorageError::Conflict);
    }
    let planned = plan_observations(transaction, &context.root_id, &observations)?;
    let mut inserted_project_files = BTreeSet::new();
    for item in &planned {
        if item.project_file.is_new && inserted_project_files.insert(&item.project_file.id) {
            insert_project_file(
                transaction,
                &item.project_file.id,
                &item.observation,
                now_ms,
            )?;
        }
        apply_planned_observation(transaction, &context, item, now_ms)?;
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
    let identity = identity_from_columns(row.get(8)?, row.get(9)?)
        .map_err(|_| sqlite_invalid_column(8, "encoded_identity"))?;
    Ok(PublishedLocation {
        id: row.get(0)?,
        project_file_id: row.get(1)?,
        scan_root_id: row.get(2)?,
        detached_scan_root_id: row.get(3)?,
        locator_key: row.get(4)?,
        relative_path: row.get(5)?,
        byte_size: map_u64(row.get(6)?).map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                6,
                "byte_size".into(),
                rusqlite::types::Type::Integer,
            )
        })?,
        modified_at_ns: row.get(7)?,
        identity,
        presence: FilePresence::parse(&row.get::<_, String>(10)?).map_err(|_| {
            rusqlite::Error::InvalidColumnType(10, "presence".into(), rusqlite::types::Type::Text)
        })?,
        last_seen_scan_run_id: row.get(11)?,
        last_seen_at_ms: row.get(12)?,
    })
}

const LOCATION_COLUMNS: &str = "SELECT id, project_file_id, scan_root_id, detached_scan_root_id,
            locator_key, relative_path, byte_size, modified_at_ns,
            identity_volume_serial, identity_file_id, presence,
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

/// Terminal input errors cannot leave a partially staged authoritative run.
/// This transition is committed together with stage cleanup. SQLite busy or
/// other database failures do not enter here: their surrounding transaction
/// rolls back and the caller may retry the same batch against the open stage.
fn invalidate_terminal_staging_tx(
    transaction: &Transaction<'_>,
    context: &PublicationContext,
    now_ms: i64,
) -> Result<()> {
    discard_staging_for_run_tx(transaction, &context.run_id, now_ms)?;
    let run_changed = transaction.execute(
        "UPDATE scan_run
         SET state = 'failed', finished_at_ms = ?1, outcome = 'failed',
             error_code = 'staging_rejected'
         WHERE id = ?2 AND scan_job_id = ?3 AND state = 'running'
           AND session_id = ?4 AND lease_token = ?5",
        params![
            now_ms,
            &context.run_id,
            &context.job_id,
            &context.session_id,
            &context.lease_token,
        ],
    )?;
    if run_changed != 1 {
        return Err(StorageError::Conflict);
    }
    let job_changed = transaction.execute(
        "UPDATE scan_job
         SET state = 'failed', updated_at_ms = ?1,
             last_error_code = 'staging_rejected'
         WHERE id = ?2 AND state = 'running'
           AND cancellation_requested = 0 AND follow_up_requested = 0",
        params![now_ms, &context.job_id],
    )?;
    if job_changed != 1 {
        return Err(StorageError::Conflict);
    }
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
        let committed: Result<Result<ScanStaging>> = self.transaction(|transaction| {
            let context = validate_owner(transaction, run_id, session_id, lease_token, now_ms)?;
            let staging = ensure_open_staging(transaction, &context, now_ms)?;
            let attempt: Result<ScanStaging> = (|| {
                if observations.len() > MAX_STAGED_BATCH_RECORDS {
                    return Err(StorageError::StagingRejected);
                }
                let mut batch_paths = BTreeSet::new();
                let mut new_count = 0_i64;
                let mut new_path_bytes = 0_i64;
                for observation in observations {
                    let path_bytes = observation_path_bytes(observation)
                        .map_err(|_| StorageError::StagingRejected)?;
                    if !batch_paths.insert(&observation.locator_key) {
                        return Err(StorageError::StagingRejected);
                    }
                    let byte_size = i64::try_from(observation.byte_size)
                        .map_err(|_| StorageError::StagingRejected)?;
                    let modified_at_ns = i64::try_from(observation.modified_at_ns)
                        .map_err(|_| StorageError::StagingRejected)?;
                    let existing = transaction
                        .query_row(
                            "SELECT relative_path, byte_size, modified_at_ns,
                                    identity_volume_serial, identity_file_id
                             FROM scan_stage_observation
                             WHERE run_id = ?1 AND locator_key = ?2",
                            params![&context.run_id, &observation.locator_key],
                            |row| {
                                let identity = identity_from_columns(row.get(3)?, row.get(4)?)
                                    .map_err(|_| sqlite_invalid_column(3, "encoded_identity"))?;
                                Ok((
                                    row.get::<_, String>(0)?,
                                    row.get::<_, i64>(1)?,
                                    row.get::<_, i64>(2)?,
                                    identity,
                                ))
                            },
                        )
                        .optional()?;
                    if let Some(existing) = existing {
                        if existing
                            != (
                                observation.relative_path.clone(),
                                byte_size,
                                modified_at_ns,
                                observation.identity.clone(),
                            )
                        {
                            return Err(StorageError::StagingRejected);
                        }
                        continue;
                    }
                    new_count = new_count
                        .checked_add(1)
                        .ok_or(StorageError::StagingRejected)?;
                    new_path_bytes = new_path_bytes
                        .checked_add(path_bytes)
                        .ok_or(StorageError::StagingRejected)?;
                    transaction.execute(
                        "INSERT INTO scan_stage_observation
                         (run_id, normalized_path, locator_key, relative_path, byte_size,
                          modified_at_ms, modified_at_ns, volume_id, filesystem_file_id,
                          identity_volume_serial, identity_file_id)
                         VALUES (?1, ?2, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        params![
                            &context.run_id,
                            &observation.locator_key,
                            &observation.relative_path,
                            byte_size,
                            legacy_modified_at_ms(modified_at_ns),
                            modified_at_ns,
                            legacy_volume_id(observation.identity.as_ref()),
                            observation
                                .identity
                                .as_ref()
                                .map(|identity| identity.file_id.as_str()),
                            observation
                                .identity
                                .as_ref()
                                .map(|identity| identity.volume_serial.as_str()),
                            observation
                                .identity
                                .as_ref()
                                .map(|identity| identity.file_id.as_str()),
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
                    return Err(StorageError::StagingRejected);
                }
                transaction.execute(
                    "UPDATE scan_stage
                     SET record_count = record_count + ?1,
                         path_bytes = path_bytes + ?2, updated_at_ms = ?3
                     WHERE run_id = ?4 AND state = 'open'",
                    params![new_count, new_path_bytes, now_ms, &context.run_id],
                )?;
                select_staging(transaction, &context.run_id)
            })();
            match attempt {
                Ok(staging) => Ok(Ok(staging)),
                Err(StorageError::StagingRejected) => {
                    invalidate_terminal_staging_tx(transaction, &context, now_ms)?;
                    Ok(Err(StorageError::StagingRejected))
                }
                Err(error) => Err(error),
            }
        });
        committed?
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

    /// Read a bounded, stable page for one root. The client sends no SQL and
    /// must echo the returned snapshot on subsequent pages. A committed
    /// publication expires the cursor and requires a restart from page one.
    pub fn query_library(&self, query: &LibraryQuery) -> Result<LibraryPage> {
        if query.scan_root_id.is_empty()
            || query.scan_root_id.contains('\0')
            || query.page_size == 0
            || query.page_size > MAX_LIBRARY_PAGE_SIZE
        {
            return Err(StorageError::InvalidSchema);
        }
        if query
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| !valid_snapshot(snapshot))
        {
            return Err(StorageError::InvalidCursor);
        }
        if let Some(cursor) = query.cursor.as_ref()
            && (!valid_cursor(cursor)
                || cursor.scan_root_id != query.scan_root_id
                || query.snapshot.as_ref() != Some(&cursor.snapshot))
        {
            return Err(StorageError::InvalidCursor);
        }
        let transaction = self.connection.unchecked_transaction()?;
        let snapshot = transaction
            .query_row(
                "SELECT last_successful_run_id, last_successful_generation,
                        last_successful_at_ms
                 FROM scan_root WHERE id = ?1",
                [&query.scan_root_id],
                |row| {
                    Ok(LibrarySnapshot {
                        last_successful_run_id: row.get(0)?,
                        last_successful_generation: row.get(1)?,
                        last_successful_at_ms: row.get(2)?,
                    })
                },
            )
            .map_err(map_not_found)?;
        if query
            .snapshot
            .as_ref()
            .is_some_and(|expected| expected != &snapshot)
        {
            return Err(StorageError::StaleCursor);
        }

        let cursor_key = query
            .cursor
            .as_ref()
            .map(|cursor| cursor.locator_key.as_str());
        let cursor_id = query
            .cursor
            .as_ref()
            .map(|cursor| cursor.location_id.as_str());
        if let Some(cursor) = query.cursor.as_ref() {
            let exists: i64 = transaction.query_row(
                "SELECT count(*) FROM file_location
                 WHERE scan_root_id = ?1 AND locator_key = ?2 AND id = ?3",
                params![
                    &query.scan_root_id,
                    &cursor.locator_key,
                    &cursor.location_id
                ],
                |row| row.get(0),
            )?;
            if exists != 1 {
                return Err(StorageError::StaleCursor);
            }
        }
        let limit = i64::try_from(query.page_size + 1).map_err(|_| StorageError::Conflict)?;
        let mut statement = transaction.prepare(&format!(
            "{LOCATION_COLUMNS}
             WHERE scan_root_id = ?1
               AND (?2 IS NULL OR locator_key > ?2
                    OR (locator_key = ?2 AND id > ?3))
             ORDER BY locator_key COLLATE BINARY, id
             LIMIT ?4"
        ))?;
        let mut locations = statement
            .query_map(
                params![&query.scan_root_id, cursor_key, cursor_id, limit],
                published_location_from_row,
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        let has_more = locations.len() > query.page_size;
        if has_more {
            locations.pop();
        }
        let next_cursor = locations.last().map(|location| LibraryCursor {
            scan_root_id: query.scan_root_id.clone(),
            snapshot: snapshot.clone(),
            locator_key: location.locator_key.clone(),
            location_id: location.id.clone(),
        });
        let page = LibraryPage {
            scan_root_id: query.scan_root_id.clone(),
            locations,
            next_cursor,
            snapshot,
            has_more,
        };
        transaction.rollback()?;
        Ok(page)
    }

    /// Unbounded storage-only fixture/maintenance read. Do not bind this
    /// method to IPC; clients use `query_library` above.
    #[cfg(test)]
    pub(crate) fn list_published_locations(&self, root_id: &str) -> Result<Vec<PublishedLocation>> {
        let exists: i64 = self.connection.query_row(
            "SELECT count(*) FROM scan_root WHERE id = ?1",
            [root_id],
            |row| row.get(0),
        )?;
        if exists != 1 {
            return Err(StorageError::NotFound);
        }
        let mut statement = self.connection.prepare(&format!(
            "{LOCATION_COLUMNS} WHERE scan_root_id = ?1
             ORDER BY locator_key COLLATE BINARY, id"
        ))?;
        statement
            .query_map([root_id], published_location_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    /// Detached locations are retained after a root is removed. They are
    /// history only and cannot be selected by a newly added root. This
    /// unbounded fixture/maintenance read is not an IPC surface.
    #[cfg(test)]
    pub(crate) fn list_detached_locations(&self) -> Result<Vec<PublishedLocation>> {
        let mut statement = self.connection.prepare(&format!(
            "{LOCATION_COLUMNS}
             WHERE scan_root_id IS NULL AND detached_scan_root_id IS NOT NULL
             ORDER BY detached_scan_root_id, locator_key COLLATE BINARY, id"
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
