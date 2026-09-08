use super::*;
use std::collections::VecDeque;

use crate::coalescer::Coalescer;
use crate::path::{
    FILE_ACTION_ADDED, FILE_ACTION_MODIFIED, FILE_ACTION_REMOVED, FILE_ACTION_RENAMED_NEW_NAME,
    FILE_ACTION_RENAMED_OLD_NAME, MAX_RELATIVE_PATH_BYTES, parse_notify_buffer,
};

fn coalescer(window: u64, max_roots: usize) -> Coalescer {
    Coalescer::new(CoalescerConfig {
        window,
        max_tracked_roots: max_roots,
    })
    .unwrap()
}

fn utf16(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .flat_map(|unit| unit.to_le_bytes())
        .collect()
}

/// Builds one padded FILE_NOTIFY_INFORMATION record.
fn record(action: u32, name_utf16_bytes: &[u8]) -> Vec<u8> {
    let mut rec = Vec::with_capacity(12 + name_utf16_bytes.len());
    rec.extend_from_slice(&0u32.to_le_bytes());
    rec.extend_from_slice(&action.to_le_bytes());
    rec.extend_from_slice(&(name_utf16_bytes.len() as u32).to_le_bytes());
    rec.extend_from_slice(name_utf16_bytes);
    while rec.len() % 4 != 0 {
        rec.push(0);
    }
    rec
}

/// Links records into a chain by patching NextEntryOffset values: the offset
/// from each record's start to the next record's start is that record's own
/// (padded) length.
fn chain(mut records: Vec<Vec<u8>>) -> Vec<u8> {
    let mut buffer = Vec::new();
    for index in 0..records.len() {
        let offset = if index + 1 < records.len() {
            records[index].len() as u32
        } else {
            0
        };
        records[index][0..4].copy_from_slice(&offset.to_le_bytes());
        buffer.extend_from_slice(&records[index]);
    }
    buffer
}

// --- Coalescer: determinism, windows, bounds ---

#[test]
fn zero_window_or_zero_root_budget_is_rejected() {
    assert!(
        Coalescer::new(CoalescerConfig {
            window: 0,
            max_tracked_roots: 1
        })
        .is_err()
    );
    assert!(
        Coalescer::new(CoalescerConfig {
            window: 1,
            max_tracked_roots: 0
        })
        .is_err()
    );
}

#[test]
fn burst_inside_one_window_collapses_into_exactly_one_hint() {
    let mut coalescer = coalescer(1_000, 8);
    for tick in 0..50 {
        coalescer.record_activity(RootId(1), 1, tick);
    }
    assert!(
        coalescer.poll(999).is_empty(),
        "nothing is due before the window closes"
    );
    assert_eq!(
        coalescer.poll(1_000),
        [WatchHint {
            root: RootId(1),
            generation: 1,
            kind: HintKind::ReconciliationRequested
        }]
    );
    assert!(
        coalescer.poll(1_001).is_empty(),
        "the same burst never re-fires"
    );
}

#[test]
fn sustained_activity_yields_at_most_one_hint_per_window() {
    let mut coalescer = coalescer(100, 8);
    let mut hints = Vec::new();
    for tick in 0..1_000u64 {
        coalescer.record_activity(RootId(1), 1, tick);
        hints.extend(coalescer.poll(tick));
    }
    hints.extend(coalescer.poll(1_000));
    let reconciliations = hints
        .iter()
        .filter(|hint| hint.kind == HintKind::ReconciliationRequested)
        .count();
    // A 100-tick window over 1,000 ticks can fire at most ten times; the
    // first window and final drain produce the small slack on both ends.
    assert!(
        (5..=12).contains(&reconciliations),
        "at most one hint per window, no runaway: {reconciliations}"
    );
}

#[test]
fn coverage_lost_is_immediate_and_never_windowed() {
    let mut coalescer = coalescer(1_000_000, 8);
    coalescer.record_coverage_lost(RootId(1), 7);
    assert_eq!(
        coalescer.poll(0),
        [WatchHint {
            root: RootId(1),
            generation: 7,
            kind: HintKind::CoverageLost
        }]
    );
    assert!(
        coalescer.poll(0).is_empty(),
        "loss hints are sticky-until-delivered, not repeated"
    );
}

#[test]
fn coverage_lost_takes_precedence_and_subsumes_the_pending_window() {
    let mut coalescer = coalescer(1_000, 8);
    coalescer.record_activity(RootId(1), 1, 0);
    coalescer.record_coverage_lost(RootId(1), 1);
    let hints = coalescer.poll(5);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].kind, HintKind::CoverageLost);
    assert!(
        coalescer.poll(10_000).is_empty(),
        "the subsumed window must not emit again"
    );
}

#[test]
fn generation_restart_replaces_state_and_drops_stale_loss() {
    let mut coalescer = coalescer(1_000, 8);
    coalescer.record_activity(RootId(1), 1, 0);
    coalescer.record_coverage_lost(RootId(1), 1);
    // A fresh generation opens a fresh window; the stale loss is moot.
    coalescer.record_activity(RootId(1), 2, 10);
    assert!(coalescer.poll(20).is_empty());
    assert_eq!(
        coalescer.poll(1_010),
        [WatchHint {
            root: RootId(1),
            generation: 2,
            kind: HintKind::ReconciliationRequested
        }]
    );
}

#[test]
fn stale_generation_loss_for_a_tracked_root_is_ignored() {
    let mut coalescer = coalescer(1_000, 8);
    coalescer.record_activity(RootId(1), 2, 0);
    coalescer.record_coverage_lost(RootId(1), 1);
    assert_eq!(
        coalescer.poll(1_000),
        [WatchHint {
            root: RootId(1),
            generation: 2,
            kind: HintKind::ReconciliationRequested
        }]
    );
}

#[test]
fn tracking_bound_rejects_and_counts_without_panicking() {
    let mut coalescer = coalescer(1_000, 1);
    coalescer.record_activity(RootId(1), 1, 0);
    coalescer.record_activity(RootId(2), 1, 0);
    assert_eq!(coalescer.rejected_signals(), 1);
    assert_eq!(coalescer.open_windows(), 1);
    assert_eq!(coalescer.poll(1_000).len(), 1);
    // The freed slot accepts a new root again.
    coalescer.record_activity(RootId(2), 1, 1_000);
    assert_eq!(coalescer.open_windows(), 1);
    assert_eq!(coalescer.rejected_signals(), 1);
}

#[test]
fn tracking_bound_rejects_coverage_lost_without_growth() {
    // P2-09 bounded coalescing: the coverage-loss path respects the same
    // tracking bound as activity. A loss for an untracked root beyond the
    // bound is rejected and counted, never queued, so memory cannot grow
    // with overflow volume.
    let mut coalescer = coalescer(1_000, 1);
    coalescer.record_activity(RootId(1), 1, 0);
    coalescer.record_coverage_lost(RootId(2), 1);
    assert_eq!(coalescer.rejected_signals(), 1);
    assert_eq!(coalescer.open_windows(), 1);
    // Only the tracked root's window is due; the rejected loss never fires.
    let hints = coalescer.poll(1_000);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].root, RootId(1));
    assert_eq!(hints[0].kind, HintKind::ReconciliationRequested);
    // Draining frees the slot: a later loss for the new root is accepted.
    coalescer.record_coverage_lost(RootId(2), 1);
    assert_eq!(coalescer.rejected_signals(), 1);
    assert_eq!(
        coalescer.poll(1_000),
        [WatchHint {
            root: RootId(2),
            generation: 1,
            kind: HintKind::CoverageLost
        }]
    );
}

#[test]
fn poll_order_is_deterministic_ascending_by_root() {
    let mut coalescer = coalescer(1_000, 8);
    for root in [RootId(3), RootId(1), RootId(2)] {
        coalescer.record_activity(root, 1, 0);
    }
    coalescer.record_coverage_lost(RootId(5), 1);
    coalescer.record_coverage_lost(RootId(4), 1);
    let hints = coalescer.poll(1_000);
    let roots: Vec<RootId> = hints.iter().map(|hint| hint.root).collect();
    // Coverage-loss hints first, then coalesced windows; ascending by root.
    assert_eq!(
        roots,
        [RootId(4), RootId(5), RootId(1), RootId(2), RootId(3)]
    );
    assert_eq!(
        hints
            .iter()
            .filter(|hint| hint.kind == HintKind::CoverageLost)
            .count(),
        2
    );
}

#[test]
fn identical_scripts_produce_identical_hint_sequences() {
    let script = [
        (RootId(2), 1u64, 100u64),
        (RootId(1), 1, 120),
        (RootId(2), 1, 150),
        (RootId(1), 2, 300),
        (RootId(2), 2, 305),
    ];
    let run = || {
        let mut coalescer = coalescer(1_000, 8);
        for &(root, generation, at) in &script {
            coalescer.record_activity(root, generation, at);
            coalescer.poll(at);
        }
        coalescer.poll(10_000)
    };
    assert_eq!(run(), run());
}

// --- Relative path validation: no public type can carry absolute paths ---

#[test]
fn relative_path_rejects_absolute_and_traversal_shapes() {
    for (rejected, expected) in [
        ("C:\\Music\\song.flp", RelativePathRejected::ColonOrNul),
        ("C:/Music/song.flp", RelativePathRejected::ColonOrNul),
        (
            "\\\\?\\C:\\Music\\song.flp",
            RelativePathRejected::RootedOrAbsolute,
        ),
        ("\\leading.flp", RelativePathRejected::RootedOrAbsolute),
        ("/leading.flp", RelativePathRejected::RootedOrAbsolute),
        ("a/../b.flp", RelativePathRejected::DotComponent),
        ("./b.flp", RelativePathRejected::DotComponent),
        ("../b.flp", RelativePathRejected::DotComponent),
        ("a/b/", RelativePathRejected::DotComponent),
        ("a//b.flp", RelativePathRejected::DotComponent),
        ("stream:ads.flp", RelativePathRejected::ColonOrNul),
        ("a:b.flp", RelativePathRejected::ColonOrNul),
        ("", RelativePathRejected::Empty),
    ] {
        assert_eq!(
            RelativePath::from_bytes(rejected.as_bytes().to_vec()),
            Err(expected),
            "{rejected}"
        );
    }
    assert_eq!(
        RelativePath::from_bytes(vec![b'a', 0xFF]),
        Err(RelativePathRejected::NotUtf8)
    );
    assert_eq!(
        RelativePath::from_bytes(vec![b'a'; MAX_RELATIVE_PATH_BYTES + 1]),
        Err(RelativePathRejected::TooLong)
    );
}

#[test]
fn relative_path_accepts_ordinary_root_relative_names() {
    for accepted in [
        "a.flp",
        "project\\song.flp",
        "project/sub/song.flp",
        "проект\\песня.flp",
    ] {
        let path = RelativePath::from_bytes(accepted.as_bytes().to_vec())
            .unwrap_or_else(|_| panic!("{accepted} must validate"));
        assert_eq!(path.as_bytes(), accepted.as_bytes());
    }
}

#[test]
fn relative_path_debug_redacts_the_bytes() {
    let path = RelativePath::from_bytes(b"project\\secret-name.flp".to_vec()).unwrap();
    let debug = format!("{path:?}");
    assert_eq!(
        debug,
        format!("RelativePath({} bytes)", b"project\\secret-name.flp".len())
    );
    assert!(!debug.contains("project") && !debug.contains('\\'));
}

// --- Notification buffer parsing ---

#[test]
fn parse_walks_the_chain_and_maps_action_codes() {
    let buffer = chain(vec![
        record(FILE_ACTION_ADDED, &utf16("new.flp")),
        record(FILE_ACTION_RENAMED_OLD_NAME, &utf16("project\\old.flp")),
        record(
            FILE_ACTION_RENAMED_NEW_NAME,
            &utf16("project\\new-name.flp"),
        ),
        record(FILE_ACTION_MODIFIED, &utf16("touched.flp")),
        record(FILE_ACTION_REMOVED, &utf16("gone.flp")),
    ]);
    let batch = parse_notify_buffer(&buffer);
    assert_eq!(batch.events.len(), 5);
    assert_eq!(batch.obscured, 0);
    assert!(!batch.truncated);
    let actions: Vec<NotifyAction> = batch
        .events
        .iter()
        .map(|event| match event {
            RawEvent::Change { action, .. } => *action,
            RawEvent::Obscured => panic!("every record must surface"),
        })
        .collect();

    assert_eq!(
        actions,
        [
            NotifyAction::Added,
            NotifyAction::RenamedOldName,
            NotifyAction::RenamedNewName,
            NotifyAction::Modified,
            NotifyAction::Removed
        ]
    );
    match &batch.events[1] {
        RawEvent::Change { relative_path, .. } => {
            assert_eq!(relative_path.as_bytes(), "project\\old.flp".as_bytes())
        }
        RawEvent::Obscured => panic!("renames must surface"),
    }
}

#[test]
fn parse_drops_obscured_records_without_surfaces() {
    let buffer = chain(vec![
        record(9, &utf16("unknown-action.flp")),
        record(FILE_ACTION_ADDED, &utf16("C:\\absolute\\song.flp")),
        record(FILE_ACTION_ADDED, &utf16("a/../escape.flp")),
        record(FILE_ACTION_ADDED, &[0x61, 0x00, 0xD8]), // odd byte count
    ]);
    let batch = parse_notify_buffer(&buffer);
    // Every visited record becomes activity: one queued item each.
    assert_eq!(batch.events.len(), 4);
    assert_eq!(batch.obscured, 4);
    assert!(
        batch
            .events
            .iter()
            .all(|event| *event == RawEvent::Obscured)
    );
}

#[test]
fn parse_stops_on_malformed_chain_offsets_and_reports_truncation() {
    let mut buffer = chain(vec![record(FILE_ACTION_ADDED, &utf16("a.flp"))]);
    buffer[0..4].copy_from_slice(&3u32.to_le_bytes()); // not a valid offset
    assert!(parse_notify_buffer(&buffer).truncated);
    let mut buffer = chain(vec![record(FILE_ACTION_ADDED, &utf16("a.flp"))]);
    buffer[0..4].copy_from_slice(&4_096u32.to_le_bytes()); // overruns
    assert!(parse_notify_buffer(&buffer).truncated);
    let mut buffer = chain(vec![record(FILE_ACTION_ADDED, &utf16("a.flp"))]);
    buffer[8..12].copy_from_slice(&4_096u32.to_le_bytes()); // name overruns
    assert!(parse_notify_buffer(&buffer).truncated);
    let empty = parse_notify_buffer(&[]);
    assert!(empty.events.is_empty() && !empty.truncated);
}

// --- Privacy regression ---

#[test]
fn privacy_regression_no_public_type_can_carry_absolute_paths() {
    // Absolute-shaped names in raw notification bytes never surface as
    // change events: they are obscured activity, counted but not carried.
    let buffer = chain(vec![
        record(FILE_ACTION_ADDED, &utf16("C:\\Music\\artist\\song.flp")),
        record(FILE_ACTION_MODIFIED, &utf16("\\\\server\\share\\song.flp")),
    ]);
    let batch = parse_notify_buffer(&buffer);
    assert_eq!(batch.obscured, 2);
    for event in &batch.events {
        let debug = format!("{event:?}");
        assert!(
            !debug.contains('\\'),
            "path separator leaked into Debug: {debug}"
        );
        assert!(
            !debug.contains(".flp"),
            "name text leaked into Debug: {debug}"
        );
    }

    // Even a deliberately validated relative path is redacted in Debug.
    let event = RawEvent::Change {
        action: NotifyAction::Modified,
        relative_path: RelativePath::from_bytes(b"project\\song.flp".to_vec()).unwrap(),
    };
    let debug = format!("{event:?}");
    assert!(
        !debug.contains("project") && !debug.contains('\\'),
        "{debug}"
    );

    // Lifecycle, hint, and error types carry no path separators at all: they
    // are structurally incapable of holding a path.
    let rendered = [
        format!("{:?}", HintKind::ReconciliationRequested),
        format!("{:?}", HintKind::CoverageLost),
        format!(
            "{:?}",
            WatchHint {
                root: RootId(1),
                generation: 1,
                kind: HintKind::CoverageLost
            }
        ),
        format!("{:?}", EndReason::Stopped),
        format!("{:?}", EndReason::RootLost),
        format!("{:?}", EndReason::WatchFailed { os_code: 5 }),
        format!(
            "{:?}",
            WatchOutcome {
                root: RootId(1),
                generation: 1,
                reason: EndReason::RootLost
            }
        ),
        format!("{:?}", StartError::RootUnavailable { os_code: 5 }),
        format!("{:?}", StartError::NotADirectory),
        format!("{:?}", StartError::ReparseRootExcluded),
        format!("{:?}", StartError::InvalidRootPath),
        format!("{:?}", StartError::InvalidConfig),
        format!("{:?}", StartError::ResourceUnavailable { os_code: 5 }),
        format!("{:?}", RawEvent::Obscured),
    ];
    for text in rendered {
        assert!(
            !text.contains('\\') && !text.contains('/'),
            "path-like text leaked: {text}"
        );
    }
}

// --- WatcherPort with a fake consumer (no OS, no storage wiring) ---

/// A scripted stand-in for the platform watcher: it produces activity with a
/// fake clock, exactly like `HandleBoundWatcher` does with a real one.
struct ScriptedWatcher {
    script: VecDeque<(RootId, u64, u64)>,
    coalescer: Coalescer,
}

impl WatcherPort for ScriptedWatcher {
    fn poll_hints(&mut self, now: u64) -> Vec<WatchHint> {
        while let Some(&(root, generation, at)) = self.script.front() {
            if at > now {
                break;
            }
            self.coalescer.record_activity(root, generation, at);
            self.script.pop_front();
        }
        self.coalescer.poll(now)
    }

    fn outcome(&mut self) -> Option<WatchOutcome> {
        None
    }
}

/// A fake consumer of the port. The next slice replaces this with durable
/// reconciliation follow-up wiring; the contract below must not change.
struct FakeConsumer {
    port: Box<dyn WatcherPort>,
    delivered: Vec<WatchHint>,
}

impl FakeConsumer {
    fn drain_until(&mut self, end: u64, step: u64) {
        let mut now = 0;
        while now <= end {
            self.delivered.extend(self.port.poll_hints(now));
            now += step;
        }
    }
}

#[test]
fn fake_consumer_receives_at_most_one_reconcile_per_root_per_window() {
    let watcher = ScriptedWatcher {
        script: VecDeque::from([
            (RootId(1), 1, 10),
            (RootId(1), 1, 20),
            (RootId(1), 1, 30),
            (RootId(2), 1, 40),
            (RootId(1), 1, 50),
        ]),
        coalescer: coalescer(100, 8),
    };
    let mut consumer = FakeConsumer {
        port: Box::new(watcher),
        delivered: Vec::new(),
    };
    consumer.drain_until(1_000, 10);
    let reconciles: Vec<&WatchHint> = consumer
        .delivered
        .iter()
        .filter(|hint| hint.kind == HintKind::ReconciliationRequested)
        .collect();
    // Both roots' bursts collapse to one hint each; the at-most-one-per-
    // window invariant is per root, so two roots yield exactly two hints.
    assert_eq!(reconciles.len(), 2);
    assert!(
        reconciles
            .iter()
            .all(|hint| hint.kind == HintKind::ReconciliationRequested)
    );
    // Deterministic delivery order: ascending root id.
    assert_eq!(reconciles[0].root, RootId(1));
    assert_eq!(reconciles[1].root, RootId(2));
}
#[test]
fn coalescer_implements_the_watcher_port_contract() {
    let mut coalescer = coalescer(1_000, 8);
    coalescer.record_coverage_lost(RootId(9), 3);
    let mut port = Box::new(coalescer) as Box<dyn WatcherPort>;
    assert_eq!(
        port.poll_hints(0),
        [WatchHint {
            root: RootId(9),
            generation: 3,
            kind: HintKind::CoverageLost
        }]
    );
    assert_eq!(
        port.outcome(),
        None,
        "the pure coalescer has no lifecycle outcome"
    );
}
