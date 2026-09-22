use super::{Database, Result, ScanRoot, ScanRootMode, StorageError};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
#[derive(Clone, Copy, Default)]
struct TestReadCounts {
    job_rows: usize,
    run_rows: usize,
    retry_window_rows: usize,
}

#[cfg(test)]
thread_local! {
    static TEST_READ_COUNTS: std::cell::RefCell<TestReadCounts> =
        const {
            std::cell::RefCell::new(TestReadCounts {
                job_rows: 0,
                run_rows: 0,
                retry_window_rows: 0,
            })
        };
}

#[cfg(test)]
fn record_job_row() {
    TEST_READ_COUNTS.with(|counts| counts.borrow_mut().job_rows += 1);
}

#[cfg(test)]
fn record_run_row() {
    TEST_READ_COUNTS.with(|counts| counts.borrow_mut().run_rows += 1);
}

#[cfg(test)]
fn record_retry_window_row() {
    TEST_READ_COUNTS.with(|counts| counts.borrow_mut().retry_window_rows += 1);
}

#[cfg(test)]
pub(crate) fn reset_test_read_counts() {
    TEST_READ_COUNTS.with(|counts| *counts.borrow_mut() = TestReadCounts::default());
}

#[cfg(test)]
pub(crate) fn test_read_counts() -> (usize, usize) {
    TEST_READ_COUNTS.with(|counts| {
        let counts = *counts.borrow();
        (counts.job_rows, counts.run_rows)
    })
}

#[cfg(test)]
pub(crate) fn test_retry_window_rows() -> usize {
    TEST_READ_COUNTS.with(|counts| counts.borrow().retry_window_rows)
}

/// The initial retry budget is deliberately small: one initial attempt plus
/// three automatic retries. It is persisted on each job so reopening the
/// database cannot reset automatic retries.
pub const DEFAULT_SCAN_MAX_ATTEMPTS: i64 = 4;
const RETRY_BACKOFF_BASE_MS: i64 = 1_000;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanKind {
    Initial,
    Manual,
    Periodic,
    Recovery,
}

impl ScanKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Manual => "manual",
            Self::Periodic => "periodic",
            Self::Recovery => "recovery",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "initial" => Ok(Self::Initial),
            "manual" => Ok(Self::Manual),
            "periodic" => Ok(Self::Periodic),
            "recovery" => Ok(Self::Recovery),
            _ => Err(StorageError::InvalidSchema),
        }
    }

    fn request_intent(self) -> ScanRequestIntent {
        match self {
            // A user explicitly asking for Scan now/Retry is allowed to
            // supersede a cancellation that is still being observed by the
            // worker. The stored kind of an active job cannot establish that
            // intent for a later request, so this is derived from the
            // incoming request at the boundary.
            Self::Manual => ScanRequestIntent::ExplicitUser,
            Self::Initial | Self::Periodic | Self::Recovery => ScanRequestIntent::Background,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScanRequestIntent {
    ExplicitUser,
    Background,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanJobState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

impl ScanJobState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "interrupted" => Ok(Self::Interrupted),
            _ => Err(StorageError::InvalidSchema),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanRunState {
    Running,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

impl ScanRunState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "interrupted" => Ok(Self::Interrupted),
            _ => Err(StorageError::InvalidSchema),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanRunOutcome {
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

pub struct ScanRunFinalization<'a> {
    pub outcome: ScanRunOutcome,
    pub terminal_error_code: Option<&'a str>,
    pub cancellation_acknowledged: bool,
}

impl ScanRunOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "interrupted" => Ok(Self::Interrupted),
            _ => Err(StorageError::InvalidSchema),
        }
    }
}

/// The only diagnostic overrides a worker may attach to a failed run. The
/// persisted representation remains a string for schema compatibility, but
/// callers cannot introduce a new product diagnostic through the terminal
/// finish API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScanFailureDiagnostic {
    AccessDenied,
    ResourceLimit,
    Unsupported,
    Unavailable,
    WorkerFailed,
}

impl ScanFailureDiagnostic {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "access_denied" => Ok(Self::AccessDenied),
            "resource_limit" => Ok(Self::ResourceLimit),
            "unsupported" => Ok(Self::Unsupported),
            "unavailable" => Ok(Self::Unavailable),
            "worker_failed" => Ok(Self::WorkerFailed),
            _ => Err(StorageError::InvalidSchema),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::AccessDenied => "access_denied",
            Self::ResourceLimit => "resource_limit",
            Self::Unsupported => "unsupported",
            Self::Unavailable => "unavailable",
            Self::WorkerFailed => "worker_failed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRootExecution {
    pub id: String,
    pub mode: ScanRootMode,
    pub configuration_revision: i64,
    pub generation: i64,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSession {
    pub id: String,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanJob {
    pub id: String,
    pub scan_root_id: String,
    pub kind: ScanKind,
    pub state: ScanJobState,
    pub retry_chain_id: String,
    pub attempt: i64,
    pub max_attempts: i64,
    pub not_before_ms: i64,
    pub priority: i64,
    pub follow_up_requested: bool,
    pub cancellation_requested: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub last_error_code: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRun {
    pub id: String,
    pub scan_job_id: String,
    pub scan_root_id: String,
    pub generation: i64,
    pub configuration_revision: i64,
    pub retry_chain_id: String,
    pub attempt: i64,
    pub session_id: String,
    pub lease_token: String,
    pub state: ScanRunState,
    pub cancellation_requested: bool,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub lease_expires_at_ms: i64,
    pub outcome: Option<ScanRunOutcome>,
    pub error_code: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueResult {
    pub job_id: String,
    pub coalesced: bool,
    pub follow_up_requested: bool,
}

/// A stable keyset position for one bounded retry sweep page. The cursor is
/// deliberately based on the existing durable job ordering so a page can
/// move past candidates that are not yet eligible without materializing the
/// rest of the historical queue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanRetryCursor {
    pub created_at_ms: i64,
    pub id: String,
}

/// One bounded page of failed jobs whose non-jittered retry backoff has
/// elapsed. The worker applies the existing deterministic per-job jitter
/// before requeueing and retains the cursor between poll ticks for fairness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanRetryCandidatePage {
    pub jobs: Vec<ScanJob>,
    pub next_cursor: Option<ScanRetryCursor>,
}

/// The one job selected for a root's status packet and the run identity that
/// belongs to that selected job. Historical rows stay in SQLite; callers do
/// not need to load them to resolve the current/terminal status.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanRootStatus {
    pub job: ScanJob,
    pub run_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeasedScan {
    pub job: ScanJob,
    pub run: ScanRun,
    pub root: ScanRootExecution,
}

type RawJob = (
    String,
    String,
    String,
    String,
    String,
    i64,
    i64,
    i64,
    i64,
    i64,
    i64,
    i64,
    i64,
    Option<String>,
);

type RawRun = (
    String,
    String,
    String,
    i64,
    i64,
    String,
    i64,
    String,
    String,
    String,
    i64,
    i64,
    Option<i64>,
    i64,
    Option<String>,
    Option<String>,
);

fn parse_flag(value: i64) -> Result<bool> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(StorageError::InvalidSchema),
    }
}

fn job_from_raw(raw: RawJob) -> Result<ScanJob> {
    #[cfg(test)]
    record_job_row();
    let (
        id,
        root_id,
        kind,
        state,
        retry_chain_id,
        attempt,
        max_attempts,
        not_before_ms,
        priority,
        follow_up_requested,
        cancellation_requested,
        created_at_ms,
        updated_at_ms,
        last_error_code,
    ) = raw;
    if attempt < 0 || max_attempts <= 0 {
        return Err(StorageError::InvalidSchema);
    }
    Ok(ScanJob {
        id,
        scan_root_id: root_id,
        kind: ScanKind::parse(&kind)?,
        state: ScanJobState::parse(&state)?,
        retry_chain_id,
        attempt,
        max_attempts,
        not_before_ms,
        priority,
        follow_up_requested: parse_flag(follow_up_requested)?,
        cancellation_requested: parse_flag(cancellation_requested)?,
        created_at_ms,
        updated_at_ms,
        last_error_code,
    })
}

fn run_from_raw(raw: RawRun) -> Result<ScanRun> {
    #[cfg(test)]
    record_run_row();
    let (
        id,
        scan_job_id,
        scan_root_id,
        generation,
        configuration_revision,
        retry_chain_id,
        attempt,
        session_id,
        lease_token,
        state,
        cancellation_requested,
        started_at_ms,
        finished_at_ms,
        lease_expires_at_ms,
        outcome,
        error_code,
    ) = raw;
    if generation < 0 || configuration_revision < 0 || attempt <= 0 {
        return Err(StorageError::InvalidSchema);
    }
    Ok(ScanRun {
        id,
        scan_job_id,
        scan_root_id,
        generation,
        configuration_revision,
        retry_chain_id,
        attempt,
        session_id,
        lease_token,
        state: ScanRunState::parse(&state)?,
        cancellation_requested: parse_flag(cancellation_requested)?,
        started_at_ms,
        finished_at_ms,
        lease_expires_at_ms,
        outcome: outcome.as_deref().map(ScanRunOutcome::parse).transpose()?,
        error_code,
    })
}

fn select_job(connection: &Connection, id: &str) -> Result<ScanJob> {
    let raw = connection
        .query_row(
            "SELECT id, scan_root_id, kind, state, retry_chain_id, attempt,
                    max_attempts, not_before_ms, priority, follow_up_requested,
                    cancellation_requested, created_at_ms, updated_at_ms, last_error_code
             FROM scan_job WHERE id = ?1",
            [id],
            raw_job_from_row,
        )
        .map_err(map_not_found)?;
    job_from_raw(raw)
}

fn select_run(connection: &Connection, id: &str) -> Result<ScanRun> {
    let raw = connection
        .query_row(
            "SELECT id, scan_job_id, scan_root_id, generation, configuration_revision,
                    retry_chain_id, attempt, session_id, lease_token, state,
                    cancellation_requested, started_at_ms, finished_at_ms,
                    lease_expires_at_ms, outcome, error_code
             FROM scan_run WHERE id = ?1",
            [id],
            raw_run_from_row,
        )
        .map_err(map_not_found)?;
    run_from_raw(raw)
}

fn raw_job_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawJob> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
        row.get(13)?,
    ))
}

fn raw_run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawRun> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
        row.get(13)?,
        row.get(14)?,
        row.get(15)?,
    ))
}

fn map_not_found(error: rusqlite::Error) -> StorageError {
    match error {
        rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound,
        other => other.into(),
    }
}

/// Build the retry window query for either the first page or a seekable
/// continuation. The CTE deliberately limits the durable candidate window
/// before checking backoff, root configuration, and slot ownership. Those
/// checks can reject every row in a page (for example, a disabled root), so
/// putting them in the CTE would let a LIMIT-only outer query scan unbounded
/// historical rows. Only LocalNtfs jobs enter automatic retry; DriveVirtual
/// jobs remain failed until an explicit user retry.
///
/// The first page binds `(now_ms, limit)`; a continuation binds
/// `(now_ms, cursor_created_at_ms, cursor_id, limit)`. The tuple comparison is
/// intentional: the nullable `OR` cursor predicate used by the old query made
/// SQLite choose an index scan instead of a seek for later pages.
pub(crate) fn retry_candidate_query(continuation: bool) -> String {
    let cursor = if continuation {
        "       AND (j.created_at_ms, j.id) > (?2, ?3)\n"
    } else {
        ""
    };
    let limit = if continuation { "?4" } else { "?2" };
    format!(
        "WITH examined AS (
         SELECT j.id, j.scan_root_id, j.kind, j.state, j.retry_chain_id,
                j.attempt, j.max_attempts, j.not_before_ms, j.priority,
                j.follow_up_requested, j.cancellation_requested,
                j.created_at_ms, j.updated_at_ms, j.last_error_code
         FROM scan_job AS j INDEXED BY scan_job_retry_order
         WHERE j.state = 'failed'
           AND j.cancellation_requested = 0
           AND j.attempt < j.max_attempts
{cursor}         ORDER BY j.created_at_ms, j.id
         LIMIT {limit}
     )
     SELECT examined.id, examined.scan_root_id, examined.kind, examined.state,
            examined.retry_chain_id, examined.attempt, examined.max_attempts,
            examined.not_before_ms, examined.priority,
            examined.follow_up_requested, examined.cancellation_requested,
            examined.created_at_ms, examined.updated_at_ms,
            examined.last_error_code,
            CASE
                WHEN examined.updated_at_ms + CASE
                         WHEN examined.attempt <= 1 THEN 1000
                         WHEN examined.attempt = 2 THEN 2000
                         ELSE 4000
                     END <= ?1
                     AND root.enabled = 1 AND root.mode = 'local_ntfs' AND NOT EXISTS (
                    SELECT 1 FROM scan_job AS active
                    WHERE active.scan_root_id = examined.scan_root_id
                      AND active.state IN ('queued', 'running')
                ) THEN 1
                ELSE 0
            END AS retry_slot_available
     FROM examined
     LEFT JOIN scan_root AS root ON root.id = examined.scan_root_id
     ORDER BY examined.created_at_ms, examined.id",
        cursor = cursor,
        limit = limit,
    )
}

fn raw_retry_candidate_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(RawJob, bool)> {
    #[cfg(test)]
    record_retry_window_row();
    Ok((raw_job_from_row(row)?, row.get::<_, i64>(14)? != 0))
}

pub(crate) const SCAN_ROOT_STATUS_QUERY: &str =
    "SELECT selected.id, selected.scan_root_id, selected.kind, selected.state,
            selected.retry_chain_id, selected.attempt, selected.max_attempts,
            selected.not_before_ms, selected.priority,
            selected.follow_up_requested, selected.cancellation_requested,
            selected.created_at_ms, selected.updated_at_ms,
            selected.last_error_code,
            CASE WHEN selected.state = 'queued' THEN NULL ELSE (
                SELECT run.id
                FROM scan_run AS run INDEXED BY scan_run_status_order
                WHERE run.scan_job_id = selected.id
                  AND (
                      (selected.state = 'running' AND run.state = 'running')
                      OR selected.state NOT IN ('queued', 'running')
                  )
                ORDER BY run.started_at_ms DESC, run.id DESC
                LIMIT 1
            ) END
     FROM scan_job AS selected
     WHERE selected.id = COALESCE(
         (SELECT active.id
          FROM scan_job AS active INDEXED BY scan_job_status_active_order
          WHERE active.scan_root_id = ?1
            AND active.state IN ('queued', 'running')
          ORDER BY active.created_at_ms DESC, active.id DESC
          LIMIT 1),
         (SELECT terminal.id
          FROM scan_job AS terminal INDEXED BY scan_job_status_terminal_order
          WHERE terminal.scan_root_id = ?1
            AND terminal.state NOT IN ('queued', 'running')
          ORDER BY terminal.created_at_ms DESC, terminal.id DESC
          LIMIT 1)
     )
     LIMIT 1";

fn select_root_execution(connection: &Connection, id: &str) -> Result<ScanRootExecution> {
    let raw = connection
        .query_row(
            "SELECT id, mode, configuration_revision, generation, enabled
             FROM scan_root WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .map_err(map_not_found)?;
    if raw.2 < 0 || raw.3 < 0 {
        return Err(StorageError::InvalidSchema);
    }
    Ok(ScanRootExecution {
        id: raw.0,
        mode: match raw.1.as_str() {
            "local_ntfs" => ScanRootMode::LocalNtfs,
            "drive_virtual" => ScanRootMode::DriveVirtual,
            _ => return Err(StorageError::InvalidSchema),
        },
        configuration_revision: raw.2,
        generation: raw.3,
        enabled: parse_flag(raw.4)?,
    })
}

fn select_session(connection: &Connection, id: &str) -> Result<ScanSession> {
    connection
        .query_row(
            "SELECT id, started_at_ms, ended_at_ms FROM scan_session WHERE id = ?1",
            [id],
            |row| {
                Ok(ScanSession {
                    id: row.get(0)?,
                    started_at_ms: row.get(1)?,
                    ended_at_ms: row.get(2)?,
                })
            },
        )
        .map_err(map_not_found)
}

pub(crate) fn wall_clock_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

pub(crate) fn invalidate_inflight_after_recovery(
    connection: &mut Connection,
    now_ms: i64,
) -> Result<()> {
    let transaction =
        connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    super::publication::discard_all_open_staging_tx(&transaction, now_ms)?;
    recover_interrupted_tx(&transaction, now_ms, "recovery")?;
    resume_interrupted_jobs_tx(&transaction, now_ms, "recovery")?;
    transaction.execute(
        "UPDATE scan_session
         SET ended_at_ms = CASE
             WHEN started_at_ms > ?1 THEN started_at_ms ELSE ?1 END
         WHERE ended_at_ms IS NULL",
        [now_ms],
    )?;
    transaction.commit()?;
    Ok(())
}

/// Fence runs left by a process that stopped before it could reap its leases.
/// LocalNtfs jobs remain the same retry chain and are requeued with the
/// persisted budget and normal backoff. DriveVirtual jobs become failed so
/// restart cannot retry a mount without an explicit user request.
fn recover_interrupted_tx(
    transaction: &Transaction<'_>,
    now_ms: i64,
    reason: &'static str,
) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT r.id, r.scan_job_id, r.cancellation_requested,
                j.cancellation_requested, j.attempt, j.max_attempts, root.mode
         FROM scan_run AS r
         JOIN scan_job AS j ON j.id = r.scan_job_id
         JOIN scan_root AS root ON root.id = r.scan_root_id
         WHERE r.state = 'running'
         ORDER BY r.id",
    )?;
    let running = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    let mut changed = 0;
    for (
        run_id,
        job_id,
        run_cancel_requested,
        job_cancel_requested,
        attempt,
        max_attempts,
        root_mode,
    ) in
        running
    {
        let cancelled = parse_flag(run_cancel_requested)? || parse_flag(job_cancel_requested)?;
        let run_state = if cancelled {
            ScanRunState::Cancelled
        } else {
            ScanRunState::Interrupted
        };
        let run_outcome = if cancelled {
            ScanRunOutcome::Cancelled
        } else {
            ScanRunOutcome::Interrupted
        };
        let run_error = if cancelled {
            "cancellation_requested"
        } else {
            reason
        };
        let run_changed = transaction.execute(
            "UPDATE scan_run
             SET state = ?1, cancellation_requested = ?2, finished_at_ms = ?3,
                 outcome = ?4, error_code = ?5
             WHERE id = ?6 AND state = 'running'",
            params![
                run_state.as_str(),
                i64::from(cancelled),
                now_ms,
                run_outcome.as_str(),
                run_error,
                &run_id,
            ],
        )?;
        if run_changed != 1 {
            continue;
        }
        changed += 1;
        super::publication::discard_staging_for_run_tx(transaction, &run_id, now_ms)?;

        let next_job_state = if cancelled {
            ScanJobState::Cancelled
        } else if root_mode == "drive_virtual" {
            // Virtual roots are manual-only: a process interruption is not
            // permission to retry a potentially unavailable remote mount.
            // Keep the same failed job so the explicit retry command can
            // resume its persisted retry chain.
            ScanJobState::Failed
        } else if attempt < max_attempts {
            ScanJobState::Queued
        } else {
            ScanJobState::Failed
        };
        let not_before_ms = if next_job_state == ScanJobState::Queued {
            now_ms
                .checked_add(retry_backoff_ms(attempt)?)
                .ok_or(StorageError::Conflict)?
        } else {
            now_ms
        };
        let job_error = if cancelled {
            "cancellation_requested"
        } else if next_job_state == ScanJobState::Failed && attempt >= max_attempts {
            "retry_exhausted"
        } else if next_job_state == ScanJobState::Failed {
            "worker_interrupted"
        } else {
            reason
        };
        let job_changed = transaction.execute(
            "UPDATE scan_job
             SET state = ?1, cancellation_requested = ?2,
                 not_before_ms = ?3, updated_at_ms = ?3,
                 last_error_code = ?4
             WHERE id = ?5 AND state = 'running'",
            params![
                next_job_state.as_str(),
                i64::from(cancelled),
                not_before_ms,
                job_error,
                &job_id,
            ],
        )?;
        if job_changed != 1 {
            return Err(StorageError::Conflict);
        }
    }
    Ok(changed)
}

/// Older execution-ledger versions left restart-fenced jobs in `interrupted`.
/// Resume at most the newest LocalNtfs job per enabled root, while respecting
/// the active-slot index and persisted retry budget. DriveVirtual jobs become
/// failed and await an explicit retry.
fn resume_interrupted_jobs_tx(
    transaction: &Transaction<'_>,
    now_ms: i64,
    reason: &'static str,
) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT j.id, j.attempt, j.max_attempts, r.mode
         FROM scan_job AS j
         JOIN scan_root AS r ON r.id = j.scan_root_id
         WHERE j.state = 'interrupted' AND j.cancellation_requested = 0
           AND r.enabled = 1
           AND NOT EXISTS (
               SELECT 1 FROM scan_job AS active
               WHERE active.scan_root_id = j.scan_root_id
                 AND active.state IN ('queued', 'running')
           )
           AND NOT EXISTS (
               SELECT 1 FROM scan_job AS newer
               WHERE newer.scan_root_id = j.scan_root_id
                 AND (newer.created_at_ms > j.created_at_ms
                      OR (newer.created_at_ms = j.created_at_ms AND newer.id > j.id))
           )
         ORDER BY j.created_at_ms, j.id",
    )?;
    let interrupted = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    let mut changed = 0;
    for (job_id, attempt, max_attempts, root_mode) in interrupted {
        let next_state = if attempt < max_attempts && root_mode == "local_ntfs" {
            ScanJobState::Queued
        } else {
            ScanJobState::Failed
        };
        let not_before_ms = if next_state == ScanJobState::Queued {
            now_ms
                .checked_add(retry_backoff_ms(attempt)?)
                .ok_or(StorageError::Conflict)?
        } else {
            now_ms
        };
        let error = if next_state == ScanJobState::Failed && attempt >= max_attempts {
            "retry_exhausted"
        } else if next_state == ScanJobState::Failed {
            "worker_interrupted"
        } else {
            reason
        };
        let updated = transaction.execute(
            "UPDATE scan_job
             SET state = ?1, not_before_ms = ?2, updated_at_ms = ?2,
                 last_error_code = ?3
             WHERE id = ?4 AND state = 'interrupted' AND cancellation_requested = 0",
            params![next_state.as_str(), not_before_ms, error, &job_id],
        )?;
        changed += updated;
    }
    Ok(changed)
}

/// Queue startup recovery only when no interrupted/queued work already owns
/// the root. Cancelled and exhausted work is never replaced by an implicit
/// recovery chain.
fn enqueue_recovery_jobs_tx(transaction: &Transaction<'_>, now_ms: i64) -> Result<()> {
    let mut statement = transaction.prepare(
        "SELECT id, mode FROM scan_root WHERE enabled = 1 ORDER BY rowid",
    )?;
    let roots = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    for (root_id, mode) in roots {
        if mode == "drive_virtual" {
            continue;
        }
        let latest = transaction
            .query_row(
                "SELECT state, cancellation_requested, attempt, max_attempts
                 FROM scan_job
                 WHERE scan_root_id = ?1
                 ORDER BY created_at_ms DESC, id DESC
                 LIMIT 1",
                [&root_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()?;
        let suppress =
            latest.is_some_and(|(state, cancellation_requested, attempt, max_attempts)| {
                state == ScanJobState::Queued.as_str()
                    || state == ScanJobState::Running.as_str()
                    // A terminal cancelled job is never an implicit recovery
                    // candidate. The flag is retained for running/interrupted
                    // compatibility rows, but terminal state is authoritative.
                    || state == ScanJobState::Cancelled.as_str()
                    || (cancellation_requested == 1
                        && state == ScanJobState::Interrupted.as_str())
                    // Retry eligibility is a durable state/budget decision;
                    // diagnostics describe why the attempt failed but cannot
                    // reset an exhausted chain after restart or recovery.
                    || (state == ScanJobState::Failed.as_str() && attempt >= max_attempts)
            });
        if !suppress {
            let _ = enqueue_scan_tx(
                transaction,
                &root_id,
                ScanKind::Recovery,
                ScanRequestIntent::Background,
                now_ms,
            )?;
        }
    }
    Ok(())
}

fn checked_next(value: i64) -> Result<i64> {
    value.checked_add(1).ok_or(StorageError::Conflict)
}

fn retry_backoff_ms(attempt: i64) -> Result<i64> {
    let shift = attempt.saturating_sub(1).min(2) as u32;
    RETRY_BACKOFF_BASE_MS
        .checked_shl(shift)
        .ok_or(StorageError::Conflict)
}

fn enqueue_scan_tx(
    transaction: &Transaction<'_>,
    root_id: &str,
    kind: ScanKind,
    intent: ScanRequestIntent,
    now_ms: i64,
) -> Result<EnqueueResult> {
    let root = select_root_execution(transaction, root_id)?;
    if !root.enabled {
        return Err(StorageError::Conflict);
    }
    let active = transaction
        .query_row(
            "SELECT id, state, follow_up_requested
             FROM scan_job
             WHERE scan_root_id = ?1 AND state IN ('queued', 'running')
             LIMIT 1",
            [root_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?;
    if let Some((job_id, state, existing_follow_up)) = active {
        let follow_up_requested = if state == ScanJobState::Running.as_str() {
            let accepted = match intent {
                ScanRequestIntent::ExplicitUser => {
                    // An explicit Scan now/Retry is newer than a durable
                    // cancellation that is still pending on the worker. It
                    // clears both cancellation mirrors and leaves one
                    // coalesced successor request behind.
                    transaction.execute(
                        "UPDATE scan_job
                         SET follow_up_requested = 1, cancellation_requested = 0,
                             updated_at_ms = ?1
                         WHERE id = ?2",
                        params![now_ms, &job_id],
                    )?;
                    transaction.execute(
                        "UPDATE scan_run SET cancellation_requested = 0
                         WHERE scan_job_id = ?1 AND state = 'running'",
                        [&job_id],
                    )?;
                    true
                }
                ScanRequestIntent::Background => {
                    // Watcher/recovery work may coalesce a follow-up only
                    // while the active attempt is not durably cancelled. It
                    // must never erase either cancellation mirror merely
                    // because it arrived between the user's cancel and the
                    // worker's cooperative stop.
                    transaction.execute(
                        "UPDATE scan_job
                         SET follow_up_requested = 1, updated_at_ms = ?1
                         WHERE id = ?2
                           AND cancellation_requested = 0
                           AND NOT EXISTS (
                               SELECT 1 FROM scan_run
                               WHERE scan_job_id = ?2
                                 AND state = 'running'
                                 AND cancellation_requested = 1
                           )",
                        params![now_ms, &job_id],
                    )? > 0
                }
            };
            if accepted {
                transaction.execute(
                    "UPDATE scan_stage SET state = 'discarded', updated_at_ms = ?1
                     WHERE run_id IN (
                         SELECT id FROM scan_run WHERE scan_job_id = ?2 AND state = 'running'
                     ) AND state = 'open'",
                    params![now_ms, &job_id],
                )?;
                transaction.execute(
                    "DELETE FROM scan_stage_observation
                     WHERE run_id IN (
                         SELECT id FROM scan_run WHERE scan_job_id = ?1 AND state = 'running'
                     )",
                    [&job_id],
                )?;
            }
            let current_follow_up: i64 = transaction.query_row(
                "SELECT follow_up_requested FROM scan_job WHERE id = ?1",
                [&job_id],
                |row| row.get(0),
            )?;
            parse_flag(current_follow_up)?
        } else {
            parse_flag(existing_follow_up)?
        };
        return Ok(EnqueueResult {
            job_id,
            coalesced: true,
            follow_up_requested,
        });
    }

    let job_id = uuid::Uuid::now_v7().to_string();
    let retry_chain_id = uuid::Uuid::now_v7().to_string();
    transaction.execute(
        "INSERT INTO scan_job
         (id, scan_root_id, kind, state, retry_chain_id, attempt, max_attempts,
          not_before_ms, priority, follow_up_requested, cancellation_requested,
          created_at_ms, updated_at_ms, last_error_code)
         VALUES (?1, ?2, ?3, 'queued', ?4, 0, ?5, ?6, 0, 0, 0, ?6, ?6, NULL)",
        params![
            &job_id,
            root_id,
            kind.as_str(),
            &retry_chain_id,
            DEFAULT_SCAN_MAX_ATTEMPTS,
            now_ms,
        ],
    )?;
    Ok(EnqueueResult {
        job_id,
        coalesced: false,
        follow_up_requested: false,
    })
}

/// Invalidate every lease and staged batch for one root.
///
/// P2-05 close-out: both configuration entry points
/// (`set_scan_root_enabled_at` for disable, `remove_scan_root_at` for remove)
/// call this inside the same immediate transaction that bumps the
/// generation/revision (disable) or detaches history and deletes the root
/// row (remove). Leases are fenced by that generation/revision/enabled
/// check on every later staging, renewal, finish, and publication call, so a
/// mid-queue or mid-run disable/remove can never publish stale rows. A
/// re-added path mints a fresh root ID whose Library starts empty; source
/// markers (byte sizes) are never mutated by the scan itself.
fn cancel_root_work(transaction: &Transaction<'_>, root_id: &str, now_ms: i64) -> Result<()> {
    super::publication::discard_staging_for_root_tx(transaction, root_id, now_ms)?;
    transaction.execute(
        "UPDATE scan_job
         SET state = 'cancelled', cancellation_requested = 1,
             updated_at_ms = ?1, last_error_code = 'root_invalidated'
         WHERE scan_root_id = ?2 AND state IN ('queued', 'running')",
        params![now_ms, root_id],
    )?;
    transaction.execute(
        "UPDATE scan_run
         SET state = 'cancelled', cancellation_requested = 1,
             finished_at_ms = ?1, outcome = 'cancelled', error_code = 'root_invalidated'
         WHERE scan_root_id = ?2 AND state = 'running'",
        params![now_ms, root_id],
    )?;
    Ok(())
}

fn reap_expired_tx(transaction: &Transaction<'_>, now_ms: i64) -> Result<usize> {
    let mut statement = transaction.prepare(
        "SELECT id, scan_job_id, cancellation_requested
         FROM scan_run
         WHERE state = 'running' AND lease_expires_at_ms <= ?1
         ORDER BY lease_expires_at_ms, id",
    )?;
    let expired = statement
        .query_map([now_ms], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    let mut changed = 0;
    for (run_id, job_id, run_cancel_requested) in expired {
        let job = transaction
            .query_row(
                "SELECT j.attempt, j.max_attempts, j.cancellation_requested, r.mode
                 FROM scan_job AS j JOIN scan_root AS r ON r.id = j.scan_root_id
                 WHERE j.id = ?1",
                [&job_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .map_err(map_not_found)?;
        let cancelled = parse_flag(run_cancel_requested)? || parse_flag(job.2)?;
        let run_state = if cancelled {
            ScanRunState::Cancelled
        } else {
            ScanRunState::Interrupted
        };
        let run_outcome = if cancelled {
            ScanRunOutcome::Cancelled
        } else {
            ScanRunOutcome::Interrupted
        };
        let run_changed = transaction.execute(
            "UPDATE scan_run
             SET state = ?1, finished_at_ms = ?2, outcome = ?3, error_code = ?4
             WHERE id = ?5 AND state = 'running' AND lease_expires_at_ms <= ?2",
            params![
                run_state.as_str(),
                now_ms,
                run_outcome.as_str(),
                if cancelled {
                    "cancellation_requested"
                } else {
                    "lease_expired"
                },
                &run_id,
            ],
        )?;
        if run_changed == 0 {
            continue;
        }
        changed += 1;
        super::publication::discard_staging_for_run_tx(transaction, &run_id, now_ms)?;
        let next_job_state = if cancelled {
            ScanJobState::Cancelled
        } else if job.3 == "drive_virtual" {
            ScanJobState::Failed
        } else if job.0 < job.1 {
            ScanJobState::Queued
        } else {
            ScanJobState::Failed
        };
        let not_before_ms = if next_job_state == ScanJobState::Queued {
            now_ms
                .checked_add(retry_backoff_ms(job.0)?)
                .ok_or(StorageError::Conflict)?
        } else {
            now_ms
        };
        transaction.execute(
            "UPDATE scan_job
             SET state = ?1, cancellation_requested = ?2, not_before_ms = ?3,
                 updated_at_ms = ?3, last_error_code = ?4
             WHERE id = ?5",
            params![
                next_job_state.as_str(),
                i64::from(cancelled),
                not_before_ms,
                if cancelled {
                    "cancellation_requested"
                } else if next_job_state == ScanJobState::Failed && job.0 >= job.1 {
                    "retry_exhausted"
                } else if next_job_state == ScanJobState::Failed {
                    "lease_expired"
                } else {
                    "lease_expired"
                },
                &job_id,
            ],
        )?;
    }
    Ok(changed)
}

fn state_for_outcome(outcome: ScanRunOutcome) -> ScanRunState {
    match outcome {
        ScanRunOutcome::Completed => ScanRunState::Completed,
        ScanRunOutcome::Failed => ScanRunState::Failed,
        ScanRunOutcome::Cancelled => ScanRunState::Cancelled,
        ScanRunOutcome::Interrupted => ScanRunState::Interrupted,
    }
}

fn error_for_outcome(outcome: ScanRunOutcome) -> Option<&'static str> {
    match outcome {
        ScanRunOutcome::Completed => None,
        ScanRunOutcome::Failed => Some("worker_failed"),
        ScanRunOutcome::Cancelled => Some("cancelled"),
        ScanRunOutcome::Interrupted => Some("worker_interrupted"),
    }
}

impl Database {
    /// Returns the revision and generation captured by a run, without
    /// exposing the native path or SQL connection to callers.
    pub fn scan_root_execution(&self, root_id: &str) -> Result<ScanRootExecution> {
        select_root_execution(&self.connection, root_id)
    }

    /// Allocate a process session ID inside the native owner before fencing
    /// work from an earlier process lifetime.
    pub fn start_scan_session(&mut self, now_ms: i64) -> Result<ScanSession> {
        let session_id = uuid::Uuid::now_v7().to_string();
        self.begin_scan_session(&session_id, now_ms)
    }

    /// Mark the current process session and fence every run left by a prior
    /// process. Interrupted work keeps its job and retry chain; roots without
    /// eligible work receive one durable recovery job.
    pub fn begin_scan_session(&mut self, session_id: &str, now_ms: i64) -> Result<ScanSession> {
        if session_id.is_empty() {
            return Err(StorageError::InvalidSchema);
        }
        self.transaction(|transaction| {
            if transaction
                .query_row(
                    "SELECT 1 FROM scan_session WHERE id = ?1",
                    [session_id],
                    |_| Ok(()),
                )
                .optional()?
                .is_some()
            {
                return Err(StorageError::Conflict);
            }
            let had_prior_session: bool =
                transaction.query_row("SELECT EXISTS(SELECT 1 FROM scan_session)", [], |row| {
                    row.get(0)
                })?;
            recover_interrupted_tx(transaction, now_ms, "restart")?;
            resume_interrupted_jobs_tx(transaction, now_ms, "restart")?;
            transaction.execute(
                "UPDATE scan_session
                 SET ended_at_ms = CASE
                     WHEN started_at_ms > ?1 THEN started_at_ms ELSE ?1 END
                 WHERE ended_at_ms IS NULL",
                [now_ms],
            )?;
            transaction.execute(
                "INSERT INTO scan_session (id, started_at_ms, ended_at_ms)
                 VALUES (?1, ?2, NULL)",
                params![session_id, now_ms],
            )?;
            if had_prior_session {
                enqueue_recovery_jobs_tx(transaction, now_ms)?;
            }
            Ok(ScanSession {
                id: session_id.to_owned(),
                started_at_ms: now_ms,
                ended_at_ms: None,
            })
        })
    }

    /// Queue one scan per root. A queued or running root already owns the
    /// active slot; running work records one coalesced follow-up request. An
    /// explicit Manual request may supersede a pending cancellation, while a
    /// Periodic/Recovery request never clears that durable cancellation.
    pub fn enqueue_scan(
        &mut self,
        root_id: &str,
        kind: ScanKind,
        now_ms: i64,
    ) -> Result<EnqueueResult> {
        let intent = kind.request_intent();
        self.transaction(|transaction| enqueue_scan_tx(transaction, root_id, kind, intent, now_ms))
    }

    /// Lease the oldest due job for an active process session.
    pub fn lease_next_scan(
        &mut self,
        session_id: &str,
        now_ms: i64,
        lease_duration_ms: i64,
    ) -> Result<Option<LeasedScan>> {
        if session_id.is_empty() || lease_duration_ms <= 0 {
            return Err(StorageError::InvalidSchema);
        }
        self.transaction(|transaction| {
            let session = select_session(transaction, session_id)?;
            if session.ended_at_ms.is_some() {
                return Err(StorageError::Conflict);
            }
            reap_expired_tx(transaction, now_ms)?;
            transaction.execute(
                "UPDATE scan_job
                 SET state = 'failed', updated_at_ms = ?1, last_error_code = 'retry_exhausted'
                 WHERE state = 'queued' AND attempt >= max_attempts",
                [now_ms],
            )?;
            if transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM scan_run WHERE state = 'running')",
                [],
                |row| row.get::<_, bool>(0),
            )? {
                return Ok(None);
            }
            let next_id = transaction
                .query_row(
                    "SELECT j.id
                     FROM scan_job AS j
                     JOIN scan_root AS r ON r.id = j.scan_root_id
                     WHERE j.state = 'queued' AND j.cancellation_requested = 0
                       AND j.attempt < j.max_attempts
                       AND j.not_before_ms <= ?1 AND r.enabled = 1
                     ORDER BY j.not_before_ms, j.priority DESC, j.created_at_ms, j.id
                     LIMIT 1",
                    [now_ms],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            let Some(job_id) = next_id else {
                return Ok(None);
            };
            let (root_id, attempt, max_attempts): (String, i64, i64) = transaction.query_row(
                "SELECT scan_root_id, attempt, max_attempts FROM scan_job WHERE id = ?1",
                [&job_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
            let attempt = checked_next(attempt)?;
            if attempt > max_attempts {
                return Err(StorageError::Conflict);
            }
            let root = select_root_execution(transaction, &root_id)?;
            if !root.enabled {
                return Ok(None);
            }
            let next_generation = checked_next(root.generation)?;
            let generation_changed = transaction.execute(
                "UPDATE scan_root
                 SET generation = ?1
                 WHERE id = ?2 AND generation = ?3 AND enabled = 1",
                params![next_generation, &root_id, root.generation],
            )?;
            if generation_changed != 1 {
                return Err(StorageError::Conflict);
            }
            let root = ScanRootExecution {
                generation: next_generation,
                ..root
            };
            // The generation allocation and job claim are one transaction so
            // a failed claim cannot consume a generation.
            let job_changed = transaction.execute(
                "UPDATE scan_job SET state = 'running', attempt = ?1,
                    updated_at_ms = ?2 WHERE id = ?3 AND state = 'queued'",
                params![attempt, now_ms, &job_id],
            )?;
            if job_changed != 1 {
                return Err(StorageError::Conflict);
            }
            let run_id = uuid::Uuid::now_v7().to_string();
            let lease_token = uuid::Uuid::now_v7().to_string();
            let retry_chain_id: String = transaction.query_row(
                "SELECT retry_chain_id FROM scan_job WHERE id = ?1",
                [&job_id],
                |row| row.get(0),
            )?;
            let lease_expires_at_ms = now_ms
                .checked_add(lease_duration_ms)
                .ok_or(StorageError::Conflict)?;
            transaction.execute(
                "INSERT INTO scan_run
                 (id, scan_job_id, scan_root_id, generation, configuration_revision,
                  retry_chain_id, attempt, session_id, lease_token, state,
                  cancellation_requested, started_at_ms, finished_at_ms,
                  lease_expires_at_ms, outcome, error_code)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'running', 0,
                         ?10, NULL, ?11, NULL, NULL)",
                params![
                    &run_id,
                    &job_id,
                    &root_id,
                    root.generation,
                    root.configuration_revision,
                    &retry_chain_id,
                    attempt,
                    session_id,
                    &lease_token,
                    now_ms,
                    lease_expires_at_ms,
                ],
            )?;
            let job = select_job(transaction, &job_id)?;
            let run = select_run(transaction, &run_id)?;
            Ok(Some(LeasedScan { job, run, root }))
        })
    }

    /// Extend a lease only while the exact session/token still owns a current
    /// running job. Expired or invalidated workers receive a fixed conflict.
    pub fn renew_scan_lease(
        &mut self,
        run_id: &str,
        session_id: &str,
        lease_token: &str,
        now_ms: i64,
        lease_duration_ms: i64,
    ) -> Result<i64> {
        if session_id.is_empty() || lease_token.is_empty() || lease_duration_ms <= 0 {
            return Err(StorageError::InvalidSchema);
        }
        let lease_expires_at_ms = now_ms
            .checked_add(lease_duration_ms)
            .ok_or(StorageError::Conflict)?;
        self.transaction(|transaction| {
            let changed = transaction.execute(
                "UPDATE scan_run
                 SET lease_expires_at_ms = ?1
                 WHERE id = ?2 AND session_id = ?3 AND lease_token = ?4
                   AND state = 'running' AND cancellation_requested = 0
                   AND lease_expires_at_ms > ?5
                   AND EXISTS (
                       SELECT 1 FROM scan_session
                       WHERE id = ?3 AND ended_at_ms IS NULL
                   )
                   AND EXISTS (
                       SELECT 1 FROM scan_job
                       WHERE id = scan_run.scan_job_id AND state = 'running'
                         AND cancellation_requested = 0
                   )
                   AND EXISTS (
                       SELECT 1 FROM scan_root
                       WHERE id = scan_run.scan_root_id AND enabled = 1
                         AND generation = scan_run.generation
                         AND configuration_revision = scan_run.configuration_revision
                   )",
                params![lease_expires_at_ms, run_id, session_id, lease_token, now_ms],
            )?;
            if changed != 1 {
                return Err(StorageError::Conflict);
            }
            Ok(lease_expires_at_ms)
        })
    }

    /// Persist a cancellation request. A running worker remains running until
    /// it observes the request or its lease is reaped; terminal outcomes are
    /// returned unchanged so a late cancel cannot roll back a commit. The
    /// durable request clears a pending follow-up: the cancellation is the
    /// newer instruction, so the attempt must end cancelled instead of
    /// spawning a successor.
    pub fn request_scan_cancellation(&mut self, run_id: &str, now_ms: i64) -> Result<ScanRunState> {
        self.transaction(|transaction| {
            let run = select_run(transaction, run_id)?;
            if run.state != ScanRunState::Running {
                return Ok(run.state);
            }
            transaction.execute(
                "UPDATE scan_run SET cancellation_requested = 1 WHERE id = ?1",
                [run_id],
            )?;
            transaction.execute(
                "UPDATE scan_job SET cancellation_requested = 1, follow_up_requested = 0,
                     updated_at_ms = ?1
                 WHERE id = ?2 AND state = 'running'",
                params![now_ms, &run.scan_job_id],
            )?;
            super::publication::discard_staging_for_run_tx(transaction, run_id, now_ms)?;
            Ok(ScanRunState::Running)
        })
    }

    /// Cancel a queued job immediately, or persist a request for its running
    /// attempt. This is useful to a UI that has a job ID before a run exists.
    pub fn cancel_scan_job(&mut self, job_id: &str, now_ms: i64) -> Result<ScanJobState> {
        self.transaction(|transaction| {
            let job = select_job(transaction, job_id)?;
            match job.state {
                ScanJobState::Queued => {
                    transaction.execute(
                        "UPDATE scan_job
                         SET state = 'cancelled', cancellation_requested = 1,
                             updated_at_ms = ?1, last_error_code = 'cancellation_requested'
                         WHERE id = ?2 AND state = 'queued'",
                        params![now_ms, job_id],
                    )?;
                    Ok(ScanJobState::Cancelled)
                }
                ScanJobState::Running => {
                    // The durable cancellation also clears a pending follow-up
                    // (same ordering rule as `request_scan_cancellation`): a
                    // newer cancellation beats an older trigger, while a later
                    // trigger supersedes this cancellation in `enqueue_scan`.
                    transaction.execute(
                        "UPDATE scan_job SET cancellation_requested = 1,
                             follow_up_requested = 0, updated_at_ms = ?1
                         WHERE id = ?2 AND state = 'running'",
                        params![now_ms, job_id],
                    )?;
                    transaction.execute(
                        "UPDATE scan_run SET cancellation_requested = 1
                         WHERE scan_job_id = ?1 AND state = 'running'",
                        [job_id],
                    )?;
                    super::publication::discard_staging_for_job_tx(transaction, job_id, now_ms)?;
                    Ok(ScanJobState::Running)
                }
                terminal => Ok(terminal),
            }
        })
    }

    /// Finish an owned non-authoritative run. The root revision, generation,
    /// session and lease are checked in the same transaction as the terminal
    /// state write. Completed runs must have an open stage and are delegated to
    /// the atomic publication path.
    pub fn finish_scan_run(
        &mut self,
        run_id: &str,
        session_id: &str,
        lease_token: &str,
        now_ms: i64,
        outcome: ScanRunOutcome,
    ) -> Result<ScanRunState> {
        self.finish_scan_run_with_error(run_id, session_id, lease_token, now_ms, outcome, None)
    }

    /// Finish an owned non-authoritative run while preserving a fixed,
    /// product-owned diagnostic when one is known. The override is used by
    /// the worker for bounded enumeration outcomes such as `resource_limit`;
    /// it never carries native errors, paths, SQL, or arbitrary user input.
    pub fn finish_scan_run_with_error(
        &mut self,
        run_id: &str,
        session_id: &str,
        lease_token: &str,
        now_ms: i64,
        outcome: ScanRunOutcome,
        terminal_error_code: Option<&str>,
    ) -> Result<ScanRunState> {
        self.finish_scan_run_with_error_internal(
            run_id,
            session_id,
            lease_token,
            now_ms,
            ScanRunFinalization {
                outcome,
                terminal_error_code,
                cancellation_acknowledged: false,
            },
        )
    }

    /// Finish a run at the host's control-intent ordering boundary. The
    /// proposed outcome and diagnostic are preserved for ordinary failures and
    /// follow-ups; when `cancellation_acknowledged` is true, the exact
    /// run/session/lease acknowledgement is authoritative in the same
    /// transaction as terminalization. This lets a cancellation accepted after
    /// the worker's earlier snapshot win before the command's separate durable
    /// write, without routing every failure through a cancellation API.
    pub fn finish_scan_run_at_control_boundary(
        &mut self,
        run_id: &str,
        session_id: &str,
        lease_token: &str,
        now_ms: i64,
        finalization: ScanRunFinalization<'_>,
    ) -> Result<ScanRunState> {
        self.finish_scan_run_with_error_internal(
            run_id,
            session_id,
            lease_token,
            now_ms,
            finalization,
        )
    }

    /// Finish a cooperatively cancelled run after the host accepted a
    /// cancellation intent for this exact leased attempt. This compatibility
    /// wrapper preserves the cancellation-specific storage API while the
    /// shared worker uses the general control boundary above.
    pub fn finish_scan_run_after_cooperative_cancellation(
        &mut self,
        run_id: &str,
        session_id: &str,
        lease_token: &str,
        now_ms: i64,
        terminal_error_code: Option<&str>,
    ) -> Result<ScanRunState> {
        self.finish_scan_run_at_control_boundary(
            run_id,
            session_id,
            lease_token,
            now_ms,
            ScanRunFinalization {
                outcome: ScanRunOutcome::Cancelled,
                terminal_error_code,
                cancellation_acknowledged: true,
            },
        )
    }

    fn finish_scan_run_with_error_internal(
        &mut self,
        run_id: &str,
        session_id: &str,
        lease_token: &str,
        now_ms: i64,
        options: ScanRunFinalization<'_>,
    ) -> Result<ScanRunState> {
        if session_id.is_empty() || lease_token.is_empty() {
            return Err(StorageError::InvalidSchema);
        }
        // Keep the string-shaped argument for compatibility with the worker
        // API, while canonicalizing it through the closed product vocabulary
        // before any transaction can persist it. Empty strings retain the
        // previous "use the outcome default" behavior.
        let terminal_error_code = options
            .terminal_error_code
            .filter(|code| !code.is_empty())
            .map(ScanFailureDiagnostic::parse)
            .transpose()?;
        self.transaction(|transaction| {
            let run = select_run(transaction, run_id)?;
            if run.state != ScanRunState::Running
                || run.session_id != session_id
                || run.lease_token != lease_token
            {
                return Err(StorageError::Conflict);
            }
            let (job_state, job_cancel, follow_up, kind): (String, i64, i64, String) = transaction
                .query_row(
                    "SELECT state, cancellation_requested, follow_up_requested, kind
                     FROM scan_job WHERE id = ?1",
                    [&run.scan_job_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )?;
            if job_state != ScanJobState::Running.as_str()
                || transaction.query_row(
                    "SELECT count(*) FROM scan_session
                         WHERE id = ?1 AND ended_at_ms IS NULL",
                    [session_id],
                    |row| row.get::<_, i64>(0),
                )? != 1
                || run.lease_expires_at_ms <= now_ms
            {
                return Err(StorageError::Conflict);
            }
            let root = select_root_execution(transaction, &run.scan_root_id)?;
            if !root.enabled
                || root.generation != run.generation
                || root.configuration_revision != run.configuration_revision
            {
                return Err(StorageError::Conflict);
            }
            // A durable cancellation or a fenced host acknowledgement is
            // authoritative over an older background follow-up. The
            // acknowledgement is accepted before the worker terminalizes and
            // is associated with this exact run by the lease checks above. A
            // newer explicit Manual request reaches this same boundary with
            // acknowledgement false, leaving the follow-up for the normal
            // successor transition below.
            let durable_cancellation_requested =
                run.cancellation_requested || parse_flag(job_cancel)?;
            let cooperative_cancellation = options.outcome == ScanRunOutcome::Cancelled;
            let follow_up_requested = parse_flag(follow_up)?;
            let cancellation_authoritative =
                durable_cancellation_requested || options.cancellation_acknowledged;
            let invalidated_by_follow_up = follow_up_requested && !cancellation_authoritative;
            let cancellation_requested = durable_cancellation_requested
                || options.cancellation_acknowledged
                || (cooperative_cancellation && !invalidated_by_follow_up);
            let effective_outcome = if invalidated_by_follow_up {
                ScanRunOutcome::Interrupted
            } else if cancellation_requested {
                ScanRunOutcome::Cancelled
            } else {
                options.outcome
            };
            let state = state_for_outcome(effective_outcome);
            let error_code = if invalidated_by_follow_up {
                Some("follow_up_requested")
            } else if effective_outcome == ScanRunOutcome::Failed {
                terminal_error_code
                    .map(ScanFailureDiagnostic::as_str)
                    .or_else(|| error_for_outcome(effective_outcome))
            } else {
                error_for_outcome(effective_outcome)
            };
            // A worker may only complete through `publish_scan_run`, which
            // validates and applies its run-scoped stage in the same
            // transaction as the completed ledger state. The compatibility
            // call below delegates when a stage exists; a bare completed row
            // still cannot precede publication. The delegate's error codes
            // are deliberately not collapsed: `NotFound` means this run has
            // no stage row at all (no open stage), while `Conflict` remains
            // the stale-lease/revision/cancellation fence signal.
            if effective_outcome == ScanRunOutcome::Completed {
                return super::publication::publish_scan_run_tx(
                    transaction,
                    run_id,
                    session_id,
                    lease_token,
                    now_ms,
                )
                .map(|_| ScanRunState::Completed);
            }
            super::publication::discard_staging_for_run_tx(transaction, run_id, now_ms)?;
            transaction.execute(
                "UPDATE scan_run
                 SET state = ?1, cancellation_requested = ?2, finished_at_ms = ?3,
                     outcome = ?4, error_code = ?5
                 WHERE id = ?6 AND state = 'running'",
                params![
                    state.as_str(),
                    i64::from(cancellation_requested),
                    now_ms,
                    effective_outcome.as_str(),
                    error_code,
                    run_id,
                ],
            )?;
            transaction.execute(
                "UPDATE scan_job
                 SET state = ?1, cancellation_requested = ?2, updated_at_ms = ?3,
                     follow_up_requested = CASE WHEN ?6 = 1 THEN 0 ELSE follow_up_requested END,
                     last_error_code = ?4
                 WHERE id = ?5 AND state = 'running'",
                params![
                    state.as_str(),
                    i64::from(cancellation_requested),
                    now_ms,
                    error_code,
                    &run.scan_job_id,
                    i64::from(options.cancellation_acknowledged),
                ],
            )?;

            // A trigger received during traversal (even while a stale
            // cancellation was pending) invalidates this attempt; its
            // observations cannot become authoritative, and the deduplicated
            // successor is scheduled in this same transaction.
            if invalidated_by_follow_up {
                let _ = enqueue_scan_tx(
                    transaction,
                    &run.scan_root_id,
                    ScanKind::parse(&kind)?,
                    ScanRequestIntent::Background,
                    now_ms,
                )?;
            }
            Ok(state)
        })
    }

    /// Requeue expired work while preserving its attempt count, or terminally
    /// fail it once the persisted retry budget is exhausted.
    pub fn reap_expired_scan_leases(&mut self, now_ms: i64) -> Result<usize> {
        self.transaction(|transaction| reap_expired_tx(transaction, now_ms))
    }

    /// Explicitly retry a failed job without resetting its retry-chain budget.
    /// A retry that would violate the durable active-slot invariant (partial
    /// unique index `scan_job_active_root`: at most one queued/running job
    /// per root) is skipped as `Ok(false)`, not failed: the slot owner's
    /// completion frees the root for a later sweep. Failing here would abort
    /// the worker's whole retry sweep before its claim, wedging every due
    /// queued job with no running work.
    pub fn retry_failed_scan_job(&mut self, job_id: &str, now_ms: i64) -> Result<bool> {
        self.transaction(|transaction| {
            let job = select_job(transaction, job_id)?;
            if job.state != ScanJobState::Failed || job.attempt >= job.max_attempts {
                return Ok(false);
            }
            let root = select_root_execution(transaction, &job.scan_root_id)?;
            if !root.enabled {
                return Ok(false);
            }
            let slot_owned: bool = transaction.query_row(
                "SELECT EXISTS(
                     SELECT 1 FROM scan_job
                     WHERE scan_root_id = ?1 AND state IN ('queued', 'running')
                 )",
                [&job.scan_root_id],
                |row| row.get(0),
            )?;
            if slot_owned {
                return Ok(false);
            }
            transaction.execute(
                "UPDATE scan_job
                 SET state = 'queued', not_before_ms = ?1, updated_at_ms = ?1,
                     cancellation_requested = 0, last_error_code = NULL
                 WHERE id = ?2 AND state = 'failed' AND attempt < max_attempts",
                params![now_ms, job_id],
            )?;
            Ok(true)
        })
    }

    /// Return one bounded, keyset-ordered page of retryable failed jobs.
    ///
    /// Storage first materializes at most `limit` durable failed candidates in
    /// creation order. Root configuration and active-slot checks are applied
    /// only to that bounded window, and the cursor advances past every row in
    /// the window even if those checks reject every row. The worker applies the
    /// existing deterministic jitter after this method returns.
    pub fn retry_candidate_page(
        &self,
        now_ms: i64,
        limit: usize,
        after: Option<&ScanRetryCursor>,
    ) -> Result<ScanRetryCandidatePage> {
        let limit = i64::try_from(limit).map_err(|_| StorageError::InvalidSchema)?;
        if limit <= 0 {
            return Err(StorageError::InvalidSchema);
        }
        let query = retry_candidate_query(after.is_some());
        let mut statement = self.connection.prepare(&query)?;
        let raw_candidates = if let Some(after) = after {
            statement
                .query_map(
                    params![now_ms, after.created_at_ms, after.id.as_str(), limit],
                    raw_retry_candidate_from_row,
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            statement
                .query_map(params![now_ms, limit], raw_retry_candidate_from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let next_cursor = (raw_candidates.len() == usize::try_from(limit).unwrap_or(usize::MAX))
            .then(|| {
                raw_candidates.last().map(|(raw_job, _)| ScanRetryCursor {
                    created_at_ms: raw_job.11,
                    id: raw_job.0.clone(),
                })
            })
            .flatten();
        let mut jobs = Vec::with_capacity(raw_candidates.len());
        for (raw_job, retry_slot_available) in raw_candidates {
            if retry_slot_available {
                jobs.push(job_from_raw(raw_job)?);
            }
        }
        Ok(ScanRetryCandidatePage { jobs, next_cursor })
    }

    pub fn scan_job(&self, job_id: &str) -> Result<ScanJob> {
        select_job(&self.connection, job_id)
    }

    pub fn scan_run(&self, run_id: &str) -> Result<ScanRun> {
        select_run(&self.connection, run_id)
    }

    /// Select the current/terminal job for one root without loading its
    /// historical siblings. Active work wins over terminal history; ties use
    /// the same `(created_at_ms, id)` order as the status contract. Queued
    /// jobs deliberately return no run ID because they have no current
    /// attempt.
    pub fn scan_root_status(&self, root_id: &str) -> Result<Option<ScanRootStatus>> {
        let selected = self
            .connection
            .query_row(SCAN_ROOT_STATUS_QUERY, [root_id], |row| {
                Ok((raw_job_from_row(row)?, row.get::<_, Option<String>>(14)?))
            })
            .optional()?;
        selected
            .map(|(raw_job, run_id)| {
                Ok(ScanRootStatus {
                    job: job_from_raw(raw_job)?,
                    run_id,
                })
            })
            .transpose()
    }

    /// Select the currently running attempt for one already-selected job.
    /// This never traverses runs belonging to another job.
    pub fn running_scan_run_for_job(&self, job_id: &str) -> Result<Option<ScanRun>> {
        let raw = self
            .connection
            .query_row(
                "SELECT id, scan_job_id, scan_root_id, generation,
                        configuration_revision, retry_chain_id, attempt,
                        session_id, lease_token, state, cancellation_requested,
                        started_at_ms, finished_at_ms, lease_expires_at_ms,
                        outcome, error_code
                 FROM scan_run
                 WHERE scan_job_id = ?1 AND state = 'running'
                 ORDER BY started_at_ms DESC, id DESC
                 LIMIT 1",
                [job_id],
                raw_run_from_row,
            )
            .optional()?;
        raw.map(run_from_raw).transpose()
    }

    /// Select the newest durable run for one already-selected job. This is
    /// used for terminal status and cancellation acknowledgements without
    /// loading unrelated run history.
    pub fn latest_scan_run_for_job(&self, job_id: &str) -> Result<Option<ScanRun>> {
        let raw = self
            .connection
            .query_row(
                "SELECT id, scan_job_id, scan_root_id, generation,
                        configuration_revision, retry_chain_id, attempt,
                        session_id, lease_token, state, cancellation_requested,
                        started_at_ms, finished_at_ms, lease_expires_at_ms,
                        outcome, error_code
                 FROM scan_run
                 WHERE scan_job_id = ?1
                 ORDER BY started_at_ms DESC, id DESC
                 LIMIT 1",
                [job_id],
                raw_run_from_row,
            )
            .optional()?;
        raw.map(run_from_raw).transpose()
    }

    pub fn list_scan_jobs(&self) -> Result<Vec<ScanJob>> {
        let mut statement = self
            .connection
            .prepare("SELECT id FROM scan_job ORDER BY created_at_ms, id")?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.into_iter()
            .map(|id| select_job(&self.connection, &id))
            .collect()
    }

    pub fn list_scan_runs(&self) -> Result<Vec<ScanRun>> {
        let mut statement = self
            .connection
            .prepare("SELECT id FROM scan_run ORDER BY started_at_ms, id")?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.into_iter()
            .map(|id| select_run(&self.connection, &id))
            .collect()
    }

    pub fn set_scan_root_enabled_at(
        &mut self,
        id: &str,
        enabled: bool,
        now_ms: i64,
    ) -> Result<ScanRoot> {
        self.transaction(|transaction| {
            super::Database::select_scan_root(transaction, id)?;
            let current = select_root_execution(transaction, id)?;
            if current.enabled != enabled {
                let next_revision = checked_next(current.configuration_revision)?;
                let next_generation = checked_next(current.generation)?;
                if !enabled {
                    cancel_root_work(transaction, id, now_ms)?;
                }
                transaction.execute(
                    "UPDATE scan_root
                     SET enabled = ?1, configuration_revision = ?2, generation = ?3
                     WHERE id = ?4",
                    params![i64::from(enabled), next_revision, next_generation, id],
                )?;
            }
            super::Database::select_scan_root(transaction, id)
        })
    }

    pub fn remove_scan_root_at(&mut self, id: &str, now_ms: i64) -> Result<()> {
        self.transaction(|transaction| {
            // Select first so an unknown ID cannot appear to have cancelled
            // work and so the delete remains one clear configuration action.
            super::Database::select_scan_root(transaction, id)?;
            cancel_root_work(transaction, id, now_ms)?;
            super::publication::detach_locations_for_root_tx(transaction, id, now_ms)?;
            let removed = transaction.execute("DELETE FROM scan_root WHERE id = ?1", [id])?;
            if removed != 1 {
                return Err(StorageError::NotFound);
            }
            Ok(())
        })
    }

    pub fn set_scan_root_enabled(&mut self, id: &str, enabled: bool) -> Result<ScanRoot> {
        self.set_scan_root_enabled_at(id, enabled, wall_clock_ms())
    }

    pub fn remove_scan_root(&mut self, id: &str) -> Result<()> {
        self.remove_scan_root_at(id, wall_clock_ms())
    }
}
