//! Windows platform layer: one handle-bound `ReadDirectoryChangesW` watch
//! per root, served by a dedicated worker thread feeding a bounded queue.
//!
//! Ownership model: `HandleBoundWatcher` (the starter) owns the stop event
//! exclusively via a `WorkerHandle` field and closes it only after joining
//! the worker (in `Drop`, after `stop()` joins). The worker owns the root
//! handle and the I/O event exclusively and closes them on exit, but only
//! borrows the stop event (`StopBorrow`) for waiting — it never closes
//! it. `stop()` signals the starter-owned event, joins the worker (which
//! cancels its own pending read), and then reads the typed outcome. Because
//! the starter's handle outlives the join, signaling after worker exit is a
//! harmless no-op on a live event and can never touch a closed handle — no
//! leaks, no panics, no cross-thread `CancelIoEx`, no use-after-close.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_BAD_NET_NAME, ERROR_BAD_NETPATH, ERROR_DELETE_PENDING,
    ERROR_DIRECTORY, ERROR_FILE_NOT_FOUND, ERROR_INVALID_HANDLE, ERROR_IO_INCOMPLETE,
    ERROR_MORE_DATA, ERROR_NETNAME_DELETED, ERROR_NOT_READY, ERROR_NOTIFY_ENUM_DIR,
    ERROR_OPERATION_ABORTED, ERROR_PATH_NOT_FOUND, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
    WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY,
    FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
    FILE_NOTIFY_CHANGE_SIZE, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    GetFileAttributesW, INVALID_FILE_ATTRIBUTES, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects};

use crate::coalescer::{Coalescer, CoalescerConfig};
use crate::path::{RawEvent, parse_notify_buffer};
use crate::{EndReason, RootId, StartError, WatchHint, WatchOutcome, WatcherPort};

const NOTIFY_FILTER: u32 = FILE_NOTIFY_CHANGE_FILE_NAME
    | FILE_NOTIFY_CHANGE_DIR_NAME
    | FILE_NOTIFY_CHANGE_LAST_WRITE
    | FILE_NOTIFY_CHANGE_SIZE;

/// Index of the stop event in the two-handle wait.
const WAIT_STOP: u32 = WAIT_OBJECT_0 + 1;

/// The watcher tracks exactly one root; the coalescer budget stays small.
const COALESCER_ROOT_BOUND: usize = 8;

const MIN_NOTIFY_BUFFER_BYTES: usize = 4096;
const MAX_NOTIFY_BUFFER_BYTES: usize = 1024 * 1024;
const MIN_VALIDITY_POLL_MS: u32 = 50;
const MAX_VALIDITY_POLL_MS: u32 = 10_000;

/// Configuration for one watch attempt. The generation must be freshly
/// allocated by the caller for every restart — strictly greater than the
/// previous generation for the same root (durable root generation contract).
/// The watcher never reuses a generation; downstream consumers key hints by
/// `(root, generation)` and must discard stale-generation signals (see
/// README). In debug builds the coalescer asserts this monotonicity when a
/// restart replaces pending state.
#[derive(Clone, Copy, Debug)]
pub struct WatcherConfig {
    /// Caller-owned monotonic generation (durable root generation contract).
    pub generation: u64,
    /// Fixed coalescing window in the caller's timestamp units.
    pub window: u64,
    /// Raw event queue capacity before drop-with-counter engages.
    pub raw_queue_bound: usize,
    /// OS notification buffer size in bytes; clamped to [4096, 1 MiB].
    pub notify_buffer_bytes: usize,
    /// Root presence re-check interval in milliseconds; clamped to
    /// [50, 10_000]. A smaller value only bounds detection latency, never
    /// correctness.
    pub validity_poll_ms: u32,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            generation: 0,
            window: 1_000_000_000,
            raw_queue_bound: 256,
            notify_buffer_bytes: 64 * 1024,
            validity_poll_ms: 500,
        }
    }
}

impl WatcherConfig {
    fn normalized(&self) -> (usize, Duration) {
        (
            self.notify_buffer_bytes
                .clamp(MIN_NOTIFY_BUFFER_BYTES, MAX_NOTIFY_BUFFER_BYTES),
            Duration::from_millis(
                self.validity_poll_ms
                    .clamp(MIN_VALIDITY_POLL_MS, MAX_VALIDITY_POLL_MS)
                    .into(),
            ),
        )
    }
}

/// Diagnostic counters for one watch. Safe codes and counters only; no paths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WatcherStats {
    /// Raw events dropped because the bounded queue was full.
    pub dropped_raw_events: u64,
    /// OS notification-buffer overflows observed (events missed by the OS).
    pub notify_buffer_overflows: u64,
    /// Records surfaced as obscured because their action code or path bytes
    /// could not be validated.
    pub obscured_events: u64,
}

pub(crate) struct Shared {
    pub(crate) root: RootId,
    pub(crate) generation: u64,
    pub(crate) queue_tx: SyncSender<RawEvent>,
    pub(crate) stopping: AtomicBool,
    pub(crate) armed: AtomicBool,
    pub(crate) coverage_lost: AtomicBool,
    pub(crate) outcome: Mutex<Option<WatchOutcome>>,
    /// OS status code captured by the worker at the terminal failure point
    /// (the `GetLastError` value that produced the outcome). Preserved so
    /// `start()` can report a pre-arm `RootLost` as `RootUnavailable` without
    /// losing the code. `0` means no failure code was recorded.
    pub(crate) exit_os_code: AtomicU32,
    pub(crate) dropped_raw_events: AtomicU64,
    pub(crate) notify_buffer_overflows: AtomicU64,
    pub(crate) obscured_events: AtomicU64,
}

impl Shared {
    /// Drop-with-overflow-counter policy: a full or disconnected queue drops
    /// the event, counts it, and raises the sticky coverage-loss signal so
    /// the missed events can never imply absence downstream.
    pub(crate) fn try_push(&self, event: RawEvent) {
        match self.queue_tx.try_send(event) {
            Ok(()) => {}
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                self.dropped_raw_events.fetch_add(1, Ordering::Relaxed);
                self.coverage_lost.store(true, Ordering::Release);
            }
        }
    }

    pub(crate) fn mark_coverage_lost(&self) {
        self.coverage_lost.store(true, Ordering::Release);
    }

    fn finish(&self, outcome: WatchOutcome) {
        if let Ok(mut slot) = self.outcome.lock() {
            *slot = Some(outcome);
        }
    }
}

/// A raw kernel handle owned exclusively by the worker thread. The `Drop`
/// impl closes it exactly once, at worker exit.
struct WorkerHandle(windows_sys::Win32::Foundation::HANDLE);

// SAFETY: a kernel handle value is process-global and usable from any thread.
// Ownership transfers to the worker thread exclusively; the Drop impl closes
// it on the worker thread at exit.
unsafe impl Send for WorkerHandle {}

impl WorkerHandle {
    fn get(&self) -> windows_sys::Win32::Foundation::HANDLE {
        self.0
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            // SAFETY: the owner closes this handle exclusively and exactly
            // once, here.
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// Borrowed view of the starter-owned stop event, transferred to the worker
/// at spawn so it can wait on it. The worker never closes it; the starter
/// (`HandleBoundWatcher`) closes it only after joining the worker, so the
/// raw value is valid for the whole worker lifetime.
#[derive(Clone, Copy)]
struct StopBorrow(HANDLE);

// SAFETY: a kernel handle value is process-global. The starter guarantees
// the event outlives the worker thread (close-after-join) and the worker
// never closes it, so sending this borrowed view to the worker is sound.
unsafe impl Send for StopBorrow {}

struct WorkerParams {
    shared: Arc<Shared>,
    /// Private UTF-16 copy of the configured root path (with trailing NUL),
    /// used only for presence re-checks. Never surfaced or logged.
    root_path: Arc<[u16]>,
    buffer_bytes: usize,
    validity_poll: Duration,
    watch: WorkerHandle,
    /// Borrowed view of the starter-owned stop event. The worker waits on it
    /// but never closes it; the starter (`HandleBoundWatcher`) owns the
    /// handle and closes it only after joining this thread.
    stop: StopBorrow,
    io_event: WorkerHandle,
}

fn worker(params: WorkerParams) {
    // The destructured `WorkerHandle` values (`watch`, `io_event`) stay alive
    // until this function returns; their `Drop` impls then close those two
    // handles exactly once, on this thread. `stop` is a borrowed view of the
    // starter-owned event and is never closed here.
    let WorkerParams {
        shared,
        root_path,
        buffer_bytes,
        validity_poll,
        watch,
        stop,
        io_event,
    } = params;
    let watch = watch.get();
    let stop_event: HANDLE = stop.0;

    // SAFETY: zero-initialization is the documented way to prepare an
    // OVERLAPPED; hEvent is set immediately below. The struct outlives every
    // pending I/O on this thread because cancellation happens before exit.
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    overlapped.hEvent = io_event.get();
    let mut buffer = vec![0u8; buffer_bytes];
    let mut armed = false;
    let mut outcome = WatchOutcome {
        root: shared.root,
        generation: shared.generation,
        reason: EndReason::Stopped,
    };
    let handles = [io_event.get(), stop_event];

    loop {
        if shared.stopping.load(Ordering::Acquire) {
            outcome.reason = EndReason::Stopped;
            break;
        }
        if !armed {
            // SAFETY: `watch` is a live directory handle opened with
            // FILE_FLAG_OVERLAPPED; the buffer stays stable while the read is
            // pending; exactly one read is pending at a time.
            let queued = unsafe {
                ReadDirectoryChangesW(
                    watch,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    1,
                    NOTIFY_FILTER,
                    std::ptr::null_mut(),
                    &mut overlapped,
                    None,
                )
            };
            if queued == 0 {
                // SAFETY: immediate failure reporting; no parameters.
                let code = unsafe { GetLastError() };
                outcome.reason = if code == ERROR_OPERATION_ABORTED {
                    EndReason::Stopped
                } else {
                    let reason = terminal_reason(code);
                    if matches!(reason, EndReason::RootLost) {
                        shared.exit_os_code.store(code, Ordering::Release);
                    }
                    reason
                };
                break;
            }
            armed = true;
            // Observability contract: once this flag is set, every change to
            // the root is observed by the pending read (or already missed and
            // flagged). `start()` waits for it so callers can rely on the
            // watch being live when start returns.
            shared.armed.store(true, Ordering::Release);
        }
        // SAFETY: both handles stay live for the whole worker lifetime. The
        // I/O event is worker-owned; the stop event is starter-owned and
        // outlives the join, so waiting on it here is always valid.
        let wait = unsafe {
            WaitForMultipleObjects(2, handles.as_ptr(), 0, validity_poll.as_millis() as u32)
        };
        match wait {
            WAIT_OBJECT_0 => {
                armed = false;
                let mut bytes = 0u32;
                // SAFETY: the OVERLAPPED belongs to the completed read on
                // this same handle; bWait is FALSE because the event fired.
                let completed = unsafe { GetOverlappedResult(watch, &overlapped, &mut bytes, 0) };
                if completed != 0 {
                    if bytes > 0 {
                        let batch = parse_notify_buffer(&buffer[..bytes as usize]);
                        shared
                            .obscured_events
                            .fetch_add(batch.obscured, Ordering::Relaxed);
                        for event in batch.events {
                            shared.try_push(event);
                        }
                        if batch.truncated {
                            shared.mark_coverage_lost();
                        }
                    }
                } else {
                    // SAFETY: immediate failure reporting; no parameters.
                    let code = unsafe { GetLastError() };
                    if code == ERROR_MORE_DATA || code == ERROR_NOTIFY_ENUM_DIR {
                        // The OS lost events: drop the partial batch and
                        // demand a full non-authoritative reconciliation.
                        shared
                            .notify_buffer_overflows
                            .fetch_add(1, Ordering::Relaxed);
                        shared.mark_coverage_lost();
                    } else if code == ERROR_IO_INCOMPLETE {
                        // The event fired but the result is not consumable
                        // yet: keep the read armed and wait again.
                        armed = true;
                    } else if code == ERROR_OPERATION_ABORTED
                        || shared.stopping.load(Ordering::Acquire)
                    {
                        outcome.reason = EndReason::Stopped;
                        break;
                    } else {
                        let reason = terminal_reason(code);
                        if matches!(reason, EndReason::RootLost) {
                            shared.exit_os_code.store(code, Ordering::Release);
                        }
                        outcome.reason = reason;
                        break;
                    }
                }
            }
            WAIT_STOP => {
                outcome.reason = EndReason::Stopped;
                break;
            }
            WAIT_TIMEOUT => {
                if let Err(code) = root_present_code(&root_path) {
                    shared.exit_os_code.store(code, Ordering::Release);
                    outcome.reason = EndReason::RootLost;
                    break;
                }
            }
            _ => {
                // SAFETY: immediate failure reporting; no parameters.
                outcome.reason = EndReason::WatchFailed {
                    os_code: unsafe { GetLastError() },
                };
                break;
            }
        }
    }
    if armed {
        // SAFETY: canceling our own pending read on our own thread; the
        // blocking wait afterwards consumes the aborted completion before
        // the handles drop.
        unsafe { CancelIoEx(watch, &overlapped) };
        let mut bytes = 0u32;
        // SAFETY: the canceled request completes promptly; wait for it so no
        // completion can touch `overlapped` after this function returns.
        unsafe { GetOverlappedResult(watch, &overlapped, &mut bytes, 1) };
    }
    shared.finish(outcome);
}

/// Classifies a terminal watch failure. Renamed/deleted/unreachable roots are
/// reported as `RootLost`; anything else is an opaque OS status code. Both
/// are observations about the watch, never about individual files.
///
/// `RootLost` covers the OS codes that mean the configured path is gone or
/// unreachable: access-denied on a revoked handle, pending delete, not-ready
/// media, bad/net-deleted network names, plus file/path-not-found (renamed
/// or deleted between opens), invalid handle (the handle's object is gone),
/// and `ERROR_DIRECTORY` (the handle is no longer a directory, e.g. replaced
/// by a file). Audit note: `ERROR_SHARING_VIOLATION` (32) deliberately stays
/// `WatchFailed` — it signals a handle-open conflict, not a lost root, and
/// must not trigger root-loss reconciliation.
pub(crate) fn terminal_reason(os_code: u32) -> EndReason {
    match os_code {
        ERROR_ACCESS_DENIED
        | ERROR_DELETE_PENDING
        | ERROR_NOT_READY
        | ERROR_BAD_NETPATH
        | ERROR_BAD_NET_NAME
        | ERROR_NETNAME_DELETED
        | ERROR_FILE_NOT_FOUND
        | ERROR_PATH_NOT_FOUND
        | ERROR_INVALID_HANDLE
        | ERROR_DIRECTORY => EndReason::RootLost,
        os_code => EndReason::WatchFailed { os_code },
    }
}

/// Maps a pre-arm worker exit to the typed start error without losing the
/// root-loss classification. `RootLost` becomes `RootUnavailable` with the
/// worker-captured OS code preserved; `WatchFailed` keeps its code; anything
/// else (clean `Stopped` or no outcome after the arm timeout) is a resource
/// failure. Pure and unit-testable.
pub(crate) fn classify_start_failure(reason: Option<EndReason>, exit_os_code: u32) -> StartError {
    match reason {
        Some(EndReason::WatchFailed { os_code }) => StartError::RootUnavailable { os_code },
        Some(EndReason::RootLost) => StartError::RootUnavailable {
            os_code: exit_os_code,
        },
        _ => StartError::ResourceUnavailable { os_code: 0 },
    }
}

fn root_present_code(root_path: &[u16]) -> Result<(), u32> {
    // Any failure to observe the configured path (not found, access denied,
    // network drop, pending delete) means the root is not verifiably present.
    // That is the safe direction: the consumer reconciles instead of trusting
    // a watch that cannot be validated. The OS code is returned so the
    // worker can preserve it for start-failure classification.
    // SAFETY: `root_path` is NUL-terminated and stays alive for the call.
    if unsafe { GetFileAttributesW(root_path.as_ptr()) } != INVALID_FILE_ATTRIBUTES {
        Ok(())
    } else {
        // SAFETY: immediate failure reporting; no parameters.
        Err(unsafe { GetLastError() })
    }
}

/// One live handle-bound watch. Not `Sync`: drive it from one thread.
///
/// The starter-owned stop event lives in `stop` and is closed only after the
/// worker is joined (see `stop()` and `Drop`), so signaling after worker
/// exit can never touch a closed handle.
pub struct HandleBoundWatcher {
    shared: Arc<Shared>,
    rx: Receiver<RawEvent>,
    coalescer: Coalescer,
    root: RootId,
    generation: u64,
    join: Mutex<Option<JoinHandle<()>>>,
    stop: WorkerHandle,
    ended: Option<WatchOutcome>,
    loss_signaled: bool,
}

impl HandleBoundWatcher {
    /// Opens the root handle (fresh per start: restarts never reuse a handle
    /// or a generation), validates the policy exclusions, and spawns the
    /// dedicated worker thread.
    pub fn start(
        root: RootId,
        root_path: &OsStr,
        config: WatcherConfig,
    ) -> Result<Self, StartError> {
        let coalescer = Coalescer::new(CoalescerConfig {
            window: config.window,
            max_tracked_roots: COALESCER_ROOT_BOUND,
        })
        .map_err(|_| StartError::InvalidConfig)?;
        if config.raw_queue_bound == 0 {
            return Err(StartError::InvalidConfig);
        }
        let mut wide: Vec<u16> = root_path.encode_wide().collect();
        if wide.is_empty() || wide.contains(&0) {
            return Err(StartError::InvalidRootPath);
        }
        wide.push(0);
        let (buffer_bytes, validity_poll) = config.normalized();

        // Policy pre-check. `GetFileAttributesW` does not follow the final
        // path component, so it can see reparse attributes that `CreateFileW`
        // (which opens through junctions) would hide. Reparse roots are a
        // policy exclusion, deliberately distinct from I/O failures.
        //
        // TOCTOU limitation (tied to #47/#48): this check races with
        // `CreateFileW` below — the root could be replaced by a junction or
        // symlink in between, and parent-directory junctions are followed by
        // both calls. TODO(#47): consider opening with
        // `FILE_FLAG_OPEN_REPARSE_POINT` plus a post-open reparse verify to
        // close the final-component window; parent-junction pass-through
        // would still need enumeration-boundary enforcement. No traversal
        // safety is claimed here: nested reparse reports remain hints for
        // the authoritative enumeration boundary.
        // SAFETY: `wide` is a NUL-terminated UTF-16 path.
        let attributes = unsafe { GetFileAttributesW(wide.as_ptr()) };
        if attributes == INVALID_FILE_ATTRIBUTES {
            // SAFETY: immediate failure reporting; no parameters.
            return Err(StartError::RootUnavailable {
                os_code: unsafe { GetLastError() },
            });
        }
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(StartError::ReparseRootExcluded);
        }
        if attributes & FILE_ATTRIBUTE_DIRECTORY == 0 {
            return Err(StartError::NotADirectory);
        }

        // SAFETY: `wide` is a NUL-terminated UTF-16 path; the security
        // attribute and template handle are unused.
        let watch = WorkerHandle(unsafe {
            CreateFileW(
                wide.as_ptr(),
                FILE_LIST_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                std::ptr::null_mut(),
            )
        });
        if watch.get().is_null() || watch.get() == INVALID_HANDLE_VALUE {
            // SAFETY: immediate failure reporting; no parameters.
            return Err(StartError::RootUnavailable {
                os_code: unsafe { GetLastError() },
            });
        }
        // SAFETY: unnamed manual-reset event; attributes unused.
        let stop = WorkerHandle(unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) });
        if stop.get().is_null() || stop.get() == INVALID_HANDLE_VALUE {
            // SAFETY: immediate failure reporting; no parameters.
            return Err(StartError::ResourceUnavailable {
                os_code: unsafe { GetLastError() },
            });
        }
        // SAFETY: unnamed auto-reset event; attributes unused.
        let io_event =
            WorkerHandle(unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) });
        if io_event.get().is_null() || io_event.get() == INVALID_HANDLE_VALUE {
            // SAFETY: immediate failure reporting; no parameters.
            return Err(StartError::ResourceUnavailable {
                os_code: unsafe { GetLastError() },
            });
        }

        let (queue_tx, rx) = sync_channel(config.raw_queue_bound);
        let shared = Arc::new(Shared {
            root,
            generation: config.generation,
            queue_tx,
            stopping: AtomicBool::new(false),
            armed: AtomicBool::new(false),
            coverage_lost: AtomicBool::new(false),
            outcome: Mutex::new(None),
            exit_os_code: AtomicU32::new(0),
            dropped_raw_events: AtomicU64::new(0),
            notify_buffer_overflows: AtomicU64::new(0),
            obscured_events: AtomicU64::new(0),
        });
        // The starter keeps ownership of `stop`; the worker gets only a
        // borrowed view and never closes it.
        let stop_borrow = StopBorrow(stop.get());
        let params = WorkerParams {
            shared: Arc::clone(&shared),
            root_path: Arc::from(wide.into_boxed_slice()),
            buffer_bytes,
            validity_poll,
            watch,
            stop: stop_borrow,
            io_event,
        };
        let join = thread::Builder::new()
            .name(format!("fruitboard-watcher-root-{}", root.0))
            .spawn(move || worker(params))
            .map_err(|_| StartError::ResourceUnavailable { os_code: 0 })?;
        // Wait until the watch is armed so callers can rely on observing
        // every change made after start() returns. If the worker exits
        // before arming, report its failure instead of a half-live watch.
        let arm_deadline = Instant::now() + Duration::from_secs(5);
        let mut armed_seen = false;
        while Instant::now() < arm_deadline {
            if shared.armed.load(Ordering::Acquire) {
                armed_seen = true;
                break;
            }
            if let Ok(slot) = shared.outcome.lock()
                && slot.is_some()
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        if !armed_seen {
            shared.stopping.store(true, Ordering::Release);
            // SAFETY: `stop` is starter-owned and outlives the join below,
            // so signaling is valid even though the worker already exited.
            unsafe { SetEvent(stop.get()) };
            let _ = join.join();
            let outcome = shared.outcome.lock().ok().and_then(|mut slot| slot.take());
            let exit_os_code = shared.exit_os_code.load(Ordering::Acquire);
            // `watch` and `io_event` were moved into the worker and closed
            // there on exit. `stop` is still starter-owned here and drops
            // (closes) only after the join above — the required order.
            drop(stop);
            return Err(classify_start_failure(
                outcome.map(|failed| failed.reason),
                exit_os_code,
            ));
        }
        Ok(Self {
            shared,
            rx,
            coalescer,
            root,
            generation: config.generation,
            join: Mutex::new(Some(join)),
            stop,
            ended: None,
            loss_signaled: false,
        })
    }

    /// Requests a clean stop and joins the worker. Idempotent: later calls
    /// re-observe the same sticky outcome. The starter-owned stop event stays
    /// valid across calls and is closed only after the join (on `Drop`), so
    /// stopping an already-exited worker is a harmless signal on a live
    /// event — never a use-after-close.
    pub fn stop(&mut self) -> Option<WatchOutcome> {
        self.shared.stopping.store(true, Ordering::Release);
        // SAFETY: signaling the starter-owned manual-reset event, which
        // outlives the join below even if the worker already exited.
        unsafe { SetEvent(self.stop.get()) };
        if let Ok(mut slot) = self.join.lock()
            && let Some(join) = slot.take()
        {
            let _ = join.join();
        }
        self.observe_worker_outcome();
        self.ended
    }

    /// Diagnostic counters snapshot. Counters only; no paths.
    pub fn stats(&self) -> WatcherStats {
        WatcherStats {
            dropped_raw_events: self.shared.dropped_raw_events.load(Ordering::Relaxed),
            notify_buffer_overflows: self.shared.notify_buffer_overflows.load(Ordering::Relaxed),
            obscured_events: self.shared.obscured_events.load(Ordering::Relaxed),
        }
    }

    /// The watched root id.
    pub fn root_id(&self) -> RootId {
        self.root
    }

    /// The generation assigned by the caller for this watch attempt.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn observe_worker_outcome(&mut self) {
        if let Ok(mut slot) = self.shared.outcome.lock()
            && slot.is_some()
        {
            self.ended = slot.take();
        }
    }
}

impl Drop for HandleBoundWatcher {
    fn drop(&mut self) {
        // Best-effort clean stop; never panics. `stop()` joins the worker
        // first; the starter-owned stop event in `self.stop` closes only
        // afterwards when the fields drop — the required close-after-join
        // order.
        let _ = self.stop();
    }
}

impl WatcherPort for HandleBoundWatcher {
    fn poll_hints(&mut self, now: u64) -> Vec<WatchHint> {
        // Relative-path bytes and action codes are intentionally not
        // inspected here: hints carry only the fact of activity.
        while self.rx.try_recv().is_ok() {
            self.coalescer
                .record_activity(self.root, self.generation, now);
        }
        if self.shared.coverage_lost.swap(false, Ordering::AcqRel) {
            self.coalescer
                .record_coverage_lost(self.root, self.generation);
        }
        self.observe_worker_outcome();
        if self.ended.is_some() && !self.loss_signaled {
            // An ended watch means coverage from watching is gone until the
            // consumer restarts it: surface one final coverage-loss hint.
            self.loss_signaled = true;
            self.coalescer
                .record_coverage_lost(self.root, self.generation);
        }
        self.coalescer.poll(now)
    }

    fn outcome(&mut self) -> Option<WatchOutcome> {
        self.observe_worker_outcome();
        self.ended
    }
}

/// Monotonic nanoseconds since first call within the process, for callers
/// that do not own a monotonic source yet. Convenience only; the port stays
/// deterministic because timestamps are always caller-supplied.
pub fn monotonic_nanos() -> u64 {
    use std::sync::OnceLock;
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod platform_unit_tests {
    use super::{
        ERROR_ACCESS_DENIED, ERROR_BAD_NET_NAME, ERROR_BAD_NETPATH, ERROR_DELETE_PENDING,
        ERROR_DIRECTORY, ERROR_FILE_NOT_FOUND, ERROR_INVALID_HANDLE, ERROR_NETNAME_DELETED,
        ERROR_NOT_READY, classify_start_failure, terminal_reason,
    };
    use crate::{EndReason, StartError};
    use windows_sys::Win32::Foundation::{ERROR_PATH_NOT_FOUND, ERROR_SHARING_VIOLATION};

    #[test]
    fn terminal_reason_maps_root_lost_codes() {
        for code in [
            ERROR_ACCESS_DENIED,
            ERROR_DELETE_PENDING,
            ERROR_NOT_READY,
            ERROR_BAD_NETPATH,
            ERROR_BAD_NET_NAME,
            ERROR_NETNAME_DELETED,
            ERROR_FILE_NOT_FOUND,
            ERROR_PATH_NOT_FOUND,
            ERROR_INVALID_HANDLE,
            ERROR_DIRECTORY,
        ] {
            assert_eq!(
                terminal_reason(code),
                EndReason::RootLost,
                "os_code {code} must classify as RootLost"
            );
        }
    }

    #[test]
    fn terminal_reason_keeps_sharing_violation_as_watch_failed() {
        // Audit: a handle-open conflict is not a lost root and must not
        // trigger root-loss reconciliation.
        assert_eq!(
            terminal_reason(ERROR_SHARING_VIOLATION),
            EndReason::WatchFailed {
                os_code: ERROR_SHARING_VIOLATION
            }
        );
        assert_eq!(
            terminal_reason(0xDEAD),
            EndReason::WatchFailed { os_code: 0xDEAD }
        );
    }

    #[test]
    fn start_failure_preserves_root_lost_instead_of_resource_unavailable() {
        // Pre-arm RootLost keeps the worker-captured code as RootUnavailable.
        assert_eq!(
            classify_start_failure(Some(EndReason::RootLost), ERROR_FILE_NOT_FOUND),
            StartError::RootUnavailable {
                os_code: ERROR_FILE_NOT_FOUND
            }
        );
        assert_eq!(
            classify_start_failure(Some(EndReason::RootLost), ERROR_DIRECTORY),
            StartError::RootUnavailable {
                os_code: ERROR_DIRECTORY
            }
        );
        // WatchFailed keeps its own code.
        assert_eq!(
            classify_start_failure(
                Some(EndReason::WatchFailed {
                    os_code: ERROR_SHARING_VIOLATION
                }),
                0
            ),
            StartError::RootUnavailable {
                os_code: ERROR_SHARING_VIOLATION
            }
        );
        // Clean stop or no outcome stays a resource failure, never a
        // misclassified root loss.
        assert_eq!(
            classify_start_failure(Some(EndReason::Stopped), 0),
            StartError::ResourceUnavailable { os_code: 0 }
        );
        assert_eq!(
            classify_start_failure(None, 0),
            StartError::ResourceUnavailable { os_code: 0 }
        );
    }
}
