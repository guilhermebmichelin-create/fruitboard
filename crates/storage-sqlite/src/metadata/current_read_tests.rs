use super::tests::{capabilities, reply, setup};
use super::*;
use crate::MIGRATIONS;
use crate::tests::TestDirectory;

fn allow_test_corruption(db: &Database) {
    // Only this disposable test database bypasses write protections so that
    // the readers encounter bytes the ordinary publication path rejects.
    db.connection
        .execute_batch(
            "DROP TRIGGER metadata_snapshot_no_update; PRAGMA ignore_check_constraints = ON;",
        )
        .unwrap();
}

#[test]
fn current_and_historical_reads_apply_identical_payload_validation() {
    for sql in [
        "UPDATE metadata_snapshot SET payload_json = '{'",
        "UPDATE metadata_snapshot SET payload_json = json_set(payload_json, '$.projectionVersion', 2)",
        "UPDATE metadata_snapshot SET payload_json = json_set(payload_json, '$.contentSha256', 'wrong')",
        "UPDATE metadata_snapshot SET payload_json = NULL",
        "UPDATE metadata_snapshot SET projection_version = 2",
        "UPDATE metadata_snapshot SET payload_json = json_set(payload_json, '$.padding', printf('%0262144d', 0))",
    ] {
        let directory = TestDirectory::new();
        let (mut db, input) = setup(&directory, MIGRATIONS.len());
        let header = db
            .publish_metadata_snapshot(&input, &capabilities(), reply(), 100)
            .unwrap();
        let historical = db.metadata_snapshot(&header.id).unwrap().unwrap();
        let current = db.current_metadata_snapshot("project").unwrap().unwrap();
        assert_eq!(current.header().id, historical.header().id);
        assert_eq!(current.payload_json(), historical.payload_json());

        allow_test_corruption(&db);
        db.connection.execute_batch(sql).unwrap();
        // An ID-only fence deliberately does not load or validate payloads.
        assert_eq!(
            db.current_metadata_snapshot_id("project").unwrap(),
            Some(header.id.clone())
        );
        assert!(matches!(
            db.metadata_snapshot(&header.id),
            Err(StorageError::InvalidSchema)
        ));
        assert!(matches!(
            db.current_metadata_snapshot("project"),
            Err(StorageError::InvalidSchema)
        ));

        db.set_scan_root_enabled(&input.root_id, false).unwrap();
        assert!(db.current_metadata_snapshot("project").unwrap().is_none());
        assert!(
            db.current_metadata_snapshot_id("project")
                .unwrap()
                .is_none()
        );
    }
}

#[test]
fn current_payload_reader_preserves_negative_outcome_validation() {
    let directory = TestDirectory::new();
    let (mut db, input) = setup(&directory, MIGRATIONS.len());
    let header = db
        .publish_metadata_snapshot(
            &input,
            &capabilities(),
            ProtocolReply::Rejected {
                code: "INVALID_PATH".into(),
            },
            100,
        )
        .unwrap();
    // Real publication never selects a rejection as the current result.
    assert!(db.current_metadata_snapshot("project").unwrap().is_none());
    assert!(
        db.metadata_snapshot(&header.id)
            .unwrap()
            .unwrap()
            .payload_json()
            .is_none()
    );
    // Construct the pointer explicitly to exercise the shared reader's
    // negative-outcome rules independently of the publication policy.
    db.connection
        .execute(
            "UPDATE project_file SET current_metadata_snapshot_id = ?1 WHERE id = 'project'",
            [&header.id],
        )
        .unwrap();
    let current = db.current_metadata_snapshot("project").unwrap().unwrap();
    assert_eq!(current.header().id, header.id);
    assert_eq!(current.header().outcome, MetadataOutcome::Rejected);
    assert!(current.payload_json().is_none());
    assert!(db.current_metadata_snapshot("unknown").unwrap().is_none());

    allow_test_corruption(&db);
    db.connection
        .execute(
            "UPDATE metadata_snapshot SET payload_json = '{\"projectionVersion\":1}'",
            [],
        )
        .unwrap();
    assert!(matches!(
        db.metadata_snapshot(&header.id),
        Err(StorageError::InvalidSchema)
    ));
    assert!(matches!(
        db.current_metadata_snapshot("project"),
        Err(StorageError::InvalidSchema)
    ));
}
