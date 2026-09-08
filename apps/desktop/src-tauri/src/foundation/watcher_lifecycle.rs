//! Deterministic desktop integration harness for the next watcher host slice.
//!
//! Compiled only by feature-enabled tests. This composes the real coalescer,
//! follow-up adapter and durable queue, without opening native watch handles
//! or starting supervisor threads. Production activation still needs bounded
//! reconnects, shutdown/join, and root-command coordination. A retained hint
//! is one full-root reconciliation request, never file-presence authority.

use fruitboard_filesystem_watcher::{
    Coalescer, CoalescerConfig, HintKind, RootId, WatchHint, WatcherPort,
};
use fruitboard_scan_execution::{
    FollowUpOutcome, RootIdMapping, ScanClock, ScanWorker, WatcherFollowUpAdapter, WorkerConfig,
};
use fruitboard_storage::{Database, StorageError};
use std::path::PathBuf;

#[derive(Clone)]
struct RootMapping {
    watch_root: RootId,
    storage_root: String,
}

impl RootIdMapping for RootMapping {
    fn storage_root_id(&self, root: RootId) -> Option<String> {
        (root == self.watch_root).then(|| self.storage_root.clone())
    }
}

/// One configured root's test supervisor state. Keeping the adapter with its
/// root lets removal drop its mapping and generation history together.
struct WatchHarness {
    root: RootId,
    generation: u64,
    enabled: bool,
    port: Coalescer,
    adapter: WatcherFollowUpAdapter<RootMapping>,
    pending: Option<WatchHint>,
}

fn new_port() -> Coalescer {
    Coalescer::new(CoalescerConfig {
        window: 10,
        max_tracked_roots: 1,
    })
    .unwrap()
}

impl WatchHarness {
    fn new(root: RootId, storage_root: &str) -> Self {
        let mut adapter = WatcherFollowUpAdapter::new(RootMapping {
            watch_root: root,
            storage_root: storage_root.to_owned(),
        });
        adapter.watch_started(root, 1);
        Self {
            root,
            generation: 1,
            enabled: true,
            port: new_port(),
            adapter,
            pending: None,
        }
    }

    fn stop(&mut self) {
        self.enabled = false;
        self.pending = None;
        self.port = new_port();
        self.adapter.watch_ended(self.root);
    }

    fn restart(&mut self) {
        self.stop();
        self.generation = self.generation.checked_add(1).unwrap();
        self.adapter.watch_started(self.root, self.generation);
        self.enabled = true;
    }

    fn retain(&mut self, hint: WatchHint) {
        // Only the generation explicitly installed by the host is current.
        if !self.enabled || hint.root != self.root || hint.generation != self.generation {
            return;
        }
        if self.pending.is_none() || hint.kind == HintKind::CoverageLost {
            self.pending = Some(hint);
        }
    }

    fn pump(
        &mut self,
        now: u64,
        deliver: impl FnOnce(
            &mut WatcherFollowUpAdapter<RootMapping>,
            WatchHint,
        ) -> Result<FollowUpOutcome, StorageError>,
    ) -> Result<FollowUpOutcome, StorageError> {
        for hint in self.port.poll_hints(now) {
            self.retain(hint);
        }
        let Some(hint) = self.pending else {
            return Ok(FollowUpOutcome::default());
        };
        // A transient storage error keeps this single coalesced request for
        // the next poll, even though the port already drained its events.
        let outcome = deliver(&mut self.adapter, hint)?;
        self.pending = None;
        Ok(outcome)
    }

    fn flush(&mut self, now: u64, database: &mut Database) -> FollowUpOutcome {
        self.pump(now, |adapter, hint| {
            adapter.process_hints(database, &[hint], &Clock)
        })
        .unwrap()
    }
}

struct Clock;
impl ScanClock for Clock {
    fn now_ms(&self) -> i64 {
        1_700_000_000_000
    }
}

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "fruitboard-watcher-lifecycle-{}",
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

fn root(database: &mut Database) -> String {
    database
        .add_scan_root("Synthetic", r"C:\synthetic-watcher-root")
        .unwrap()
        .id
}

#[test]
fn burst_during_running_scan_sets_one_durable_follow_up() {
    let directory = TestDirectory::new();
    let mut database = Database::open(&directory.0).unwrap();
    let root = root(&mut database);
    let worker = ScanWorker::new(WorkerConfig::default()).unwrap();
    let session = worker.start_session(&mut database, &Clock).unwrap();
    worker
        .request_manual_scan(&mut database, &root, &Clock)
        .unwrap();
    let active = worker
        .claim(&mut database, &session.id, &Clock)
        .unwrap()
        .unwrap();
    let job = &active.leased.run.scan_job_id;
    assert!(!database.scan_job(job).unwrap().follow_up_requested);
    let mut watch = WatchHarness::new(RootId(1), &root);
    for _ in 0..10_000 {
        watch.port.record_activity(watch.root, watch.generation, 0);
    }
    let result = watch.flush(10, &mut database);
    assert_eq!(result.requests.len(), 1);
    assert!(result.requests[0].coalesced);
    assert_eq!(result.requests[0].job_id, *job);
    assert!(database.scan_job(job).unwrap().follow_up_requested);
    assert_eq!(database.list_scan_jobs().unwrap().len(), 1);
    assert!(watch.flush(20, &mut database).requests.is_empty());
}

#[test]
fn failed_delivery_retains_one_request_and_overflow_takes_precedence() {
    let directory = TestDirectory::new();
    let mut database = Database::open(&directory.0).unwrap();
    let root = root(&mut database);
    let mut watch = WatchHarness::new(RootId(1), &root);
    watch.port.record_activity(watch.root, watch.generation, 0);
    assert!(matches!(
        watch.pump(10, |_, _| Err(StorageError::Io)),
        Err(StorageError::Io)
    ));
    assert!(watch.pending.is_some());
    assert!(database.list_scan_jobs().unwrap().is_empty());
    for _ in 0..10_000 {
        watch
            .port
            .record_coverage_lost(watch.root, watch.generation);
    }
    assert!(watch.pump(20, |_, _| Err(StorageError::Io)).is_err());
    assert_eq!(watch.pending.unwrap().kind, HintKind::CoverageLost);
    let result = watch.flush(30, &mut database);
    assert_eq!(result.requests.len(), 1);
    assert_eq!(result.overflow, 1);
    assert!(watch.pending.is_none());
    assert_eq!(database.list_scan_jobs().unwrap().len(), 1);
    assert!(watch.flush(40, &mut database).requests.is_empty());
}

#[test]
fn disable_and_restart_fence_undelivered_old_and_unseeded_hints() {
    let directory = TestDirectory::new();
    let mut database = Database::open(&directory.0).unwrap();
    let root = root(&mut database);
    let mut watch = WatchHarness::new(RootId(1), &root);
    let old = WatchHint {
        root: watch.root,
        generation: 1,
        kind: HintKind::CoverageLost,
    };
    watch.retain(old);
    database
        .set_scan_root_enabled_at(&root, false, Clock.now_ms())
        .unwrap();
    watch.stop();
    watch.retain(old);
    assert!(watch.flush(10, &mut database).requests.is_empty());
    assert!(database.list_scan_jobs().unwrap().is_empty());
    database
        .set_scan_root_enabled_at(&root, true, Clock.now_ms())
        .unwrap();
    watch.restart();
    watch.retain(old);
    watch.retain(WatchHint {
        generation: 3,
        ..old
    });
    assert!(watch.pending.is_none());
    watch.retain(WatchHint {
        generation: 2,
        ..old
    });
    assert_eq!(watch.flush(20, &mut database).requests.len(), 1);
}

#[test]
fn removed_and_readded_root_cannot_receive_the_old_watch_request() {
    let directory = TestDirectory::new();
    let mut database = Database::open(&directory.0).unwrap();
    let old_root = root(&mut database);
    let mut watch = WatchHarness::new(RootId(1), &old_root);
    let old = WatchHint {
        root: watch.root,
        generation: 1,
        kind: HintKind::CoverageLost,
    };
    watch.retain(old);
    database
        .remove_scan_root_at(&old_root, Clock.now_ms())
        .unwrap();
    // Storage suppresses delivery even before the host observes removal.
    assert_eq!(watch.flush(10, &mut database).unknown_root_dropped, 1);
    drop(watch);
    let new_root = root(&mut database);
    assert_ne!(new_root, old_root);
    let mut replacement = WatchHarness::new(RootId(2), &new_root);
    replacement.retain(old);
    assert!(replacement.pending.is_none());
    replacement
        .port
        .record_coverage_lost(replacement.root, replacement.generation);
    let result = replacement.flush(20, &mut database);
    assert_eq!(result.requests.len(), 1);
    assert_eq!(result.requests[0].storage_root_id, new_root);
    assert_eq!(database.list_scan_jobs().unwrap().len(), 1);
}

#[test]
fn durable_disable_suppresses_a_hint_before_host_configuration_catches_up() {
    let directory = TestDirectory::new();
    let mut database = Database::open(&directory.0).unwrap();
    let root = root(&mut database);
    let mut watch = WatchHarness::new(RootId(1), &root);
    watch
        .port
        .record_coverage_lost(watch.root, watch.generation);
    database
        .set_scan_root_enabled_at(&root, false, Clock.now_ms())
        .unwrap();
    assert_eq!(watch.flush(10, &mut database).suppressed, 1);
    assert!(watch.pending.is_none());
    assert!(database.list_scan_jobs().unwrap().is_empty());
}
