//! Handle-bound filesystem watching for one configured root, with bounded
//! coalesced hints. Windows-only platform layer; the pure coalescing core is
//! cross-platform.
//!
//! This is the #37 watcher foundation slice. It is deliberately NOT the
//! integrated watcher: nothing here writes to storage, nothing decides file
//! state, and no follow-up is wired to the durable queue yet. The only
//! consumer seam is [`WatcherPort`]; the next slice will connect it to
//! durable reconciliation follow-ups.
//!
//! # Contracts
//!
//! - Hints are observations, never authority. A
//!   [`HintKind::ReconciliationRequested`] hint says only that activity was
//!   observed under a root; a [`HintKind::CoverageLost`] hint says that events
//!   were missed or the watch ended. Consumers must turn both into a full,
//!   NON-AUTHORITATIVE reconciliation follow-up. Missed events never imply
//!   absence (P2-09 invariant).
//! - Burst coalescing is a fixed, non-extending window: at most one
//!   reconciliation hint per root per window, deterministic under an injected
//!   clock (see [`Coalescer`]).
//! - Overflow policy is drop-with-counter: the raw event queue drops on a
//!   full queue (counted), OS notification-buffer overflows drop the partial
//!   batch (counted), and both raise the sticky coverage-loss signal.
//! - Lifecycle is typed and leak-free: [`EndReason::Stopped`],
//!   [`EndReason::RootLost`], or [`EndReason::WatchFailed`]; `stop()` is
//!   idempotent, joins the worker thread, and a restart opens a fresh handle
//!   with a fresh caller-supplied generation that must be strictly greater
//!   than the previous generation for the same root. Hints are keyed by
//!   `(root, generation)` so downstream consumers can discard
//!   stale-generation signals.
//! - Reparse points are policy exclusions, distinct from I/O failures: a
//!   reparse-point root observed at start is refused with
//!   [`StartError::ReparseRootExcluded`] without any I/O claim. Limitation
//!   (tied to #47/#48): the pre-check races with the handle open
//!   (GetFileAttributesW-to-CreateFileW window), parent-directory junctions
//!   are followed by the OS open, and nested reparse activity inside the
//!   subtree still arrives as hints. No traversal safety is claimed; the
//!   watcher performs no per-event I/O and enforcement of nested reparse
//!   exclusions belongs to the authoritative enumeration boundary.
//! - Privacy: no public type can carry an absolute path. Notification names
//!   are validated to be root-relative and relative bytes are redacted from
//!   `Debug`. Errors never echo the root path. This crate performs no
//!   logging, no content reads, no hydration, and no parsing.
//!
//! DriveFS (#47) and non-NTFS volumes (#48) are explicitly unverified; the
//! crate README documents the ignored fixtures that must never be counted as
//! support.

mod coalescer;
mod path;
#[cfg(windows)]
mod platform;
#[cfg(test)]
mod tests;
#[cfg(all(test, windows))]
mod tests_windows;

pub use coalescer::{Coalescer, CoalescerConfig, InvalidCoalescerConfig};
pub use path::{NotifyAction, RawEvent, RelativePath, RelativePathRejected};
#[cfg(windows)]
pub use platform::{HandleBoundWatcher, WatcherConfig, WatcherStats, monotonic_nanos};

/// Identifies one configured watch root. Opaque key supplied by the caller;
/// it carries no path material.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootId(pub u64);

/// What a hint asks the consumer to do. Hints never decide file state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintKind {
    /// Coalesced activity under the root. At most one per root per window.
    /// This is a scheduling hint only; the follow-up reconciliation remains
    /// the sole authority for presence, absence, and changes.
    ReconciliationRequested,
    /// Coverage was lost: the OS notification buffer overflowed, the raw
    /// queue dropped events, a batch was truncated, or the watch ended.
    /// Consumers must turn this into a full, NON-AUTHORITATIVE reconciliation
    /// follow-up. Missed events never imply absence.
    CoverageLost,
}

/// One coalesced hint, keyed by root id and the watch generation that
/// produced it so consumers can discard stale-generation signals.
/// Generations are caller-supplied and must be strictly monotonic per root
/// across restarts; a restart replaces all pending state for that root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WatchHint {
    pub root: RootId,
    pub generation: u64,
    pub kind: HintKind,
}

/// Why a watch ended. Typed observations about the watch, never about
/// individual files; a `RootLost` outcome never proves any file is missing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    /// `stop()` was requested; the worker canceled its pending read, joined
    /// cleanly, and released the handle.
    Stopped,
    /// The configured root is no longer verifiably present at its configured
    /// path (renamed, deleted, or unreachable). An observation about the
    /// configured path only.
    RootLost,
    /// The OS refused to continue the watch. Carries an opaque OS status code
    /// for diagnostics; never a path or message text.
    WatchFailed { os_code: u32 },
}

/// Typed terminal outcome of one watch generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WatchOutcome {
    pub root: RootId,
    pub generation: u64,
    pub reason: EndReason,
}

/// A typed start failure. Variants never echo the root path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartError {
    /// The root path could not be opened as a directory watch handle, or the
    /// watch failed before arming because the root was lost (the
    /// worker-captured OS code is preserved). This is an I/O failure
    /// classification, deliberately distinct from resource exhaustion.
    RootUnavailable { os_code: u32 },
    /// The opened handle is not a directory. This is an I/O failure
    /// classification.
    NotADirectory,
    /// The root is a reparse point: a policy exclusion consistent with the
    /// enumeration contract, deliberately distinct from I/O failures.
    ReparseRootExcluded,
    /// The root path could not be encoded for the OS boundary.
    InvalidRootPath,
    /// The configuration was rejected (nonpositive window or bounds).
    InvalidConfig,
    /// A kernel resource needed for the watch was unavailable.
    ResourceUnavailable { os_code: u32 },
}

/// The only consumer-facing watcher contract: coalesced hints keyed by root
/// id, plus the typed lifecycle outcome. Timestamps are caller-supplied
/// monotonic values in one unit scheme shared with the configured window, so
/// the whole port is deterministic under a fake clock.
///
/// Storage follow-up wiring is intentionally absent from this crate.
pub trait WatcherPort {
    /// Drains pending raw signals, applies coalescing, and returns all hints
    /// currently due in deterministic order.
    fn poll_hints(&mut self, now: u64) -> Vec<WatchHint>;

    /// The typed terminal outcome of the watch, if it has ended. Sticky:
    /// continues to report the same outcome once observed.
    fn outcome(&mut self) -> Option<WatchOutcome>;
}

impl WatcherPort for Coalescer {
    fn poll_hints(&mut self, now: u64) -> Vec<WatchHint> {
        Coalescer::poll(self, now)
    }

    fn outcome(&mut self) -> Option<WatchOutcome> {
        None
    }
}
