use super::*;
use crate::MIGRATIONS;
use crate::metadata::tests::{bytes, capabilities, reply, setup};
use crate::tests::TestDirectory;

fn fixture() -> (TestDirectory, Database, MetadataInput) {
    let directory = TestDirectory::new();
    let (db, input) = setup(&directory, MIGRATIONS.len());
    db.connection
        .execute_batch("UPDATE file_location SET identity_volume_serial='7',identity_file_id='11';")
        .unwrap();
    let input = db
        .capture_metadata_input(&input.root_id, "location")
        .unwrap();
    (directory, db, input)
}
fn ready(db: &Database, input: &MetadataInput) -> String {
    match db.analysis_request(input).unwrap() {
        AnalysisRequest::Ready { key } => key,
        AnalysisRequest::Blocked(reason) => panic!("unexpected fixed policy: {reason:?}"),
    }
}
fn fresh(db: &Database, input: &MetadataInput) -> MetadataInput {
    db.capture_metadata_input(&input.root_id, input.location_id())
        .unwrap()
}

#[test]
fn explicit_requests_reject_replays_and_preserve_the_three_attempt_budget() {
    let (_dir, mut db, input) = fixture();
    let session = db.start_analysis_session(1).unwrap();
    for attempt in 1..=3 {
        let key = ready(&db, &input);
        assert_eq!(
            db.request_analysis_job(&input, &key, attempt * 10).unwrap(),
            AnalysisRequestOutcome::Queued
        );
        let queued = db.analysis_status("location").unwrap().unwrap();
        assert_eq!(queued.attempt, attempt - 1);
        assert!(matches!(
            db.request_analysis_job(&input, &key, attempt * 10 + 1),
            Err(StorageError::Conflict)
        ));
        assert_eq!(
            db.analysis_status("location").unwrap().unwrap().job_id,
            queued.job_id
        );
        assert!(matches!(
            db.analysis_request(&input).unwrap(),
            AnalysisRequest::Blocked(AnalysisRequestBlock::Pending)
        ));
        let lease = db
            .claim_analysis_job(&session, attempt * 10 + 2)
            .unwrap()
            .unwrap();
        db.fail_analysis_job(&lease, AnalysisFailure::SourceUnavailable, attempt * 10 + 3)
            .unwrap();
        assert_eq!(
            db.analysis_status("location").unwrap().unwrap().attempt,
            attempt
        );
    }
    assert!(matches!(
        db.analysis_request(&input).unwrap(),
        AnalysisRequest::Blocked(AnalysisRequestBlock::AttemptLimit)
    ));
    let count: i64 = db
        .connection
        .query_row("SELECT count(*) FROM analysis_job", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn reanalysis_keeps_good_facts_and_history_through_queue_and_failure() {
    let (_dir, mut db, input) = fixture();
    let good = db
        .publish_metadata_snapshot(&input, &capabilities(), reply(), 1)
        .unwrap();
    let input = fresh(&db, &input);
    let session = db.start_analysis_session(2).unwrap();
    let key = ready(&db, &input);
    db.request_analysis_job(&input, &key, 3).unwrap();
    // Discovery normally skips fresh facts; an explicit queued attempt survives.
    db.discover_analysis_jobs(None, 128, 4).unwrap();
    let lease = db.claim_analysis_job(&session, 5).unwrap().unwrap();
    db.fail_analysis_job(&lease, AnalysisFailure::SourceUnavailable, 6)
        .unwrap();
    assert_eq!(
        db.current_metadata_snapshot("project")
            .unwrap()
            .unwrap()
            .header()
            .id,
        good.id
    );
    let key = ready(&db, &input);
    db.request_analysis_job(&input, &key, 7).unwrap();
    let lease = db.claim_analysis_job(&session, 8).unwrap().unwrap();
    assert_eq!(db.analysis_status("location").unwrap().unwrap().attempt, 2);
    let next = db
        .complete_analysis_job(
            &lease,
            &capabilities(),
            reply(),
            &fruitboard_flp_parser::sha256_hex(&bytes()),
            9,
        )
        .unwrap();
    assert_ne!(next.id, good.id);
    assert_eq!(
        db.current_metadata_snapshot("project")
            .unwrap()
            .unwrap()
            .header()
            .id,
        next.id
    );
    assert!(
        db.metadata_snapshot(&good.id)
            .unwrap()
            .unwrap()
            .payload_json()
            .is_some()
    );
}

#[test]
fn old_keys_fence_restored_source_revisions_identity_and_publication_order() {
    for transition in 0..5 {
        let (_dir, mut db, input) = fixture();
        let key = ready(&db, &input);
        match transition {
            0 => {
                db.set_scan_root_enabled_at(&input.root_id, false, 1)
                    .unwrap();
                db.set_scan_root_enabled_at(&input.root_id, true, 2)
                    .unwrap();
            }
            1 => {
                db.connection.execute_batch("UPDATE file_location SET modified_at_ns=modified_at_ns+1;UPDATE file_location SET modified_at_ns=modified_at_ns-1;").unwrap();
            }
            2 => {
                db.connection.execute_batch("UPDATE file_location SET identity_file_id='12';UPDATE file_location SET identity_file_id='11';").unwrap();
            }
            3 => {
                db.publish_metadata_snapshot(&input, &capabilities(), reply(), 1)
                    .unwrap();
            }
            _ => {
                db.connection.execute_batch("UPDATE project_file SET byte_size=byte_size+1;UPDATE project_file SET byte_size=byte_size-1;").unwrap();
            }
        }
        let current = fresh(&db, &input);
        assert!(
            matches!(
                db.request_analysis_job(&current, &key, 3),
                Err(StorageError::Conflict)
            ),
            "transition {transition}"
        );
        let next_key = ready(&db, &current);
        assert_ne!(key, next_key);
        db.request_analysis_job(&current, &next_key, 4).unwrap();
        assert_eq!(db.analysis_status("location").unwrap().unwrap().attempt, 0);
    }
}

#[test]
fn action_keys_do_not_authorize_aliases_and_pending_work_keeps_its_backoff() {
    let (_dir, mut db, input) = fixture();
    let key = ready(&db, &input);
    db.connection.execute("INSERT INTO file_location(id,project_file_id,scan_root_id,normalized_path,locator_key,relative_path,byte_size,modified_at_ms,modified_at_ns,created_at_ms,updated_at_ms,identity_volume_serial,identity_file_id) VALUES ('alias','project',?1,'v1:i:alias.flp','v1:i:alias.flp','alias.flp',?2,42,42000123,1,1,'7','11')",params![input.root_id,bytes().len() as i64]).unwrap();
    let alias = db.capture_metadata_input(&input.root_id, "alias").unwrap();
    assert!(matches!(
        db.request_analysis_job(&alias, &key, 1),
        Err(StorageError::Conflict)
    ));
    let session = db.start_analysis_session(1).unwrap();
    db.request_analysis_job(&input, &key, 2).unwrap();
    let lease = db.claim_analysis_job(&session, 3).unwrap().unwrap();
    db.fail_analysis_job(&lease, AnalysisFailure::ParserTransport, 4)
        .unwrap();
    let pending_key =
        super::key(&input, cell(&db.connection, "location").unwrap().as_ref()).unwrap();
    assert_eq!(
        db.request_analysis_job(&input, &pending_key, 5).unwrap(),
        AnalysisRequestOutcome::Blocked(AnalysisRequestBlock::Pending)
    );
    assert!(db.claim_analysis_job(&session, 6).unwrap().is_none());
    assert_eq!(db.analysis_status("location").unwrap().unwrap().attempt, 1);
}

#[test]
fn queue_capacity_is_rechecked_without_creating_unbounded_cells() {
    let (_dir, mut db, input) = fixture();
    let key = ready(&db, &input);
    for i in 0..128 {
        let name = format!("alias{i:03}.flp");
        let locator = format!("v1:i:{name}");
        db.connection.execute("INSERT INTO file_location(id,project_file_id,scan_root_id,normalized_path,locator_key,relative_path,byte_size,modified_at_ms,modified_at_ns,created_at_ms,updated_at_ms,identity_volume_serial,identity_file_id) VALUES (?1,'project',?2,?3,?3,?4,?5,42,42000123,1,1,'7','11')",params![format!("alias{i:03}"),input.root_id,locator,name,bytes().len() as i64]).unwrap();
    }
    db.discover_analysis_jobs(None, 128, 1).unwrap();
    assert_eq!(
        db.request_analysis_job(&input, &key, 2).unwrap(),
        AnalysisRequestOutcome::Blocked(AnalysisRequestBlock::QueueFull)
    );
    assert!(db.analysis_status("location").unwrap().is_none());
    let first = db.analysis_status("alias000").unwrap().unwrap();
    db.cancel_analysis_job(&first.job_id, 3).unwrap();
    assert_eq!(
        db.request_analysis_job(&input, &key, 4).unwrap(),
        AnalysisRequestOutcome::Queued
    );
    let pending: i64 = db
        .connection
        .query_row(
            "SELECT count(*) FROM analysis_job WHERE state IN ('queued','running')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(pending, 128);
}

#[test]
fn unqualified_and_oversized_sources_cannot_be_requested() {
    let directory = TestDirectory::new();
    let (db, input) = setup(&directory, MIGRATIONS.len());
    assert!(matches!(
        db.analysis_request(&input).unwrap(),
        AnalysisRequest::Blocked(AnalysisRequestBlock::UnqualifiedSource)
    ));
    let (_dir, db, input) = fixture();
    db.connection
        .execute(
            "UPDATE file_location SET byte_size=?1",
            [MAX_FILE_BYTES as i64 + 1],
        )
        .unwrap();
    db.connection
        .execute(
            "UPDATE project_file SET byte_size=?1",
            [MAX_FILE_BYTES as i64 + 1],
        )
        .unwrap();
    let input = fresh(&db, &input);
    assert!(matches!(
        db.analysis_request(&input).unwrap(),
        AnalysisRequest::Blocked(AnalysisRequestBlock::TooLarge)
    ));
}

#[test]
fn ordinary_sizes_and_exact_ceiling_can_be_discovered_or_explicitly_requested() {
    for size in [4_601_596, 25_000_000, MAX_FILE_BYTES] {
        for explicit in [false, true] {
            let (_dir, mut db, input) = fixture();
            db.connection
                .execute("UPDATE file_location SET byte_size=?1", [size as i64])
                .unwrap();
            db.connection
                .execute("UPDATE project_file SET byte_size=?1", [size as i64])
                .unwrap();
            let input = fresh(&db, &input);
            let key = ready(&db, &input);
            if explicit {
                assert_eq!(
                    db.request_analysis_job(&input, &key, 2).unwrap(),
                    AnalysisRequestOutcome::Queued
                );
            } else {
                db.discover_analysis_jobs(None, 128, 2).unwrap();
            }
            let queued = db.analysis_status("location").unwrap().unwrap();
            assert_eq!(queued.state, AnalysisState::Queued);
            assert_eq!(queued.attempt, 0);
        }
    }
    let (_dir, mut db, input) = fixture();
    db.connection
        .execute(
            "UPDATE file_location SET byte_size=?1",
            [MAX_FILE_BYTES as i64 + 1],
        )
        .unwrap();
    db.connection
        .execute(
            "UPDATE project_file SET byte_size=?1",
            [MAX_FILE_BYTES as i64 + 1],
        )
        .unwrap();
    let input = fresh(&db, &input);
    db.discover_analysis_jobs(None, 128, 2).unwrap();
    assert!(db.analysis_status("location").unwrap().is_none());
    assert!(matches!(
        db.analysis_request(&input).unwrap(),
        AnalysisRequest::Blocked(AnalysisRequestBlock::TooLarge)
    ));
}

#[test]
fn explicit_cancellation_retry_does_not_reset_started_attempts() {
    let (_dir, mut db, input) = fixture();
    let session = db.start_analysis_session(1).unwrap();
    db.request_analysis_job(&input, &ready(&db, &input), 2)
        .unwrap();
    let first = db.analysis_status("location").unwrap().unwrap();
    db.cancel_analysis_job(&first.job_id, 3).unwrap();
    db.request_analysis_job(&input, &ready(&db, &input), 4)
        .unwrap();
    let lease = db.claim_analysis_job(&session, 5).unwrap().unwrap();
    db.fail_analysis_job(&lease, AnalysisFailure::Cancelled, 6)
        .unwrap();
    db.request_analysis_job(&input, &ready(&db, &input), 7)
        .unwrap();
    assert_eq!(db.analysis_status("location").unwrap().unwrap().attempt, 1);
    db.claim_analysis_job(&session, 8).unwrap().unwrap();
    assert_eq!(db.analysis_status("location").unwrap().unwrap().attempt, 2);
}

#[test]
fn only_a_new_source_generation_or_parser_can_replace_an_exhausted_budget() {
    for new_parser in [false, true] {
        let (_dir, mut db, input) = fixture();
        let session = db.start_analysis_session(1).unwrap();
        db.request_analysis_job(&input, &ready(&db, &input), 2)
            .unwrap();
        let lease = db.claim_analysis_job(&session, 3).unwrap().unwrap();
        db.fail_analysis_job(&lease, AnalysisFailure::SourceUnavailable, 4)
            .unwrap();
        db.connection
            .execute("UPDATE analysis_job SET attempt=3", [])
            .unwrap();
        assert!(matches!(
            db.analysis_request(&input).unwrap(),
            AnalysisRequest::Blocked(AnalysisRequestBlock::AttemptLimit)
        ));
        if new_parser {
            db.connection
                .execute(
                    "UPDATE analysis_job SET adapter_version='obsolete-reader'",
                    [],
                )
                .unwrap();
        } else {
            db.connection.execute_batch("UPDATE file_location SET modified_at_ns=modified_at_ns+1;UPDATE project_file SET modified_at_ns=modified_at_ns+1;").unwrap();
        }
        let current = fresh(&db, &input);
        let key = ready(&db, &current);
        db.request_analysis_job(&current, &key, 5).unwrap();
        assert_eq!(db.analysis_status("location").unwrap().unwrap().attempt, 0);
        db.claim_analysis_job(&session, 6).unwrap().unwrap();
        assert_eq!(db.analysis_status("location").unwrap().unwrap().attempt, 1);
    }
}
