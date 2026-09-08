//! Independent fault tests for the real watcher supervisor seam.
//!
//! These tests use the supervisor's `WatchFactory` and `WatchHandle` seams,
//! then drive `sync_roots`, `poll`, and `shutdown` with a fake clock. They do
//! not reimplement supervisor state or open native handles.

use super::watcher_supervisor::WatchRootConfig;
use super::watcher_supervisor::{WatchFactory, WatchHandle, WatcherSupervisor};
use fruitboard_filesystem_watcher::{
    Coalescer, CoalescerConfig, EndReason, RootId, StartError, WatchHint, WatchOutcome,
    WatcherConfig, WatcherPort,
};
use fruitboard_scan_execution::{FollowUpOutcome, ScanClock, ScanWorker, WorkerConfig};
use fruitboard_storage::{Database, ScanJobState, ScanRoot, StorageError};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

const NOW_MS: i64 = 1_700_000_000_000;
const FIRST_RETRY_NS: u64 = 250_000_000;
const SECOND_RETRY_NS: u64 = 750_000_000;

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

    fn record_coverage_lost(&self, root: RootId, generation: u64) {
        self.coalescer
            .lock()
            .unwrap()
            .record_coverage_lost(root, generation);
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

struct DatabaseBlocker {
    release: Option<Sender<()>>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Drop for DatabaseBlocker {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        if let Some(join) = self.join.take() {
            join.join().unwrap();
        }
    }
}

impl Harness {
    fn new() -> Self {
        let directory = TestDirectory::new();
        let mut database = Database::open(&directory.0).unwrap();
        let root = database
            .add_scan_root("Synthetic", r"C:\synthetic-watcher-supervisor")
            .unwrap();
        let configuration_revision = database
            .scan_root_execution(&root.id)
            .unwrap()
            .configuration_revision;
        let database = Arc::new(Mutex::new(database));
        let factory = FakeFactory::default();
        let mut supervisor = WatcherSupervisor::new(factory.clone());
        supervisor.sync_roots(
            &[WatchRootConfig {
                root: root.clone(),
                configuration_revision,
            }],
            0,
        );
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
                let mut database = match database.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
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
        let configs = roots
            .iter()
            .map(|root| WatchRootConfig {
                root: root.clone(),
                configuration_revision: database
                    .scan_root_execution(&root.id)
                    .unwrap()
                    .configuration_revision,
            })
            .collect::<Vec<_>>();
        self.supervisor.sync_roots(&configs, now_ns);
    }

    fn jobs(&self) -> usize {
        match self.database.lock() {
            Ok(database) => database.list_scan_jobs().unwrap().len(),
            Err(poisoned) => poisoned.into_inner().list_scan_jobs().unwrap().len(),
        }
    }

    fn jobs_for(&self, root_id: &str) -> usize {
        match self.database.lock() {
            Ok(database) => database
                .list_scan_jobs()
                .unwrap()
                .into_iter()
                .filter(|job| job.scan_root_id == root_id)
                .count(),
            Err(poisoned) => poisoned
                .into_inner()
                .list_scan_jobs()
                .unwrap()
                .into_iter()
                .filter(|job| job.scan_root_id == root_id)
                .count(),
        }
    }

    fn set_enabled(&self, enabled: bool, now_ms: i64) {
        let mut database = match self.database.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        database
            .set_scan_root_enabled_at(&self.root.id, enabled, now_ms)
            .unwrap();
    }

    fn config(&self, root: &ScanRoot) -> WatchRootConfig {
        let database = match self.database.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        WatchRootConfig {
            root: root.clone(),
            configuration_revision: database
                .scan_root_execution(&root.id)
                .unwrap()
                .configuration_revision,
        }
    }

    fn poll(&mut self, now_ns: u64) {
        self.supervisor.poll(&self.database, &Clock, now_ns);
    }

    fn block_database(&self) -> DatabaseBlocker {
        let database = self.database.clone();
        let (ready_tx, ready_rx): (Sender<()>, Receiver<()>) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let join = std::thread::spawn(move || {
            let _guard = database.lock().unwrap();
            ready_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        ready_rx.recv().unwrap();
        DatabaseBlocker {
            release: Some(release_tx),
            join: Some(join),
        }
    }
}

#[test]
fn failed_hint_delivery_retains_one_pending_request_until_retry() {
    let mut harness = Harness::new();
    let handle = harness.factory.handles().into_iter().next().unwrap();
    let watcher_root = harness
        .supervisor
        .watcher_root_for(&harness.root.id)
        .unwrap();
    handle.record_coverage_lost(watcher_root, 1);
    let blocker = harness.block_database();
    harness.poll(0);
    drop(blocker);
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
    let handle = harness.factory.handles().into_iter().next().unwrap();
    let watcher_root = harness
        .supervisor
        .watcher_root_for(&harness.root.id)
        .unwrap();
    handle.record_coverage_lost(watcher_root, 1);
    harness.set_enabled(false, NOW_MS);

    let blocker = harness.block_database();
    harness.poll(0);
    drop(blocker);
    let outcome = harness.deliver_pending().unwrap().unwrap();
    assert_eq!(outcome.suppressed, 1);
    assert!(outcome.requests.is_empty());
    assert!(harness.supervisor.pending_for(&harness.root.id).is_none());
    assert_eq!(harness.jobs(), 0);

    harness.sync(1);
    harness.set_enabled(true, NOW_MS + 1);
    harness.sync(2);
    let fresh = harness.factory.handles().into_iter().last().unwrap();
    let fresh_watcher_root = harness
        .supervisor
        .watcher_root_for(&harness.root.id)
        .unwrap();
    fresh.record_coverage_lost(fresh_watcher_root, 2);
    harness.poll(u64::MAX);
    assert_eq!(harness.jobs_for(&harness.root.id), 1);
}

#[test]
fn ended_watch_restarts_with_a_new_generation_without_duplicate_queue() {
    let mut harness = Harness::new();
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
    assert_eq!(harness.jobs(), 1);

    harness.sync(u64::MAX);
    assert!(harness.supervisor.is_watching(&harness.root.id));
    assert_eq!(harness.supervisor.generation_for(&harness.root.id), Some(2));
    // The first end reconciliation owns the queued slot. Reconnect does not
    // create a second queue entry while that durable work is still queued.
    harness.poll(u64::MAX);
    assert_eq!(harness.jobs(), 1);
}

#[test]
fn future_generation_replay_is_dropped_before_adapter_promotion() {
    let mut harness = Harness::new();
    harness.poll(0);
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
    let config = harness.config(&harness.root);
    supervisor.sync_roots(std::slice::from_ref(&config), 0);
    assert_eq!(factory.starts().len(), 2);
    supervisor.sync_roots(std::slice::from_ref(&config), FIRST_RETRY_NS - 1);
    assert_eq!(factory.starts().len(), 2);
    supervisor.sync_roots(std::slice::from_ref(&config), FIRST_RETRY_NS);
    assert_eq!(factory.starts().len(), 3);
    supervisor.sync_roots(std::slice::from_ref(&config), SECOND_RETRY_NS - 1);
    assert_eq!(factory.starts().len(), 3);
    supervisor.sync_roots(std::slice::from_ref(&config), SECOND_RETRY_NS);
    let starts = factory.starts();
    assert_eq!(starts.len(), 4);
    assert!(
        starts
            .iter()
            .any(|(_, generation)| *generation > first_generation)
    );
    assert!(supervisor.is_watching(&harness.root.id));
    supervisor.poll(&harness.database, &Clock, u64::MAX);
    assert_eq!(harness.jobs(), 1);
}

#[test]
fn burst_activity_produces_one_durable_follow_up() {
    let mut harness = Harness::new();
    let worker = ScanWorker::new(WorkerConfig::default()).unwrap();
    let active = {
        let mut database = harness.database.lock().unwrap();
        let session = worker.start_session(&mut database, &Clock).unwrap();
        worker
            .request_manual_scan(&mut database, &harness.root.id, &Clock)
            .unwrap();
        worker
            .claim(&mut database, &session.id, &Clock)
            .unwrap()
            .unwrap()
    };
    let job_id = active.leased.run.scan_job_id.clone();
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
    let database = harness.database.lock().unwrap();
    let job = database.scan_job(&job_id).unwrap();
    assert_eq!(job.state, ScanJobState::Running);
    assert!(job.follow_up_requested);
    assert_eq!(database.list_scan_jobs().unwrap().len(), 1);
}

#[test]
fn coverage_loss_precedes_activity_in_the_real_supervisor_poll() {
    let mut harness = Harness::new();
    let current = harness.factory.handles().into_iter().next().unwrap();
    let watcher_root = harness
        .supervisor
        .watcher_root_for(&harness.root.id)
        .unwrap();
    current.record_activity(watcher_root, 1, 0);
    current.record_coverage_lost(watcher_root, 1);
    let blocker = harness.block_database();
    harness.poll(1_000_000_000);
    drop(blocker);

    let pending = harness.supervisor.pending_for(&harness.root.id).unwrap();
    assert_eq!(
        pending.kind,
        fruitboard_filesystem_watcher::HintKind::CoverageLost
    );
    let outcome = harness.deliver_pending().unwrap().unwrap();
    assert_eq!(outcome.overflow, 1);
    assert_eq!(harness.jobs(), 1);
}

#[test]
fn running_scan_receives_one_follow_up_without_publication() {
    let mut harness = Harness::new();
    let worker = ScanWorker::new(WorkerConfig::default()).unwrap();
    let active = {
        let mut database = harness.database.lock().unwrap();
        let session = worker.start_session(&mut database, &Clock).unwrap();
        worker
            .request_manual_scan(&mut database, &harness.root.id, &Clock)
            .unwrap();
        worker
            .claim(&mut database, &session.id, &Clock)
            .unwrap()
            .unwrap()
    };
    let job_id = active.leased.run.scan_job_id.clone();
    let current = harness.factory.handles().into_iter().next().unwrap();
    let watcher_root = harness
        .supervisor
        .watcher_root_for(&harness.root.id)
        .unwrap();
    current.record_coverage_lost(watcher_root, 1);
    harness.poll(0);

    let database = harness.database.lock().unwrap();
    let job = database.scan_job(&job_id).unwrap();
    assert_eq!(job.state, ScanJobState::Running);
    assert!(job.follow_up_requested);
    assert_eq!(database.list_scan_jobs().unwrap().len(), 1);
}

#[test]
fn startup_gap_does_not_revive_a_cancelled_scan_chain() {
    let mut harness = Harness::new();
    let worker = ScanWorker::new(WorkerConfig::default()).unwrap();
    let job_id = {
        let mut database = harness.database.lock().unwrap();
        worker.start_session(&mut database, &Clock).unwrap();
        worker
            .request_manual_scan(&mut database, &harness.root.id, &Clock)
            .unwrap()
            .job_id
    };
    harness
        .database
        .lock()
        .unwrap()
        .cancel_scan_job(&job_id, NOW_MS + 1)
        .unwrap();

    harness.poll(0);
    let database = harness.database.lock().unwrap();
    assert_eq!(database.list_scan_jobs().unwrap().len(), 1);
    assert_eq!(
        database.scan_job(&job_id).unwrap().state,
        ScanJobState::Cancelled
    );
    drop(database);
    assert!(harness.supervisor.pending_for(&harness.root.id).is_none());
}

#[test]
fn disable_reenable_remove_readd_uses_fresh_storage_identity_and_fences_old_hints() {
    let mut harness = Harness::new();
    harness.poll(0);
    let old_root = harness.root.clone();
    let old_watcher = harness.supervisor.watcher_root_for(&old_root.id).unwrap();

    let mut disabled = old_root.clone();
    disabled.enabled = false;
    harness
        .database
        .lock()
        .unwrap()
        .set_scan_root_enabled_at(&old_root.id, false, NOW_MS)
        .unwrap();
    let disabled_config = harness.config(&disabled);
    harness
        .supervisor
        .sync_roots(std::slice::from_ref(&disabled_config), 1);
    assert!(!harness.supervisor.is_watching(&old_root.id));

    harness
        .database
        .lock()
        .unwrap()
        .set_scan_root_enabled_at(&old_root.id, true, NOW_MS + 1)
        .unwrap();
    let enabled_config = harness.config(&old_root);
    harness
        .supervisor
        .sync_roots(std::slice::from_ref(&enabled_config), 2);
    assert_eq!(harness.supervisor.generation_for(&old_root.id), Some(2));
    assert!(harness.supervisor.is_watching(&old_root.id));

    harness
        .database
        .lock()
        .unwrap()
        .remove_scan_root_at(&old_root.id, NOW_MS + 2)
        .unwrap();
    harness.supervisor.sync_roots(&[], 3);
    assert!(harness.supervisor.watcher_root_for(&old_root.id).is_none());

    let replacement = harness
        .database
        .lock()
        .unwrap()
        .add_scan_root("Replacement", r"C:\synthetic-replacement")
        .unwrap();
    assert_ne!(replacement.id, old_root.id);
    let replacement_config = harness.config(&replacement);
    harness
        .supervisor
        .sync_roots(std::slice::from_ref(&replacement_config), 4);
    assert_eq!(
        harness.supervisor.watcher_root_for(&replacement.id),
        Some(old_watcher)
    );
    assert_eq!(harness.supervisor.generation_for(&replacement.id), Some(3));

    let current = harness.factory.handles().into_iter().last().unwrap();
    current.record_activity(old_watcher, 1, 0);
    current.record_coverage_lost(old_watcher, 1);
    harness
        .supervisor
        .poll(&harness.database, &Clock, 1_000_000_000);
    assert_eq!(harness.jobs_for(&replacement.id), 1);
}

#[test]
fn disable_then_reenable_between_polls_fences_the_old_generation_hint() {
    let mut harness = Harness::new();
    let current = harness.factory.handles().into_iter().next().unwrap();
    let watcher_root = harness
        .supervisor
        .watcher_root_for(&harness.root.id)
        .unwrap();
    current.record_coverage_lost(watcher_root, 1);

    // Both durable mutations happen before the native supervisor observes
    // either one. A bool-only configuration comparison would leave generation
    // one live and enqueue this terminal hint for a root that was disabled in
    // between; the host must fence the old configuration snapshot.
    harness.set_enabled(false, NOW_MS);
    harness.set_enabled(true, NOW_MS + 1);
    harness.poll(0);
    assert_eq!(harness.jobs(), 0);

    harness.sync(u64::MAX);
    assert_eq!(harness.supervisor.generation_for(&harness.root.id), Some(2));
    harness.poll(u64::MAX);
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
    first.record_coverage_lost(watcher_root, 1);
    first.end(watcher_root, 1);

    // Holding the shared database lock is a deterministic delivery failure. The supervisor
    // must retain the pending terminal obligation while it fences the ended
    // generation and stops the handle.
    let blocker = harness.block_database();
    harness.supervisor.poll(&harness.database, &Clock, 0);
    drop(blocker);
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
    let watcher_root = harness
        .supervisor
        .watcher_root_for(&harness.root.id)
        .unwrap();
    first.record_coverage_lost(watcher_root, 1);
    let blocker = harness.block_database();
    harness.poll(0);
    drop(blocker);
    assert!(harness.supervisor.pending_for(&harness.root.id).is_some());

    harness.supervisor.shutdown();
    harness.supervisor.shutdown();
    assert_eq!(first.stop_count(), 1);
    assert!(!harness.supervisor.is_watching(&harness.root.id));
    assert!(harness.supervisor.pending_for(&harness.root.id).is_none());
}
