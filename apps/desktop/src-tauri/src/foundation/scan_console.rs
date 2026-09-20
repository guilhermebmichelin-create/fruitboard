//! Hidden scan-console IPC for the Phase 2 filesystem-only journey.
//!
//! This module exposes the durable scan worker (`crates/scan-execution`) and
//! the typed Library read model through Tauri commands shaped EXACTLY to the
//! client seam the draft PR #63 defined (`apps/client/src/library/contracts.ts`
//! on `feat/phase-2-library-scan-ui`), so a future native adapter maps 1:1:
//!
//! - `scan_now`, `cancel_scan`, `retry_scan`, `list_scan_statuses`,
//!   `get_library_page`, `get_scan_console_state`.
//! - `scan-status-changed` events after state transitions and poll-loop run
//!   completions. Renderer subscription wiring is deliberately not built.
//!
//! The commands are always registered so the client contract is stable. With
//! the `scan-console` cargo feature off, every console command returns the
//! typed `unavailable`/`scan_console_disabled` envelope and
//! `get_scan_console_state` reports `{enabled: false}`; the scanner crates are
//! not even compiled into default builds. With the feature on, the host
//! (`scan_console_host`) owns the one worker, the process session and the
//! poll-loop thread, all sharing the single native `Database` owner behind the
//! same `Mutex` as the rest of the command layer.
//!
//! Safety and privacy rules (never relaxed):
//!
//! - Errors carry fixed codes and fixed safe copy only. SQL, lease tokens,
//!   correlation tokens, and absolute paths never enter an error payload.
//!   `rootCanonicalPath` appears only inside the `root` management fields of
//!   `ScanStatus` and the Library page records, per the client contract.
//! - Library pages are root-scoped and snapshot-fenced exactly like the
//!   storage contract: a page request carries `(cursor, snapshotId)`; a
//!   replaced committed snapshot returns the typed `stale_cursor` error; a
//!   malformed or wrong-root token returns `invalid_cursor`.
//! - No file contents are read, hashed, hydrated, or parsed anywhere in this
//!   module. The worker only ever performs metadata traversal through the
//!   typed enumeration crate.

use super::command::CommandRuntime;
use super::errors::{AppError, DiagnosticCode, ErrorCode};
use fruitboard_storage::{
    Database, FilePresence, LibraryCursor, LibrarySnapshot, PublishedLocation, ScanRoot,
    StorageError,
};
#[cfg(feature = "scan-console")]
use fruitboard_storage::{
    LibraryQuery, MAX_LIBRARY_PAGE_SIZE, ScanJob, ScanJobState, ScanKind, ScanRootPublication,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
#[cfg(feature = "scan-console")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Maximum accepted size of an opaque cursor/snapshot token. Bounds decode
/// cost; the storage boundary enforces the real key/location bounds.
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
const MAX_OPAQUE_TOKEN_BYTES: usize = 64 * 1024;

/// The worker poll loop sleeps this long between idle ticks; action events
/// wake it immediately through a channel, so there is no busy spin.
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
pub(crate) const SCAN_CONSOLE_POLL_INTERVAL_MS: u64 = 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
pub(crate) enum ScanExecutionState {
    Idle,
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
    Interrupted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
pub(crate) enum ScanStartOutcome {
    Queued,
    AlreadyQueued,
    AlreadyRunning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
pub(crate) enum CancelScanOutcome {
    Cancelled,
    CancellationRequested,
    AlreadyCancelled,
    AlreadyCompleted,
    AlreadyFailed,
    #[allow(
        dead_code,
        reason = "kept for exact parity with the client CancelScanOutcome union"
    )]
    NotFound,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanProgressCounters {
    files_observed: u64,
    directories_visited: u64,
    total_files: Option<u64>,
}

impl ScanProgressCounters {
    #[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
    pub(crate) fn with_files_observed(files_observed: u64) -> Self {
        Self {
            files_observed,
            directories_visited: 0,
            total_files: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanStartResult {
    root_id: String,
    job_id: String,
    run_id: Option<String>,
    outcome: ScanStartOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancelScanResult {
    root_id: String,
    job_id: String,
    run_id: Option<String>,
    outcome: CancelScanOutcome,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanStatus {
    root: ScanRoot,
    state: ScanExecutionState,
    job_id: Option<String>,
    run_id: Option<String>,
    cancellation_requested: bool,
    /// Native-authoritative retry action. A false value for terminal work
    /// means the renderer must start a new explicit scan instead of reviving
    /// this job or guessing at its retry budget.
    retry_available: bool,
    counters: ScanProgressCounters,
    last_successful_scan_at: Option<String>,
    last_outcome_at: Option<String>,
    /// Closed client code set only (`access_denied` | `unavailable` |
    /// `unsupported` | `resource_limit` | `conflict` | `not_found` |
    /// `cancelled` | `internal`). Fixed mapping; raw diagnostics never leak.
    error_code: Option<ErrorCode>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LibraryFilePresence {
    Present,
    Missing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublishedFileLocation {
    location_id: String,
    root_id: String,
    root_display_name: String,
    root_canonical_path: String,
    file_name: String,
    relative_path: String,
    /// Canonical unsigned decimal string (§2.1 of the integration contract);
    /// never a JSON number.
    byte_size: String,
    /// UTC RFC 3339 with nanosecond precision retained.
    modified_at: String,
    presence: LibraryFilePresence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibraryPageResponse {
    root_id: String,
    snapshot_id: String,
    records: Vec<PublishedFileLocation>,
    next_cursor: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanConsoleState {
    enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
pub(crate) struct ScanStatusChangedRoot {
    root_id: String,
    state: ScanExecutionState,
    counters: ScanProgressCounters,
}

/// Typed `scan-status-changed` event payload: root ids, states and counters
/// only — never paths, tokens, SQL, or diagnostic details.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
pub(crate) struct ScanStatusChangedEvent {
    roots: Vec<ScanStatusChangedRoot>,
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
impl ScanStatusChangedEvent {
    pub(crate) fn transition(
        root_id: String,
        state: ScanExecutionState,
        counters: ScanProgressCounters,
    ) -> Self {
        Self {
            roots: vec![ScanStatusChangedRoot {
                root_id,
                state,
                counters,
            }],
        }
    }
}

/// Emits the typed status event. Production uses the Tauri app handle; tests
/// record events in memory. Implementations must not block for long.
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
pub(crate) trait ScanEventSink: Send + Sync {
    fn emit(&self, event: &ScanStatusChangedEvent);
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ScanNowRequest {
    schema_version: u64,
    root_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CancelScanRequest {
    schema_version: u64,
    job_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RetryScanRequest {
    schema_version: u64,
    job_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ListScanStatusesRequest {
    schema_version: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct GetLibraryPageRequest {
    schema_version: u64,
    root_id: String,
    /// Requested page size. The native boundary clamps to 1..=200 per the
    /// client seam (`LibraryPageRequest.limit`); out-of-range values never
    /// reach storage.
    limit: u64,
    /// Opaque continuation token; null starts the current committed snapshot.
    cursor: Option<String>,
    /// Opaque committed-snapshot identity; null means "current". When both
    /// are present the storage contract requires them to agree.
    snapshot_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct GetScanConsoleStateRequest {
    schema_version: u64,
}

/// One console service for the process. It shares the single native database
/// owner (`Arc<Mutex<Database>>`) with the rest of the command layer; with
/// `scan-console` enabled it also owns the host (worker + session + poll-loop
/// thread) from `scan_console_host`. The host is installed during Tauri
/// `setup` (it needs the app handle for events), so command handlers observe
/// it as always present.
pub(crate) struct ScanConsoleService {
    #[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
    database: Arc<Mutex<Database>>,
    #[cfg(feature = "scan-console")]
    clock: Arc<dyn fruitboard_scan_execution::ScanClock + Send + Sync>,
    #[cfg(feature = "scan-console")]
    host: Mutex<Option<Arc<super::scan_console_host::ScanConsoleHost>>>,
    #[cfg(feature = "scan-console")]
    initialization_started: AtomicBool,
    #[cfg(all(test, feature = "scan-console"))]
    retry_snapshot_hook: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

#[cfg(not(feature = "scan-console"))]
impl ScanConsoleService {
    pub(crate) fn new(database: Arc<Mutex<Database>>) -> Self {
        Self { database }
    }

    fn scan_console_disabled() -> AppError {
        AppError::new(ErrorCode::Unavailable, DiagnosticCode::ScanConsoleDisabled)
    }

    fn scan_now(&self, _root_id: String) -> Result<ScanStartResult, AppError> {
        Err(Self::scan_console_disabled())
    }

    fn cancel_scan(&self, _job_id: String) -> Result<CancelScanResult, AppError> {
        Err(Self::scan_console_disabled())
    }

    fn retry_scan(&self, _job_id: String) -> Result<ScanStartResult, AppError> {
        Err(Self::scan_console_disabled())
    }

    fn list_scan_statuses(&self) -> Result<Vec<ScanStatus>, AppError> {
        Err(Self::scan_console_disabled())
    }

    fn get_library_page(
        &self,
        _root_id: String,
        _limit: u64,
        _cursor: Option<String>,
        _snapshot_id: Option<String>,
    ) -> Result<LibraryPageResponse, AppError> {
        Err(Self::scan_console_disabled())
    }

    fn get_scan_console_state(&self) -> Result<ScanConsoleState, AppError> {
        Ok(ScanConsoleState { enabled: false })
    }

    /// Configuration mutations have no watcher side effect when the feature
    /// is disabled. The command remains a recoverable unavailable surface.
    pub(crate) fn configuration_changed(&self) {}

    pub(crate) fn shutdown(&self) {}
}

#[cfg(feature = "scan-console")]
impl ScanConsoleService {
    pub(crate) fn new_enabled(database: Arc<Mutex<Database>>) -> Self {
        Self {
            database,
            clock: Arc::new(fruitboard_scan_execution::SystemClock),
            host: Mutex::new(None),
            initialization_started: AtomicBool::new(false),
            #[cfg(all(test, feature = "scan-console"))]
            retry_snapshot_hook: Mutex::new(None),
        }
    }

    /// Begin the process session, install the host, and spawn the poll-loop
    /// thread. Runs once during Tauri `setup`; storage failure aborts startup.
    pub(crate) fn initialize(
        &self,
        app: tauri::AppHandle,
    ) -> Result<(), fruitboard_storage::StorageError> {
        if self.initialization_started.swap(true, Ordering::AcqRel) {
            return Err(fruitboard_storage::StorageError::Conflict);
        }
        let mut database = self
            .database
            .lock()
            .map_err(|_| fruitboard_storage::StorageError::Io)?;
        let host = super::scan_console_host::build_host(
            &mut database,
            self.clock.clone(),
            Arc::new(super::scan_console_host::TauriEventSink::new(app)),
        )?;
        drop(database);
        let host = Arc::new(host);
        host.spawn(self.database.clone())?;
        *self
            .host
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(host.clone());
        Ok(())
    }

    fn host(&self) -> Result<Arc<super::scan_console_host::ScanConsoleHost>, AppError> {
        self.host
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .ok_or_else(|| {
                AppError::new(ErrorCode::Unavailable, DiagnosticCode::ScanConsoleDisabled)
            })
    }

    /// Wake the independent watcher and scan lifecycle loops after a durable
    /// root mutation. Callers invoke this only after releasing the database
    /// mutex, so configuration acknowledgement never waits on host work.
    pub(crate) fn configuration_changed(&self) {
        if let Some(host) = self
            .host
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
        {
            host.wake();
        }
    }

    pub(crate) fn shutdown(&self) {
        let host = self
            .host
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(host) = host {
            host.shutdown(&self.database);
        }
    }

    fn scan_now(&self, root_id: String) -> Result<ScanStartResult, AppError> {
        self.enqueue_scan(&root_id, ScanKind::Manual)
    }

    /// Queue `kind` for `root_id` and report the durable result. Shared by the
    /// explicit Scan now command and the Retry fresh-chain fallback so both
    /// converge identically (the enqueue dedups onto any active slot and wakes
    /// the host when new work is queued).
    fn enqueue_scan(&self, root_id: &str, kind: ScanKind) -> Result<ScanStartResult, AppError> {
        let host = self.host()?;
        let worker = host.clone_worker();
        let mut database = self.database.lock().map_err(|_| storage_failed())?;
        let result = worker
            .request_scan(&mut database, root_id, kind, self.clock.as_ref())
            .map_err(map_enqueue_storage_error)?;
        // The enqueue transaction is authoritative about which retry chain
        // accepted this request. Looking up the newest-created job by root can
        // select a newer terminal history row while an older retry is queued
        // or running, turning a successful coalesce into a false conflict.
        let job = database
            .scan_job(&result.job_id)
            .map_err(map_enqueue_storage_error)?;
        let run_id = database
            .running_scan_run_for_job(&job.id)
            .map_err(map_scan_storage_error)?
            .map(|run| run.id);
        let (outcome, state) = if !result.coalesced && job.state == ScanJobState::Queued {
            (ScanStartOutcome::Queued, Some(ScanExecutionState::Queued))
        } else {
            match job.state {
                ScanJobState::Queued => (ScanStartOutcome::AlreadyQueued, None),
                ScanJobState::Running => (ScanStartOutcome::AlreadyRunning, None),
                // The enqueue committed but the durable state already moved
                // on (a worker finished the coalesced attempt); report the
                // closed conflict instead of guessing.
                _ => return Err(scan_job_conflict()),
            }
        };
        if let Some(state) = state {
            host.emit_transition(root_id, state);
            host.wake();
        }
        Ok(ScanStartResult {
            root_id: root_id.to_owned(),
            job_id: job.id,
            run_id,
            outcome,
        })
    }

    fn cancel_scan(&self, job_id: String) -> Result<CancelScanResult, AppError> {
        let host = self.host()?;
        // The in-memory cancellation mirror is set first (never blocks,
        // never needs the database) so the running traversal stops at its
        // next cooperative check; the durable write below is the
        // acknowledged authority.
        host.request_cancellation(&job_id);
        let mut database = self.database.lock().map_err(|_| storage_failed())?;
        let job = database.scan_job(&job_id).map_err(map_scan_storage_error)?;
        let state = database
            .cancel_scan_job(&job_id, self.clock.as_ref().now_ms())
            .map_err(map_scan_storage_error)?;
        let run_id = database
            .latest_scan_run_for_job(&job_id)
            .map_err(map_scan_storage_error)?
            .map(|run| run.id);
        // The pre-cancel job state decides the outcome; the write result
        // reconciles the worker commit race (a running attempt that the
        // worker finished or cancelled first).
        let (outcome, emitted) = match (job.state, state) {
            (ScanJobState::Queued, ScanJobState::Cancelled) => (
                CancelScanOutcome::Cancelled,
                Some(ScanExecutionState::Cancelled),
            ),
            (ScanJobState::Running, ScanJobState::Running) => {
                (CancelScanOutcome::CancellationRequested, None)
            }
            (ScanJobState::Running, ScanJobState::Completed) => {
                (CancelScanOutcome::AlreadyCompleted, None)
            }
            (ScanJobState::Running, ScanJobState::Cancelled) => {
                (CancelScanOutcome::AlreadyCancelled, None)
            }
            (ScanJobState::Running, ScanJobState::Failed) => {
                (CancelScanOutcome::AlreadyFailed, None)
            }
            (ScanJobState::Running, ScanJobState::Interrupted) => {
                (CancelScanOutcome::AlreadyFailed, None)
            }
            (ScanJobState::Cancelled, _) => (CancelScanOutcome::AlreadyCancelled, None),
            (ScanJobState::Completed, _) => (CancelScanOutcome::AlreadyCompleted, None),
            (ScanJobState::Failed | ScanJobState::Interrupted, _) => {
                (CancelScanOutcome::AlreadyFailed, None)
            }
            _ => return Err(scan_job_conflict()),
        };
        if let Some(state) = emitted {
            host.emit_transition(&job.scan_root_id, state);
            host.wake();
        }
        Ok(CancelScanResult {
            root_id: job.scan_root_id,
            job_id,
            run_id,
            outcome,
        })
    }

    fn retry_scan(&self, job_id: String) -> Result<ScanStartResult, AppError> {
        let host = self.host()?;
        // The job row, its running attempt, and a failed-chain requeue are
        // one database snapshot. Holding one guard across those reads closes
        // the old gap where a worker could finish between the job read and
        // the run read, producing `already_running` with `runId: null`.
        let (job, running_run_id, failed_requeued) = {
            let mut database = self.database.lock().map_err(|_| storage_failed())?;
            let job = database.scan_job(&job_id).map_err(map_scan_storage_error)?;
            #[cfg(all(test, feature = "scan-console"))]
            if let Some(hook) = self
                .retry_snapshot_hook
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
            {
                // Test-only synchronization seam: the hook runs while the
                // database guard is held and is never compiled into a
                // production binary or renderer command.
                hook();
            }
            let running_run_id = if job.state == ScanJobState::Running {
                database
                    .running_scan_run_for_job(&job_id)
                    .map_err(map_scan_storage_error)?
                    .map(|run| run.id)
            } else {
                None
            };
            let failed_requeued = if job.state == ScanJobState::Failed {
                database
                    .retry_failed_scan_job(&job_id, self.clock.as_ref().now_ms())
                    .map_err(map_scan_storage_error)?
            } else {
                false
            };
            (job, running_run_id, failed_requeued)
        };
        match job.state {
            ScanJobState::Queued => Ok(ScanStartResult {
                root_id: job.scan_root_id,
                job_id,
                run_id: None,
                outcome: ScanStartOutcome::AlreadyQueued,
            }),
            ScanJobState::Running => Ok(ScanStartResult {
                root_id: job.scan_root_id,
                run_id: running_run_id,
                job_id,
                outcome: ScanStartOutcome::AlreadyRunning,
            }),
            ScanJobState::Failed => {
                if failed_requeued {
                    let root_id = job.scan_root_id;
                    host.emit_transition(&root_id, ScanExecutionState::Queued);
                    host.wake();
                    return Ok(ScanStartResult {
                        root_id,
                        job_id,
                        run_id: None,
                        outcome: ScanStartOutcome::Queued,
                    });
                }
                // The specific failed chain cannot be revived as-is: its
                // persisted budget is exhausted or a newer attempt already
                // owns the active slot. Start a fresh chain exactly like an
                // explicit scan so Retry converges instead of dead-ending on
                // the safe conflict; the failed row is retained, never reset.
                self.enqueue_scan(&job.scan_root_id, ScanKind::Manual)
            }
            // Cancelled and interrupted chains are never revived. An explicit
            // Retry supersedes them with fresh work instead of returning the
            // state-change conflict, because the user's action is a new scan
            // request, not a request to roll the durable ledger back.
            ScanJobState::Cancelled | ScanJobState::Interrupted => {
                self.enqueue_scan(&job.scan_root_id, ScanKind::Manual)
            }
            // Completed work has nothing to retry; the renderer offers the
            // explicit `Scan now` action for it.
            ScanJobState::Completed => Err(scan_job_conflict()),
        }
    }

    fn list_scan_statuses(&self) -> Result<Vec<ScanStatus>, AppError> {
        let host = self.host()?;
        let database = self.database.lock().map_err(|_| storage_failed())?;
        let roots = database.list_scan_roots().map_err(map_scan_storage_error)?;
        let mut statuses = Vec::with_capacity(roots.len());
        for root in roots {
            let selected = database
                .scan_root_status(&root.id)
                .map_err(map_scan_storage_error)?;
            let job = selected.as_ref().map(|status| &status.job);
            let run_id = selected
                .as_ref()
                .and_then(|status| status.run_id.as_deref());
            statuses.push(build_scan_status(&database, &host, root, job, run_id));
        }
        Ok(statuses)
    }

    fn get_library_page(
        &self,
        root_id: String,
        limit: u64,
        cursor: Option<String>,
        snapshot_id: Option<String>,
    ) -> Result<LibraryPageResponse, AppError> {
        if root_id.is_empty() {
            return Err(invalid_request());
        }
        let page_size = usize::try_from(limit.clamp(1, MAX_LIBRARY_PAGE_SIZE as u64))
            .map_err(|_| invalid_request())?;
        let cursor = cursor
            .map(|token| decode_cursor_token(&token, &root_id).map_err(map_cursor_reject))
            .transpose()?;
        let snapshot = snapshot_id
            .map(|token| decode_snapshot_token(&token).map_err(map_cursor_reject))
            .transpose()?;
        // The storage contract binds a cursor to its committed snapshot; the
        // client seam's request carries no snapshot field, so a plain cursor
        // implicitly echoes the snapshot it was minted against. Storage then
        // returns stale_cursor when that snapshot was replaced, or
        // invalid_cursor when an explicit snapshot disagrees.
        let snapshot = match (&cursor, &snapshot) {
            (Some(cursor), None) => Some(cursor.snapshot.clone()),
            _ => snapshot,
        };
        let database = self.database.lock().map_err(|_| storage_failed())?;
        let root = database
            .list_scan_roots()
            .map_err(map_scan_storage_error)?
            .into_iter()
            .find(|root| root.id == root_id)
            .ok_or_else(unknown_root)?;
        let page = database
            .query_library(&LibraryQuery {
                scan_root_id: root_id.clone(),
                page_size,
                cursor,
                snapshot,
            })
            .map_err(map_page_storage_error)?;
        let records = page
            .locations
            .iter()
            .map(|location| to_published_file_location(location, &root))
            .collect();
        // The storage page always carries a next cursor when rows exist; the
        // client seam treats `nextCursor: null` as the end of the list, so the
        // adapter only exposes one while more rows are pending.
        let next_cursor = if page.has_more {
            page.next_cursor.as_ref().map(encode_cursor_token)
        } else {
            None
        };
        Ok(LibraryPageResponse {
            root_id,
            snapshot_id: encode_snapshot_token(&page.snapshot),
            records,
            next_cursor,
        })
    }

    fn get_scan_console_state(&self) -> Result<ScanConsoleState, AppError> {
        Ok(ScanConsoleState { enabled: true })
    }
}

#[cfg(feature = "scan-console")]
impl Drop for ScanConsoleService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(feature = "scan-console")]
fn build_scan_status(
    database: &Database,
    host: &super::scan_console_host::ScanConsoleHost,
    root: ScanRoot,
    job: Option<&ScanJob>,
    run_id: Option<&str>,
) -> ScanStatus {
    let retry_available = job.is_some_and(|job| {
        job.state == ScanJobState::Failed && job.attempt < job.max_attempts && root.enabled
    });
    let state = match job {
        None => ScanExecutionState::Idle,
        Some(job) => match job.state {
            ScanJobState::Queued => ScanExecutionState::Queued,
            ScanJobState::Running => ScanExecutionState::Running,
            ScanJobState::Completed => ScanExecutionState::Completed,
            ScanJobState::Cancelled => ScanExecutionState::Cancelled,
            ScanJobState::Failed => ScanExecutionState::Failed,
            ScanJobState::Interrupted => ScanExecutionState::Interrupted,
        },
    };
    let mut counters = host.counters_for(&root.id);
    if state == ScanExecutionState::Running
        && let Some(run_id) = run_id
    {
        counters = database
            .scan_staging(run_id)
            .ok()
            .map(|staging| ScanProgressCounters::with_files_observed(staging.record_count as u64))
            .unwrap_or(counters);
    }
    let last_successful_scan_at = database
        .scan_root_publication(&root.id)
        .ok()
        .and_then(|publication: ScanRootPublication| publication.last_successful_at_ms)
        .map(unix_ms_to_rfc3339);
    let last_outcome_at = job.map(|job| unix_ms_to_rfc3339(job.updated_at_ms));
    let error_code = match state {
        // A queued/running attempt has not produced an outcome yet.
        ScanExecutionState::Queued | ScanExecutionState::Running | ScanExecutionState::Idle => None,
        _ => host
            .last_error_for(&root.id)
            .or_else(|| map_job_error_code(job.and_then(|job| job.last_error_code.as_deref()))),
    };
    ScanStatus {
        root,
        state,
        job_id: job.map(|job| job.id.clone()),
        run_id: run_id.map(str::to_owned),
        cancellation_requested: job.is_some_and(|job| job.cancellation_requested),
        retry_available,
        counters,
        last_successful_scan_at,
        last_outcome_at,
        error_code,
    }
}

/// Fixed-code mapping for the durable job diagnostics. Unknown codes become
/// the safe internal code; the raw string never crosses the boundary.
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn map_job_error_code(code: Option<&str>) -> Option<ErrorCode> {
    match code {
        None => None,
        Some("cancelled" | "cancellation_requested") => Some(ErrorCode::Cancelled),
        Some("worker_failed") => Some(ErrorCode::Internal),
        Some("access_denied") => Some(ErrorCode::AccessDenied),
        Some("resource_limit") => Some(ErrorCode::ResourceLimit),
        Some("unsupported") => Some(ErrorCode::Unsupported),
        Some("unavailable") => Some(ErrorCode::Unavailable),
        Some("worker_interrupted" | "follow_up_requested" | "root_invalidated" | "restart") => {
            Some(ErrorCode::Conflict)
        }
        Some("retry_exhausted") => Some(ErrorCode::ResourceLimit),
        Some(_) => Some(ErrorCode::Internal),
    }
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn to_published_file_location(
    location: &PublishedLocation,
    root: &ScanRoot,
) -> PublishedFileLocation {
    PublishedFileLocation {
        location_id: location.id.clone(),
        root_id: root.id.clone(),
        root_display_name: root.display_name.clone(),
        root_canonical_path: root.canonical_path.clone(),
        file_name: file_name_of(&location.relative_path),
        relative_path: location.relative_path.clone(),
        byte_size: location.byte_size.to_string(),
        modified_at: unix_ns_to_rfc3339(location.modified_at_ns),
        presence: match location.presence {
            FilePresence::Present => LibraryFilePresence::Present,
            FilePresence::Missing => LibraryFilePresence::Missing,
        },
    }
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn file_name_of(relative_path: &str) -> String {
    relative_path
        .rsplit(['\\', '/'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(relative_path)
        .to_owned()
}

/// Opaque cursor token encoding. The client round-trips these strings
/// verbatim; the encoding is internal and versioned. `i64` values are
/// canonical decimal strings so no value could exceed JavaScript's
/// exact-integer range if a future consumer ever parsed one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
struct SnapshotToken {
    #[serde(rename = "v")]
    version: u64,
    last_successful_run_id: Option<String>,
    last_successful_generation: Option<String>,
    last_successful_at_ms: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
struct CursorToken {
    #[serde(rename = "v")]
    version: u64,
    root_id: String,
    snapshot: SnapshotToken,
    locator_key: String,
    location_id: String,
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
const OPAQUE_TOKEN_VERSION: u64 = 1;

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn encode_snapshot_token(snapshot: &LibrarySnapshot) -> String {
    serde_json::to_string(&SnapshotToken {
        version: OPAQUE_TOKEN_VERSION,
        last_successful_run_id: snapshot.last_successful_run_id.clone(),
        last_successful_generation: snapshot
            .last_successful_generation
            .map(|value| value.to_string()),
        last_successful_at_ms: snapshot
            .last_successful_at_ms
            .map(|value| value.to_string()),
    })
    .expect("the opaque snapshot token always serializes")
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn encode_cursor_token(cursor: &LibraryCursor) -> String {
    serde_json::to_string(&CursorToken {
        version: OPAQUE_TOKEN_VERSION,
        root_id: cursor.scan_root_id.clone(),
        snapshot: SnapshotToken {
            version: OPAQUE_TOKEN_VERSION,
            last_successful_run_id: cursor.snapshot.last_successful_run_id.clone(),
            last_successful_generation: cursor
                .snapshot
                .last_successful_generation
                .map(|value| value.to_string()),
            last_successful_at_ms: cursor
                .snapshot
                .last_successful_at_ms
                .map(|value| value.to_string()),
        },
        locator_key: cursor.locator_key.clone(),
        location_id: cursor.location_id.clone(),
    })
    .expect("the opaque cursor token always serializes")
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn decode_snapshot_token(token: &str) -> Result<LibrarySnapshot, ()> {
    if token.len() > MAX_OPAQUE_TOKEN_BYTES {
        return Err(());
    }
    let token: SnapshotToken = serde_json::from_str(token).map_err(|_| ())?;
    if token.version != OPAQUE_TOKEN_VERSION {
        return Err(());
    }
    if token
        .last_successful_run_id
        .as_deref()
        .is_some_and(|id| id.is_empty())
    {
        return Err(());
    }
    let generation = token
        .last_successful_generation
        .as_deref()
        .map(parse_canonical_i64)
        .transpose()?;
    let at_ms = token
        .last_successful_at_ms
        .as_deref()
        .map(parse_canonical_i64)
        .transpose()?;
    // The committed snapshot is all-or-none: a token must not mix present and
    // absent fields, mirroring storage's own valid_snapshot rule.
    let fields = [
        token.last_successful_run_id.as_deref().map(|_| ()),
        generation.map(|_| ()),
        at_ms.map(|_| ()),
    ];
    if !(fields.iter().all(Option::is_some) || fields.iter().all(Option::is_none)) {
        return Err(());
    }
    Ok(LibrarySnapshot {
        last_successful_run_id: token.last_successful_run_id,
        last_successful_generation: generation,
        last_successful_at_ms: at_ms,
    })
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn decode_cursor_token(token: &str, root_id: &str) -> Result<LibraryCursor, ()> {
    if token.len() > MAX_OPAQUE_TOKEN_BYTES {
        return Err(());
    }
    let token: CursorToken = serde_json::from_str(token).map_err(|_| ())?;
    if token.version != OPAQUE_TOKEN_VERSION
        || token.root_id != root_id
        || token.locator_key.is_empty()
        || token.location_id.is_empty()
    {
        return Err(());
    }
    Ok(LibraryCursor {
        scan_root_id: token.root_id,
        snapshot: decode_snapshot_token(&serde_json::to_string(&token.snapshot).map_err(|_| ())?)?,
        locator_key: token.locator_key,
        location_id: token.location_id,
    })
}

/// Canonical unsigned decimal `i64` (generations and millisecond stamps are
/// never negative). Rejects signs, leading zeroes, empty and non-digit text.
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn parse_canonical_i64(value: &str) -> Result<i64, ()> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(());
    }
    value.parse::<i64>().map_err(|_| ())
}

/// UTC RFC 3339 from Unix nanoseconds, retaining up to nine fractional digits
/// with trailing zeroes trimmed (matching the client's `modifiedAt` parser).
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn unix_ns_to_rfc3339(nanoseconds: i64) -> String {
    let seconds = nanoseconds.div_euclid(1_000_000_000);
    let nanos = nanoseconds.rem_euclid(1_000_000_000);
    let days = seconds.div_euclid(86_400);
    let time_of_day = seconds.rem_euclid(86_400);
    let (hour, minute, second) = (
        time_of_day / 3_600,
        (time_of_day % 3_600) / 60,
        time_of_day % 60,
    );
    let (year, month, day) = civil_from_days(days);
    let fraction = if nanos == 0 {
        String::new()
    } else {
        let mut digits = format!("{nanos:09}");
        while digits.ends_with('0') {
            digits.pop();
        }
        format!(".{digits}")
    };
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}{fraction}Z")
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn unix_ms_to_rfc3339(milliseconds: i64) -> String {
    unix_ns_to_rfc3339(milliseconds.saturating_mul(1_000_000))
}

/// Days to civil date (Howard Hinnant's algorithm); deterministic for the
/// full i64 day range.
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month as u32, day as u32)
}

fn invalid_request() -> AppError {
    AppError::invalid_request(DiagnosticCode::RequestSchemaValidationFailed)
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn storage_failed() -> AppError {
    AppError::new(ErrorCode::Internal, DiagnosticCode::StorageFailed)
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn unknown_root() -> AppError {
    AppError::new(ErrorCode::NotFound, DiagnosticCode::UnknownScanRoot)
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn unknown_scan_job() -> AppError {
    AppError::new(ErrorCode::NotFound, DiagnosticCode::UnknownScanJob)
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn scan_job_conflict() -> AppError {
    AppError::new(ErrorCode::Conflict, DiagnosticCode::ScanJobConflict)
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn map_cursor_reject(_: ()) -> AppError {
    AppError::new(
        ErrorCode::InvalidCursor,
        DiagnosticCode::InvalidLibraryCursor,
    )
}

/// Storage errors from scan-control operations. Fixed codes only; raw storage
/// diagnostics never cross the boundary.
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn map_scan_storage_error(error: StorageError) -> AppError {
    match error {
        StorageError::Busy => AppError::new(ErrorCode::Unavailable, DiagnosticCode::StorageBusy),
        StorageError::NotFound => unknown_scan_job(),
        StorageError::Conflict => scan_job_conflict(),
        _ => storage_failed(),
    }
}

/// Storage errors from enqueueing a scan: `NotFound` means the root is
/// unknown, never a job.
#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn map_enqueue_storage_error(error: StorageError) -> AppError {
    match error {
        StorageError::Busy => AppError::new(ErrorCode::Unavailable, DiagnosticCode::StorageBusy),
        StorageError::NotFound => unknown_root(),
        StorageError::Conflict => {
            AppError::new(ErrorCode::Conflict, DiagnosticCode::ScanRootConflict)
        }
        _ => storage_failed(),
    }
}

#[cfg_attr(not(feature = "scan-console"), allow(dead_code))]
fn map_page_storage_error(error: StorageError) -> AppError {
    match error {
        StorageError::InvalidCursor => AppError::new(
            ErrorCode::InvalidCursor,
            DiagnosticCode::InvalidLibraryCursor,
        ),
        StorageError::StaleCursor => {
            AppError::new(ErrorCode::StaleCursor, DiagnosticCode::StaleLibraryCursor)
        }
        StorageError::NotFound => unknown_root(),
        StorageError::Busy => AppError::new(ErrorCode::Unavailable, DiagnosticCode::StorageBusy),
        StorageError::Conflict => scan_job_conflict(),
        _ => storage_failed(),
    }
}

fn decode_request<Request: DeserializeOwned>(request: Option<Value>) -> Result<Request, AppError> {
    request
        .and_then(|value| serde_json::from_value(value).ok())
        .ok_or_else(invalid_request)
}

pub(crate) fn handle_scan_now(
    commands: &CommandRuntime,
    scan: &ScanConsoleService,
    request: Option<Value>,
) -> super::CommandEnvelope<ScanStartResult> {
    commands.execute("scan_now", move || {
        let request: ScanNowRequest = decode_request(request)?;
        if request.schema_version != super::COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        scan.scan_now(request.root_id)
    })
}

pub(crate) fn handle_cancel_scan(
    commands: &CommandRuntime,
    scan: &ScanConsoleService,
    request: Option<Value>,
) -> super::CommandEnvelope<CancelScanResult> {
    commands.execute("cancel_scan", move || {
        let request: CancelScanRequest = decode_request(request)?;
        if request.schema_version != super::COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        scan.cancel_scan(request.job_id)
    })
}

pub(crate) fn handle_retry_scan(
    commands: &CommandRuntime,
    scan: &ScanConsoleService,
    request: Option<Value>,
) -> super::CommandEnvelope<ScanStartResult> {
    commands.execute("retry_scan", move || {
        let request: RetryScanRequest = decode_request(request)?;
        if request.schema_version != super::COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        scan.retry_scan(request.job_id)
    })
}

pub(crate) fn handle_list_scan_statuses(
    commands: &CommandRuntime,
    scan: &ScanConsoleService,
    request: Option<Value>,
) -> super::CommandEnvelope<Vec<ScanStatus>> {
    commands.execute("list_scan_statuses", move || {
        let request: ListScanStatusesRequest = decode_request(request)?;
        if request.schema_version != super::COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        scan.list_scan_statuses()
    })
}

pub(crate) fn handle_get_library_page(
    commands: &CommandRuntime,
    scan: &ScanConsoleService,
    request: Option<Value>,
) -> super::CommandEnvelope<LibraryPageResponse> {
    commands.execute("get_library_page", move || {
        let request: GetLibraryPageRequest = decode_request(request)?;
        if request.schema_version != super::COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        scan.get_library_page(
            request.root_id,
            request.limit,
            request.cursor,
            request.snapshot_id,
        )
    })
}

pub(crate) fn handle_get_scan_console_state(
    commands: &CommandRuntime,
    scan: &ScanConsoleService,
    request: Option<Value>,
) -> super::CommandEnvelope<ScanConsoleState> {
    commands.execute("get_scan_console_state", move || {
        let request: GetScanConsoleStateRequest = decode_request(request)?;
        if request.schema_version != super::COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        scan.get_scan_console_state()
    })
}

#[cfg(all(test, not(feature = "scan-console")))]
mod disabled_tests;
#[cfg(all(test, feature = "scan-console"))]
mod tests;
