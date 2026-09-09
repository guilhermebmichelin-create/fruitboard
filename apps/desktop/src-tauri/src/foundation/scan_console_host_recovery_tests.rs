use super::{
    CancellationMirror, ScanConsoleHost, ScanEventSink, ScanStatusChangedEvent, build_host,
};
use fruitboard_filesystem_enumeration::Cancellation;
use fruitboard_scan_execution::{ScanClock, ScanWorker, SystemClock, WorkerConfig};
use fruitboard_storage::{Database, ScanJobState, ScanKind, ScanRunState};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct NoopSink;

impl ScanEventSink for NoopSink {
    fn emit(&self, _event: &ScanStatusChangedEvent) {}
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "fruitboard-scan-console-recovery-{}",
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir(&path).expect("create test directory");
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn active_host() -> (
    TestDirectory,
    Arc<Mutex<Database>>,
    Arc<ScanConsoleHost>,
    String,
) {
    let directory = TestDirectory::new();
    let database = Arc::new(Mutex::new(
        Database::open(&directory.0).expect("open test database"),
    ));
    let host = {
        let mut database_guard = database.lock().expect("database lock");
        Arc::new(
            build_host(
                &mut database_guard,
                Arc::new(SystemClock),
                Arc::new(NoopSink),
            )
            .expect("build host"),
        )
    };
    let root_id = database
        .lock()
        .expect("database lock")
        .add_scan_root("Synthetic", r"C:\synthetic-root")
        .expect("add root")
        .id;
    let job_id = database
        .lock()
        .expect("database lock")
        .enqueue_scan(&root_id, ScanKind::Manual, SystemClock.now_ms())
        .expect("enqueue scan")
        .job_id;
    assert!(host.claim_due(&database));
    (directory, database, host, job_id)
}

#[test]
fn shutdown_fences_active_run_as_interrupted_for_restart_recovery() {
    let (_directory, database, host, job_id) = active_host();
    let (chain_id, run_id) = {
        let database = database.lock().expect("database lock");
        let job = database.scan_job(&job_id).expect("running job");
        let run = database
            .list_scan_runs()
            .expect("scan runs")
            .into_iter()
            .find(|run| run.scan_job_id == job_id)
            .expect("running scan");
        assert_eq!(job.state, ScanJobState::Running);
        assert_eq!(run.state, ScanRunState::Running);
        (job.retry_chain_id, run.id)
    };

    host.shutdown(&database);

    {
        let database = database.lock().expect("database lock");
        let job = database.scan_job(&job_id).expect("fenced job");
        let run = database.scan_run(&run_id).expect("fenced run");
        assert_eq!(job.state, ScanJobState::Interrupted);
        assert!(!job.cancellation_requested);
        assert_eq!(run.state, ScanRunState::Interrupted);
        assert!(!run.cancellation_requested);
    }

    let worker = ScanWorker::new(WorkerConfig::default()).expect("worker config");
    {
        let mut database = database.lock().expect("database lock");
        worker
            .start_session(&mut database, &SystemClock)
            .expect("restart session");
        let recovered = database.scan_job(&job_id).expect("recovered job");
        assert_eq!(recovered.state, ScanJobState::Queued);
        assert_eq!(recovered.retry_chain_id, chain_id);
        assert!(!recovered.cancellation_requested);
    }
}

#[test]
fn shutdown_preserves_a_durable_user_cancellation() {
    let (_directory, database, host, job_id) = active_host();
    database
        .lock()
        .expect("database lock")
        .cancel_scan_job(&job_id, SystemClock.now_ms())
        .expect("request cancellation");

    host.shutdown(&database);

    let database = database.lock().expect("database lock");
    let job = database.scan_job(&job_id).expect("cancelled job");
    let run = database
        .list_scan_runs()
        .expect("scan runs")
        .into_iter()
        .find(|run| run.scan_job_id == job_id)
        .expect("cancelled scan");
    assert_eq!(job.state, ScanJobState::Cancelled);
    assert!(job.cancellation_requested);
    assert_eq!(run.state, ScanRunState::Cancelled);
    assert!(run.cancellation_requested);
}

#[test]
fn shutdown_preserves_mirror_only_user_cancellation_before_durable_commit() {
    let (_directory, database, host, job_id) = active_host();
    host.request_cancellation(&job_id);

    host.shutdown(&database);

    {
        let database = database.lock().expect("database lock");
        let job = database.scan_job(&job_id).expect("cancelled job");
        let run = database
            .list_scan_runs()
            .expect("scan runs")
            .into_iter()
            .find(|run| run.scan_job_id == job_id)
            .expect("cancelled scan");
        assert_eq!(job.state, ScanJobState::Cancelled);
        assert!(job.cancellation_requested);
        assert_eq!(run.state, ScanRunState::Cancelled);
        assert!(run.cancellation_requested);
    }

    let worker = ScanWorker::new(WorkerConfig::default()).expect("worker config");
    let mut database = database.lock().expect("database lock");
    worker
        .start_session(&mut database, &SystemClock)
        .expect("restart session");
    assert_eq!(
        database.scan_job(&job_id).expect("cancelled job").state,
        ScanJobState::Cancelled
    );
}

#[test]
fn a_cancellation_registered_after_stop_is_seen_by_the_mirror() {
    let (_directory, database, host, _job_id) = active_host();
    let scan = host
        .pending
        .lock()
        .expect("pending lock")
        .as_ref()
        .expect("claimed scan")
        .scan
        .clone();
    host.stopping
        .store(true, std::sync::atomic::Ordering::Release);
    let flag = host.register_cancellation(&scan);
    let mirror = CancellationMirror {
        flag,
        stopping: host.stopping.clone(),
    };

    assert!(mirror.is_cancelled());
    host.shutdown(&database);
}

// Regression for the installed queue stall (final 2026-09-09 regression, §5):
// after an unavailable-root automatic retry completes, a watcher follow-up
// stayed queued for minutes with no running job, every later `scan_now`
// coalesced to `already_queued`, and a restart never restored progress.
//
// Cause: `service_retries` aborted the whole sweep when requeueing a failed
// job whose root already owned the one active (queued/running) slot
// (`scan_job_active_root` partial unique index), and `claim_due` skips the
// claim whenever the sweep errors — so one root holding {queued +
// retry-eligible failed} wedged the global single worker on every tick,
// durably across restarts. The retry must skip the occupied slot instead of
// failing the sweep, so due queued follow-ups (on every root) still run and
// the skipped chain converges once the slot frees.
//
// The test drives the real host loop composition (`tick` =
// `service_retries` + `claim` + shared-staging `execute`), not a manual
// worker poll, with a scripted online/offline port and a fake clock: no
// wall-clock sleeps.
#[test]
fn failed_retry_behind_queued_follow_up_does_not_block_the_worker() {
    use fruitboard_filesystem_enumeration::{
        DirectoryCaseSensitivity, DirectoryCursor, DirectoryEntry, EntryKind, FileMetadata,
        FilesystemPort, FilesystemQualification, IdentityQualification, OpenedDirectory, PortError,
        QualifiedIdentity, RootMetadata,
    };
    #[derive(Clone)]
    struct ScheduleClock(Arc<std::sync::Mutex<i64>>);
    impl ScanClock for ScheduleClock {
        fn now_ms(&self) -> i64 {
            *self.0.lock().unwrap()
        }
    }

    struct TogglePort {
        offline: bool,
    }
    impl FilesystemPort for TogglePort {
        fn inspect_root(&mut self, _root: &std::path::Path) -> Result<RootMetadata, PortError> {
            if self.offline {
                return Err(PortError::NotFound);
            }
            Ok(RootMetadata {
                metadata: FileMetadata {
                    kind: EntryKind::Directory,
                    byte_size: 0,
                    modified_unix_ns: 1_700_000_000_000_000_000,
                    identity: Some(QualifiedIdentity {
                        volume_serial: 7,
                        file_id: 999,
                        qualification: IdentityQualification::LocalNtfs,
                    }),
                    reparse_point: false,
                    recall_or_offline: false,
                },
                qualification: FilesystemQualification::LocalNtfs,
            })
        }
        fn open_root(&mut self, _root: &std::path::Path) -> Result<OpenedDirectory, PortError> {
            if self.offline {
                return Err(PortError::NotFound);
            }
            Ok(OpenedDirectory {
                metadata: FileMetadata {
                    kind: EntryKind::Directory,
                    byte_size: 0,
                    modified_unix_ns: 1_700_000_000_000_000_000,
                    identity: Some(QualifiedIdentity {
                        volume_serial: 7,
                        file_id: 999,
                        qualification: IdentityQualification::LocalNtfs,
                    }),
                    reparse_point: false,
                    recall_or_offline: false,
                },
                case_sensitivity: DirectoryCaseSensitivity::Insensitive,
                cursor: Box::new(EmptyCursor),
            })
        }
    }
    struct EmptyCursor;
    impl DirectoryCursor for EmptyCursor {
        fn next_entry(&mut self) -> Result<Option<DirectoryEntry>, PortError> {
            Ok(None)
        }
        fn read_metadata(&mut self, _entry: &DirectoryEntry) -> Result<FileMetadata, PortError> {
            Err(PortError::NotFound)
        }
        fn open_directory(
            &mut self,
            _entry: &DirectoryEntry,
        ) -> Result<OpenedDirectory, PortError> {
            Err(PortError::NotFound)
        }
    }

    let directory = TestDirectory::new();
    let clock = ScheduleClock(Arc::new(std::sync::Mutex::new(1_700_000_000_000)));
    let database = Arc::new(Mutex::new(
        Database::open(&directory.0).expect("open test database"),
    ));
    let host = {
        let mut guard = database.lock().expect("database lock");
        Arc::new(
            build_host(&mut guard, Arc::new(clock.clone()), Arc::new(NoopSink))
                .expect("build host"),
        )
    };
    let root_id = database
        .lock()
        .expect("database lock")
        .add_scan_root("Synthetic", r"C:\synthetic-root")
        .expect("add root")
        .id;
    let now = || clock.now_ms();
    let enqueue_manual = || {
        database
            .lock()
            .expect("database lock")
            .enqueue_scan(&root_id, ScanKind::Manual, now())
            .expect("enqueue manual")
            .job_id
    };
    let enqueue_periodic = || {
        database
            .lock()
            .expect("database lock")
            .enqueue_scan(&root_id, ScanKind::Periodic, now())
            .expect("enqueue periodic")
            .job_id
    };
    let advance = |ms: i64| {
        *clock.0.lock().unwrap() += ms;
    };
    let job_state = |id: &str| {
        database
            .lock()
            .expect("database lock")
            .scan_job(id)
            .expect("job")
            .state
    };
    let job_attempt = |id: &str| {
        database
            .lock()
            .expect("database lock")
            .scan_job(id)
            .expect("job")
            .attempt
    };
    let tick_online = || {
        host.tick(&database, &mut TogglePort { offline: false });
    };
    let tick_offline = || {
        host.tick(&database, &mut TogglePort { offline: true });
    };

    // 1. Baseline publishes.
    let base = enqueue_manual();
    tick_online();
    assert_eq!(
        job_state(&base),
        ScanJobState::Completed,
        "baseline publishes"
    );

    // 2. Manual scan while unavailable fails (attempt 1).
    let j1 = enqueue_manual();
    tick_offline();
    assert_eq!(job_state(&j1), ScanJobState::Failed, "manual fails offline");

    // 3. Watcher-style follow-up while still away fails too.
    let j1b = enqueue_periodic();
    assert_ne!(j1b, j1, "failed chain owns no slot: fresh follow-up job");
    tick_offline();
    assert_eq!(
        job_state(&j1b),
        ScanJobState::Failed,
        "follow-up fails offline"
    );

    // 4. Root restored; the automatic retry of the manual chain completes on
    // the next host tick instead of wedging behind the failed follow-up.
    advance(1_500);
    tick_online();
    assert_eq!(
        job_state(&j1),
        ScanJobState::Completed,
        "restored automatic retry of the manual chain completes"
    );
    assert_eq!(job_attempt(&j1), 2, "retry budget advances, never resets");
    assert_eq!(
        job_state(&j1b),
        ScanJobState::Failed,
        "occupied slot skips the second retry without deleting it"
    );

    // 5. Post-recovery watcher follow-up is queued (scan_now coalesces).
    let j2 = enqueue_periodic();
    let coalesced = database
        .lock()
        .expect("database lock")
        .enqueue_scan(&root_id, ScanKind::Manual, now())
        .expect("scan_now coalesces");
    assert!(coalesced.coalesced, "scan_now coalesces onto the follow-up");
    assert_eq!(coalesced.job_id, j2, "coalesced onto the follow-up job");

    // 6. The follow-up runs on the next host ticks instead of stalling
    // queued with no running job; the skipped chain then converges too.
    for _ in 0..5 {
        tick_online();
    }
    assert_eq!(
        job_state(&j2),
        ScanJobState::Completed,
        "post-recovery watcher follow-up runs instead of stalling queued"
    );
    assert_eq!(job_attempt(&j2), 1, "follow-up runs its first attempt");
    tick_online();
    assert_eq!(
        job_state(&j1b),
        ScanJobState::Completed,
        "skipped retry converges once the slot frees"
    );
    assert_eq!(job_attempt(&j1b), 2, "retry budget advances, never resets");

    // No queued job was deleted, reset, or suppressed to clear the stall.
    let jobs = database
        .lock()
        .expect("database lock")
        .list_scan_jobs()
        .expect("jobs");
    assert_eq!(jobs.len(), 4, "every chain is retained");
    assert!(
        database
            .lock()
            .expect("database lock")
            .list_scan_runs()
            .expect("runs")
            .iter()
            .all(|run| run.state != ScanRunState::Running),
        "no leaked running run"
    );
}
