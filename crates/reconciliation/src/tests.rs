use super::*;

fn metadata(id: Option<u128>, size: u64) -> Metadata {
    Metadata {
        size,
        modified_ns: size as i128,
        identity: id.map(|file| Identity { volume: 1, file }),
    }
}
fn seen(path: &str, id: Option<u128>, size: u64) -> Observation {
    Observation {
        path: path.into(),
        metadata: metadata(id, size),
    }
}
fn initial(observed: &[Observation]) -> Vec<Location> {
    reconcile(&[], observed, Outcome::Complete)
        .unwrap()
        .locations
}

#[test]
fn unchanged_tree_is_idempotent_and_order_independent() {
    let files = [seen("b.flp", Some(2), 2), seen("a.flp", Some(1), 1)];
    let before = initial(&files);
    let plan = reconcile(&before, &files, Outcome::Complete).unwrap();
    assert!(plan.changes.is_empty());
    assert_eq!(plan.locations, before);
    let reversed = [files[1].clone(), files[0].clone()];
    assert_eq!(
        reconcile(&before, &reversed, Outcome::Complete).unwrap(),
        plan
    );
}

#[test]
fn additions_and_modifications_preserve_other_locations() {
    let before = initial(&[seen("a.flp", Some(1), 1), seen("b.flp", Some(2), 2)]);
    let files = [
        seen("a.flp", Some(1), 3),
        seen("b.flp", Some(2), 2),
        seen("c.flp", Some(3), 1),
    ];
    let plan = reconcile(&before, &files, Outcome::Complete).unwrap();
    assert_eq!(
        plan.changes,
        [
            Change::Modified("a.flp".into()),
            Change::Added("c.flp".into())
        ]
    );
    assert_eq!(plan.locations[1], before[1]);
}

#[test]
fn rename_retains_old_path_history_and_physical_evidence() {
    let before = initial(&[seen("old.flp", Some(1), 1)]);
    let files = [seen("new.flp", Some(1), 1)];
    let plan = reconcile(&before, &files, Outcome::Complete).unwrap();
    assert!(plan.changes.contains(&Change::RenameEvidence {
        from: "old.flp".into(),
        to: "new.flp".into()
    }));
    assert_eq!(plan.locations.len(), 2);
    assert!(plan.locations[0].present);
    assert!(!plan.locations[1].present);
    assert!(
        reconcile(&plan.locations, &files, Outcome::Complete)
            .unwrap()
            .changes
            .is_empty()
    );
}

#[test]
fn stable_path_with_new_identity_is_replacement_even_with_identical_metadata() {
    let before = initial(&[seen("a.flp", Some(1), 1)]);
    let plan = reconcile(&before, &[seen("a.flp", Some(2), 1)], Outcome::Complete).unwrap();
    assert_eq!(plan.changes, [Change::Replaced("a.flp".into())]);
    assert_eq!(
        plan.locations[0].metadata.identity,
        metadata(Some(2), 1).identity
    );
}

#[test]
fn every_incomplete_outcome_rejects_positive_and_absence_changes() {
    let before = initial(&[seen("a.flp", Some(1), 1), seen("b.flp", Some(2), 1)]);
    let retained = before.clone();
    for outcome in [
        Outcome::Partial,
        Outcome::Cancelled,
        Outcome::Offline,
        Outcome::Denied,
        Outcome::ResourceLimit,
    ] {
        for files in [
            vec![],
            vec![seen("a.flp", Some(1), 9), seen("new.flp", Some(3), 1)],
        ] {
            assert_eq!(
                reconcile(&before, &files, outcome),
                Err(Rejected::Incomplete(outcome))
            );
            assert_eq!(before, retained);
        }
    }
}

#[test]
fn authoritative_empty_tree_marks_missing_once_and_restore_is_per_path() {
    let before = initial(&[seen("a.flp", Some(1), 1)]);
    let missing = reconcile(&before, &[], Outcome::Complete).unwrap();
    assert_eq!(missing.changes, [Change::Missing("a.flp".into())]);
    assert!(
        reconcile(&missing.locations, &[], Outcome::Complete)
            .unwrap()
            .changes
            .is_empty()
    );
    let restored = reconcile(
        &missing.locations,
        &[seen("a.flp", Some(1), 1)],
        Outcome::Complete,
    )
    .unwrap();
    assert_eq!(restored.changes, [Change::Restored("a.flp".into())]);
    assert!(restored.locations[0].present);
}

#[test]
fn restored_path_can_also_be_replaced() {
    let before = initial(&[seen("a.flp", Some(1), 1)]);
    let missing = reconcile(&before, &[], Outcome::Complete).unwrap();
    let restored = reconcile(
        &missing.locations,
        &[seen("a.flp", Some(2), 1)],
        Outcome::Complete,
    )
    .unwrap();
    assert_eq!(
        restored.changes,
        [
            Change::Restored("a.flp".into()),
            Change::Replaced("a.flp".into())
        ]
    );
}

#[test]
fn hardlink_removal_preserves_surviving_alias_identity() {
    let files = [seen("a.flp", Some(1), 1), seen("b.flp", Some(1), 1)];
    let before = initial(&files);
    let plan = reconcile(&before, &files[1..], Outcome::Complete).unwrap();
    assert_eq!(plan.changes, [Change::Missing("a.flp".into())]);
    assert_eq!(plan.locations[1], before[1]);
    assert_eq!(
        plan.locations[0].metadata.identity,
        plan.locations[1].metadata.identity
    );
}

#[test]
fn alias_churn_does_not_claim_rename() {
    let before = initial(&[seen("a.flp", Some(1), 1), seen("b.flp", Some(1), 1)]);
    let plan = reconcile(
        &before,
        &[seen("b.flp", Some(1), 1), seen("c.flp", Some(1), 1)],
        Outcome::Complete,
    )
    .unwrap();
    assert!(
        !plan
            .changes
            .iter()
            .any(|change| matches!(change, Change::RenameEvidence { .. }))
    );
}

#[test]
fn fallback_never_infers_rename_from_size_time_or_name() {
    let before = initial(&[seen("a.flp", None, 1)]);
    let plan = reconcile(&before, &[seen("b.flp", None, 1)], Outcome::Complete).unwrap();
    assert_eq!(
        plan.changes,
        [
            Change::Missing("a.flp".into()),
            Change::Added("b.flp".into())
        ]
    );
    assert!(
        reconcile(&before, &[seen("a.flp", None, 1)], Outcome::Complete)
            .unwrap()
            .changes
            .is_empty()
    );
}

#[test]
fn stale_historical_identity_does_not_prove_a_rename() {
    let before = initial(&[seen("a.flp", Some(1), 1)]);
    let missing = reconcile(&before, &[], Outcome::Complete).unwrap();
    let plan = reconcile(
        &missing.locations,
        &[seen("b.flp", Some(1), 1)],
        Outcome::Complete,
    )
    .unwrap();
    assert_eq!(plan.changes, [Change::Added("b.flp".into())]);
}

#[test]
fn duplicate_and_invalid_paths_reject_the_whole_run_without_path_diagnostics() {
    let a = seen("a.flp", Some(1), 1);
    assert_eq!(
        reconcile(&[], &[a.clone(), a.clone()], Outcome::Complete),
        Err(Rejected::DuplicatePath)
    );
    let prior = initial(&[a]);
    assert_eq!(
        reconcile(
            &[prior[0].clone(), prior[0].clone()],
            &[],
            Outcome::Complete
        ),
        Err(Rejected::DuplicatePath)
    );
    for path in [
        "",
        "../a.flp",
        "C:\\private\\a.flp",
        "/a.flp",
        "a//b.flp",
        "a/./b.flp",
    ] {
        assert_eq!(
            reconcile(&[], &[seen(path, None, 1)], Outcome::Complete),
            Err(Rejected::InvalidPath)
        );
    }
}

#[test]
fn resource_limits_reject_without_partial_output() {
    let files = vec![seen("a.flp", None, 1); MAX_RECORDS + 1];
    assert_eq!(
        reconcile(&[], &files, Outcome::Complete),
        Err(Rejected::ResourceLimit)
    );
    assert_eq!(
        reconcile(
            &[],
            &[seen(&"x".repeat(MAX_PATH_BYTES + 1), None, 1)],
            Outcome::Complete
        ),
        Err(Rejected::ResourceLimit)
    );
}
