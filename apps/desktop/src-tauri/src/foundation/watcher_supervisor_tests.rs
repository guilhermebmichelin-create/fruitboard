//! Independent fault tests for the real watcher supervisor seam.
//!
//! These tests use the supervisor's `WatchFactory` and `WatchHandle` seams,
//! then drive `sync_roots`, `poll`, and `shutdown` with a fake clock. They do
//! not reimplement supervisor state or open native handles.

use super::watcher_supervisor::{WatchFactory, WatchHandle, WatcherSupervisor};
use fruitboard_filesystem_watcher::{
    Coalescer, CoalescerConfig, EndReason, RootId, StartError, WatchHint, WatchOutcome,
    WatcherConfig, WatcherPort,
};
use fruitboard_scan_execution::{FollowUpOutcome, ScanClock};
use fruitboard_storage::{Database, ScanRoot, StorageError};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const NOW_MS: i64 = 1_700_000_000_000;

struct Clock;

impl ScanClock for Clock {
    fn now_ms(&self) -> i64 {
        NOW_MS
    }
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "fruitboard-watcher-supervisor-tests-{}",
            uuid::Uuid::now_v7()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone)]
struct FakeHandleState {
    coalescer: Arc<Mutex<Coalescer>>,
    outcome: Arc<Mutex<Option<WatchOutcome>>>,
    stop_count: Arc<AtomicUsize>,
}

impl FakeHandleState {
    fn new(window: u64) -> Self {
        Self {
            coalescer: Arc::new(Mutex::new(
                Coalescer::new(CoalescerConfig {
                    window,
                    max_tracked_roots: 1,
                })
                .unwrap(),
            )),
            outcome: Arc::new(Mutex::new(None)),
            stop_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn record_activity(&self, root: RootId, generation: u64, now: u64) {
        self.coalescer
            .lock()
            .unwrap()
            .record_activity(root, generation, now);
    }

    fn end(&self, root: RootId, generation: u64) {
        *self.outcome.lock().unwrap() = Some(WatchOutcome {
            root,
            generation,
            reason: EndReason::WatchFailed { os_code: 0xDEAD },
        });
    }

    fn stop_count(&self) -> usize {
        self.stop_count.load(Ordering::SeqCst)
    }
}

struct FakeHandle {
    state: FakeHandleState,
}

impl WatcherPort for FakeHandle {
    fn poll_hints(&mut self, now: u64) -> Vec<WatchHint> {
        self.state.coalescer.lock().unwrap().poll_hints(now)
    }

    fn outcome(&mut self) -> Option<WatchOutcome> {
        *self.state.outcome.lock().unwrap()
    }
}

impl WatchHandle for FakeHandle {
    fn stop(&mut self) {
        self.state.stop_count.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Clone, Default)]
struct FakeFactory {
    starts: Arc<Mutex<Vec<(RootId, u64)>>>,
    handles: Arc<Mutex<Vec<FakeHandleState>>>,
    failures: Arc<Mutex<VecDeque<StartError>>>,
}

impl FakeFactory {
    fn fail_next(&self, error: StartError) {
        self.failures.lock().unwrap().push_back(error);
    }

    fn handles(&self) -> Vec<FakeHandleState> {
        self.handles.lock().unwrap().clone()
    }

    fn starts(&self) -> Vec<(RootId, u64)> {
        self.starts.lock().unwrap().clone()
    }
}

impl WatchFactory for FakeFactory {
    fn start(
        &self,
        root: RootId,
        _path: &Path,
        config: WatcherConfig,
    ) -> Result<Box<dyn WatchHandle>, StartError> {
        self.starts.lock().unwrap().push((root, config.generation));
        if let Some(error) = self.failures.lock().unwrap().pop_front() {
            return Err(error);
        }
        let state = FakeHandleState::new(config.window);
        self.handles.lock().unwrap().push(state.clone());
        Ok(Box::new(FakeHandle { state }))
    }
}

struct Harness {
    #[allow(dead_code)]
    directory: TestDirectory,
    database: Arc<Mutex<Database>>,
    root: ScanRoot,
    factory: FakeFactory,
    supervisor: WatcherSupervisor<FakeFactory>,
}

impl Harness {
    fn new() -> Self {
        let directory = TestDirectory::new();
        let mut database = Database::open(&directory.0).unwrap();
        let root = database
            .add_scan_root("Synthetic", r"C:\synthetic-watcher-supervisor")
            .unwrap();
        let database = Arc::new(Mutex::new(database));
        let factory = FakeFactory::default();
        let mut supervisor = WatcherSupervisor::new(factory.clone());
        supervisor.sync_roots(&[root.clone()], 0);
        Self {
            directory,
            database,
            root,
            factory,
            supervisor,
        }
    }

    fn deliver_pending(&mut self) -> Result<Option<FollowUpOutcome>, StorageError> {
        let database = self.database.clone();
        let root_id = self.root.id.clone();
        self.supervisor
            .deliver_pending_with(&root_id, move |adapter, hint| {
                let mut database = database.lock().unwrap();
                adapter.process_hints(&mut database, &[hint], &Clock)
            })
    }

    fn fail_delivery(&mut self) -> Result<Option<FollowUpOutcome>, StorageError> {
        let root_id = self.root.id.clone();
        self.supervisor
            .deliver_pending_with(&root_id, |_adapter, _hint| Err(StorageError::Io))
    }

    fn sync(&mut self, now_ns: u64) {
        let database = match self.database.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let roots = database.list_scan_roots().unwrap();
        self.supervisor.sync_roots(&roots, now_ns);
    }

    fn jobs(&self) -> usize {
        self.database
            .lock()
            .unwrap()
            .list_scan_jobs()
            .unwrap()
            .len()
    }
}

#[test]
fn failed_hint_delivery_retains_one_pending_request_until_retry() {
    let mut harness = Harness::new();
    assert!(harness.supervisor.pending_for(&harness.root.id).is_some());

    assert!(matches!(harness.fail_delivery(), Err(StorageError::Io)));
    assert!(harness.supervisor.pending_for(&harness.root.id).is_some());
    assert_eq!(harness.jobs(), 0);

    let outcome = harness.deliver_pending().unwrap().unwrap();
    assert_eq!(outcome.requests.len(), 1);
    assert!(harness.supervisor.pending_for(&harness.root.id).is_none());
    assert_eq!(harness.jobs(), 1);
}

#[test]
fn durable_disable_suppresses_pending_hint_before_configuration_catches_up() {
    let mut harness = Harness::new();
    harness
        .database
        .lock()
        .unwrap()
        .set_scan_root_enabled_at(&harness.root.id, false, NOW_MS)
        .unwrap();

    let outcome = harness.deliver_pending().unwrap().unwrap();
    assert_eq!(outcome.suppressed, 1);
    assert!(outcome.requests.is_empty());
    assert!(harness.supervisor.pending_for(&harness.root.id).is_none());
    assert_eq!(harness.jobs(), 0);

    harness
        .database
        .lock()
        .unwrap()
        .set_scan_root_enabled_at(&harness.root.id, true, NOW_MS + 1)
        .unwrap();
    harness.sync(1);
    let outcome = harness.deliver_pending().unwrap().unwrap();
    assert_eq!(outcome.requests.len(), 1);
    assert_eq!(harness.jobs(), 1);
}

#[test]
fn ended_watch_restarts_with_a_new_generation_and_reconciles() {
    let mut harness = Harness::new();
    let _ = harness.deliver_pending().unwrap();
    let first = harness.factory.handles().into_iter().next().unwrap();
    let watcher_root = harness
        .supervisor
        .watcher_root_for(&harness.root.id)
        .unwrap();
    first.end(watcher_root, 1);

    harness.supervisor.poll(&harness.database, &Clock, 0);
    assert!(!harness.supervisor.is_watching(&harness.root.id));
    assert_eq!(harness.supervisor.generation_for(&harness.root.id), Some(1));
    assert_eq!(first.stop_count(), 1);

    harness.sync(u64::MAX);
    assert!(harness.supervisor.is_watching(&harness.root.id));
    assert_eq!(harness.supervisor.generation_for(&harness.root.id), Some(2));
    let outcome = harness.deliver_pending().unwrap().unwrap();
    assert_eq!(outcome.overflow, 1);
    assert_eq!(outcome.requests.len(), 1);
}

#[test]
fn future_generation_replay_is_dropped_before_adapter_promotion() {
    let mut harness = Harness::new();
    let _ = harness.deliver_pending().unwrap();
    let current = harness.factory.handles().into_iter().next().unwrap();
    let watcher_root = harness
        .supervisor
        .watcher_root_for(&harness.root.id)
        .unwrap();
    current.record_activity(watcher_root, 99, 0);

    harness
        .supervisor
        .poll(&harness.database, &Clock, 1_000_000_000);
    assert!(harness.supervisor.pending_for(&harness.root.id).is_none());
    assert_eq!(harness.jobs(), 1);
}

#[test]
fn start_failures_back_off_and_consume_strictly_new_generations() {
    let mut harness = Harness::new();
    // The first start already happened during construction; force the next
    // two reconnect attempts to fail, then allow the third to start.
    harness
        .factory
        .fail_next(StartError::RootUnavailable { os_code: 2 });
    harness
        .factory
        .fail_next(StartError::ResourceUnavailable { os_code: 8 });
    let first = harness.factory.starts();
    assert_eq!(first.len(), 1);
    let first_generation = first[0].1;
    harness.supervisor.shutdown();
    // A fresh supervisor is needed to model reconnect attempts after the
    // explicit shutdown terminal; this keeps shutdown and retry state
    // independent in the deterministic test.
    let factory = harness.factory.clone();
    let mut supervisor = WatcherSupervisor::new(factory.clone());
    supervisor.sync_roots(&[harness.root.clone()], 0);
    supervisor.sync_roots(&[harness.root.clone()], u64::MAX);
    supervisor.sync_roots(&[harness.root.clone()], u64::MAX);
    let starts = factory.starts();
    assert!(
        starts
            .iter()
            .any(|(_, generation)| *generation > first_generation)
    );
    assert!(supervisor.is_watching(&harness.root.id));
    assert!(supervisor.pending_for(&harness.root.id).is_some());
}

#[test]
fn burst_activity_produces_one_durable_follow_up() {
    let mut harness = Harness::new();
    let _ = harness.deliver_pending().unwrap();
    let current = harness.factory.handles().into_iter().next().unwrap();
    let watcher_root = harness
        .supervisor
        .watcher_root_for(&harness.root.id)
        .unwrap();
    for _ in 0..10_000 {
        current.record_activity(watcher_root, 1, 0);
    }
    harness
        .supervisor
        .poll(&harness.database, &Clock, 1_000_000_000);
    assert_eq!(harness.jobs(), 1);
    harness
        .supervisor
        .poll(&harness.database, &Clock, 2_000_000_000);
    assert_eq!(harness.jobs(), 1);
}

#[test]
fn terminal_coverage_delivery_error_survives_end_and_reconnect() {
    let mut harness = Harness::new();
    let first = harness.factory.handles().into_iter().next().unwrap();
    let watcher_root = harness
        .supervisor
        .watcher_root_for(&harness.root.id)
        .unwrap();
    assert!(matches!(harness.fail_delivery(), Err(StorageError::Io)));
    first.end(watcher_root, 1);

    // Poisoning is a deterministic database delivery failure. The supervisor
    // must retain the pending terminal obligation while it fences the ended
    // generation and stops the handle.
    let poisoned = harness.database.clone();
    let _ = std::thread::spawn(move || {
        let _guard = poisoned.lock().unwrap();
        panic!("intentional database delivery barrier");
    })
    .join();
    harness.supervisor.poll(&harness.database, &Clock, 0);
    assert!(!harness.supervisor.is_watching(&harness.root.id));
    assert!(harness.supervisor.pending_for(&harness.root.id).is_some());

    // Reconnect attempts may all fail while the durable database is healthy;
    // the terminal coverage obligation must stay retained until a handle is
    // eventually live again.
    harness
        .factory
        .fail_next(StartError::RootUnavailable { os_code: 2 });
    harness
        .factory
        .fail_next(StartError::ResourceUnavailable { os_code: 8 });
    harness.sync(u64::MAX);
    assert!(!harness.supervisor.is_watching(&harness.root.id));
    assert!(harness.supervisor.pending_for(&harness.root.id).is_some());
    harness.sync(u64::MAX);
    assert!(!harness.supervisor.is_watching(&harness.root.id));
    assert!(harness.supervisor.pending_for(&harness.root.id).is_some());

    harness.sync(u64::MAX);
    let pending = harness.supervisor.pending_for(&harness.root.id).unwrap();
    assert_eq!(pending.generation, 4);
    let database = harness.database.clone();
    let outcome = harness
        .supervisor
        .deliver_pending_with(&harness.root.id, move |adapter, hint| {
            let mut database = match database.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            adapter.process_hints(&mut database, &[hint], &Clock)
        })
        .unwrap()
        .unwrap();
    assert_eq!(outcome.requests.len(), 1);
    assert!(harness.supervisor.pending_for(&harness.root.id).is_none());
}

#[test]
fn shutdown_stops_each_handle_once_and_drops_pending_work() {
    let mut harness = Harness::new();
    let first = harness.factory.handles().into_iter().next().unwrap();
    assert!(harness.supervisor.pending_for(&harness.root.id).is_some());

    harness.supervisor.shutdown();
    harness.supervisor.shutdown();
    assert_eq!(first.stop_count(), 1);
    assert!(!harness.supervisor.is_watching(&harness.root.id));
    assert!(harness.supervisor.pending_for(&harness.root.id).is_none());
}
