use super::*;
use crate::metadata::tests::{bytes, capabilities, reply, setup};
use crate::tests::TestDirectory;
use crate::{MIGRATIONS, MetadataOutcome};
use fruitboard_flp_parser::{parse_bytes, sha256_hex};

fn fixture() -> (TestDirectory, Database, MetadataInput) {
    let directory = TestDirectory::new();
    let (db, input) = setup(&directory, MIGRATIONS.len());
    (directory, db, input)
}
fn discover(db: &mut Database, now: i64) {
    assert!(
        db.discover_analysis_jobs(None, MAX_ANALYSIS_BATCH, now)
            .unwrap()
            .is_none()
    );
}

#[test]
fn analysis_coalesces_bounds_ownership_and_atomically_publishes() {
    let (_dir, mut db, input) = fixture();
    let session = db.start_analysis_session(1).unwrap();
    discover(&mut db, 2);
    let first = db.analysis_status("location").unwrap().unwrap();
    discover(&mut db, 3);
    assert_eq!(
        db.analysis_status("location").unwrap().unwrap().job_id,
        first.job_id
    );
    let lease = db.claim_analysis_job(&session, 4).unwrap().unwrap();
    assert!(db.claim_analysis_job(&session, 5).unwrap().is_none());
    assert!(db.analysis_lease_current(&lease, 5).unwrap());
    let snapshot = db
        .complete_analysis_job(&lease, &capabilities(), reply(), &sha256_hex(&bytes()), 6)
        .unwrap();
    assert_eq!(snapshot.outcome, MetadataOutcome::Complete);
    let status = db.analysis_status("location").unwrap().unwrap();
    assert_eq!(status.state, AnalysisState::Complete);
    assert_eq!(status.snapshot_id.as_deref(), Some(snapshot.id.as_str()));
    assert_eq!(status.attempt, 1);
    assert!(matches!(
        db.complete_analysis_job(&lease, &capabilities(), reply(), &sha256_hex(&bytes()), 7),
        Err(StorageError::Conflict)
    ));
    discover(&mut db, 8);
    assert!(db.claim_analysis_job(&session, 8).unwrap().is_none());
    assert_eq!(
        db.current_metadata_snapshot(&input.project_file_id)
            .unwrap()
            .unwrap()
            .header()
            .id,
        snapshot.id
    );
}

#[test]
fn analysis_recovery_retains_budget_and_fences_previous_process() {
    let (_dir, mut db, _) = fixture();
    let mut session = db.start_analysis_session(1).unwrap();
    discover(&mut db, 2);
    let first = db.claim_analysis_job(&session, 3).unwrap().unwrap();
    for attempt in 1..=3 {
        session = db.start_analysis_session(4 + attempt).unwrap();
        assert!(!db.analysis_lease_current(&first, 5 + attempt).unwrap());
        assert!(matches!(
            db.complete_analysis_job(
                &first,
                &capabilities(),
                reply(),
                &sha256_hex(&bytes()),
                5 + attempt
            ),
            Err(StorageError::Conflict)
        ));
        let status = db.analysis_status("location").unwrap().unwrap();
        assert_eq!(status.attempt, attempt);
        if attempt < 3 {
            assert_eq!(status.state, AnalysisState::Queued);
            let _ = db
                .claim_analysis_job(&session, 5 + attempt)
                .unwrap()
                .unwrap();
        } else {
            assert_eq!(status.state, AnalysisState::Failed);
        }
    }
    discover(&mut db, 20);
    assert!(db.claim_analysis_job(&session, 20).unwrap().is_none());
}

#[test]
fn analysis_expired_cancelled_disabled_and_replaced_sources_cannot_commit() {
    for transition in 0..4 {
        let (_dir, mut db, input) = fixture();
        let session = db.start_analysis_session(1).unwrap();
        discover(&mut db, 2);
        let lease = db.claim_analysis_job(&session, 3).unwrap().unwrap();
        match transition {
            0 => {
                assert!(!db.analysis_lease_current(&lease, 60_004).unwrap());
                let replacement = db.claim_analysis_job(&session, 60_004).unwrap().unwrap();
                assert_ne!(replacement.token, lease.token);
            }
            1 => db.cancel_analysis_job(lease.job_id(), 4).unwrap(),
            2 => {
                db.set_scan_root_enabled_at(&input.root_id, false, 4)
                    .unwrap();
            }
            _ => {
                db.connection.execute_batch("UPDATE file_location SET modified_at_ns=modified_at_ns+1;UPDATE file_location SET modified_at_ns=modified_at_ns-1;").unwrap();
                discover(&mut db, 4);
            }
        }
        assert!(matches!(
            db.complete_analysis_job(
                &lease,
                &capabilities(),
                reply(),
                &sha256_hex(&bytes()),
                if transition == 0 { 60_005 } else { 5 }
            ),
            Err(StorageError::Conflict)
        ));
        assert!(db.current_metadata_snapshot("project").unwrap().is_none());
    }
}

#[test]
fn analysis_invalid_hash_and_terminal_write_failure_roll_back_everything() {
    let (_dir, mut db, _) = fixture();
    let session = db.start_analysis_session(1).unwrap();
    discover(&mut db, 2);
    let lease = db.claim_analysis_job(&session, 3).unwrap().unwrap();
    assert!(
        db.complete_analysis_job(&lease, &capabilities(), reply(), &"0".repeat(64), 4)
            .is_err()
    );
    db.connection.execute_batch("CREATE TRIGGER fail_analysis_terminal BEFORE UPDATE OF state ON analysis_job WHEN NEW.state='complete' BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(
        db.complete_analysis_job(&lease, &capabilities(), reply(), &sha256_hex(&bytes()), 4)
            .is_err()
    );
    let count: i64 = db
        .connection
        .query_row("SELECT count(*) FROM metadata_snapshot", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    assert!(db.current_metadata_snapshot("project").unwrap().is_none());
    assert!(db.analysis_lease_current(&lease, 5).unwrap());
    db.connection
        .execute_batch("DROP TRIGGER fail_analysis_terminal;")
        .unwrap();
    db.complete_analysis_job(&lease, &capabilities(), reply(), &sha256_hex(&bytes()), 6)
        .unwrap();
}

#[test]
fn analysis_transport_retries_are_bounded_and_explicit_cancel_survives_restart() {
    let (_dir, mut db, _) = fixture();
    let session = db.start_analysis_session(1).unwrap();
    discover(&mut db, 2);
    for attempt in 1..=3 {
        let now = attempt * 2_000;
        let lease = db.claim_analysis_job(&session, now).unwrap().unwrap();
        db.fail_analysis_job(&lease, AnalysisFailure::ParserTransport, now + 1)
            .unwrap();
        assert_eq!(
            db.analysis_status("location").unwrap().unwrap().attempt,
            attempt
        );
        assert!(db.claim_analysis_job(&session, now + 2).unwrap().is_none());
    }
    assert_eq!(
        db.analysis_status("location").unwrap().unwrap().state,
        AnalysisState::Failed
    );
    let (_dir, mut db, _) = fixture();
    let session = db.start_analysis_session(1).unwrap();
    discover(&mut db, 2);
    let status = db.analysis_status("location").unwrap().unwrap();
    db.cancel_analysis_job(&status.job_id, 3).unwrap();
    let next = db.start_analysis_session(4).unwrap();
    discover(&mut db, 5);
    assert!(db.claim_analysis_job(&next, 5).unwrap().is_none());
    assert!(matches!(
        db.claim_analysis_job(&session, 5),
        Err(StorageError::Conflict)
    ));
}

#[test]
fn analysis_failed_attempt_preserves_current_good_snapshot_and_does_not_loop_aliases() {
    let (_dir, mut db, input) = fixture();
    let good = db
        .publish_metadata_snapshot(&input, &capabilities(), reply(), 1)
        .unwrap();
    let input = db
        .capture_metadata_input(&input.root_id, "location")
        .unwrap();
    let failed = db
        .publish_metadata_snapshot(
            &input,
            &capabilities(),
            ProtocolReply::Result(parse_bytes(b"invalid")),
            2,
        )
        .unwrap();
    assert_eq!(failed.outcome, MetadataOutcome::Failed);
    assert_eq!(
        db.current_metadata_snapshot("project")
            .unwrap()
            .unwrap()
            .header()
            .id,
        good.id
    );
    db.connection.execute("INSERT INTO file_location(id,project_file_id,scan_root_id,normalized_path,locator_key,relative_path,byte_size,modified_at_ms,modified_at_ns,created_at_ms,updated_at_ms)
        VALUES ('second','project',?1,'v1:i:alias.flp','v1:i:alias.flp','alias.flp',?2,42,42000123,1,1)",params![input.root_id,bytes().len() as i64]).unwrap();
    let session = db.start_analysis_session(3).unwrap();
    discover(&mut db, 4);
    assert!(db.claim_analysis_job(&session, 5).unwrap().is_none());
    let count: i64 = db
        .connection
        .query_row("SELECT count(*) FROM analysis_job", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn analysis_discovery_visits_bounded_location_pages_and_caps_pending_cells() {
    let (_dir, mut db, input) = fixture();
    for i in 0..200 {
        let name = format!("alias{i:03}.flp");
        let key = format!("v1:i:{name}");
        db.connection.execute("INSERT INTO file_location(id,project_file_id,scan_root_id,normalized_path,locator_key,relative_path,byte_size,modified_at_ms,modified_at_ns,created_at_ms,updated_at_ms)
            VALUES (?1,'project',?2,?3,?3,?4,?5,42,42000123,1,1)",params![format!("location-{i:03}"),input.root_id,key,name,bytes().len() as i64]).unwrap();
    }
    let mut cursor = None;
    let mut pages = 0;
    loop {
        cursor = db.discover_analysis_jobs(cursor.as_ref(), 30, 1).unwrap();
        pages += 1;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(pages, 7);
    let count: i64 = db
        .connection
        .query_row(
            "SELECT count(*) FROM analysis_job WHERE state='queued'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 128);
    for limit in [0, MAX_ANALYSIS_BATCH + 1] {
        assert!(matches!(
            db.discover_analysis_jobs(None, limit, 1),
            Err(StorageError::InvalidCursor)
        ));
    }
}

#[test]
fn analysis_negative_alias_publications_do_not_revive_failed_or_cancelled_work() {
    for cancel_first in [false, true] {
        let (_dir, mut db, input) = fixture();
        db.connection.execute("INSERT INTO file_location(id,project_file_id,scan_root_id,normalized_path,locator_key,relative_path,byte_size,modified_at_ms,modified_at_ns,created_at_ms,updated_at_ms)
            VALUES ('second','project',?1,'v1:i:alias.flp','v1:i:alias.flp','alias.flp',?2,42,42000123,1,1)",params![input.root_id,bytes().len() as i64]).unwrap();
        let session = db.start_analysis_session(1).unwrap();
        discover(&mut db, 2);
        let cancelled_id = if cancel_first {
            let status = db.analysis_status("location").unwrap().unwrap();
            db.cancel_analysis_job(&status.job_id, 3).unwrap();
            Some(status.job_id)
        } else {
            None
        };
        let mut published = 0;
        for now in 4..16 {
            discover(&mut db, now);
            if let Some(lease) = db.claim_analysis_job(&session, now).unwrap() {
                db.complete_analysis_job(
                    &lease,
                    &capabilities(),
                    ProtocolReply::Result(parse_bytes(b"invalid")),
                    &sha256_hex(&bytes()),
                    now,
                )
                .unwrap();
                published += 1;
            }
        }
        assert_eq!(published, if cancel_first { 1 } else { 2 });
        assert!(db.current_metadata_snapshot("project").unwrap().is_none());
        assert!(db.claim_analysis_job(&session, 20).unwrap().is_none());
        if let Some(cancelled_id) = cancelled_id {
            let status = db.analysis_status("location").unwrap().unwrap();
            assert_eq!(status.state, AnalysisState::Cancelled);
            assert_eq!(status.job_id, cancelled_id);
            assert_eq!(status.attempt, 0);
        }
    }
}

#[test]
fn analysis_publication_fence_refresh_preserves_retry_budget_and_backoff() {
    let (_dir, mut db, input) = fixture();
    let session = db.start_analysis_session(1).unwrap();
    discover(&mut db, 2);
    for attempt in 1..=3 {
        let now = attempt * 2_000;
        let lease = db.claim_analysis_job(&session, now).unwrap().unwrap();
        db.fail_analysis_job(&lease, AnalysisFailure::ParserTransport, now + 1)
            .unwrap();
        let current = db
            .capture_metadata_input(&input.root_id, "location")
            .unwrap();
        db.publish_metadata_snapshot(
            &current,
            &capabilities(),
            ProtocolReply::Result(parse_bytes(b"invalid")),
            now + 2,
        )
        .unwrap();
        discover(&mut db, now + 3);
        let status = db.analysis_status("location").unwrap().unwrap();
        assert_eq!(status.attempt, attempt);
        assert_eq!(
            status.state,
            if attempt < 3 {
                AnalysisState::Queued
            } else {
                AnalysisState::Failed
            }
        );
        assert!(db.claim_analysis_job(&session, now + 4).unwrap().is_none());
    }
    discover(&mut db, 20_000);
    assert!(db.claim_analysis_job(&session, 20_000).unwrap().is_none());
}

#[test]
fn analysis_schema10_upgrade_backup_and_recovery_retain_inflight_queue() {
    let directory = TestDirectory::new();
    let (mut db, input) = setup(&directory, 10);
    assert_eq!(db.schema_version().unwrap(), MIGRATIONS.len());
    let session = db.start_analysis_session(1).unwrap();
    discover(&mut db, 2);
    let old = db.claim_analysis_job(&session, 3).unwrap().unwrap();
    let backup = db.create_backup().unwrap();
    drop(db);
    let recovered_dir = TestDirectory::new();
    Database::recover_to(&backup, recovered_dir.path()).unwrap();
    let mut recovered = Database::open(recovered_dir.path()).unwrap();
    let new_session = recovered.start_analysis_session(4).unwrap();
    assert!(!recovered.analysis_lease_current(&old, 5).unwrap());
    assert_eq!(
        recovered
            .analysis_status(&input.location_id)
            .unwrap()
            .unwrap()
            .state,
        AnalysisState::Queued
    );
    assert!(
        recovered
            .claim_analysis_job(&new_session, 5)
            .unwrap()
            .is_some()
    );
}

#[test]
#[ignore = "controlled subprocess for analysis_killed_completion_recovers_without_partial_publication"]
fn analysis_crash_child() {
    use std::io::Write;
    let directory =
        std::env::var_os("FRUITBOARD_ANALYSIS_CRASH_TEST").expect("controlled fixture required");
    let mut db = Database::open(Path::new(&directory)).unwrap();
    let session = db.start_analysis_session(1).unwrap();
    discover(&mut db, 2);
    let lease = db.claim_analysis_job(&session, 3).unwrap().unwrap();
    db.complete_analysis_observed(
        &lease,
        &capabilities(),
        reply(),
        &sha256_hex(&bytes()),
        4,
        || {
            println!("analysis-transaction-ready");
            std::io::stdout().flush().unwrap();
            let mut control = String::new();
            std::io::stdin().read_line(&mut control).unwrap();
            assert_eq!(control, "commit\n");
        },
    )
    .unwrap();
}

#[test]
fn analysis_killed_completion_recovers_without_partial_publication() {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    let (directory, db, _) = fixture();
    drop(db);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "analysis::tests::analysis_crash_child",
            "--ignored",
            "--nocapture",
        ])
        .env("FRUITBOARD_ANALYSIS_CRASH_TEST", directory.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let ready = BufReader::new(stdout)
            .lines()
            .map_while(std::result::Result::ok)
            .any(|line| line == "analysis-transaction-ready");
        let _ = sender.send(ready);
    });
    let ready = receiver
        .recv_timeout(std::time::Duration::from_secs(15))
        .unwrap_or(false);
    // Terminate only the child we own, after both snapshot and terminal job
    // writes happened but before their shared SQLite transaction committed.
    child.kill().unwrap();
    child.wait().unwrap();
    reader.join().unwrap();
    assert!(ready, "bounded child handshake must precede termination");
    let mut db = Database::open(directory.path()).unwrap();
    let count: i64 = db
        .connection
        .query_row("SELECT count(*) FROM metadata_snapshot", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    assert!(db.current_metadata_snapshot("project").unwrap().is_none());
    let old = db.analysis_status("location").unwrap().unwrap();
    assert_eq!(old.state, AnalysisState::Running);
    assert_eq!(old.attempt, 1);
    assert!(old.snapshot_id.is_none());
    let session = db.start_analysis_session(5).unwrap();
    assert_eq!(
        db.analysis_status("location").unwrap().unwrap().state,
        AnalysisState::Queued
    );
    let lease = db.claim_analysis_job(&session, 6).unwrap().unwrap();
    let snapshot = db
        .complete_analysis_job(&lease, &capabilities(), reply(), &sha256_hex(&bytes()), 7)
        .unwrap();
    let done = db.analysis_status("location").unwrap().unwrap();
    assert_eq!(done.state, AnalysisState::Complete);
    assert_eq!(done.attempt, 2);
    assert_eq!(done.snapshot_id.as_deref(), Some(snapshot.id.as_str()));
}
