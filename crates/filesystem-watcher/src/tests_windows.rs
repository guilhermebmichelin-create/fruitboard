//! Windows integration tests: real handle lifecycle, delivery, coalescing,
//! overflow signals, and policy exclusions against a real NTFS temp tree.
//! Fixtures that require environments CI cannot verify (DriveFS, network
//! shares, ACL revocation, real OS buffer-overflow timing) are `#[ignore]`d
//! and explicitly labeled unverified; they are never evidence of support.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64};
use std::sync::mpsc::sync_channel;
use std::time::{Duration, Instant};

use super::path::RawEvent;
use super::platform::{HandleBoundWatcher, Shared, WatcherConfig, monotonic_nanos};
use super::{EndReason, HintKind, RootId, StartError, WatchHint, WatchOutcome, WatcherPort};

fn watcher_config(generation: u64, window_ns: u64, raw_queue_bound: usize) -> WatcherConfig {
    WatcherConfig {
        generation,
        window: window_ns,
        raw_queue_bound,
        notify_buffer_bytes: 64 * 1024,
        validity_poll_ms: 50,
    }
}

fn temp_root(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("fruitboard-watcher-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp root must be creatable");
    dir
}

fn wait_for_hint(
    watcher: &mut HandleBoundWatcher,
    deadline: Duration,
    wanted: impl Fn(&WatchHint) -> bool,
) -> Option<WatchHint> {
    let limit = Instant::now() + deadline;
    loop {
        for hint in watcher.poll_hints(monotonic_nanos()) {
            if wanted(&hint) {
                return Some(hint);
            }
        }
        if Instant::now() >= limit {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn wait_for_outcome(watcher: &mut HandleBoundWatcher, deadline: Duration) -> Option<WatchOutcome> {
    let limit = Instant::now() + deadline;
    loop {
        if let Some(outcome) = watcher.outcome() {
            return Some(outcome);
        }
        if Instant::now() >= limit {
            return None;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn live_watch_delivers_coalesced_hints_and_stops_cleanly() {
    let dir = temp_root("coalesced");
    let mut watcher = HandleBoundWatcher::start(
        RootId(1),
        dir.as_os_str(),
        watcher_config(11, 2_000_000_000, 1024),
    )
    .expect("live watch must start");
    assert_eq!(watcher.root_id(), RootId(1));
    assert_eq!(watcher.generation(), 11);

    for index in 0..200 {
        std::fs::write(dir.join(format!("probe-{index}.flp")), b"hint")
            .expect("probe files must be writable");
    }

    let hint = wait_for_hint(&mut watcher, Duration::from_secs(60), |hint| {
        hint.kind == HintKind::ReconciliationRequested
    })
    .expect("activity must produce at least one coalesced hint within the deadline");
    assert_eq!(hint.root, RootId(1));
    assert_eq!(hint.generation, 11);

    let outcome = watcher.stop();
    assert_eq!(
        outcome,
        Some(WatchOutcome {
            root: RootId(1),
            generation: 11,
            reason: EndReason::Stopped
        })
    );
    // stop() is idempotent and keeps reporting the same typed outcome.
    assert_eq!(watcher.stop(), outcome);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn overflow_surfaces_as_an_immediate_coverage_lost_hint() {
    let dir = temp_root("overflow");
    // Capacity 1 and the minimum buffer size guarantee misses on both
    // paths: rapid creations overflow the OS notification buffer and/or
    // fill the queue, and the test polls only after creating them.
    let mut watcher = HandleBoundWatcher::start(
        RootId(2),
        dir.as_os_str(),
        WatcherConfig {
            generation: 3,
            window: 1_000_000_000,
            raw_queue_bound: 1,
            notify_buffer_bytes: 4096,
            validity_poll_ms: 50,
        },
    )
    .expect("live watch must start");
    for index in 0..300 {
        std::fs::write(dir.join(format!("burst-{index}.flp")), b"burst")
            .expect("burst files must be writable");
    }

    let hint = wait_for_hint(&mut watcher, Duration::from_secs(60), |hint| {
        hint.kind == HintKind::CoverageLost
    })
    .expect("missed events must raise the coverage-loss signal");
    assert_eq!(hint.root, RootId(2));
    assert_eq!(hint.generation, 3);
    let stats = watcher.stats();
    assert!(
        stats.dropped_raw_events + stats.notify_buffer_overflows >= 1,
        "at least one miss counter must be visible: {stats:?}"
    );

    assert_eq!(
        watcher.stop().map(|outcome| outcome.reason),
        Some(EndReason::Stopped)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn renaming_the_root_ends_the_watch_with_a_typed_root_lost_outcome() {
    let dir = temp_root("rename");
    let renamed = dir.parent().unwrap().join(format!(
        "{}-renamed",
        dir.file_name().unwrap().to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&renamed);
    let mut watcher = HandleBoundWatcher::start(
        RootId(3),
        dir.as_os_str(),
        watcher_config(5, 1_000_000_000, 64),
    )
    .expect("live watch must start");

    std::fs::rename(&dir, &renamed).expect("an open watch handle must not block renaming the root");

    let outcome = wait_for_outcome(&mut watcher, Duration::from_secs(30))
        .expect("the renamed root must end the watch");
    assert_eq!(outcome.reason, EndReason::RootLost);
    assert_eq!(outcome.generation, 5);

    // The ended watch must also surface one final coverage-loss hint so
    // consumers reconcile instead of trusting a dead watch.
    let hint = wait_for_hint(&mut watcher, Duration::from_secs(10), |hint| {
        hint.kind == HintKind::CoverageLost
    })
    .expect("the ended watch must surface a final coverage-loss hint");
    assert_eq!(hint.generation, 5);

    assert_eq!(
        watcher.stop().map(|outcome| outcome.reason),
        Some(EndReason::RootLost)
    );
    let _ = std::fs::remove_dir_all(&renamed);
}

#[test]
fn deleting_the_root_ends_the_watch_with_a_typed_root_lost_outcome() {
    let dir = temp_root("delete");
    let mut watcher = HandleBoundWatcher::start(
        RootId(4),
        dir.as_os_str(),
        watcher_config(6, 1_000_000_000, 64),
    )
    .expect("live watch must start");

    // The open handle uses FILE_SHARE_DELETE, so the removal succeeds and
    // the directory becomes pending-delete; the presence re-check then
    // observes that the configured root is gone.
    std::fs::remove_dir(&dir).expect("removal must be permitted by the share mode");

    let outcome = wait_for_outcome(&mut watcher, Duration::from_secs(30))
        .expect("the deleted root must end the watch");
    assert_eq!(outcome.reason, EndReason::RootLost);
    assert_eq!(
        watcher.stop().map(|outcome| outcome.reason),
        Some(EndReason::RootLost)
    );
}

#[test]
fn policy_exclusions_are_typed_and_distinct_from_io_failures() {
    let target = temp_root("junction-target");
    let link = target.parent().unwrap().join(format!(
        "{}-link",
        target.file_name().unwrap().to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&link);
    use std::os::windows::process::CommandExt;
    let output = Command::new("cmd")
        .arg("/C")
        .raw_arg(format!(
            "mklink /J \"{}\" \"{}\"",
            link.display(),
            target.display()
        ))
        .output()
        .expect("mklink must be invocable");
    assert!(
        output.status.success(),
        "junction fixture must be creatable: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // A reparse-point root is a policy exclusion, not an I/O failure.
    assert!(matches!(
        HandleBoundWatcher::start(
            RootId(5),
            link.as_os_str(),
            watcher_config(1, 1_000_000_000, 64)
        ),
        Err(StartError::ReparseRootExcluded)
    ));

    // A file root is an I/O failure classification.
    let file = target.join("plain.txt");
    std::fs::write(&file, b"not a directory").unwrap();
    assert!(matches!(
        HandleBoundWatcher::start(
            RootId(5),
            file.as_os_str(),
            watcher_config(1, 1_000_000_000, 64)
        ),
        Err(StartError::NotADirectory)
    ));

    // A missing root is an I/O failure classification.
    let missing = target.join("does-not-exist");
    assert!(matches!(
        HandleBoundWatcher::start(
            RootId(5),
            missing.as_os_str(),
            watcher_config(1, 1_000_000_000, 64)
        ),
        Err(StartError::RootUnavailable { .. })
    ));

    let _ = std::fs::remove_dir_all(&link);
    let _ = std::fs::remove_dir_all(&target);
}

#[test]
fn restart_uses_a_fresh_handle_and_generation() {
    let dir = temp_root("restart");
    {
        let mut watcher = HandleBoundWatcher::start(
            RootId(7),
            dir.as_os_str(),
            watcher_config(1, 500_000_000, 64),
        )
        .expect("first watch must start");
        std::fs::write(dir.join("first.flp"), b"one").unwrap();
        let hint = wait_for_hint(&mut watcher, Duration::from_secs(60), |hint| {
            hint.kind == HintKind::ReconciliationRequested
        })
        .expect("the first generation must deliver a hint");
        assert_eq!(hint.generation, 1);
        assert_eq!(
            watcher.stop().map(|outcome| outcome.reason),
            Some(EndReason::Stopped)
        );
    }
    {
        let mut watcher = HandleBoundWatcher::start(
            RootId(7),
            dir.as_os_str(),
            watcher_config(2, 500_000_000, 64),
        )
        .expect("restarted watch must start with a fresh handle");
        std::fs::write(dir.join("second.flp"), b"two").unwrap();
        let hint = wait_for_hint(&mut watcher, Duration::from_secs(60), |hint| {
            hint.kind == HintKind::ReconciliationRequested
        })
        .expect("the fresh generation must deliver a hint");
        assert_eq!(
            hint.generation, 2,
            "stale-generation hints must never reappear"
        );
        assert_eq!(
            watcher.stop().map(|outcome| outcome.reason),
            Some(EndReason::Stopped)
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn raw_queue_drop_policy_counts_and_flags_coverage_loss() {
    let (queue_tx, rx) = sync_channel(1);
    let shared = Shared {
        root: RootId(9),
        generation: 1,
        queue_tx,
        stopping: AtomicBool::new(false),
        armed: AtomicBool::new(false),
        coverage_lost: AtomicBool::new(false),
        outcome: Mutex::new(None),
        exit_os_code: AtomicU32::new(0),
        dropped_raw_events: AtomicU64::new(0),
        notify_buffer_overflows: AtomicU64::new(0),
        obscured_events: AtomicU64::new(0),
    };
    shared.try_push(RawEvent::Obscured); // fills the single slot
    shared.try_push(RawEvent::Obscured); // dropped with a counter
    assert_eq!(
        shared
            .dropped_raw_events
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
    assert!(
        shared
            .coverage_lost
            .load(std::sync::atomic::Ordering::SeqCst)
    );
    drop(rx); // disconnected queues take the same drop path
    shared.try_push(RawEvent::Obscured);
    assert_eq!(
        shared
            .dropped_raw_events
            .load(std::sync::atomic::Ordering::SeqCst),
        2
    );
}

// --- Regression: starter-owned stop event is safe after worker exit (P1) ---
//
// The stop event is owned by `HandleBoundWatcher` and closed only after the
// worker is joined, so signaling an already-exited worker must be a harmless
// no-op on a live event — never a use-after-close — and must keep reporting
// the same sticky outcome.

#[test]
fn stop_after_worker_exit_is_safe_and_sticky() {
    let dir = temp_root("stop-after-exit");
    let mut watcher = HandleBoundWatcher::start(
        RootId(20),
        dir.as_os_str(),
        watcher_config(1, 1_000_000_000, 64),
    )
    .expect("live watch must start");

    std::fs::remove_dir(&dir).expect("removal must be permitted by the share mode");
    let outcome = wait_for_outcome(&mut watcher, Duration::from_secs(30))
        .expect("the deleted root must end the watch");
    assert_eq!(outcome.reason, EndReason::RootLost);

    // Stopping an already-exited worker must not hang, crash, or reclassify:
    // repeated stops keep reporting the same sticky RootLost outcome.
    assert_eq!(
        watcher.stop().map(|outcome| outcome.reason),
        Some(EndReason::RootLost)
    );
    assert_eq!(
        watcher.stop().map(|outcome| outcome.reason),
        Some(EndReason::RootLost)
    );
    assert_eq!(
        watcher.outcome().map(|outcome| outcome.reason),
        Some(EndReason::RootLost)
    );
}

#[test]
fn start_failure_after_thread_spawn_never_touches_a_closed_handle() {
    // Best-effort race for the pre-arm window (root deleted between
    // `CreateFileW` and the first read): a deleter thread removes the root
    // while `start()` is opening it. Either the start fails — which must
    // classify as `RootUnavailable` (never `ResourceUnavailable{0}`) without
    // hanging or crashing — or it succeeds, in which case the watch must end
    // with `RootLost` and stop cleanly via the P1 path above. The
    // deterministic classification contract itself is pinned by
    // `platform_unit_tests::start_failure_preserves_root_lost_instead_of_resource_unavailable`.
    let mut saw_root_loss = false;
    for attempt in 0..20 {
        let dir = temp_root(&format!("start-race-{attempt}"));
        let victim = dir.clone();
        let deleter = std::thread::spawn(move || std::fs::remove_dir_all(&victim).ok());
        let started = HandleBoundWatcher::start(
            RootId(21),
            dir.as_os_str(),
            watcher_config(1, 1_000_000_000, 64),
        );
        let _ = deleter.join();
        match started {
            Err(StartError::RootUnavailable { .. }) => {
                saw_root_loss = true;
            }
            Err(other) => {
                panic!("pre-arm failure must not lose the root-loss class: {other:?}");
            }
            Ok(mut watcher) => {
                let _ = std::fs::remove_dir_all(&dir);
                if let Some(outcome) = wait_for_outcome(&mut watcher, Duration::from_secs(30)) {
                    assert_eq!(outcome.reason, EndReason::RootLost);
                    saw_root_loss = true;
                }
                // Safe even if the worker already exited (P1).
                let _ = watcher.stop();
                let _ = std::fs::remove_dir_all(&dir);
            }
        }
    }
    assert!(
        saw_root_loss,
        "the delete-around-start scenario must surface at least one root-loss signal"
    );
}

// --- Explicitly unverified fixtures (never counted as support) ---

fn env_root(name: &str) -> Option<PathBuf> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
}

#[test]
#[ignore = "unverified (#47/#48): requires a manually provisioned network share; \
           network roots are not qualified by this crate"]
fn network_root_watch_fixture_is_unverified() {
    let Some(root) = env_root("FRUITBOARD_WATCHER_NETWORK_ROOT") else {
        return;
    };
    let mut watcher = HandleBoundWatcher::start(
        RootId(90),
        root.as_os_str(),
        watcher_config(1, 1_000_000_000, 64),
    )
    .expect("watch must start when the fixture exists");
    let _ = wait_for_outcome(&mut watcher, Duration::from_secs(5));
    let _ = watcher.stop();
}

#[test]
#[ignore = "manual-only, unverified (#47): DriveFS is not qualified; requires a manually \
           provisioned Google Drive filesystem root via FRUITBOARD_WATCHER_DRIVEFS_ROOT"]
fn drivefs_root_watch_fixture_is_unverified() {
    let Some(root) = env_root("FRUITBOARD_WATCHER_DRIVEFS_ROOT") else {
        return;
    };
    let mut watcher = HandleBoundWatcher::start(
        RootId(91),
        root.as_os_str(),
        watcher_config(1, 1_000_000_000, 64),
    )
    .expect("watch must start when the fixture exists");
    let _ = wait_for_outcome(&mut watcher, Duration::from_secs(5));
    let _ = watcher.stop();
}

#[test]
#[ignore = "unverified: ACL revocation mid-watch is environment-dependent and \
           must be observed manually before any claim is made"]
fn acl_revocation_fixture_is_unverified() {
    let Some(root) = env_root("FRUITBOARD_WATCHER_ACL_ROOT") else {
        return;
    };
    let mut watcher = HandleBoundWatcher::start(
        RootId(92),
        root.as_os_str(),
        watcher_config(1, 1_000_000_000, 64),
    )
    .expect("watch must start when the fixture exists");
    let _ = wait_for_outcome(&mut watcher, Duration::from_secs(5));
    let _ = watcher.stop();
}

#[test]
#[ignore = "unverified: real OS buffer-overflow timing is environment-dependent; \
           the hermetic drop and coverage-loss tests carry the contract"]
fn real_buffer_overflow_fixture_is_unverified() {
    let Some(root) = env_root("FRUITBOARD_WATCHER_OVERFLOW_ROOT") else {
        return;
    };
    let config = WatcherConfig {
        generation: 1,
        window: 1_000_000_000,
        raw_queue_bound: 8,
        notify_buffer_bytes: 4096,
        validity_poll_ms: 50,
    };
    let mut watcher = HandleBoundWatcher::start(RootId(93), root.as_os_str(), config)
        .expect("watch must start when the fixture exists");
    for index in 0..20_000 {
        let _ = std::fs::write(root.join(format!("overflow-{index}.flp")), b"overflow");
    }
    let _ = wait_for_hint(&mut watcher, Duration::from_secs(60), |hint| {
        hint.kind == HintKind::CoverageLost
    });
    let _ = watcher.stop();
}
