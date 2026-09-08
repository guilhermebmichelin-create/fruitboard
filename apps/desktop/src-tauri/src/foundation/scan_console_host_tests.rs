use super::{HostSignal, ScanConsoleHost, ScanEventSink, ScanStatusChangedEvent, build_host};
use fruitboard_scan_execution::{ScanWorker, SystemClock, WorkerConfig};
use fruitboard_storage::{Database, StorageError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};

struct NoopSink;

impl ScanEventSink for NoopSink {
    fn emit(&self, _event: &ScanStatusChangedEvent) {}
}

fn unstarted_host() -> Arc<ScanConsoleHost> {
    let worker = ScanWorker::new(WorkerConfig::default()).expect("default worker config");
    Arc::new(ScanConsoleHost::new(
        worker,
        "test-session".to_owned(),
        Arc::new(SystemClock),
        Arc::new(NoopSink),
    ))
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
