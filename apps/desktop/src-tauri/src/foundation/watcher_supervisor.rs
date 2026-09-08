//! Native watcher ownership for the feature-gated scan console.
//!
//! The supervisor is deliberately a separate loop from the scan worker. A
//! filesystem traversal can take a long time, and watcher hints must still be
//! drained while that traversal is running so a running scan receives a
//! durable follow-up request. The supervisor owns native paths and handles;
//! only opaque watcher ids, generations, and durable root ids reach the
//! follow-up adapter.

use fruitboard_filesystem_watcher::{
    HandleBoundWatcher, HintKind, RootId, StartError, WatchHint, WatcherConfig, WatcherPort,
    monotonic_nanos,
};
use fruitboard_scan_execution::{
    FollowUpOutcome, RootIdMapping, ScanClock, WatcherFollowUpAdapter,
};
use fruitboard_storage::{Database, ScanJobState, ScanRoot, StorageError};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Duration;

/// The native supervisor wakes often enough to observe ended handles quickly,
/// while the watcher coalescer still bounds the durable follow-up rate.
pub(crate) const SUPERVISOR_POLL_INTERVAL: Duration = Duration::from_millis(100);

const WATCH_WINDOW_NS: u64 = 1_000_000_000;
const RETRY_BASE_NS: u64 = 250_000_000;
const RETRY_MAX_NS: u64 = 5_000_000_000;
const WATCH_RAW_QUEUE_BOUND: usize = 256;
const WATCH_NOTIFY_BUFFER_BYTES: usize = 64 * 1024;
const WATCH_VALIDITY_POLL_MS: u32 = 500;

/// A handle that can be polled by the supervisor and explicitly stopped by
/// it. The explicit stop operation is part of the lifecycle contract: the
/// native worker must be joined before the OS handle is released.
pub(crate) trait WatchHandle: WatcherPort + Send {
    fn stop(&mut self);
}

impl WatchHandle for HandleBoundWatcher {
    fn stop(&mut self) {
        let _ = HandleBoundWatcher::stop(self);
    }
}

/// Factory seam used by production and deterministic supervisor tests.
pub(crate) trait WatchFactory: Send + Sync + 'static {
    fn start(
        &self,
        root: RootId,
        path: &Path,
        config: WatcherConfig,
    ) -> Result<Box<dyn WatchHandle>, StartError>;
}

/// Production factory. Root paths are consumed only at this native boundary.
pub(crate) struct NativeWatchFactory;

impl WatchFactory for NativeWatchFactory {
    fn start(
        &self,
        root: RootId,
        path: &Path,
        config: WatcherConfig,
    ) -> Result<Box<dyn WatchHandle>, StartError> {
        Ok(Box::new(HandleBoundWatcher::start(
            root,
            path.as_os_str(),
            config,
        )?))
    }
}

/// One durable root snapshot plus the configuration revision read from the
/// same database snapshot. The revision fence catches a disable/re-enable
/// pair that commits between two supervisor polls while `enabled` ends up
/// true again.
#[derive(Clone)]
pub(crate) struct WatchRootConfig {
    pub(crate) root: ScanRoot,
    pub(crate) configuration_revision: i64,
}

#[derive(Clone)]
pub(crate) struct RootMapping {
    roots: Arc<Mutex<BTreeMap<RootId, String>>>,
}

impl RootIdMapping for RootMapping {
    fn storage_root_id(&self, root: RootId) -> Option<String> {
        self.roots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&root)
            .cloned()
    }
}

struct ManagedRoot {
    storage_root_id: String,
    path: PathBuf,
    watcher_root: RootId,
    configuration_revision: i64,
    /// The last attempted/current generation. A failed start still consumes
    /// a generation so every later attempt is strictly newer.
    generation: u64,
    enabled: bool,
    watcher: Option<Box<dyn WatchHandle>>,
    pending: Option<WatchHint>,
    /// True when the pending coverage hint is a supervisor-created startup or
    /// reconnect gap. Native watcher observations are never suppressed by
    /// terminal-job recovery policy.
    pending_is_gap: bool,
    retry_at_ns: u64,
    retry_attempt: u32,
    /// A watch restart leaves a coverage gap. The flag is converted to a
    /// fresh-generation coverage hint only after a new handle is live.
    gap_reconciliation: bool,
}

/// One bounded native watcher supervisor. It is normally owned by the
/// supervisor thread; the methods are also exposed to the feature-gated test
/// module so the exact lifecycle can be driven without sleeps.
pub(crate) struct WatcherSupervisor<F: WatchFactory> {
    factory: F,
    mapping: RootMapping,
    adapter: WatcherFollowUpAdapter<RootMapping>,
    roots: BTreeMap<String, ManagedRoot>,
    next_watcher_root: u64,
    free_watcher_roots: BTreeSet<RootId>,
    retired_generations: BTreeMap<RootId, u64>,
}

impl<F: WatchFactory> WatcherSupervisor<F> {
    pub(crate) fn new(factory: F) -> Self {
        let mapping = RootMapping {
            roots: Arc::new(Mutex::new(BTreeMap::new())),
        };
        Self {
            factory,
            adapter: WatcherFollowUpAdapter::new(mapping.clone()),
            mapping,
            roots: BTreeMap::new(),
            next_watcher_root: 1,
            free_watcher_roots: BTreeSet::new(),
            retired_generations: BTreeMap::new(),
        }
    }

    /// Reconcile the native configuration against the durable root list.
    /// Database access is intentionally outside this method so callers can
    /// drop the database mutex before opening/stopping a native handle.
    pub(crate) fn sync_roots(&mut self, configured: &[WatchRootConfig], now_ns: u64) {
        let desired: BTreeSet<String> = configured
            .iter()
            .map(|config| config.root.id.clone())
            .collect();
        let removed: Vec<String> = self
            .roots
            .keys()
            .filter(|id| !desired.contains(*id))
            .cloned()
            .collect();
        for id in removed {
            self.remove_root(&id);
        }

        for config in configured {
            let root = &config.root;
            if !self.roots.contains_key(&root.id) {
                let watcher_root = self.allocate_watcher_root();
                self.mapping_insert(watcher_root, root.id.clone());
                let generation = self
                    .retired_generations
                    .get(&watcher_root)
                    .copied()
                    .unwrap_or_default();
                self.roots.insert(
                    root.id.clone(),
                    ManagedRoot {
                        storage_root_id: root.id.clone(),
                        path: PathBuf::from(&root.canonical_path),
                        watcher_root,
                        configuration_revision: config.configuration_revision,
                        generation,
                        enabled: root.enabled,
                        watcher: None,
                        pending: None,
                        pending_is_gap: false,
                        retry_at_ns: 0,
                        retry_attempt: 0,
                        gap_reconciliation: root.enabled,
                    },
                );
            }

            let mut should_start = false;
            if let Some(state) = self.roots.get_mut(&root.id) {
                state.path = PathBuf::from(&root.canonical_path);
                let revision_changed =
                    state.configuration_revision != config.configuration_revision;
                if revision_changed {
                    // A disable/re-enable pair may leave `enabled` true by
                    // the time this poll observes it. Fence the old handle
                    // before attempting the new configuration so delayed
                    // native hints cannot cross that durable revision gap.
                    if state.watcher.is_some() {
                        Self::stop_state(state);
                        self.adapter.watch_ended(state.watcher_root);
                    }
                    // A failed or ended old watch can still retain a hint;
                    // its durable configuration revision is obsolete even
                    // when there is no live handle left to stop.
                    state.pending = None;
                    state.pending_is_gap = false;
                }
                if revision_changed || state.enabled != root.enabled {
                    state.configuration_revision = config.configuration_revision;
                    if !root.enabled {
                        Self::stop_state(state);
                        state.pending = None;
                        state.pending_is_gap = false;
                        state.gap_reconciliation = false;
                        self.adapter.watch_ended(state.watcher_root);
                    } else {
                        state.gap_reconciliation = true;
                        state.retry_at_ns = now_ns;
                        state.retry_attempt = 0;
                        should_start = true;
                    }
                    state.enabled = root.enabled;
                } else if root.enabled && state.watcher.is_none() {
                    should_start = now_ns >= state.retry_at_ns;
                }
            }
            if should_start {
                self.try_start(&root.id, now_ns);
            }
        }
    }

    /// Drain all live watches and deliver at most one hint per root per pass.
    /// Each database delivery is a short transaction and occurs independently
    /// from every other root, so a failed delivery cannot duplicate earlier
    /// successes from the same batch.
    pub(crate) fn poll(&mut self, database: &Mutex<Database>, clock: &dyn ScanClock, now_ns: u64) {
        let roots: Vec<String> = self.roots.keys().cloned().collect();
        for root_id in roots {
            self.poll_root(&root_id, database, clock, now_ns);
        }
    }

    /// Stop every live watch, fence every adapter generation, and drop all
    /// retained hints. `WatchHandle::stop` joins before each handle is
    /// released; calling shutdown twice is harmless.
    pub(crate) fn shutdown(&mut self) {
        let ids: Vec<String> = self.roots.keys().cloned().collect();
        for id in ids {
            if let Some(state) = self.roots.get_mut(&id) {
                Self::stop_state(state);
                state.pending = None;
                state.pending_is_gap = false;
                self.adapter.watch_ended(state.watcher_root);
            }
        }
    }

    /// Test seam for a single-root delivery. Production uses `poll`, while
    /// fault-injection tests can return a transient storage error and verify
    /// that the retained coalesced hint survives for the next attempt.
    pub(crate) fn deliver_pending_with(
        &mut self,
        root_id: &str,
        deliver: impl FnOnce(
            &mut WatcherFollowUpAdapter<RootMapping>,
            WatchHint,
        ) -> Result<FollowUpOutcome, StorageError>,
    ) -> Result<Option<FollowUpOutcome>, StorageError> {
        let Some(state) = self.roots.get(root_id) else {
            return Ok(None);
        };
        let Some(hint) = state.pending else {
            return Ok(None);
        };
        if state.watcher.is_none() || hint.generation != state.generation {
            return Ok(None);
        }
        let outcome = deliver(&mut self.adapter, hint)?;
        if let Some(state) = self.roots.get_mut(root_id)
            && state.pending == Some(hint)
        {
            state.pending = None;
            state.pending_is_gap = false;
        }
        Ok(Some(outcome))
    }

    #[allow(dead_code)]
    pub(crate) fn watcher_root_for(&self, storage_root_id: &str) -> Option<RootId> {
        self.roots
            .get(storage_root_id)
            .map(|root| root.watcher_root)
    }

    #[allow(dead_code)]
    pub(crate) fn generation_for(&self, storage_root_id: &str) -> Option<u64> {
        self.roots.get(storage_root_id).map(|root| root.generation)
    }

    #[allow(dead_code)]
    pub(crate) fn pending_for(&self, storage_root_id: &str) -> Option<WatchHint> {
        self.roots
            .get(storage_root_id)
            .and_then(|root| root.pending)
    }

    #[allow(dead_code)]
    pub(crate) fn is_watching(&self, storage_root_id: &str) -> bool {
        self.roots
            .get(storage_root_id)
            .is_some_and(|root| root.watcher.is_some())
    }

    fn allocate_watcher_root(&mut self) -> RootId {
        if let Some(root) = self.free_watcher_roots.pop_first() {
            return root;
        }
        let root = RootId(self.next_watcher_root);
        self.next_watcher_root = self.next_watcher_root.saturating_add(1).max(1);
        root
    }

    fn mapping_insert(&self, watcher_root: RootId, storage_root_id: String) {
        self.mapping
            .roots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(watcher_root, storage_root_id);
    }

    fn mapping_remove(&self, watcher_root: RootId) {
        self.mapping
            .roots
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&watcher_root);
    }

    fn remove_root(&mut self, storage_root_id: &str) {
        if let Some(mut state) = self.roots.remove(storage_root_id) {
            Self::stop_state(&mut state);
            self.adapter.watch_ended(state.watcher_root);
            self.mapping_remove(state.watcher_root);
            self.retired_generations
                .insert(state.watcher_root, state.generation);
            self.free_watcher_roots.insert(state.watcher_root);
        }
    }

    fn stop_state(state: &mut ManagedRoot) {
        if let Some(mut watcher) = state.watcher.take() {
            watcher.stop();
        }
    }

    fn try_start(&mut self, storage_root_id: &str, now_ns: u64) {
        let Some(state) = self.roots.get_mut(storage_root_id) else {
            return;
        };
        if !state.enabled || state.watcher.is_some() || now_ns < state.retry_at_ns {
            return;
        }
        let Some(generation) = state.generation.checked_add(1) else {
            state.retry_at_ns = u64::MAX;
            return;
        };
        state.generation = generation;
        let config = WatcherConfig {
            generation,
            window: WATCH_WINDOW_NS,
            raw_queue_bound: WATCH_RAW_QUEUE_BOUND,
            notify_buffer_bytes: WATCH_NOTIFY_BUFFER_BYTES,
            validity_poll_ms: WATCH_VALIDITY_POLL_MS,
        };
        match self.factory.start(state.watcher_root, &state.path, config) {
            Ok(watcher) => {
                state.watcher = Some(watcher);
                state.retry_at_ns = 0;
                state.retry_attempt = 0;
                self.adapter.watch_started(state.watcher_root, generation);
                if state.pending.is_some() {
                    state.pending = Some(WatchHint {
                        root: state.watcher_root,
                        generation,
                        kind: HintKind::CoverageLost,
                    });
                }
            }
            Err(_) => {
                // A failed attempt consumed this generation. The adapter is
                // never seeded for an unstarted handle, and a later retry is
                // assigned another strictly newer generation.
                state.retry_attempt = state.retry_attempt.saturating_add(1);
                state.retry_at_ns = now_ns.saturating_add(retry_delay(state.retry_attempt));
                state.gap_reconciliation = true;
            }
        }
    }

    fn poll_root(
        &mut self,
        storage_root_id: &str,
        database: &Mutex<Database>,
        clock: &dyn ScanClock,
        now_ns: u64,
    ) {
        self.prepare_gap(storage_root_id, database);
        let (hints, ended) = {
            let Some(state) = self.roots.get_mut(storage_root_id) else {
                return;
            };
            let Some(watcher) = state.watcher.as_mut() else {
                return;
            };
            let hints = watcher.poll_hints(now_ns);
            let ended = watcher.outcome();
            (hints, ended)
        };

        for hint in hints {
            self.retain_hint(storage_root_id, hint);
        }
        if ended.is_some() {
            let watcher_root = self
                .roots
                .get(storage_root_id)
                .map(|state| state.watcher_root);
            if let Some(watcher_root) = watcher_root {
                // HandleBoundWatcher surfaces this as a coverage hint too;
                // synthesize it as a backstop for a port that reports an end
                // without a due coalescer signal.
                self.retain_hint(
                    storage_root_id,
                    WatchHint {
                        root: watcher_root,
                        generation: self
                            .roots
                            .get(storage_root_id)
                            .map(|state| state.generation)
                            .unwrap_or_default(),
                        kind: HintKind::CoverageLost,
                    },
                );
            }
            self.gate_coverage_pending(storage_root_id, database);
        }

        // Deliver while the old generation is still active. If this fails,
        // finish_watch fences the old adapter generation and the retained
        // obligation is translated to the fresh generation on restart.
        let _ = self.deliver_pending_from_database(storage_root_id, database, clock);

        if ended.is_some() {
            self.finish_watch(storage_root_id, now_ns);
        }
    }

    fn retain_hint(&mut self, storage_root_id: &str, hint: WatchHint) {
        let Some(state) = self.roots.get_mut(storage_root_id) else {
            return;
        };
        // The supervisor's fence is exact. In particular, a newer replayed
        // hint cannot promote the adapter behind the supervisor's back.
        if state.watcher.is_none()
            || hint.root != state.watcher_root
            || hint.generation != state.generation
        {
            return;
        }
        if state.pending.is_none() || hint.kind == HintKind::CoverageLost {
            state.pending = Some(hint);
            state.pending_is_gap = false;
        }
    }

    fn deliver_pending_from_database(
        &mut self,
        storage_root_id: &str,
        database: &Mutex<Database>,
        clock: &dyn ScanClock,
    ) -> Result<Option<FollowUpOutcome>, StorageError> {
        let expected_revision = self
            .roots
            .get(storage_root_id)
            .map(|state| state.configuration_revision);
        self.deliver_pending_with(storage_root_id, |adapter, hint| {
            let mut db = match database.try_lock() {
                Ok(db) => db,
                Err(std::sync::TryLockError::Poisoned(_)) => return Err(StorageError::Io),
                Err(std::sync::TryLockError::WouldBlock) => return Err(StorageError::Busy),
            };
            let Some(expected_revision) = expected_revision else {
                return Ok(FollowUpOutcome {
                    unknown_root_dropped: 1,
                    ..FollowUpOutcome::default()
                });
            };
            let execution = match db.scan_root_execution(storage_root_id) {
                Ok(execution) => execution,
                Err(StorageError::NotFound) => {
                    return Ok(FollowUpOutcome {
                        unknown_root_dropped: 1,
                        ..FollowUpOutcome::default()
                    });
                }
                Err(error) => return Err(error),
            };
            if !execution.enabled || execution.configuration_revision != expected_revision {
                return Ok(FollowUpOutcome {
                    stale_generation_dropped: 1,
                    ..FollowUpOutcome::default()
                });
            }
            adapter.process_hints(&mut db, &[hint], clock)
        })
    }

    fn prepare_gap(&mut self, storage_root_id: &str, database: &Mutex<Database>) {
        let Some(state) = self.roots.get(storage_root_id) else {
            return;
        };
        if !state.enabled || state.watcher.is_none() || !state.gap_reconciliation {
            return;
        }
        let allowed = match database.try_lock() {
            Ok(db) => gap_reconciliation_allowed(&db, &state.storage_root_id),
            Err(std::sync::TryLockError::Poisoned(_)) => Err(StorageError::Io),
            Err(std::sync::TryLockError::WouldBlock) => Err(StorageError::Busy),
        };
        let Ok(allowed) = allowed else {
            return;
        };
        let Some(state) = self.roots.get_mut(storage_root_id) else {
            return;
        };
        state.gap_reconciliation = false;
        if !allowed && state.pending_is_gap {
            state.pending = None;
            state.pending_is_gap = false;
        } else if allowed && state.pending.is_none() {
            state.pending = Some(WatchHint {
                root: state.watcher_root,
                generation: state.generation,
                kind: HintKind::CoverageLost,
            });
            state.pending_is_gap = true;
        }
    }

    fn gate_coverage_pending(&mut self, storage_root_id: &str, database: &Mutex<Database>) {
        let Some(state) = self.roots.get(storage_root_id) else {
            return;
        };
        if state
            .pending
            .is_none_or(|hint| hint.kind != HintKind::CoverageLost)
            || !state.pending_is_gap
        {
            return;
        }
        let allowed = match database.try_lock() {
            Ok(db) => gap_reconciliation_allowed(&db, &state.storage_root_id),
            Err(std::sync::TryLockError::Poisoned(_)) => Err(StorageError::Io),
            Err(std::sync::TryLockError::WouldBlock) => Err(StorageError::Busy),
        };
        let Ok(allowed) = allowed else {
            return;
        };
        if !allowed && let Some(state) = self.roots.get_mut(storage_root_id) {
            state.pending = None;
            state.pending_is_gap = false;
        }
    }

    fn finish_watch(&mut self, storage_root_id: &str, now_ns: u64) {
        let Some(state) = self.roots.get_mut(storage_root_id) else {
            return;
        };
        Self::stop_state(state);
        self.adapter.watch_ended(state.watcher_root);
        state.gap_reconciliation = true;
        state.retry_attempt = state.retry_attempt.saturating_add(1);
        state.retry_at_ns = now_ns.saturating_add(retry_delay(state.retry_attempt));
        // The retained hint is intentionally left in place. It is old
        // generation state and cannot be replayed through the adapter; a
        // successful restart rewrites it to the new generation.
    }
}

fn retry_delay(attempt: u32) -> u64 {
    let shift = attempt.saturating_sub(1).min(5);
    RETRY_BASE_NS
        .saturating_mul(1u64 << shift)
        .min(RETRY_MAX_NS)
}

/// Start the native supervisor loop. It has its own wake channel and thread
/// so a scan traversal never prevents watcher polling or handle shutdown.
pub(crate) fn run_native_supervisor(
    database: Arc<Mutex<Database>>,
    clock: Arc<dyn ScanClock + Send + Sync>,
    receiver: mpsc::Receiver<SupervisorSignal>,
    shutdown_requested: Arc<std::sync::atomic::AtomicBool>,
) {
    let mut supervisor = WatcherSupervisor::new(NativeWatchFactory);
    while !shutdown_requested.load(std::sync::atomic::Ordering::Acquire) {
        let now_ns = monotonic_nanos();
        let roots_result: Result<Vec<WatchRootConfig>, StorageError> = match database.try_lock() {
            Ok(db) => db.list_scan_roots().and_then(|roots| {
                roots
                    .into_iter()
                    .map(|root| {
                        let configuration_revision =
                            db.scan_root_execution(&root.id)?.configuration_revision;
                        Ok(WatchRootConfig {
                            root,
                            configuration_revision,
                        })
                    })
                    .collect()
            }),
            Err(std::sync::TryLockError::Poisoned(_)) => Err(StorageError::Io),
            Err(std::sync::TryLockError::WouldBlock) => Err(StorageError::Busy),
        };
        if shutdown_requested.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        // A transient read failure leaves the last known configuration in
        // place. Continue polling those handles so native hints are drained
        // and ended watches are fenced; delivery will retain any pending hint
        // until a short database transaction succeeds.
        if let Ok(roots) = roots_result {
            supervisor.sync_roots(&roots, now_ns);
        }
        if shutdown_requested.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        supervisor.poll(&database, clock.as_ref(), now_ns);
        match receiver.recv_timeout(SUPERVISOR_POLL_INTERVAL) {
            Ok(SupervisorSignal::Shutdown) => break,
            Ok(SupervisorSignal::Wake) | Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    supervisor.shutdown();
}

fn gap_reconciliation_allowed(
    database: &Database,
    storage_root_id: &str,
) -> Result<bool, StorageError> {
    let latest = database
        .list_scan_jobs()?
        .into_iter()
        .filter(|job| job.scan_root_id == storage_root_id)
        .max_by(|left, right| {
            left.created_at_ms
                .cmp(&right.created_at_ms)
                .then_with(|| left.id.cmp(&right.id))
        });
    Ok(match latest {
        None => true,
        Some(job) => match job.state {
            // A startup/reconnect gap is still relevant when an existing
            // queued or running scan began before the new watch was armed;
            // enqueue_scan coalesces this onto that work (and marks a running
            // attempt for a follow-up). An interrupted attempt likewise needs
            // an explicit fresh trigger after session recovery, unless its
            // durable cancellation flag says the chain was intentionally
            // stopped.
            ScanJobState::Queued | ScanJobState::Running => true,
            ScanJobState::Interrupted => !job.cancellation_requested,
            ScanJobState::Cancelled => false,
            ScanJobState::Failed => job.attempt < job.max_attempts,
            ScanJobState::Completed => true,
        },
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SupervisorSignal {
    Wake,
    Shutdown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use fruitboard_filesystem_watcher::{Coalescer, CoalescerConfig, WatchOutcome};
    use fruitboard_storage::{Database, ScanRootAvailability};
    use std::sync::Arc;

    struct FakeWatchState {
        port: Coalescer,
        ended: Option<WatchOutcome>,
        stopped: bool,
    }

    struct FakeHandle {
        state: Arc<Mutex<FakeWatchState>>,
    }

    impl WatcherPort for FakeHandle {
        fn poll_hints(&mut self, now: u64) -> Vec<WatchHint> {
            self.state.lock().unwrap().port.poll_hints(now)
        }

        fn outcome(&mut self) -> Option<WatchOutcome> {
            self.state.lock().unwrap().ended
        }
    }

    impl WatchHandle for FakeHandle {
        fn stop(&mut self) {
            self.state.lock().unwrap().stopped = true;
        }
    }

    #[derive(Clone, Default)]
    struct FakeFactory {
        starts: Arc<Mutex<Vec<(RootId, u64)>>>,
        states: Arc<Mutex<Vec<Arc<Mutex<FakeWatchState>>>>>,
    }

    impl WatchFactory for FakeFactory {
        fn start(
            &self,
            root: RootId,
            _path: &Path,
            config: WatcherConfig,
        ) -> Result<Box<dyn WatchHandle>, StartError> {
            self.starts.lock().unwrap().push((root, config.generation));
            let state = Arc::new(Mutex::new(FakeWatchState {
                port: Coalescer::new(CoalescerConfig::default()).unwrap(),
                ended: None,
                stopped: false,
            }));
            self.states.lock().unwrap().push(state.clone());
            Ok(Box::new(FakeHandle { state }))
        }
    }

    struct Clock;
    impl ScanClock for Clock {
        fn now_ms(&self) -> i64 {
            1_700_000_000_000
        }
    }

    fn configured_root(id: &str, enabled: bool) -> ScanRoot {
        ScanRoot {
            id: id.to_owned(),
            display_name: id.to_owned(),
            canonical_path: format!(r"C:\synthetic-{id}"),
            enabled,
            availability: ScanRootAvailability::Available,
            last_error_code: None,
        }
    }

    fn watch_config(root: &ScanRoot, configuration_revision: i64) -> WatchRootConfig {
        WatchRootConfig {
            root: root.clone(),
            configuration_revision,
        }
    }

    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "fruitboard-watcher-supervisor-{}",
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

    #[test]
    fn retry_delay_is_bounded() {
        assert_eq!(retry_delay(1), RETRY_BASE_NS);
        assert_eq!(retry_delay(100), RETRY_MAX_NS);
    }

    #[test]
    fn startup_disable_reenable_and_remove_readd_fence_identity() {
        let factory = FakeFactory::default();
        let mut supervisor = WatcherSupervisor::new(factory);
        let root = configured_root("root-a", true);
        supervisor.sync_roots(&[watch_config(&root, 1)], 0);
        let watcher_root = supervisor.watcher_root_for(&root.id).unwrap();
        assert_eq!(supervisor.generation_for(&root.id), Some(1));
        assert!(supervisor.is_watching(&root.id));

        let mut disabled = root.clone();
        disabled.enabled = false;
        supervisor.sync_roots(&[watch_config(&disabled, 2)], 1);
        assert!(!supervisor.is_watching(&root.id));
        supervisor.sync_roots(&[watch_config(&root, 3)], 2);
        assert_eq!(supervisor.generation_for(&root.id), Some(2));
        assert_eq!(supervisor.watcher_root_for(&root.id), Some(watcher_root));

        supervisor.sync_roots(&[], 3);
        let replacement = configured_root("root-b", true);
        supervisor.sync_roots(&[watch_config(&replacement, 1)], 4);
        assert_eq!(supervisor.watcher_root_for("root-b"), Some(watcher_root));
        assert_eq!(supervisor.generation_for("root-b"), Some(3));
    }

    #[test]
    fn enabled_revision_change_restarts_and_fences_the_old_watch() {
        let factory = FakeFactory::default();
        let states = factory.states.clone();
        let mut supervisor = WatcherSupervisor::new(factory);
        let root = configured_root("root-a", true);
        supervisor.sync_roots(&[watch_config(&root, 1)], 0);
        supervisor.sync_roots(&[watch_config(&root, 2)], 1);

        assert_eq!(supervisor.generation_for(&root.id), Some(2));
        assert!(supervisor.is_watching(&root.id));
        let states = states.lock().unwrap();
        assert_eq!(states.len(), 2);
        assert!(states[0].lock().unwrap().stopped);
        assert!(!states[1].lock().unwrap().stopped);
    }

    #[test]
    fn startup_gap_is_one_durable_follow_up_and_delivery_failure_is_retained() {
        let directory = TestDirectory::new();
        let database = Arc::new(Mutex::new(Database::open(&directory.0).unwrap()));
        let root = database
            .lock()
            .unwrap()
            .add_scan_root("Synthetic", r"C:\synthetic-root")
            .unwrap();
        let factory = FakeFactory::default();
        let mut supervisor = WatcherSupervisor::new(factory);
        supervisor.sync_roots(&[watch_config(&root, 0)], 0);
        supervisor.poll(&database, &Clock, 0);
        assert_eq!(database.lock().unwrap().list_scan_jobs().unwrap().len(), 1);
        assert!(supervisor.pending_for(&root.id).is_none());

        let watcher_root = supervisor.watcher_root_for(&root.id).unwrap();
        let generation = supervisor.generation_for(&root.id).unwrap();
        supervisor.roots.get_mut(&root.id).unwrap().pending = Some(WatchHint {
            root: watcher_root,
            generation,
            kind: HintKind::ReconciliationRequested,
        });
        let held = database.lock().unwrap();
        assert!(matches!(
            supervisor.deliver_pending_from_database(&root.id, &database, &Clock),
            Err(StorageError::Busy)
        ));
        assert!(supervisor.pending_for(&root.id).is_some());
        drop(held);
        assert!(
            supervisor
                .deliver_pending_from_database(&root.id, &database, &Clock)
                .unwrap()
                .is_some()
        );
        assert!(supervisor.pending_for(&root.id).is_none());
    }

    #[test]
    fn native_coverage_loss_precedes_activity_and_restart_uses_fresh_generation() {
        let directory = TestDirectory::new();
        let database = Arc::new(Mutex::new(Database::open(&directory.0).unwrap()));
        let root = database
            .lock()
            .unwrap()
            .add_scan_root("Synthetic", r"C:\synthetic-root")
            .unwrap();
        let factory = FakeFactory::default();
        let states = factory.states.clone();
        let mut supervisor = WatcherSupervisor::new(factory);
        supervisor.sync_roots(&[watch_config(&root, 0)], 0);
        supervisor.poll(&database, &Clock, 0);
        let watcher_root = supervisor.watcher_root_for(&root.id).unwrap();
        let generation = supervisor.generation_for(&root.id).unwrap();
        let state = states.lock().unwrap()[0].clone();
        {
            let mut state = state.lock().unwrap();
            state.port.record_activity(watcher_root, generation, 1);
            state.port.record_coverage_lost(watcher_root, generation);
            state.ended = Some(WatchOutcome {
                root: watcher_root,
                generation,
                reason: fruitboard_filesystem_watcher::EndReason::WatchFailed { os_code: 7 },
            });
        }
        let held = database.lock().unwrap();
        supervisor.poll(&database, &Clock, 2);
        assert_eq!(
            supervisor.pending_for(&root.id).unwrap().generation,
            generation
        );
        assert!(!supervisor.is_watching(&root.id));
        assert_eq!(supervisor.generation_for(&root.id), Some(generation));
        drop(held);
        supervisor.sync_roots(&[watch_config(&root, 0)], RETRY_BASE_NS + 2);
        assert!(supervisor.is_watching(&root.id));
        assert_eq!(supervisor.generation_for(&root.id), Some(generation + 1));
        assert_eq!(
            supervisor.pending_for(&root.id).unwrap().generation,
            generation + 1
        );
        let held = database.lock().unwrap();
        supervisor.poll(&database, &Clock, RETRY_BASE_NS + 3);
        assert_eq!(
            supervisor.pending_for(&root.id).unwrap().generation,
            generation + 1
        );
        drop(held);
        supervisor.poll(&database, &Clock, RETRY_BASE_NS + 4);
        assert!(supervisor.pending_for(&root.id).is_none());
    }
}
