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
