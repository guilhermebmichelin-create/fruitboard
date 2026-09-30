use super::{
    DiagnosticCode, HostSignal, LogEventKind, SafeDiagnostic, ScanConsoleHost, ScanEventSink,
    ScanStatusChangedEvent, build_host,
};
use crate::foundation::test_support::RecordingLogSink;
use fruitboard_scan_execution::{ScanWorker, SystemClock, WorkerConfig};
use fruitboard_storage::{Database, StorageError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

struct NoopSink;

impl ScanEventSink for NoopSink {
    fn emit(&self, _event: &ScanStatusChangedEvent) {}
}

fn unstarted_host_with_logs(logs: Arc<RecordingLogSink>) -> Arc<ScanConsoleHost> {
    let worker = ScanWorker::new(WorkerConfig::default()).expect("default worker config");
    Arc::new(ScanConsoleHost::new(
        worker,
        "test-session".to_owned(),
        Arc::new(SystemClock),
        Arc::new(NoopSink),
        logs,
    ))
}

fn unstarted_host() -> Arc<ScanConsoleHost> {
    unstarted_host_with_logs(Arc::new(RecordingLogSink::default()))
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir();

        loop {
            let suffix = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!(
                "fruitboard-scan-console-host-{}-{}",
                std::process::id(),
                suffix
            ));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("create test directory {}: {error}", path.display()),
            }
        }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn started_empty_host() -> (TestDirectory, Arc<Mutex<Database>>, Arc<ScanConsoleHost>) {
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
                Arc::new(RecordingLogSink::default()),
            )
            .expect("build host"),
        )
    };
    (directory, database, host)
}

#[test]
fn wake_signals_are_capacity_one_and_coalesced() {
    let host = unstarted_host();
    let (worker_sender, worker_receiver) = mpsc::sync_channel(1);
    *host.worker_signal.lock().expect("worker signal lock") = Some(worker_sender);

    #[cfg(windows)]
    let (watcher_sender, watcher_receiver) = mpsc::sync_channel(1);
    #[cfg(windows)]
    {
        *host.watcher_signal.lock().expect("watcher signal lock") = Some(watcher_sender);
    }

    host.wake();
    host.wake();
    host.wake();

    assert_eq!(
        worker_receiver.try_recv(),
        Ok(HostSignal::Wake),
        "the first wake is retained"
    );
    assert!(matches!(
        worker_receiver.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));

    #[cfg(windows)]
    {
        assert_eq!(
            watcher_receiver.try_recv(),
            Ok(super::SupervisorSignal::Wake),
            "the watcher wake is retained"
        );
        assert!(matches!(
            watcher_receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }
}

#[test]
fn spawned_empty_host_shutdown_joins_every_loop() {
    let (_directory, database, host) = started_empty_host();
    host.spawn(database.clone()).expect("spawn host loops");
    assert!(host.worker_join.lock().expect("worker join lock").is_some());
    #[cfg(windows)]
    assert!(
        host.watcher_join
            .lock()
            .expect("watcher join lock")
            .is_some()
    );

    host.shutdown(&database);

    assert!(host.stopping.load(Ordering::Acquire));
    assert!(host.worker_join.lock().expect("worker join lock").is_none());
    #[cfg(windows)]
    assert!(
        host.watcher_join
            .lock()
            .expect("watcher join lock")
            .is_none()
    );
}

#[test]
fn shutdown_is_idempotent_and_spawn_is_one_shot() {
    let (_directory, database, host) = started_empty_host();
    host.spawn(database.clone()).expect("first spawn");
    assert_eq!(host.spawn(database.clone()), Err(StorageError::Conflict));

    host.shutdown(&database);
    host.shutdown(&database);
    assert_eq!(host.spawn(database), Err(StorageError::Conflict));
}

#[test]
fn shutdown_join_releases_thread_host_reference() {
    let (_directory, database, host) = started_empty_host();
    let weak_host = Arc::downgrade(&host);
    host.spawn(database.clone()).expect("spawn host loops");
    host.shutdown(&database);

    // The worker owns the only additional host reference. Joining it makes
    // the release deterministic; no timeout or scheduler sleep is needed.
    assert_eq!(Arc::strong_count(&host), 1);
    drop(host);
    assert!(weak_host.upgrade().is_none());
}

/// Regression test for the shutdown/claim_deadlock: `claim_due` takes the
/// `lifecycle` lock as its first action every poll tick, and the pre-fix
/// `shutdown` held that lock across both `join`s, so a tick that reached
/// `claim_due` during shutdown wedged app exit forever. The poll gate parks
/// the native loop exactly before `claim_due`, and the bounded wait turns the
/// pre-fix deadlock into a test failure instead of a hang.
#[test]
fn shutdown_completes_while_a_poll_tick_contends_for_the_lifecycle_lock() {
    let (_directory, database, host) = started_empty_host();
    let poll_gate = host.arm_poll_gate();
    host.spawn(database.clone()).expect("spawn host loops");

    let (done_sender, done_receiver) = mpsc::sync_channel(1);
    let shutdown_host = host.clone();
    let shutdown_database = database.clone();
    std::thread::spawn(move || {
        shutdown_host.shutdown(&shutdown_database);
        let _ = done_sender.send(());
    });

    // `shutdown_requested` is set first, under the lifecycle lock. Once it
    // is observable the shutdown thread owns (or has just released) that
    // lock, so releasing the gate now deterministically drives the parked
    // tick into `claim_due` while the shutdown is between lock acquisition
    // and join completion.
    let deadline = Instant::now() + Duration::from_secs(10);
    while !host.shutdown_requested.load(Ordering::Acquire) && Instant::now() < deadline {}
    assert!(
        host.shutdown_requested.load(Ordering::Acquire),
        "the shutdown thread must reach its critical section"
    );
    drop(poll_gate);

    assert!(
        done_receiver.recv_timeout(Duration::from_secs(20)).is_ok(),
        "shutdown must not deadlock against a poll tick contending for the lifecycle lock"
    );
    assert!(host.stopping.load(Ordering::Acquire));
}

/// A failed durable fence must still stop the host before the joins; only
/// the user-cancellation mirror stays untouched so no false user
/// cancellation can be recorded. `stopping` is asserted before the parked
/// poll loop is released, which the pre-fix ordering (stop only after
/// joins) cannot satisfy.
#[test]
fn shutdown_stops_the_host_even_when_the_durable_fence_cannot_run() {
    let (_directory, database, host) = started_empty_host();
    let poll_gate = host.arm_poll_gate();
    host.spawn(database.clone()).expect("spawn host loops");

    // Poison the shared database mutex: `fence_active_run` can no longer
    // terminalize anything durably and reports failure.
    let poisoner = {
        let database = database.clone();
        std::thread::spawn(move || {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _guard = database.lock().expect("database lock");
                panic!("poison the shared database lock on purpose");
            }));
        })
    };
    poisoner.join().expect("poisoning thread exits");

    let (done_sender, done_receiver) = mpsc::sync_channel(1);
    let shutdown_host = host.clone();
    let shutdown_database = database.clone();
    std::thread::spawn(move || {
        shutdown_host.shutdown(&shutdown_database);
        let _ = done_sender.send(());
    });

    let deadline = Instant::now() + Duration::from_secs(10);
    while !host.stopping.load(Ordering::Acquire) && Instant::now() < deadline {}
    assert!(
        host.stopping.load(Ordering::Acquire),
        "stopping must be set before the joins even when the fence fails"
    );
    drop(poll_gate);
    assert!(
        done_receiver.recv_timeout(Duration::from_secs(20)).is_ok(),
        "shutdown must complete with an unavailable fence"
    );
}

#[test]
fn a_panicking_lifecycle_thread_is_contained_logged_and_stops_the_host() {
    let logs = Arc::new(RecordingLogSink::default());
    let host = unstarted_host_with_logs(logs.clone());

    host.run_guarded_thread(
        "scan_console_thread",
        DiagnosticCode::ScanConsoleThreadPanicked,
        || panic!("synthetic lifecycle thread failure"),
    );

    assert!(
        host.stopping.load(Ordering::Acquire),
        "a panicked lifecycle thread stops the host"
    );
    assert!(host.shutdown_requested.load(Ordering::Acquire));
    let events = logs.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, LogEventKind::ThreadPanicked);
    assert_eq!(events[0].operation, "scan_console_thread");
    assert_eq!(
        events[0].diagnostic.as_ref().map(SafeDiagnostic::as_str),
        Some("scan_console_thread_panicked"),
        "the log carries only the fixed diagnostic code"
    );
    assert!(events[0].error_code.is_none());
}
