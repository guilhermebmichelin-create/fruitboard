//! Durable follow-up scheduling from watcher hints (#37, P2-09 storage half).
//!
//! Watcher hints are scheduling observations, never authority: a hint only
//! says that activity was observed (or that coverage was lost) under a root,
//! and it never decides file state. This module translates the watcher's
//! coalesced hints and overflow signals into durable follow-up requests on
//! storage's deduplicated queue, completing the storage half of the P2-09
//! contract:
//!
//! - A burst of activity inside one coalescing window arrives as at most one
//!   hint per root, and repeated hints never grow the durable queue: storage
//!   coalesces every trigger onto the one queued follow-up or the one running
//!   attempt (`enqueue_scan` dedup). This adapter leans on that durable dedup
//!   instead of reimplementing a second queue.
//! - A coverage-loss signal (OS notification overflow, dropped or truncated
//!   batch, ended watch) produces the same single full-reconciliation
//!   follow-up, with the overflow cause recorded in the request's safe
//!   metadata (root id, job id, cause). Overflow must never be interpreted as
//!   absence evidence: the follow-up is a full-root reconciliation, exactly
//!   like any other trigger.
//! - Generation fencing: hints carry the watcher generation that produced
//!   them. The host must call [`WatcherFollowUpAdapter::watch_started`] for
//!   each root when it installs its watch, before processing that watch's
//!   hints; until the seed arrives, hints for the root are dropped. This
//!   closes the restart bootstrap window: after a host restart the adapter's
//!   generation table is empty, and a replayed hint from a dead generation
//!   can never be adopted as the current one. A hint whose `(root,
//!   generation)` is older than the adapter's current generation for that
//!   root is likewise dropped — fresh generations exist after a watcher
//!   restart (strictly monotonic per root, #69 contract), and a root that
//!   was removed and re-added is a fresh durable root id. When a watch ends,
//!   the host calls [`WatcherFollowUpAdapter::watch_ended`] so replayed
//!   hints from the dead watch are dropped until a strictly newer
//!   generation starts a fresh watch.
//! - Suppression lives in storage: disabled roots refuse requests
//!   ([`StorageError::Conflict`], counted as suppressed), removed roots return
//!   [`StorageError::NotFound`] or are dropped by the host's root mapping
//!   (counted as unknown), and roots with active running work coalesce the
//!   hint onto the running attempt's follow-up flag without growing the
//!   queue. Cancelled chains are never revived, but a new trigger starts a
//!   fresh chain.
//! - Privacy: no absolute path, no relative path, and no root display name
//!   crosses this boundary. Hints carry only opaque watcher root ids and
//!   generations; the host's [`RootIdMapping`] resolves them to opaque
//!   durable storage root ids; follow-up requests carry only root ids, job
//!   ids and counters.
//!
//! This module does not wire the adapter into any host. The desktop host owns
//! watcher lifecycle, the mapping table, the poll loop and activation behind
//! its console flag; `tests.rs` provides a compiling fake consumer proving
//! the exact trait shape the host binds.

use std::collections::BTreeMap;

use fruitboard_filesystem_watcher::{HintKind, RootId, WatchHint};
use fruitboard_storage::{Database, ScanKind, StorageError};

use crate::ScanClock;

/// Why a follow-up was requested. The cause is safe diagnostic metadata only:
/// it carries no path, no file name, and no file state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FollowUpCause {
    /// Coalesced activity inside one watcher window. At most one such hint is
    /// produced per root per window by the watcher's coalescer.
    ActivityBurst,
    /// The watcher lost coverage (OS notification overflow, dropped or
    /// truncated batch, ended watch): events may have been missed. The
    /// follow-up is a full reconciliation and is never evidence of absence.
    Overflow,
}

/// One durable follow-up request produced from a watcher hint. Only opaque
/// ids and counters cross this boundary; there is no path data anywhere in
/// this type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FollowUpRequest {
    /// The watcher root id that produced the hint.
    pub root: RootId,
    /// The durable storage root id the follow-up targets.
    pub storage_root_id: String,
    /// The durable job identity of the follow-up (the storage dedup target).
    pub job_id: String,
    /// True when storage merged this request onto an existing queued or
    /// running slot instead of creating a new job.
    pub coalesced: bool,
    /// Why the follow-up was requested.
    pub cause: FollowUpCause,
}

/// What one [`WatcherFollowUpAdapter::process_hints`] call did. Counters are
/// per-call; the adapter also keeps cumulative counters for diagnostics.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FollowUpOutcome {
    /// One entry per hint that produced (or coalesced onto) a durable
    /// follow-up request.
    pub requests: Vec<FollowUpRequest>,
    /// Hints dropped because their `(root, generation)` is older than the
    /// current generation, or because the host has not seeded the watch yet
    /// (stale watch signals, idempotent replayed drops, bootstrap drops).
    pub stale_generation_dropped: usize,
    /// Hints dropped because no durable root matched: the host mapping no
    /// longer knows the root, or storage reports it removed.
    pub unknown_root_dropped: usize,
    /// Hints suppressed by storage's own suppression (disabled root). The
    /// root must not receive a follow-up; storage refused the request.
    pub suppressed: usize,
    /// Requests driven by a coverage-loss (overflow) signal. Overflow always
    /// schedules the same full reconciliation as any other trigger; it is
    /// never absence evidence.
    pub overflow: usize,
}

/// Resolves a watcher root id to the durable storage root id.
///
/// The host owns the watcher↔storage root table together with its per-root
/// watch state. Implementations return `None` for roots the host no longer
/// tracks (removed or never-configured roots); the adapter drops such hints
/// without touching storage. Only opaque ids cross this boundary; a
/// resolution must never introduce a path.
pub trait RootIdMapping {
    /// The durable storage root id for `root`, or `None` when the host no
    /// longer tracks that watcher root.
    fn storage_root_id(&self, root: RootId) -> Option<String>;
}

/// Current watch state of one root as observed by the adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RootWatchState {
    /// A live watch; hints at this generation are current.
    Active(u64),
    /// The watch ended; hints at or below this generation are stale until a
    /// strictly newer generation starts a fresh watch.
    Ended(u64),
}

/// Translates coalesced watcher hints into durable follow-up requests.
///
/// The adapter is deterministic under an injected [`ScanClock`] and holds no
/// wall-clock state of its own. It keeps only a bounded per-root generation
/// table and cumulative counters, so it cannot grow with event volume beyond
/// the number of configured roots.
#[derive(Clone, Debug)]
pub struct WatcherFollowUpAdapter<M: RootIdMapping> {
    mapping: M,
    generations: BTreeMap<RootId, RootWatchState>,
    overflow_follow_ups: BTreeMap<RootId, u64>,
    stale_dropped: usize,
    unknown_dropped: usize,
    suppressed: usize,
}

impl<M: RootIdMapping> WatcherFollowUpAdapter<M> {
    /// A fresh adapter over the host's root mapping. No hint is accepted
    /// until the host seeds each root with [`WatcherFollowUpAdapter::
    /// watch_started`]: this closes the restart bootstrap window where a
    /// replayed hint from a dead generation would otherwise be adopted as
    /// the current one.
    pub fn new(mapping: M) -> Self {
        Self {
            mapping,
            generations: BTreeMap::new(),
            overflow_follow_ups: BTreeMap::new(),
            stale_dropped: 0,
            unknown_dropped: 0,
            suppressed: 0,
        }
    }

    /// Process one batch of coalesced hints: fence each hint's generation,
    /// resolve the durable root, and enqueue the deduplicated follow-up.
    ///
    /// Storage is the dedup authority: repeated triggers for the same root
    /// coalesce onto the one queued follow-up or the running attempt's
    /// follow-up flag, so a burst of hints never grows the queue beyond one
    /// follow-up per root. A `NotFound` (root removed since mapping) and a
    /// `Conflict` (root disabled) are counted and dropped, never surfaced as
    /// errors: suppression is storage's contract, asserted in tests rather
    /// than reimplemented here.
    pub fn process_hints(
        &mut self,
        db: &mut Database,
        hints: &[WatchHint],
        clock: &dyn ScanClock,
    ) -> Result<FollowUpOutcome, StorageError> {
        let mut outcome = FollowUpOutcome::default();
        for &hint in hints {
            if !self.fence(hint.root, hint.generation) {
                outcome.stale_generation_dropped += 1;
                continue;
            }
            let Some(storage_root_id) = self.mapping.storage_root_id(hint.root) else {
                outcome.unknown_root_dropped += 1;
                continue;
            };
            match db.enqueue_scan(&storage_root_id, ScanKind::Periodic, clock.now_ms()) {
                Ok(result) => {
                    let cause = match hint.kind {
                        HintKind::CoverageLost => {
                            *self.overflow_follow_ups.entry(hint.root).or_insert(0) += 1;
                            FollowUpCause::Overflow
                        }
                        HintKind::ReconciliationRequested => FollowUpCause::ActivityBurst,
                    };
                    if cause == FollowUpCause::Overflow {
                        outcome.overflow += 1;
                    }
                    outcome.requests.push(FollowUpRequest {
                        root: hint.root,
                        storage_root_id,
                        job_id: result.job_id,
                        coalesced: result.coalesced,
                        cause,
                    });
                }
                Err(StorageError::NotFound) => outcome.unknown_root_dropped += 1,
                Err(StorageError::Conflict) => outcome.suppressed += 1,
                Err(error) => return Err(error),
            }
        }
        self.stale_dropped += outcome.stale_generation_dropped;
        self.unknown_dropped += outcome.unknown_root_dropped;
        self.suppressed += outcome.suppressed;
        Ok(outcome)
    }

    /// Tell the adapter that the host started (or restarted) a live watch
    /// for `root` at `generation`. The host must call this when it installs
    /// each watch — before processing any hint from it — so replayed hints
    /// from a dead generation are dropped at bootstrap instead of being
    /// adopted as the current one. Seeding the same generation again is
    /// idempotent; a strictly newer generation promotes the fence like any
    /// fresh watch.
    pub fn watch_started(&mut self, root: RootId, generation: u64) {
        self.generations
            .insert(root, RootWatchState::Active(generation));
    }

    /// Tell the adapter that the watch for `root` has ended (host stopped it
    /// after disable, removal or shutdown). Replayed hints from the ended
    /// watch are dropped until a strictly newer generation starts a fresh
    /// watch, so a stale replay can never schedule work for a dead watch.
    pub fn watch_ended(&mut self, root: RootId) {
        let state = self.generations.get(&root).copied();
        let generation = match state {
            Some(RootWatchState::Active(generation)) | Some(RootWatchState::Ended(generation)) => {
                generation
            }
            None => return,
        };
        self.generations
            .insert(root, RootWatchState::Ended(generation));
    }

    /// Cumulative overflow-driven follow-ups requested for `root`.
    pub fn overflow_follow_ups(&self, root: RootId) -> u64 {
        self.overflow_follow_ups.get(&root).copied().unwrap_or(0)
    }

    /// Cumulative hints dropped as stale-generation signals.
    pub fn stale_generation_dropped(&self) -> usize {
        self.stale_dropped
    }

    /// Cumulative hints dropped because no durable root matched.
    pub fn unknown_root_dropped(&self) -> usize {
        self.unknown_dropped
    }

    /// Cumulative hints suppressed by storage (disabled roots).
    pub fn suppressed(&self) -> usize {
        self.suppressed
    }

    /// Generation fence: accepts the hint when its generation is the current
    /// one, promotes a strictly newer generation (fresh watch), and drops an
    /// older one. A hint for a root the host has not seeded yet is dropped:
    /// the bootstrap window must never adopt a replayed generation. A hint
    /// at or below an ended watch's generation is dropped until a strictly
    /// newer generation revives the root.
    fn fence(&mut self, root: RootId, generation: u64) -> bool {
        let state = self.generations.get(&root).copied();
        match state {
            None => false,
            Some(RootWatchState::Active(current)) => {
                if generation == current {
                    true
                } else if generation > current {
                    self.generations
                        .insert(root, RootWatchState::Active(generation));
                    true
                } else {
                    false
                }
            }
            Some(RootWatchState::Ended(current)) if generation > current => {
                self.generations
                    .insert(root, RootWatchState::Active(generation));
                true
            }
            Some(RootWatchState::Ended(_)) => false,
        }
    }
}
