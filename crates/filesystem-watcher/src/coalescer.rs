//! Deterministic coalescing of watcher activity into bounded hints.
//!
//! The coalescer is pure and fake-clock injectable: all timestamps are
//! caller-supplied monotonic values in one unit scheme (tests use scripted
//! fake clocks; the future durable consumer owns a real monotonic source).
//! A burst collapses into at most one [`HintKind::ReconciliationRequested`]
//! hint per root per fixed, non-extending window. Coverage loss is sticky and
//! delivered immediately because missed events must never imply absence.

use std::collections::BTreeMap;
use std::mem;

use crate::{HintKind, RootId, WatchHint};

/// Coalescer configuration. `window` uses the same unit scheme as the
/// timestamps supplied to [`Coalescer::record_activity`] and
/// [`Coalescer::poll`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoalescerConfig {
    pub window: u64,
    pub max_tracked_roots: usize,
}

impl Default for CoalescerConfig {
    fn default() -> Self {
        // One second in nanoseconds; a small root budget matches the
        // deployment bound of at most 100 configured roots.
        Self {
            window: 1_000_000_000,
            max_tracked_roots: 64,
        }
    }
}

/// The configuration was invalid (zero window or zero root budget).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidCoalescerConfig;

#[derive(Clone, Copy, Debug)]
struct WindowState {
    generation: u64,
    closes_at: u64,
}

/// Collapses raw watcher activity into bounded, deterministic hints.
///
/// State is bounded: at most `max_tracked_roots` open windows plus at most
/// `max_tracked_roots` pending coverage-loss marks (drained on the next
/// poll). Signals beyond the bound are rejected and counted; they are never
/// queued, so memory cannot grow with event volume.
pub struct Coalescer {
    window: u64,
    max_tracked_roots: usize,
    windows: BTreeMap<RootId, WindowState>,
    lost: BTreeMap<RootId, u64>,
    rejected: u64,
}

impl Coalescer {
    pub fn new(config: CoalescerConfig) -> Result<Self, InvalidCoalescerConfig> {
        if config.window == 0 || config.max_tracked_roots == 0 {
            return Err(InvalidCoalescerConfig);
        }
        Ok(Self {
            window: config.window,
            max_tracked_roots: config.max_tracked_roots,
            windows: BTreeMap::new(),
            lost: BTreeMap::new(),
            rejected: 0,
        })
    }

    /// Records raw activity on `root` for `generation` at time `now`.
    ///
    /// The first activity opens a fixed window closing at `now + window`;
    /// activity inside an open window never extends it, so bursts collapse
    /// into at most one hint per window. Activity stamped with a different
    /// generation than the open window replaces it: a restart must never
    /// inherit the previous watch's pending state. Callers must supply a
    /// fresh generation per restart that is strictly greater than the
    /// previous generation for the same root; downstream consumers key hints
    /// by `(root, generation)` and discard stale-generation signals.
    pub fn record_activity(&mut self, root: RootId, generation: u64, now: u64) {
        if let Some(state) = self.windows.get_mut(&root) {
            if state.generation != generation {
                debug_assert!(
                    generation > state.generation,
                    "watcher generations must be monotonic per root: got {generation} after {}",
                    state.generation
                );
                *state = WindowState {
                    generation,
                    closes_at: now.saturating_add(self.window),
                };
                self.lost.remove(&root);
            }
            return;
        }
        if let Some(&lost_generation) = self.lost.get(&root)
            && lost_generation != generation
        {
            // Coverage loss from a dead generation is moot once a fresh
            // generation observes activity. Fresh means greater: restarts
            // must never reuse a generation.
            debug_assert!(
                generation > lost_generation,
                "watcher generations must be monotonic per root: got {generation} after {lost_generation}"
            );
            self.lost.remove(&root);
        }
        if !self.windows.contains_key(&root)
            && !self.lost.contains_key(&root)
            && self.windows.len() + self.lost.len() >= self.max_tracked_roots
        {
            self.rejected += 1;
            return;
        }
        self.windows.insert(
            root,
            WindowState {
                generation,
                closes_at: now.saturating_add(self.window),
            },
        );
    }

    /// Marks that events were missed for `root` (OS notification overflow,
    /// queue drop, truncated batch, or the watch ended). Sticky until the
    /// next poll, and delivered immediately — never windowed. A stale-
    /// generation loss for a currently tracked root is ignored: that window
    /// already belongs to a newer watch.
    pub fn record_coverage_lost(&mut self, root: RootId, generation: u64) {
        if let Some(state) = self.windows.get(&root) {
            // A stale-generation loss for a currently tracked root is
            // ignored: that window already belongs to a newer watch.
            if state.generation != generation {
                return;
            }
        }
        if self.lost.get(&root) == Some(&generation) {
            return; // already sticky for this generation
        }
        let tracked = self.windows.contains_key(&root) || self.lost.contains_key(&root);
        if !tracked && self.windows.len() + self.lost.len() >= self.max_tracked_roots {
            self.rejected += 1;
            return;
        }
        self.lost.insert(root, generation);
    }

    /// Emits all currently due hints in deterministic order: coverage-loss
    /// hints first, then coalesced window hints, each ascending by root id.
    /// At most one hint per root per poll; a coverage-loss hint subsumes any
    /// pending window for the same root.
    pub fn poll(&mut self, now: u64) -> Vec<WatchHint> {
        let mut hints = Vec::new();
        for (root, generation) in mem::take(&mut self.lost) {
            self.windows.remove(&root);
            hints.push(WatchHint {
                root,
                generation,
                kind: HintKind::CoverageLost,
            });
        }
        let due: Vec<RootId> = self
            .windows
            .iter()
            .filter(|(_, state)| now >= state.closes_at)
            .map(|(root, _)| *root)
            .collect();
        for root in due {
            let state = self.windows.remove(&root).expect("window tracked above");
            hints.push(WatchHint {
                root,
                generation: state.generation,
                kind: HintKind::ReconciliationRequested,
            });
        }
        hints
    }

    /// Number of currently open coalescing windows (bound assertion hook).
    pub fn open_windows(&self) -> usize {
        self.windows.len()
    }

    /// Count of signals rejected by the tracking bound.
    pub fn rejected_signals(&self) -> u64 {
        self.rejected
    }
}
