//! Integrated durable scan worker for the Phase 2 filesystem-only journey
//! (#36 enumeration + #38 durable execution + #40 publication composition).
//!
//! This crate composes the three landed scanner foundations into one bounded
//! manual-scan journey that is still completely hidden from the renderer:
//!
//! - `fruitboard-storage` owns the durable queue, deduplicated triggers,
//!   leases, staged batches and the fenced atomic publication. This crate
//!   never executes SQL itself and never touches committed Library rows
//!   outside the typed storage API.
//! - `fruitboard-filesystem-enumeration` owns bounded metadata traversal via
//!   `enumerate_into_run`. This crate feeds it the leased run through a
//!   run-scoped staging adapter; it never reads file contents, hashes,
//!   hydrates placeholders, or parses FLP data.
//! - `fruitboard-reconciliation` owns the deterministic per-path decision
//!   core. After an authoritative traversal the worker classifies the run's
//!   observed changes against the committed rows. Classification is advisory
//!   evidence; it never gates or bypasses the durable publication.
//!
//! Authoritative-completion invariant: only a run whose enumeration reported
//! `Complete` while the generation/revision/lease/cancellation fences still
//! hold is published by `publish_scan_run`. Every other outcome discards
//! staging through `finish_scan_run` and leaves committed results untouched.
//! A stale worker (expired or replaced lease) cannot commit: every staging,
//! renewal and publication call revalidates the session, lease token, root
//! revision and the durable cancellation flag inside storage's transaction.
//!
//! There is no Tauri dependency, no IPC surface, no renderer command and no
//! production scan entry point. The host owns the one global worker instance,
//! a started process session and the poll loop; the filesystem port is
//! injected, which keeps this crate portable and lets tests script every
//! outcome deterministically. Watcher wiring stays out of the crate too: the
//! host owns watcher lifecycle and activation, while
//! [`WatcherFollowUpAdapter`] (see [`followups`]) is the compile-time seam
//! that turns coalesced watcher hints into durable follow-up requests on the
//! host's poll loop.
use fruitboard_filesystem_enumeration as enumeration;
use fruitboard_reconciliation as reconciliation;

use enumeration::{
    Cancellation, EnumerationLimits, FilesystemPort, ObservationBatch, RunScopedStorage, SinkError,
};
use fruitboard_storage::{
    Database, EncodedIdentity as StagedIdentity, EnqueueResult, FilePresence, LeasedScan,
    LibraryQuery, MAX_LIBRARY_PAGE_SIZE, MAX_STAGED_PATH_BYTES, MAX_STAGED_RECORDS,
    PublishedLocation, ScanJob, ScanJobState, ScanKind, ScanObservation as StagedObservation,
    ScanPublication, ScanRunOutcome, ScanRunState, ScanSession, StorageError,
};
use reconciliation::{
    Identity as PlanIdentity, Location as PlanLocation, Metadata as PlanMetadata,
    Observation as PlanObservation, Outcome as PlanOutcome, reconcile,
};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

mod followups;
pub use followups::{
    FollowUpCause, FollowUpOutcome, FollowUpRequest, RootIdMapping, WatcherFollowUpAdapter,
};

/// Injected, monotonic-enough worker clock. Tests supply a deterministic
/// fake; the host supplies [`SystemClock`]. No wall-clock sleeps anywhere in
/// this crate.
pub trait ScanClock {
    fn now_ms(&self) -> i64;
}

/// Production clock over the system wall clock. Milliseconds since the Unix
/// epoch, clamped to the durable `i64` range like storage's own clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl ScanClock for SystemClock {
    fn now_ms(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| {
                let millis = duration.as_millis().min(i64::MAX as u128);
                millis as i64
            })
            .unwrap_or_default()
    }
}

/// Worker tuning. The defaults mirror the accepted Phase 2 budgets: a 30 s
/// lease renewed every 5 s, batch bounds already enforced by the enumerator
/// and staging (512 records, 256 MiB), and the durable 10,000-record staging
/// quota. Retries stay on storage's persisted per-job chain (1/2/4 s backoff
/// plus bounded jitter, never reset across restarts).
#[derive(Clone, Debug)]
pub struct WorkerConfig {
    pub lease_duration_ms: i64,
    pub lease_renewal_interval_ms: i64,
    pub enumeration_limits: EnumerationLimits,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            lease_duration_ms: 30_000,
            lease_renewal_interval_ms: 5_000,
            enumeration_limits: EnumerationLimits {
                // A traversal that would exceed the staging quota fails as a
                // resource limit before provisional work is staged, instead of
                // relying on the storage-side terminal rejection.
                max_observations: MAX_STAGED_RECORDS as usize,
                max_total_path_bytes: MAX_STAGED_PATH_BYTES as usize,
                ..EnumerationLimits::default()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerConfigError {
    LeaseDuration,
    RenewalInterval,
}

/// The single scan worker for the whole process. One instance means at most
/// one running scan; storage additionally refuses to lease while any run is
/// running and coalesces repeated triggers into one queued follow-up.
#[derive(Clone, Debug)]
pub struct ScanWorker {
    config: WorkerConfig,
}

impl ScanWorker {
    pub fn new(config: WorkerConfig) -> Result<Self, WorkerConfigError> {
        if config.lease_duration_ms <= 0 {
            return Err(WorkerConfigError::LeaseDuration);
        }
        if config.lease_renewal_interval_ms <= 0
            || config.lease_renewal_interval_ms >= config.lease_duration_ms
        {
            return Err(WorkerConfigError::RenewalInterval);
        }
        Ok(Self { config })
    }

    /// Begin the process session and fence prior-session work: previous
    /// running leases become interrupted, their staging is discarded, and
    /// enabled roots without eligible interrupted/queued work receive one
    /// deduplicated recovery job (storage enforces the suppression rules,
    /// including cancelled and exhausted chains).
    pub fn start_session(
        &self,
        db: &mut Database,
        clock: &dyn ScanClock,
    ) -> Result<ScanSession, StorageError> {
        db.start_scan_session(clock.now_ms())
    }

    /// Queue a manual scan for one root through storage's dedup API. A queued
    /// or running root already owns the active slot; running work records one
    /// coalesced follow-up request.
    pub fn request_manual_scan(
        &self,
        db: &mut Database,
        root_id: &str,
        clock: &dyn ScanClock,
    ) -> Result<EnqueueResult, StorageError> {
        db.enqueue_scan(root_id, ScanKind::Manual, clock.now_ms())
    }

    /// Lease the oldest due job and open its staging session. Returns `None`
    /// when the worker is idle. The returned lease is fenced: every later
    /// storage call revalidates the lease, revision and cancellation state.
    pub fn claim(
        &self,
        db: &mut Database,
        session_id: &str,
        clock: &dyn ScanClock,
    ) -> Result<Option<ActiveScan>, StorageError> {
        let now = clock.now_ms();
        let Some(leased) = db.lease_next_scan(session_id, now, self.config.lease_duration_ms)?
        else {
            return Ok(None);
        };
        db.begin_scan_staging(
            &leased.run.id,
            &leased.run.session_id,
            &leased.run.lease_token,
            now,
        )?;
        Ok(Some(ActiveScan {
            leased,
            staging_begun_at_ms: now,
        }))
    }

    /// Enumerate, stage and publish (or fail) one claimed scan. The lease,
    /// durable cancellation/invalidation flags and the root revision are
    /// checked between batches, before publication, and again inside the
    /// publication transaction itself.
    pub fn execute<P: FilesystemPort, C: Cancellation>(
        &self,
        db: &mut Database,
        scan: ActiveScan,
        port: &mut P,
        cancellation: &C,
        clock: &dyn ScanClock,
    ) -> ScanExecution {
        let now = clock.now_ms();
        match self.fence_state(db, &scan, now) {
            FenceState::Unreadable => {
                return self.execution(&scan, ScanExecutionStatus::Fenced, None, None, None);
            }
            FenceState::Invalidated => {
                return self.resolve(db, &scan, clock, false, None);
            }
            FenceState::Held => {}
        }
        let Some(root_path) = root_path(db, &scan.leased.run.scan_root_id) else {
            // The removal transaction already cancelled this run and detached
            // its committed history; there is nothing left to enumerate.
            return self.resolve(db, &scan, clock, false, None);
        };

        let mut adapter = StagingAdapter {
            db,
            clock,
            run_id: scan.leased.run.id.clone(),
            job_id: scan.leased.run.scan_job_id.clone(),
            session_id: scan.leased.run.session_id.clone(),
            lease_token: scan.leased.run.lease_token.clone(),
            lease_duration_ms: self.config.lease_duration_ms,
            renewal_interval_ms: self.config.lease_renewal_interval_ms,
            next_renewal_ms: now.saturating_add(self.config.lease_renewal_interval_ms),
            plan: PlanBuffer::default(),
        };
        let mut progress = enumeration::NoProgress;
        let report = enumeration::enumerate_into_run(
            port,
            Path::new(&root_path),
            &self.config.enumeration_limits,
            cancellation,
            &mut adapter,
            scan.leased.run.id.clone(),
            &mut progress,
        );
        let StagingAdapter { db, plan, .. } = adapter;
        let outcome = report.outcome;

        if !report.authoritative {
            let forced_cancel = outcome == enumeration::Outcome::Cancelled;
            return self.resolve(db, &scan, clock, forced_cancel, Some(outcome));
        }

        // Pre-publication fence: an invalidation that arrived between the last
        // batch and the final apply must stop here, before any staged record
        // can be published.
        let now = clock.now_ms();
        match self.fence_state(db, &scan, now) {
            FenceState::Unreadable => {
                return self.execution(
                    &scan,
                    ScanExecutionStatus::Fenced,
                    Some(outcome),
                    None,
                    None,
                );
            }
            FenceState::Invalidated => {
                return self.resolve(db, &scan, clock, false, Some(outcome));
            }
            FenceState::Held => {}
        }
        let changes = change_plan(db, &scan.leased.run.scan_root_id, &plan);
        match db.publish_scan_run(
            &scan.leased.run.id,
            &scan.leased.run.session_id,
            &scan.leased.run.lease_token,
            clock.now_ms(),
        ) {
            Ok(publication) => self.execution(
                &scan,
                ScanExecutionStatus::Published,
                Some(outcome),
                Some(publication),
                changes,
            ),
            Err(_) => self.resolve(db, &scan, clock, false, Some(outcome)),
        }
    }

    /// Enumerate, stage and publish one claimed scan without holding the
    /// database mutex across filesystem I/O.
    ///
    /// Filesystem traversal runs with no database lock held; the mutex is
    /// acquired only for short per-batch staging transactions (each batch is
    /// bounded to `<=512` records by the enumerator and re-checked by
    /// storage's `MAX_STAGED_BATCH_RECORDS` fence) and for the final atomic
    /// publication. Every staging and publication call still revalidates the
    /// generation, revision, lease token and durable cancellation flags
    /// inside storage's transaction, and the staging adapter re-reads the
    /// durable flags between batches so a cancelled run never publishes.
    pub fn execute_shared<P: FilesystemPort, C: Cancellation>(
        &self,
        db: &Mutex<Database>,
        scan: ActiveScan,
        port: &mut P,
        cancellation: &C,
        clock: &dyn ScanClock,
    ) -> ScanExecution {
        let now = clock.now_ms();
        match self.fence_state_shared(db, &scan, now) {
            FenceState::Unreadable => {
                return self.execution(&scan, ScanExecutionStatus::Fenced, None, None, None);
            }
            FenceState::Invalidated => {
                return self.resolve_shared(db, &scan, clock, false, None);
            }
            FenceState::Held => {}
        }
        let Some(root_path) = root_path_shared(db, &scan.leased.run.scan_root_id) else {
            // Same removal case as `execute`: the run was detached already.
            return self.resolve_shared(db, &scan, clock, false, None);
        };

        let mut adapter = SharedStagingAdapter {
            db,
            clock,
            run_id: scan.leased.run.id.clone(),
            job_id: scan.leased.run.scan_job_id.clone(),
            session_id: scan.leased.run.session_id.clone(),
            lease_token: scan.leased.run.lease_token.clone(),
            lease_duration_ms: self.config.lease_duration_ms,
            renewal_interval_ms: self.config.lease_renewal_interval_ms,
            next_renewal_ms: now.saturating_add(self.config.lease_renewal_interval_ms),
            plan: PlanBuffer::default(),
        };
        let mut progress = enumeration::NoProgress;
        let report = enumeration::enumerate_into_run(
            port,
            Path::new(&root_path),
            &self.config.enumeration_limits,
            cancellation,
            &mut adapter,
            scan.leased.run.id.clone(),
            &mut progress,
        );
        let plan = adapter.plan;
        let outcome = report.outcome;

        if !report.authoritative {
            let forced_cancel = outcome == enumeration::Outcome::Cancelled;
            return self.resolve_shared(db, &scan, clock, forced_cancel, Some(outcome));
        }

        // Pre-publication fence before any staged record can be published.
        let now = clock.now_ms();
        match self.fence_state_shared(db, &scan, now) {
            FenceState::Unreadable => {
                return self.execution(
                    &scan,
                    ScanExecutionStatus::Fenced,
                    Some(outcome),
                    None,
                    None,
                );
            }
            FenceState::Invalidated => {
                return self.resolve_shared(db, &scan, clock, false, Some(outcome));
            }
            FenceState::Held => {}
        }
        let changes = change_plan_shared(db, &scan.leased.run.scan_root_id, &plan);
        let publish = {
            let Ok(mut guard) = db.lock() else {
                return self.resolve_shared(db, &scan, clock, false, Some(outcome));
            };
            guard.publish_scan_run(
                &scan.leased.run.id,
                &scan.leased.run.session_id,
                &scan.leased.run.lease_token,
                clock.now_ms(),
            )
        };
        match publish {
            Ok(publication) => self.execution(
                &scan,
                ScanExecutionStatus::Published,
                Some(outcome),
                Some(publication),
                changes,
            ),
            Err(_) => self.resolve_shared(db, &scan, clock, false, Some(outcome)),
        }
    }

    /// One worker tick: claim and execute the next due scan, if any.
    pub fn poll<P: FilesystemPort, C: Cancellation>(
        &self,
        db: &mut Database,
        session_id: &str,
        port: &mut P,
        cancellation: &C,
        clock: &dyn ScanClock,
    ) -> Result<Option<ScanExecution>, StorageError> {
        let Some(scan) = self.claim(db, session_id, clock)? else {
            return Ok(None);
        };
        Ok(Some(self.execute(db, scan, port, cancellation, clock)))
    }

    /// Requeue failed jobs whose persisted automatic-retry backoff (1/2/4 s
    /// plus bounded jitter, derived from the job identity) has elapsed. The
    /// retry chain lives on the durable job row, so restarting never resets
    /// the attempt budget; exhausted and cancelled chains are never revived.
    pub fn service_retries(
        &self,
        db: &mut Database,
        clock: &dyn ScanClock,
    ) -> Result<usize, StorageError> {
        let now = clock.now_ms();
        let mut requeued = 0;
        for job in db.list_scan_jobs()? {
            if job.state != ScanJobState::Failed || job.attempt >= job.max_attempts {
                continue;
            }
            if Self::retry_eligible_at(&job) > now {
                continue;
            }
            match db.retry_failed_scan_job(&job.id, now) {
                Ok(true) => requeued += 1,
                Ok(false) => {}
                // A removed root leaves its terminal chain in place; there is
                // no configuration left to retry against.
                Err(StorageError::NotFound) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(requeued)
    }

    /// Deterministic retry-eligibility instant for a failed job: storage's
    /// exponential backoff (1/2/4 s) plus at most 20% jitter, keyed on the
    /// immutable job ID so every worker computes the same schedule.
    pub fn retry_eligible_at(job: &ScanJob) -> i64 {
        let shift = job.attempt.saturating_sub(1).min(2) as u32;
        let backoff = 1_000_i64.checked_shl(shift).unwrap_or(4_000);
        let jitter = backoff.saturating_mul(jitter_numerator(&job.id)) / 1_000;
        job.updated_at_ms
            .saturating_add(backoff)
            .saturating_add(jitter)
    }

    fn fence_state(&self, db: &mut Database, scan: &ActiveScan, now: i64) -> FenceState {
        let (Ok(run), Ok(job)) = (
            db.scan_run(&scan.leased.run.id),
            db.scan_job(&scan.leased.run.scan_job_id),
        ) else {
            return FenceState::Unreadable;
        };
        if run.state != ScanRunState::Running
            || job.state != ScanJobState::Running
            || run.cancellation_requested
            || job.cancellation_requested
            || job.follow_up_requested
            || run.lease_expires_at_ms <= now
        {
            return FenceState::Invalidated;
        }
        FenceState::Held
    }

    /// Terminal resolution for a run the worker could not publish. Storage's
    /// `finish_scan_run` discards the staging and records the outcome in the
    /// same transaction that validates the lease and revision; a Conflict
    /// means another actor (reaper, disable/remove, replacement worker)
    /// already decided, so this worker reports the observed state without
    /// claiming a rollback it did not perform.
    fn resolve(
        &self,
        db: &mut Database,
        scan: &ActiveScan,
        clock: &dyn ScanClock,
        forced_cancel: bool,
        outcome: Option<enumeration::Outcome>,
    ) -> ScanExecution {
        let now = clock.now_ms();
        let (Ok(run), Ok(job)) = (
            db.scan_run(&scan.leased.run.id),
            db.scan_job(&scan.leased.run.scan_job_id),
        ) else {
            return self.execution(scan, ScanExecutionStatus::Fenced, outcome, None, None);
        };
        if run.state != ScanRunState::Running {
            return self.execution(scan, status_from_state(run.state), outcome, None, None);
        }
        let terminal = if forced_cancel || run.cancellation_requested || job.cancellation_requested
        {
            ScanRunOutcome::Cancelled
        } else if job.follow_up_requested {
            // A trigger during traversal invalidates this attempt; storage
            // schedules the one deduplicated follow-up in the same
            // transaction.
            ScanRunOutcome::Interrupted
        } else {
            ScanRunOutcome::Failed
        };
        match db.finish_scan_run_with_error(
            &scan.leased.run.id,
            &scan.leased.run.session_id,
            &scan.leased.run.lease_token,
            now,
            terminal,
            durable_error_code(outcome),
        ) {
            Ok(state) => self.execution(scan, status_from_state(state), outcome, None, None),
            Err(error) => self.execution_fenced(scan, outcome, &error),
        }
    }

    fn fence_state_shared(&self, db: &Mutex<Database>, scan: &ActiveScan, now: i64) -> FenceState {
        let Ok(guard) = db.lock() else {
            return FenceState::Unreadable;
        };
        let (Ok(run), Ok(job)) = (
            guard.scan_run(&scan.leased.run.id),
            guard.scan_job(&scan.leased.run.scan_job_id),
        ) else {
            return FenceState::Unreadable;
        };
        if run.state != ScanRunState::Running
            || job.state != ScanJobState::Running
            || run.cancellation_requested
            || job.cancellation_requested
            || job.follow_up_requested
            || run.lease_expires_at_ms <= now
        {
            return FenceState::Invalidated;
        }
        FenceState::Held
    }

    /// Shared-mutex variant of [`Self::resolve`]: each durable step is its own
    /// short transaction so filesystem I/O never holds the lock. The final
    /// `finish_scan_run` still validates lease, revision and cancellation
    /// atomically, so the cancel/commit race keeps SQLite serialization as the
    /// winner.
    fn resolve_shared(
        &self,
        db: &Mutex<Database>,
        scan: &ActiveScan,
        clock: &dyn ScanClock,
        forced_cancel: bool,
        outcome: Option<enumeration::Outcome>,
    ) -> ScanExecution {
        let now = clock.now_ms();
        let (run, job) = {
            let Ok(guard) = db.lock() else {
                return self.execution(scan, ScanExecutionStatus::Fenced, outcome, None, None);
            };
            let (Ok(run), Ok(job)) = (
                guard.scan_run(&scan.leased.run.id),
                guard.scan_job(&scan.leased.run.scan_job_id),
            ) else {
                return self.execution(scan, ScanExecutionStatus::Fenced, outcome, None, None);
            };
            (run, job)
        };
        if run.state != ScanRunState::Running {
            return self.execution(scan, status_from_state(run.state), outcome, None, None);
        }
        let terminal = if forced_cancel || run.cancellation_requested || job.cancellation_requested
        {
            ScanRunOutcome::Cancelled
        } else if job.follow_up_requested {
            ScanRunOutcome::Interrupted
        } else {
            ScanRunOutcome::Failed
        };
        let finished = {
            let Ok(mut guard) = db.lock() else {
                return self.execution(scan, ScanExecutionStatus::Fenced, outcome, None, None);
            };
            guard.finish_scan_run_with_error(
                &scan.leased.run.id,
                &scan.leased.run.session_id,
                &scan.leased.run.lease_token,
                now,
                terminal,
                durable_error_code(outcome),
            )
        };
        match finished {
            Ok(state) => self.execution(scan, status_from_state(state), outcome, None, None),
            Err(error) => self.execution_fenced(scan, outcome, &error),
        }
    }

    fn execution(
        &self,
        scan: &ActiveScan,
        status: ScanExecutionStatus,
        outcome: Option<enumeration::Outcome>,
        publication: Option<ScanPublication>,
        changes: Option<ChangeSummary>,
    ) -> ScanExecution {
        ScanExecution {
            run_id: scan.leased.run.id.clone(),
            job_id: scan.leased.run.scan_job_id.clone(),
            scan_root_id: scan.leased.run.scan_root_id.clone(),
            status,
            authoritative: status == ScanExecutionStatus::Published,
            enumeration_outcome: outcome,
            publication,
            changes,
            error_code: None,
        }
    }

    fn execution_fenced(
        &self,
        scan: &ActiveScan,
        outcome: Option<enumeration::Outcome>,
        error: &StorageError,
    ) -> ScanExecution {
        let mut execution = self.execution(scan, ScanExecutionStatus::Fenced, outcome, None, None);
        execution.error_code = Some(error.to_string());
        execution
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FenceState {
    Held,
    Invalidated,
    Unreadable,
}

/// A leased scan with its open staging session. Dropping this value without
/// calling [`ScanWorker::execute`] leaves the staging provisional and the
/// lease to expire: the reaper, the next session start, or disable/remove
/// discard it, so a crashed worker can never publish.
#[derive(Clone, Debug)]
pub struct ActiveScan {
    pub leased: LeasedScan,
    pub staging_begun_at_ms: i64,
}

/// What the worker did with one claimed scan. `Published` is the only status
/// under which committed Library rows changed; every other status leaves the
/// previous committed results exactly as they were.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanExecution {
    pub run_id: String,
    pub job_id: String,
    pub scan_root_id: String,
    pub status: ScanExecutionStatus,
    pub authoritative: bool,
    pub enumeration_outcome: Option<enumeration::Outcome>,
    pub publication: Option<ScanPublication>,
    pub changes: Option<ChangeSummary>,
    /// Fixed storage diagnostics code only (never SQL, values or paths).
    pub error_code: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanExecutionStatus {
    Published,
    Failed,
    Cancelled,
    Interrupted,
    /// The worker could not finalize this run itself because durable state
    /// had already been decided elsewhere (expired/replaced lease, reaped
    /// run, disabled/removed root). No committed row changed because of this
    /// worker.
    Fenced,
}

fn status_from_state(state: ScanRunState) -> ScanExecutionStatus {
    match state {
        ScanRunState::Completed => ScanExecutionStatus::Published,
        ScanRunState::Failed => ScanExecutionStatus::Failed,
        ScanRunState::Cancelled => ScanExecutionStatus::Cancelled,
        ScanRunState::Interrupted => ScanExecutionStatus::Interrupted,
        ScanRunState::Running => ScanExecutionStatus::Fenced,
    }
}

fn durable_error_code(outcome: Option<enumeration::Outcome>) -> Option<&'static str> {
    match outcome {
        Some(enumeration::Outcome::Denied) => Some("access_denied"),
        Some(enumeration::Outcome::UnsupportedFilesystem) => Some("unsupported"),
        Some(enumeration::Outcome::ResourceLimit) => Some("resource_limit"),
        Some(enumeration::Outcome::RootUnavailable) => Some("unavailable"),
        Some(enumeration::Outcome::Cancelled) => Some("cancelled"),
        Some(
            enumeration::Outcome::Partial
            | enumeration::Outcome::Invalid
            | enumeration::Outcome::SinkFailed,
        ) => Some("worker_failed"),
        Some(enumeration::Outcome::Complete) | None => None,
    }
}

fn root_path(db: &Database, root_id: &str) -> Option<String> {
    db.list_scan_roots()
        .ok()?
        .into_iter()
        .find(|root| root.id == root_id)
        .map(|root| root.canonical_path)
}

fn root_path_shared(db: &Mutex<Database>, root_id: &str) -> Option<String> {
    let guard = db.lock().ok()?;
    root_path(&guard, root_id)
}

/// Bounded in-memory mirror of the observations this worker staged, used only
/// to derive the advisory reconciliation change summary after an
/// authoritative traversal. It is capped at the durable staging quota.
#[derive(Default)]
struct PlanBuffer {
    observations: Vec<PlanObservation>,
    truncated: bool,
}

impl PlanBuffer {
    const CAPACITY: usize = MAX_STAGED_RECORDS as usize;

    fn absorb(&mut self, records: &[enumeration::Observation]) {
        if self.truncated {
            return;
        }
        if self.observations.len().saturating_add(records.len()) > Self::CAPACITY {
            self.truncated = true;
            self.observations.clear();
            return;
        }
        for record in records {
            self.observations.push(PlanObservation {
                path: record.display_path.as_str().to_owned(),
                metadata: PlanMetadata {
                    size: record.byte_size,
                    modified_ns: record.modified_unix_ns,
                    identity: record.identity.as_ref().map(|identity| PlanIdentity {
                        volume: identity.volume_serial,
                        file: identity.file_id,
                    }),
                },
            });
        }
    }
}

/// Advisory per-path decision counts from the reconciliation core. They are
/// deterministic evidence for the manual-scan journey; the durable
/// publication applies its own fenced decisions and remains the authority.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChangeSummary {
    pub computed: bool,
    pub added: usize,
    pub modified: usize,
    pub replaced: usize,
    pub identity_uncertain: usize,
    pub missing: usize,
    pub restored: usize,
    pub renames: usize,
}

impl ChangeSummary {
    fn from_plan(plan: &reconciliation::Plan) -> Self {
        let mut summary = Self {
            computed: true,
            ..Self::default()
        };
        for change in &plan.changes {
            match change {
                reconciliation::Change::Added(_) => summary.added += 1,
                reconciliation::Change::Modified(_) => summary.modified += 1,
                reconciliation::Change::Replaced(_) => summary.replaced += 1,
                reconciliation::Change::IdentityUncertain { .. } => summary.identity_uncertain += 1,
                reconciliation::Change::Missing(_) => summary.missing += 1,
                reconciliation::Change::Restored(_) => summary.restored += 1,
                reconciliation::Change::RenameEvidence { .. } => summary.renames += 1,
            }
        }
        summary
    }
}

/// Compare the committed rows of one root with the run's complete observation
/// set through the reconciliation core. Returns `None` when the comparison
/// cannot be made inside the reference bounds (huge histories, unreadable
/// pages, duplicate or invalid paths) — that never blocks publication.
fn change_plan(db: &Database, root_id: &str, buffer: &PlanBuffer) -> Option<ChangeSummary> {
    if buffer.truncated {
        return None;
    }
    let mut previous = Vec::new();
    let mut cursor = None;
    let mut snapshot = None;
    loop {
        let page = db
            .query_library(&LibraryQuery {
                scan_root_id: root_id.to_owned(),
                page_size: MAX_LIBRARY_PAGE_SIZE,
                cursor: cursor.clone(),
                snapshot: snapshot.clone(),
            })
            .ok()?;
        for location in &page.locations {
            previous.push(plan_location(location));
        }
        if !page.has_more {
            break;
        }
        if previous.len() > PlanBuffer::CAPACITY {
            return None;
        }
        cursor = page.next_cursor;
        snapshot = Some(page.snapshot);
    }
    if previous.len().saturating_add(buffer.observations.len()) > PlanBuffer::CAPACITY {
        return None;
    }
    let plan = reconcile(&previous, &buffer.observations, PlanOutcome::Complete).ok()?;
    Some(ChangeSummary::from_plan(&plan))
}

/// Shared-mutex variant of [`change_plan`]: each library page is its own
/// short read transaction so a concurrent status or page query can interleave.
/// No concurrent publication for the same root is possible while its run is
/// `running` (storage refuses a second lease), so the snapshot stays stable.
fn change_plan_shared(
    db: &Mutex<Database>,
    root_id: &str,
    buffer: &PlanBuffer,
) -> Option<ChangeSummary> {
    if buffer.truncated {
        return None;
    }
    let mut previous = Vec::new();
    let mut cursor = None;
    let mut snapshot = None;
    loop {
        let page = {
            let guard = db.lock().ok()?;
            guard
                .query_library(&LibraryQuery {
                    scan_root_id: root_id.to_owned(),
                    page_size: MAX_LIBRARY_PAGE_SIZE,
                    cursor: cursor.clone(),
                    snapshot: snapshot.clone(),
                })
                .ok()?
        };
        for location in &page.locations {
            previous.push(plan_location(location));
        }
        if !page.has_more {
            break;
        }
        if previous.len() > PlanBuffer::CAPACITY {
            return None;
        }
        cursor = page.next_cursor;
        snapshot = Some(page.snapshot);
    }
    if previous.len().saturating_add(buffer.observations.len()) > PlanBuffer::CAPACITY {
        return None;
    }
    let plan = reconcile(&previous, &buffer.observations, PlanOutcome::Complete).ok()?;
    Some(ChangeSummary::from_plan(&plan))
}

fn plan_location(location: &PublishedLocation) -> PlanLocation {
    PlanLocation {
        path: location.relative_path.clone(),
        metadata: PlanMetadata {
            size: location.byte_size,
            modified_ns: i128::from(location.modified_at_ns),
            identity: location.identity.as_ref().and_then(|identity| {
                Some(PlanIdentity {
                    volume: identity.volume_serial.parse().ok()?,
                    file: identity.file_id.parse().ok()?,
                })
            }),
        },
        present: location.presence == FilePresence::Present,
    }
}

/// Convert the enumeration boundary's handoff DTOs into the storage staging
/// DTOs. The two crates mirror the same field shapes but stay deliberately
/// decoupled, so the worker performs the field-for-field mapping.
fn staged_identity(identity: &enumeration::EncodedIdentity) -> StagedIdentity {
    StagedIdentity {
        volume_serial: identity.volume_serial.clone(),
        file_id: identity.file_id.clone(),
    }
}

fn staged_observation(observation: &enumeration::ScanObservation) -> StagedObservation {
    StagedObservation {
        locator_key: observation.locator_key.clone(),
        relative_path: observation.relative_path.clone(),
        byte_size: observation.byte_size,
        modified_at_ns: observation.modified_at_ns,
        identity: observation.identity.as_ref().map(staged_identity),
    }
}

/// Storage's staging adapter for one run: converts bounded enumerator batches
/// into the durable staging DTO and revalidates the worker's fences between
/// batches. Any fence violation (lease loss, durable cancellation, queued
/// invalidation, terminal run) aborts the traversal immediately with
/// `SinkError::Unavailable`, so no further provisional batch is staged and
/// the run ends non-authoritative.
struct StagingAdapter<'a> {
    db: &'a mut Database,
    clock: &'a dyn ScanClock,
    run_id: String,
    job_id: String,
    session_id: String,
    lease_token: String,
    lease_duration_ms: i64,
    renewal_interval_ms: i64,
    next_renewal_ms: i64,
    plan: PlanBuffer,
}

impl RunScopedStorage for StagingAdapter<'_> {
    fn stage_batch(&mut self, _run_id: &str, batch: ObservationBatch) -> Result<(), SinkError> {
        let now = self.clock.now_ms();
        if now >= self.next_renewal_ms {
            self.db
                .renew_scan_lease(
                    &self.run_id,
                    &self.session_id,
                    &self.lease_token,
                    now,
                    self.lease_duration_ms,
                )
                .map_err(|_| SinkError::Unavailable)?;
            self.next_renewal_ms = now.saturating_add(self.renewal_interval_ms);
        }
        if self.fence_violated(now) {
            return Err(SinkError::Unavailable);
        }
        let mut observations = Vec::with_capacity(batch.records.len());
        for record in &batch.records {
            match record.to_scan_observation() {
                Ok(observation) => observations.push(staged_observation(&observation)),
                Err(_) => return Err(SinkError::Rejected),
            }
        }
        self.plan.absorb(&batch.records);
        match self.db.stage_scan_observations(
            &self.run_id,
            &self.session_id,
            &self.lease_token,
            now,
            &observations,
        ) {
            Ok(_) => Ok(()),
            Err(StorageError::StagingRejected) => Err(SinkError::Rejected),
            Err(_) => Err(SinkError::Unavailable),
        }
    }

    fn invalidate_run(&mut self, _run_id: &str) {
        // Deliberate no-op: the worker terminalizes the run through
        // `finish_scan_run`, which discards the staging in the same durable
        // transaction that records the outcome. If this worker dies before
        // that, the lease reaper, the next session start, or disable/remove
        // discard the staging; a panicking traversal therefore cannot leave
        // publishable state behind either.
    }
}

impl StagingAdapter<'_> {
    fn fence_violated(&self, now: i64) -> bool {
        let (Ok(run), Ok(job)) = (
            self.db.scan_run(&self.run_id),
            self.db.scan_job(&self.job_id),
        ) else {
            return true;
        };
        run.state != ScanRunState::Running
            || job.state != ScanJobState::Running
            || run.cancellation_requested
            || job.cancellation_requested
            || job.follow_up_requested
            || run.lease_expires_at_ms <= now
    }
}

/// Shared-mutex staging adapter for [`ScanWorker::execute_shared`]: the same
/// bounded DTO conversion and per-batch fence revalidation as
/// [`StagingAdapter`], but each durable step is its own short transaction.
/// The database mutex is never held across filesystem I/O — it is acquired
/// for lease renewal, then released, then re-acquired for the fence read,
/// then released, then re-acquired for the staging write — so console reads
/// interleave between batches. A poisoned or busy mutex maps to
/// `SinkError::Unavailable`, which ends the run non-authoritative exactly
/// like a fence violation.
struct SharedStagingAdapter<'a> {
    db: &'a Mutex<Database>,
    clock: &'a dyn ScanClock,
    run_id: String,
    job_id: String,
    session_id: String,
    lease_token: String,
    lease_duration_ms: i64,
    renewal_interval_ms: i64,
    next_renewal_ms: i64,
    plan: PlanBuffer,
}

impl RunScopedStorage for SharedStagingAdapter<'_> {
    fn stage_batch(&mut self, _run_id: &str, batch: ObservationBatch) -> Result<(), SinkError> {
        let now = self.clock.now_ms();
        if now >= self.next_renewal_ms {
            let renewed = {
                let Ok(mut guard) = self.db.lock() else {
                    return Err(SinkError::Unavailable);
                };
                guard.renew_scan_lease(
                    &self.run_id,
                    &self.session_id,
                    &self.lease_token,
                    now,
                    self.lease_duration_ms,
                )
            };
            renewed.map_err(|_| SinkError::Unavailable)?;
            self.next_renewal_ms = now.saturating_add(self.renewal_interval_ms);
        }
        if self.fence_violated(now) {
            return Err(SinkError::Unavailable);
        }
        let mut observations = Vec::with_capacity(batch.records.len());
        for record in &batch.records {
            match record.to_scan_observation() {
                Ok(observation) => observations.push(staged_observation(&observation)),
                Err(_) => return Err(SinkError::Rejected),
            }
        }
        self.plan.absorb(&batch.records);
        let staged = {
            let Ok(mut guard) = self.db.lock() else {
                return Err(SinkError::Unavailable);
            };
            guard.stage_scan_observations(
                &self.run_id,
                &self.session_id,
                &self.lease_token,
                now,
                &observations,
            )
        };
        match staged {
            Ok(_) => Ok(()),
            Err(StorageError::StagingRejected) => Err(SinkError::Rejected),
            Err(_) => Err(SinkError::Unavailable),
        }
    }

    fn invalidate_run(&mut self, _run_id: &str) {
        // Same deliberate no-op as `StagingAdapter::invalidate_run`.
    }
}

impl SharedStagingAdapter<'_> {
    fn fence_violated(&self, now: i64) -> bool {
        let Ok(guard) = self.db.lock() else {
            return true;
        };
        let (Ok(run), Ok(job)) = (guard.scan_run(&self.run_id), guard.scan_job(&self.job_id))
        else {
            return true;
        };
        run.state != ScanRunState::Running
            || job.state != ScanJobState::Running
            || run.cancellation_requested
            || job.cancellation_requested
            || job.follow_up_requested
            || run.lease_expires_at_ms <= now
    }
}

/// Deterministic bounded jitter numerator in `0..=200` thousandths of the
/// backoff (<= 20%), derived from the immutable job ID.
fn jitter_numerator(job_id: &str) -> i64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in job_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (hash % 201) as i64
}

#[cfg(test)]
mod tests;
