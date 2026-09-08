//! Feature-gated scan-console host: the one `ScanWorker`, the started process
//! session, and the poll-loop thread that drive the REAL Windows filesystem
//! port.
//!
//! The host is only compiled with the `scan-console` cargo feature. It shares
//! the single native `Database` owner behind the same `Arc<Mutex<Database>>`
//! as the command layer; filesystem traversal runs with no database lock
//! held and the mutex is acquired only for short per-batch staging
//! transactions (`<=512` records) and for the final atomic publication, so
//! `list_scan_statuses` and `get_library_page` stay responsive mid-scan.
//! The typed command shapes and the durability semantics are the contract;
//! the single-connection design is documented in
//! `docs/review/phase-2-ipc/README.md`.
//!
//! ## Cancellation composition
//!
//! The enumerator polls a [`Cancellation`] between entries and batches; the
//! trait contract forbids blocking. The single SQLite connection cannot be
//! re-entered while the worker holds it, so the host uses a scoped in-memory
//! mirror: the `cancel_scan` command flips the mirror for the exact running
//! job first — it never blocks and never needs the database, so the traversal
//! stops at its next cooperative check — and then commits the durable
//! cancellation flag through storage, which remains the acknowledged
//! authority. Between batches the scan-execution staging adapter
//! independently re-reads the durable cancellation flags from storage, so the
//! run can never publish after a durable cancel even if the mirror were lost.
//! The mirror is keyed by job ID and replaced on every claim, so a stale flag
//! can never abort a later run of another chain.
//!
//! ## Deterministic tests
//!
//! The loop is split into `claim_due` and `execute_pending` so tests can stop
//! between the two steps (e.g. cancel a leased running job) without any
//! wall-clock sleeps; `tick` is the production composition.

use super::errors::ErrorCode;
use super::scan_console::{
    SCAN_CONSOLE_POLL_INTERVAL_MS, ScanEventSink, ScanExecutionState, ScanProgressCounters,
    ScanStatusChangedEvent,
};
use fruitboard_filesystem_enumeration::{
    Cancellation, FilesystemPort, Outcome, WindowsFilesystemPort,
};
use fruitboard_scan_execution::{
    ActiveScan, ScanClock, ScanExecution, ScanExecutionStatus, ScanWorker,
};
use fruitboard_storage::{Database, ScanJobState, ScanRunOutcome, StorageError};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Duration;

#[cfg(windows)]
use super::watcher_supervisor::{SupervisorSignal, run_native_supervisor};

#[derive(Clone)]
struct ActiveCancellation {
    job_id: String,
    run_id: String,
    session_id: String,
    lease_token: String,
    flag: Arc<AtomicBool>,
}

/// The claimed-but-not-yet-executed scan of the current poll tick. Tests can
/// drive `claim_due` and `execute_pending` separately so the cancel command
/// can run while the job is durably `running`.
struct PendingScan {
    scan: ActiveScan,
    flag: Arc<AtomicBool>,
    job_id: String,
    root_id: String,
}

/// Cooperative cancellation mirror described in the module docs. Never
/// blocks; only the cancel command flips it, before the durable commit.
struct CancellationMirror {
    flag: Arc<AtomicBool>,
    stopping: Arc<AtomicBool>,
}

impl Cancellation for CancellationMirror {
    fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed) || self.stopping.load(Ordering::Acquire)
    }
}

/// Tauri event sink: emits the typed `scan-status-changed` event to the
/// renderer. Renderer subscription wiring is a later slice; this only
/// publishes the payload.
pub(crate) struct TauriEventSink {
    app: tauri::AppHandle,
}

impl TauriEventSink {
    pub(crate) fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl ScanEventSink for TauriEventSink {
    fn emit(&self, event: &ScanStatusChangedEvent) {
        use tauri::Emitter;
        let _ = self.app.emit("scan-status-changed", event);
    }
}

/// The host state shared by the command layer and the poll-loop thread.
pub(crate) struct ScanConsoleHost {
    worker: Mutex<ScanWorker>,
    session_id: String,
    clock: Arc<dyn ScanClock + Send + Sync>,
    sink: Arc<dyn ScanEventSink>,
    cancellation: Mutex<Option<ActiveCancellation>>,
    pending: Mutex<Option<PendingScan>>,
    last_counters: Mutex<HashMap<String, ScanProgressCounters>>,
    last_error: Mutex<HashMap<String, ErrorCode>>,
    worker_signal: Mutex<Option<mpsc::SyncSender<HostSignal>>>,
    #[cfg(windows)]
    watcher_signal: Mutex<Option<mpsc::SyncSender<SupervisorSignal>>>,
    worker_join: Mutex<Option<std::thread::JoinHandle<()>>>,
    #[cfg(windows)]
    watcher_join: Mutex<Option<std::thread::JoinHandle<()>>>,
    lifecycle: Mutex<()>,
    shutdown_requested: Arc<AtomicBool>,
    stopping: Arc<AtomicBool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HostSignal {
    Wake,
    Shutdown,
}

impl ScanConsoleHost {
    pub(crate) fn new(
        worker: ScanWorker,
        session_id: String,
        clock: Arc<dyn ScanClock + Send + Sync>,
        sink: Arc<dyn ScanEventSink>,
    ) -> Self {
        Self {
            worker: Mutex::new(worker),
            session_id,
            clock,
            sink,
            cancellation: Mutex::new(None),
            pending: Mutex::new(None),
            last_counters: Mutex::new(HashMap::new()),
            last_error: Mutex::new(HashMap::new()),
            worker_signal: Mutex::new(None),
            #[cfg(windows)]
            watcher_signal: Mutex::new(None),
            worker_join: Mutex::new(None),
            #[cfg(windows)]
            watcher_join: Mutex::new(None),
            lifecycle: Mutex::new(()),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            stopping: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn clone_worker(&self) -> ScanWorker {
        self.worker
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Flip the cancellation mirror for the running job, if any. Called by
    /// the cancel command before it commits the durable request; the durable
    /// write remains the acknowledged authority.
    pub(crate) fn request_cancellation(&self, job_id: &str) {
        if let Some(active) = self
            .cancellation
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .filter(|active| active.job_id == job_id)
        {
            active.flag.store(true, Ordering::Relaxed);
        }
    }

    fn register_cancellation(&self, scan: &ActiveScan) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        *self
            .cancellation
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(ActiveCancellation {
            job_id: scan.leased.run.scan_job_id.clone(),
            run_id: scan.leased.run.id.clone(),
            session_id: scan.leased.run.session_id.clone(),
            lease_token: scan.leased.run.lease_token.clone(),
            flag: flag.clone(),
        });
        flag
    }

    fn clear_cancellation(&self, job_id: &str) {
        let mut guard = self
            .cancellation
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if guard.as_ref().is_some_and(|active| active.job_id == job_id) {
            *guard = None;
        }
    }

    /// Emit `scan-status-changed` for one root with its current counters.
    pub(crate) fn emit_transition(&self, root_id: &str, state: ScanExecutionState) {
        let counters = self.counters_for(root_id);
        self.sink.emit(&ScanStatusChangedEvent::transition(
            root_id.to_owned(),
            state,
            counters,
        ));
    }

    pub(crate) fn counters_for(&self, root_id: &str) -> ScanProgressCounters {
        self.last_counters
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(root_id)
            .copied()
            .unwrap_or_default()
    }

    fn record_counters(&self, root_id: &str, files_observed: Option<u64>) {
        if let Some(files_observed) = files_observed {
            self.last_counters
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(
                    root_id.to_owned(),
                    ScanProgressCounters::with_files_observed(files_observed),
                );
        }
    }

    /// The last captured scan error code for one root (from the last poll
    /// completion), or `None` when the last run was clean or unobserved.
    pub(crate) fn last_error_for(&self, root_id: &str) -> Option<ErrorCode> {
        self.last_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(root_id)
            .copied()
    }

    fn clear_error(&self, root_id: &str) {
        self.last_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(root_id);
    }

    fn record_error(&self, root_id: &str, error: Option<ErrorCode>) {
        let mut guard = self
            .last_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        match error {
            Some(error) => {
                guard.insert(root_id.to_owned(), error);
            }
            None => {
                guard.remove(root_id);
            }
        }
    }

    /// Wake both lifecycle loops immediately after a state transition. Each
    /// channel is capacity-one and wake signals are coalesced, so this never
    /// blocks or grows with an event burst.
    pub(crate) fn wake(&self) {
        if let Some(sender) = self
            .worker_signal
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
        {
            let _ = sender.try_send(HostSignal::Wake);
        }
        #[cfg(windows)]
        if let Some(sender) = self
            .watcher_signal
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
        {
            let _ = sender.try_send(SupervisorSignal::Wake);
        }
    }

    /// Fence an active lease as interrupted, stop both lifecycle loops, and
    /// join them. Repeated calls are idempotent. The durable fence happens
    /// before the global stop flag reaches the traversal cancellation mirror,
    /// so closing the host preserves restart recovery instead of recording a
    /// user cancellation.
    pub(crate) fn shutdown(&self, database: &Mutex<Database>) {
        let _lifecycle = self
            .lifecycle
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.shutdown_requested.store(true, Ordering::Release);
        let fence_succeeded = self.fence_active_run(database);
        if fence_succeeded {
            self.stopping.store(true, Ordering::Release);
            if let Some(active) = self
                .cancellation
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .as_ref()
            {
                active.flag.store(true, Ordering::Relaxed);
            }
        }
        if let Some(sender) = self
            .worker_signal
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
        {
            let _ = sender.try_send(HostSignal::Shutdown);
        }
        #[cfg(windows)]
        if let Some(sender) = self
            .watcher_signal
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
        {
            let _ = sender.try_send(SupervisorSignal::Shutdown);
        }
        if let Some(join) = self
            .worker_join
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            let _ = join.join();
        }
        #[cfg(windows)]
        if let Some(join) = self
            .watcher_join
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            let _ = join.join();
        }
        if !fence_succeeded {
            // The worker was asked to stop through the independent lifecycle
            // flag, but its traversal mirror was left untouched. It can
            // therefore finish naturally instead of turning an unavailable
            // shutdown fence into a false user cancellation.
            self.stopping.store(true, Ordering::Release);
        }
    }

    /// Start the scan poll-loop and, on Windows, the independent native
    /// watcher supervisor. The watcher loop must remain live while scan
    /// traversal is executing, so it is intentionally a second joined thread.
    pub(crate) fn spawn(
        self: &Arc<Self>,
        database: Arc<Mutex<Database>>,
    ) -> Result<(), StorageError> {
        let lifecycle = self
            .lifecycle
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if self.stopping.load(Ordering::Acquire)
            || self.shutdown_requested.load(Ordering::Acquire)
            || self
                .worker_join
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .is_some()
            || {
                #[cfg(windows)]
                {
                    self.watcher_join
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .is_some()
                }
                #[cfg(not(windows))]
                {
                    false
                }
            }
        {
            return Err(StorageError::Conflict);
        }
        let (worker_sender, worker_receiver) = mpsc::sync_channel::<HostSignal>(1);
        *self
            .worker_signal
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(worker_sender);
        #[cfg(windows)]
        let (watcher_sender, watcher_receiver) = mpsc::sync_channel::<SupervisorSignal>(1);
        #[cfg(windows)]
        {
            *self
                .watcher_signal
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some(watcher_sender);
            let watcher_database = database.clone();
            let watcher_clock = self.clock.clone();
            let watcher_shutdown_requested = self.shutdown_requested.clone();
            let watcher_join = std::thread::Builder::new()
                .name("fruitboard-watcher-supervisor".to_owned())
                .spawn(move || {
                    run_native_supervisor(
                        watcher_database,
                        watcher_clock,
                        watcher_receiver,
                        watcher_shutdown_requested,
                    )
                });
            let join = match watcher_join {
                Ok(join) => join,
                Err(_) => {
                    drop(lifecycle);
                    self.shutdown(&database);
                    return Err(StorageError::Io);
                }
            };
            *self
                .watcher_join
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some(join);
        }
        let host = self.clone();
        let worker_database = database.clone();
        let worker_join = std::thread::Builder::new()
            .name("fruitboard-scan-console".to_owned())
            .spawn(move || {
                let mut port = WindowsFilesystemPort::new();
                while !host.shutdown_requested.load(Ordering::Acquire) {
                    host.tick(&worker_database, &mut port);
                    match worker_receiver
                        .recv_timeout(Duration::from_millis(SCAN_CONSOLE_POLL_INTERVAL_MS))
                    {
                        Ok(HostSignal::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            break;
                        }
                        Ok(HostSignal::Wake) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                }
            });
        let join = match worker_join {
            Ok(join) => join,
            Err(_) => {
                drop(lifecycle);
                self.shutdown(&database);
                return Err(StorageError::Io);
            }
        };
        *self
            .worker_join
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(join);
        drop(lifecycle);
        Ok(())
    }

    /// Service retries, claim the oldest due job, and register the
    /// cancellation mirror. Keeps the claimed scan in `pending` so tests can
    /// interleave commands before execution. Returns whether a scan is due.
    /// Both durable steps are short transactions; no filesystem I/O holds the
    /// database mutex.
    pub(crate) fn claim_due(&self, database: &Mutex<Database>) -> bool {
        let _lifecycle = self
            .lifecycle
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if self.stopping.load(Ordering::Acquire) || self.shutdown_requested.load(Ordering::Acquire)
        {
            return false;
        }
        let worker = self.clone_worker();
        let Ok(mut db) = database.lock() else {
            return false;
        };
        if self.stopping.load(Ordering::Acquire) || self.shutdown_requested.load(Ordering::Acquire)
        {
            return false;
        }
        if worker
            .service_retries(&mut db, self.clock.as_ref())
            .is_err()
        {
            return false;
        }
        let Some(scan) = (match worker.claim(&mut db, &self.session_id, self.clock.as_ref()) {
            Ok(scan) => scan,
            Err(_) => return false,
        }) else {
            return false;
        };
        let job_id = scan.leased.run.scan_job_id.clone();
        let root_id = scan.leased.run.scan_root_id.clone();
        let flag = self.register_cancellation(&scan);
        // A new attempt has no outcome yet: the previous error no longer
        // describes the current work.
        self.clear_error(&root_id);
        self.emit_transition(&root_id, ScanExecutionState::Running);
        *self.pending.lock().unwrap_or_else(PoisonError::into_inner) = Some(PendingScan {
            scan,
            flag,
            job_id,
            root_id,
        });
        true
    }

    /// Execute the pending scan (if any) and emit its terminal transition.
    /// Filesystem traversal runs with no database mutex held; the worker
    /// acquires short per-batch staging transactions (`<=512` records) and
    /// one final atomic publication transaction, so `list_scan_statuses` and
    /// `get_library_page` stay responsive mid-scan. The worker mutex is also
    /// released during traversal (the worker is cloned before executing).
    pub(crate) fn execute_pending<P: FilesystemPort>(
        &self,
        database: &Mutex<Database>,
        port: &mut P,
    ) -> bool {
        let Some(pending) = self
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        else {
            return false;
        };
        let worker = self.clone_worker();
        let execution = worker.execute_shared(
            database,
            pending.scan,
            port,
            &CancellationMirror {
                flag: pending.flag,
                stopping: self.stopping.clone(),
            },
            self.clock.as_ref(),
        );
        // Short read for the fenced-state fallback only; every other status
        // maps without touching the database.
        let state = match database.lock() {
            Ok(db) => state_from_execution(&execution, &db),
            Err(_) => state_from_execution_fallback(&execution),
        };
        let files_observed = files_observed_from_execution(&execution);
        let error = error_from_execution(&execution);
        self.clear_cancellation(&pending.job_id);
        self.record_counters(&pending.root_id, files_observed);
        self.record_error(&pending.root_id, error);
        self.emit_transition(&pending.root_id, state);
        true
    }

    /// One poll tick: service retries, claim, execute (production loop body).
    pub(crate) fn tick<P: FilesystemPort>(&self, database: &Mutex<Database>, port: &mut P) {
        self.claim_due(database);
        self.execute_pending(database, port);
    }

    fn fence_active_run(&self, database: &Mutex<Database>) -> bool {
        let Ok(mut database) = database.lock() else {
            return false;
        };
        let Some(active) = self
            .cancellation
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
        else {
            return true;
        };
        let outcome = if active.flag.load(Ordering::Relaxed) {
            ScanRunOutcome::Cancelled
        } else {
            ScanRunOutcome::Interrupted
        };
        match database.finish_scan_run(
            &active.run_id,
            &active.session_id,
            &active.lease_token,
            self.clock.now_ms(),
            outcome,
        ) {
            Ok(_) | Err(StorageError::Conflict | StorageError::NotFound) => true,
            Err(_) => false,
        }
    }
}

fn state_from_execution(execution: &ScanExecution, database: &Database) -> ScanExecutionState {
    match execution.status {
        ScanExecutionStatus::Published => ScanExecutionState::Completed,
        ScanExecutionStatus::Failed => ScanExecutionState::Failed,
        ScanExecutionStatus::Cancelled => ScanExecutionState::Cancelled,
        ScanExecutionStatus::Interrupted => ScanExecutionState::Interrupted,
        // Another actor already decided this run (reaper, disable/remove,
        // replacement worker): the durable job state is authoritative.
        ScanExecutionStatus::Fenced => match database.scan_job(&execution.job_id) {
            Ok(job) => match job.state {
                ScanJobState::Queued => ScanExecutionState::Queued,
                ScanJobState::Running => ScanExecutionState::Running,
                ScanJobState::Completed => ScanExecutionState::Completed,
                ScanJobState::Cancelled => ScanExecutionState::Cancelled,
                ScanJobState::Failed => ScanExecutionState::Failed,
                ScanJobState::Interrupted => ScanExecutionState::Interrupted,
            },
            Err(_) => ScanExecutionState::Interrupted,
        },
    }
}

/// Fallback when the database mutex itself is poisoned: non-fenced statuses
/// map without the database; a fenced run cannot be resolved, so it reports
/// the honest `interrupted` fallback (same as the unreadable-job branch).
fn state_from_execution_fallback(execution: &ScanExecution) -> ScanExecutionState {
    match execution.status {
        ScanExecutionStatus::Published => ScanExecutionState::Completed,
        ScanExecutionStatus::Failed => ScanExecutionState::Failed,
        ScanExecutionStatus::Cancelled => ScanExecutionState::Cancelled,
        ScanExecutionStatus::Interrupted => ScanExecutionState::Interrupted,
        ScanExecutionStatus::Fenced => ScanExecutionState::Interrupted,
    }
}

/// Derive the reported files-observed counter from one finished execution.
/// `None` keeps the previous value (fenced runs change nothing).
fn files_observed_from_execution(execution: &ScanExecution) -> Option<u64> {
    match execution.status {
        ScanExecutionStatus::Published => execution
            .publication
            .as_ref()
            .map(|publication| publication.location_count as u64),
        ScanExecutionStatus::Fenced => None,
        _ => Some(0),
    }
}

/// Map a finished execution to the closed client error-code set. Status is
/// authoritative (a fence-cancelled run has no enumeration outcome); the
/// enumeration outcome refines failures. This is the most specific honest
/// code while the process lives; after a restart the persisted durable
/// diagnostics fall back to the coarser mapping.
fn error_from_execution(execution: &ScanExecution) -> Option<ErrorCode> {
    match execution.status {
        ScanExecutionStatus::Published | ScanExecutionStatus::Fenced => None,
        ScanExecutionStatus::Cancelled => Some(ErrorCode::Cancelled),
        ScanExecutionStatus::Interrupted => Some(ErrorCode::Conflict),
        ScanExecutionStatus::Failed => match execution.enumeration_outcome {
            Some(Outcome::Denied) => Some(ErrorCode::AccessDenied),
            Some(Outcome::UnsupportedFilesystem) => Some(ErrorCode::Unsupported),
            Some(Outcome::ResourceLimit) => Some(ErrorCode::ResourceLimit),
            Some(Outcome::RootUnavailable) => Some(ErrorCode::Unavailable),
            Some(Outcome::Cancelled) => Some(ErrorCode::Cancelled),
            Some(Outcome::Partial | Outcome::Invalid | Outcome::SinkFailed) => {
                Some(ErrorCode::Internal)
            }
            Some(Outcome::Complete) => None,
            None => Some(ErrorCode::Internal),
        },
    }
}

/// Build the production host: begin the process session (fencing prior-session
/// work and scheduling deduplicated recovery) and return the host ready for
/// `spawn`.
pub(crate) fn build_host(
    database: &mut Database,
    clock: Arc<dyn ScanClock + Send + Sync>,
    sink: Arc<dyn ScanEventSink>,
) -> Result<ScanConsoleHost, StorageError> {
    let worker = ScanWorker::new(fruitboard_scan_execution::WorkerConfig::default())
        .map_err(|_| StorageError::InvalidSchema)?;
    let session = worker.start_session(database, clock.as_ref())?;
    Ok(ScanConsoleHost::new(worker, session.id, clock, sink))
}

#[cfg(test)]
#[path = "scan_console_host_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "scan_console_host_recovery_tests.rs"]
mod recovery_tests;
