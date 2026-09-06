use super::{Database, Result, ScanRoot, StorageError};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRootExecution {
    pub id: String,
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
            |row| {
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
            },
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
            |row| {
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
            },
        )
        .map_err(map_not_found)?;
    run_from_raw(raw)
}

fn map_not_found(error: rusqlite::Error) -> StorageError {
    match error {
        rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound,
        other => other.into(),
    }
}

fn select_root_execution(connection: &Connection, id: &str) -> Result<ScanRootExecution> {
    let raw = connection
        .query_row(
            "SELECT id, configuration_revision, generation, enabled
             FROM scan_root WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .map_err(map_not_found)?;
    if raw.1 < 0 || raw.2 < 0 {
        return Err(StorageError::InvalidSchema);
    }
    Ok(ScanRootExecution {
        id: raw.0,
        configuration_revision: raw.1,
        generation: raw.2,
        enabled: parse_flag(raw.3)?,
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
    transaction.execute(
        "UPDATE scan_run
         SET state = 'interrupted', finished_at_ms = ?1,
             outcome = 'interrupted', error_code = 'recovery'
         WHERE state = 'running'",
        [now_ms],
    )?;
    transaction.execute(
        "UPDATE scan_job
         SET state = 'interrupted', updated_at_ms = ?1, last_error_code = 'recovery'
         WHERE state = 'running'",
        [now_ms],
    )?;
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
            transaction.execute(
                "UPDATE scan_job
                 SET follow_up_requested = 1, updated_at_ms = ?1
                 WHERE id = ?2",
                params![now_ms, &job_id],
            )?;
            true
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

fn cancel_root_work(transaction: &Transaction<'_>, root_id: &str, now_ms: i64) -> Result<()> {
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
                "SELECT attempt, max_attempts, cancellation_requested
                 FROM scan_job WHERE id = ?1",
                [&job_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
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
        let next_job_state = if cancelled {
            ScanJobState::Cancelled
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
                } else if next_job_state == ScanJobState::Failed {
                    "retry_exhausted"
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
    /// process. Enabled roots receive one durable recovery job each.
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
            transaction.execute(
                "UPDATE scan_run
                 SET state = 'interrupted', finished_at_ms = ?1,
                     outcome = 'interrupted', error_code = 'restart'
                 WHERE state = 'running'",
                [now_ms],
            )?;
            transaction.execute(
                "UPDATE scan_job
                 SET state = 'interrupted', updated_at_ms = ?1, last_error_code = 'restart'
                 WHERE state = 'running'",
                [now_ms],
            )?;
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

            let roots = if had_prior_session {
                let mut statement = transaction
                    .prepare("SELECT id FROM scan_root WHERE enabled = 1 ORDER BY rowid")?;
                Some(
                    statement
                        .query_map([], |row| row.get::<_, String>(0))?
                        .collect::<rusqlite::Result<Vec<_>>>()?,
                )
            } else {
                None
            };
            if let Some(roots) = roots {
                for root_id in roots {
                    let _ = enqueue_scan_tx(transaction, &root_id, ScanKind::Recovery, now_ms)?;
                }
            }
            Ok(ScanSession {
                id: session_id.to_owned(),
                started_at_ms: now_ms,
                ended_at_ms: None,
            })
        })
    }

    /// Queue one scan per root. A queued or running root already owns the
    /// active slot; running work records one coalesced follow-up request.
    pub fn enqueue_scan(
        &mut self,
        root_id: &str,
        kind: ScanKind,
        now_ms: i64,
    ) -> Result<EnqueueResult> {
        self.transaction(|transaction| enqueue_scan_tx(transaction, root_id, kind, now_ms))
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
            transaction.execute(
                "UPDATE scan_job SET state = 'running', attempt = ?1,
                    updated_at_ms = ?2 WHERE id = ?3 AND state = 'queued'",
                params![attempt, now_ms, &job_id],
            )?;
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
    /// returned unchanged so a late cancel cannot roll back a commit.
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
                "UPDATE scan_job SET cancellation_requested = 1, updated_at_ms = ?1
                 WHERE id = ?2 AND state = 'running'",
                params![now_ms, &run.scan_job_id],
            )?;
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
                    transaction.execute(
                        "UPDATE scan_job SET cancellation_requested = 1, updated_at_ms = ?1
                         WHERE id = ?2 AND state = 'running'",
                        params![now_ms, job_id],
                    )?;
                    transaction.execute(
                        "UPDATE scan_run SET cancellation_requested = 1
                         WHERE scan_job_id = ?1 AND state = 'running'",
                        [job_id],
                    )?;
                    Ok(ScanJobState::Running)
                }
                terminal => Ok(terminal),
            }
        })
    }

    /// Finish an owned run. The root revision, generation, session and lease
    /// are checked in the same transaction as the terminal state write.
    pub fn finish_scan_run(
        &mut self,
        run_id: &str,
        session_id: &str,
        lease_token: &str,
        now_ms: i64,
        outcome: ScanRunOutcome,
    ) -> Result<ScanRunState> {
        if session_id.is_empty() || lease_token.is_empty() {
            return Err(StorageError::InvalidSchema);
        }
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
            let cancellation_requested = run.cancellation_requested || parse_flag(job_cancel)?;
            let effective_outcome = if cancellation_requested {
                ScanRunOutcome::Cancelled
            } else {
                outcome
            };
            let state = state_for_outcome(effective_outcome);
            transaction.execute(
                "UPDATE scan_run
                 SET state = ?1, finished_at_ms = ?2, outcome = ?3, error_code = ?4
                 WHERE id = ?5 AND state = 'running'",
                params![
                    state.as_str(),
                    now_ms,
                    effective_outcome.as_str(),
                    error_for_outcome(effective_outcome),
                    run_id,
                ],
            )?;
            transaction.execute(
                "UPDATE scan_job
                 SET state = ?1, updated_at_ms = ?2,
                     last_error_code = ?3
                 WHERE id = ?4 AND state = 'running'",
                params![
                    state.as_str(),
                    now_ms,
                    error_for_outcome(effective_outcome),
                    &run.scan_job_id
                ],
            )?;

            // A trigger received during traversal gets one fresh job after a
            // successful/failed attempt. User cancellation never auto-retries.
            if follow_up == 1
                && !cancellation_requested
                && effective_outcome != ScanRunOutcome::Cancelled
            {
                let _ = enqueue_scan_tx(
                    transaction,
                    &run.scan_root_id,
                    ScanKind::parse(&kind)?,
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

    pub fn scan_job(&self, job_id: &str) -> Result<ScanJob> {
        select_job(&self.connection, job_id)
    }

    pub fn scan_run(&self, run_id: &str) -> Result<ScanRun> {
        select_run(&self.connection, run_id)
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
