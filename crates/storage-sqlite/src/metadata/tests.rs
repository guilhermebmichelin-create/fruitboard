use super::*;
use crate::tests::TestDirectory;
use crate::{MIGRATIONS, StartupView};
use fruitboard_flp_parser::validation::validate_descriptor;
use fruitboard_flp_parser::{parse_bytes, sha256_hex};
use serde_json::{Value, json};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

pub(crate) fn capabilities() -> ParserCapabilities {
    validate_descriptor(&json!({"adapter":ADAPTER_ID,"adapterVersion":ADAPTER_VERSION,
        "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","channelGeneratorNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars"],
        "maxFileBytes":4194304,"maxEvents":100000,"maxChannels":256,"maxEventBytes":2097152,"maxPatterns":1024,"maxPlaylistClips":1024})).unwrap()
}
pub(crate) fn bytes() -> Vec<u8> {
    fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp"),
    )
    .unwrap()
}
fn raw_reply() -> Value {
    let bytes = bytes();
    let mut value = parse_bytes(&bytes);
    value["inputFingerprint"] = json!({"size":bytes.len(),"modifiedAtMs":42,"hash":{"algorithm":"sha256","value":sha256_hex(&bytes)}});
    value["filesystemCreatedAtMs"] = json!({"status":"extracted","value":u64::MAX});
    value
}
pub(crate) fn reply() -> ProtocolReply {
    ProtocolReply::Result(raw_reply())
}
fn unsupported_reply() -> ProtocolReply {
    let mut value = raw_reply();
    value["outcome"] = json!("unsupported");
    value["code"] = json!("UNSUPPORTED_SAVED_VERSION");
    value["savedVersion"] = json!({"status":"extracted","value":"99.1.0.123"});
    for field in [
        "baseTempoBpm",
        "channelCount",
        "channelNames",
        "sampleReferences",
    ] {
        value[field] = json!({"status":"unsupported","reason":"UNSUPPORTED_SAVED_VERSION"});
    }
    value["diagnostics"] = json!([]);
    ProtocolReply::Result(value)
}
pub(crate) fn setup(directory: &TestDirectory, version: usize) -> (Database, MetadataInput) {
    let mut database =
        Database::open_with_migrations(directory.path(), &MIGRATIONS[..version]).unwrap();
    let root = database
        .add_scan_root("Synthetic", "C:\\Synthetic\\Projects")
        .unwrap();
    database.connection.execute("INSERT INTO project_file(id, display_filename, extension, byte_size, modified_at_ms, modified_at_ns, created_at_ms, updated_at_ms)
        VALUES ('project', 'fixture.flp', '.flp', ?1, 42, 42000123, 1, 1)", [bytes().len() as i64]).unwrap();
    database.connection.execute("INSERT INTO file_location(id, project_file_id, scan_root_id, normalized_path, locator_key, relative_path, byte_size, modified_at_ms, modified_at_ns, created_at_ms, updated_at_ms)
        VALUES ('location', 'project', ?1, 'v1:i:fixture.flp', 'v1:i:fixture.flp', 'fixture.flp', ?2, 42, 42000123, 1, 1)", params![root.id, bytes().len() as i64]).unwrap();
    if version < MIGRATIONS.len() {
        drop(database);
        database = Database::open(directory.path()).unwrap();
    }
    let input = database
        .capture_metadata_input(&root.id, "location")
        .unwrap();
    (database, input)
}
fn count(database: &Database) -> i64 {
    database
        .connection
        .query_row("SELECT count(*) FROM metadata_snapshot", [], |row| {
            row.get(0)
        })
        .unwrap()
}

#[test]
fn metadata_round_trip_keeps_only_validated_fields_provenance_and_immutable_history() {
    let directory = TestDirectory::new();
    let (mut database, input) = setup(&directory, MIGRATIONS.len());
    let mut raw = raw_reply();
    // Construct validated inference fields to exercise storage; this does not
    // establish compatibility for another GUI-produced project.
    raw["playlistPatternClips"] = json!({"status":"extracted","value":[{"patternId":1,"startTick":0,"lengthTick":384,"trackToken":500}]});
    raw["playlistPatternEndTick"] = json!({"status":"extracted","value":384});
    raw["playlistPatternSpanBars"] = json!({"status":"inferred","value":1.0,"method":"pattern-clip-span-at-verified-meter","confidence":"low"});
    let bpm = raw["baseTempoBpm"]["value"].as_f64().unwrap();
    raw["playlistPatternNominalSeconds"] = json!({"status":"inferred","value":4.0*60.0/bpm,"method":"constant-base-tempo-over-pattern-clips","confidence":"low","assumptions":["tempo remains at base BPM","only verified pattern clips define span"]});
    raw["unvalidatedPrivateExtension"] = json!("must never be retained");
    let header = database
        .publish_metadata_snapshot(&input, &capabilities(), ProtocolReply::Result(raw), 100)
        .unwrap();
    assert_eq!(header.outcome, MetadataOutcome::Complete);
    assert_eq!(header.adapter_id, ADAPTER_ID);
    assert_eq!(header.adapter_version, ADAPTER_VERSION);
    assert_eq!(header.protocol_version, PROTOCOL_VERSION);
    assert_eq!(header.parser_schema_version, SCHEMA_VERSION);
    assert_eq!(header.input_modified_at_ns, 42000123);
    let snapshot = database.metadata_snapshot(&header.id).unwrap().unwrap();
    let payload: Value = serde_json::from_str(snapshot.payload_json().unwrap()).unwrap();
    assert!(payload.get("unvalidatedPrivateExtension").is_none());
    assert!(payload.get("patternNames").is_none());
    assert_eq!(
        payload["filesystemCreatedAtMs"]["value"].as_u64(),
        Some(u64::MAX)
    );
    assert_eq!(payload["channelNames"]["items"][0]["status"], "extracted");
    assert_eq!(
        payload["channelGeneratorNames"]["items"][0]["value"],
        "Sampler"
    );
    assert_eq!(
        payload["channelGeneratorNames"]["items"][0]["status"],
        "inferred"
    );
    assert_eq!(
        payload["pluginReferences"]["coverage"],
        "top-level-saved-references"
    );
    assert_eq!(
        payload["playlistPatternNominalSeconds"]["confidence"],
        "low"
    );
    assert_eq!(
        payload["playlistPatternNominalSeconds"]["assumptions"][0],
        "tempo remains at base BPM"
    );
    assert!(
        database
            .connection
            .execute(
                "UPDATE metadata_snapshot SET parsed_at_ms = 101 WHERE id = ?1",
                [&header.id]
            )
            .is_err()
    );
    assert!(
        database
            .connection
            .execute("DELETE FROM metadata_snapshot WHERE id = ?1", [&header.id])
            .is_err()
    );
    let stored = snapshot.payload_json().unwrap().to_owned();
    let root_id = input.root_id.clone();
    drop(database);
    let database = Database::open(directory.path()).unwrap();
    assert_eq!(
        database
            .current_metadata_snapshot("project")
            .unwrap()
            .unwrap()
            .payload_json(),
        Some(stored.as_str())
    );
    assert_eq!(
        database
            .capture_metadata_input(&root_id, "location")
            .unwrap()
            .byte_size,
        bytes().len() as u64
    );
}

#[test]
fn invalid_generator_claim_cannot_replace_a_valid_snapshot_or_store_extension_text() {
    let directory = TestDirectory::new();
    let (mut database, input) = setup(&directory, MIGRATIONS.len());
    let mut valid = raw_reply();
    valid["channelGeneratorNames"] = json!({"status":"unsupported","reason":"GENERATOR_CLASS_UNVERIFIED","items":[{"channelIndex":0,"name":{"status":"unsupported","reason":"GENERATOR_CLASS_UNVERIFIED","private":"unvalidated plugin text"}}]});
    let saved = database
        .publish_metadata_snapshot(
            &input,
            &capabilities(),
            ProtocolReply::Result(valid.clone()),
            1,
        )
        .unwrap();
    let payload = database
        .current_metadata_snapshot("project")
        .unwrap()
        .unwrap();
    let raw = payload.payload_json().unwrap();
    assert!(!raw.contains("unvalidated plugin text"));
    assert!(raw.contains("GENERATOR_CLASS_UNVERIFIED"));
    valid["channelGeneratorNames"]["items"][0]["name"] =
        json!({"status":"extracted","value":"Private Unverified Plugin"});
    assert!(
        database
            .publish_metadata_snapshot(&input, &capabilities(), ProtocolReply::Result(valid), 2)
            .is_err()
    );
    assert_eq!(
        database
            .current_metadata_snapshot("project")
            .unwrap()
            .unwrap()
            .header()
            .id,
        saved.id
    );
}

#[test]
fn metadata_partial_failed_unsupported_and_rejected_outcomes_stay_distinct() {
    let directory = TestDirectory::new();
    let (mut database, input) = setup(&directory, MIGRATIONS.len());
    let mut partial = raw_reply();
    partial["outcome"] = json!("partial");
    partial["diagnostics"] = json!([{"code":"UNSUPPORTED_EVENT","eventId":255}]);
    let partial = database
        .publish_metadata_snapshot(&input, &capabilities(), ProtocolReply::Result(partial), 1)
        .unwrap();
    assert_eq!(partial.outcome, MetadataOutcome::Partial);
    let payload: Value = serde_json::from_str(
        database
            .metadata_snapshot(&partial.id)
            .unwrap()
            .unwrap()
            .payload_json()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(payload["diagnostics"][0], "UNSUPPORTED_EVENT_255");
    for (result, expected, code) in [
        (
            ProtocolReply::Result(parse_bytes(b"invalid")),
            MetadataOutcome::Failed,
            Some("INVALID_HEADER"),
        ),
        (
            ProtocolReply::Rejected {
                code: "INVALID_PATH".into(),
            },
            MetadataOutcome::Rejected,
            Some("INVALID_PATH"),
        ),
        (unsupported_reply(), MetadataOutcome::Unsupported, None),
    ] {
        let input = database
            .capture_metadata_input(&input.root_id, "location")
            .unwrap();
        let header = database
            .publish_metadata_snapshot(&input, &capabilities(), result, 2)
            .unwrap();
        assert_eq!(header.outcome, expected);
        assert_eq!(header.parser_code.as_deref(), code);
        assert!(
            database
                .metadata_snapshot(&header.id)
                .unwrap()
                .unwrap()
                .payload_json()
                .is_none()
        );
        if expected == MetadataOutcome::Unsupported {
            assert_eq!(
                header.unsupported_saved_version.as_deref(),
                Some("99.1.0.123")
            );
        }
    }
    assert_eq!(count(&database), 4);
}

#[test]
fn metadata_invalid_mismatched_and_oversized_replies_have_no_side_effects() {
    let directory = TestDirectory::new();
    let (mut database, input) = setup(&directory, MIGRATIONS.len());
    let mut mismatch = raw_reply();
    mismatch["inputFingerprint"]["size"] = json!(1);
    let mut oversize = raw_reply();
    oversize["extra"] = json!("x".repeat(MAX_METADATA_JSON_BYTES));
    let mut malformed = raw_reply();
    malformed["baseTempoBpm"]["value"] = json!(0);
    for value in [mismatch, oversize, malformed] {
        assert!(matches!(
            database.publish_metadata_snapshot(
                &input,
                &capabilities(),
                ProtocolReply::Result(value),
                1
            ),
            Err(StorageError::InvalidSchema)
        ));
        assert_eq!(count(&database), 0);
        assert!(
            database
                .current_metadata_snapshot("project")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            database
                .capture_metadata_input(&input.root_id, "location")
                .unwrap()
                .publication_revision,
            0
        );
    }
}

#[test]
fn metadata_every_source_change_and_restore_fences_late_replies_and_current_reads() {
    for sql in [
        "UPDATE project_file SET byte_size = byte_size + 1 WHERE id='project'; UPDATE project_file SET byte_size = byte_size - 1 WHERE id='project';",
        "UPDATE file_location SET modified_at_ns = modified_at_ns + 1 WHERE id='location'; UPDATE file_location SET modified_at_ns = modified_at_ns - 1 WHERE id='location';",
        "UPDATE file_location SET relative_path='renamed.flp' WHERE id='location'; UPDATE file_location SET relative_path='fixture.flp' WHERE id='location';",
        "UPDATE file_location SET identity_volume_serial='1', identity_file_id='2' WHERE id='location'; UPDATE file_location SET identity_volume_serial=NULL, identity_file_id=NULL WHERE id='location';",
        "UPDATE file_location SET presence='missing' WHERE id='location'; UPDATE file_location SET presence='present' WHERE id='location';",
    ] {
        let directory = TestDirectory::new();
        let (mut database, first_input) = setup(&directory, MIGRATIONS.len());
        let first = database
            .publish_metadata_snapshot(&first_input, &capabilities(), reply(), 1)
            .unwrap();
        let pending = database
            .capture_metadata_input(&first_input.root_id, "location")
            .unwrap();
        database.connection.execute_batch(sql).unwrap();
        assert!(matches!(
            database.publish_metadata_snapshot(&pending, &capabilities(), reply(), 2),
            Err(StorageError::Conflict)
        ));
        assert!(
            database
                .current_metadata_snapshot("project")
                .unwrap()
                .is_none()
        );
        assert!(database.metadata_snapshot(&first.id).unwrap().is_some());
        let restored = database
            .capture_metadata_input(&first_input.root_id, "location")
            .unwrap();
        database
            .publish_metadata_snapshot(&restored, &capabilities(), reply(), 3)
            .unwrap();
        assert!(
            database
                .current_metadata_snapshot("project")
                .unwrap()
                .is_some()
        );
        assert_eq!(count(&database), 2);
    }
}

#[test]
fn metadata_roots_disable_reenable_remove_and_foreign_location_preserve_history() {
    let directory = TestDirectory::new();
    let (mut database, input) = setup(&directory, MIGRATIONS.len());
    let first = database
        .publish_metadata_snapshot(&input, &capabilities(), reply(), 1)
        .unwrap();
    let pending = database
        .capture_metadata_input(&input.root_id, "location")
        .unwrap();
    assert!(matches!(
        database.capture_metadata_input("foreign-root", "location"),
        Err(StorageError::NotFound)
    ));
    database
        .set_scan_root_enabled_at(&input.root_id, false, 2)
        .unwrap();
    assert!(
        database
            .current_metadata_snapshot("project")
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        database.publish_metadata_snapshot(&pending, &capabilities(), reply(), 3),
        Err(StorageError::Conflict)
    ));
    database
        .set_scan_root_enabled_at(&input.root_id, true, 4)
        .unwrap();
    assert!(matches!(
        database.publish_metadata_snapshot(&pending, &capabilities(), reply(), 5),
        Err(StorageError::Conflict)
    ));
    assert!(
        database
            .current_metadata_snapshot("project")
            .unwrap()
            .is_none()
    );
    database.remove_scan_root_at(&input.root_id, 6).unwrap();
    assert!(database.metadata_snapshot(&first.id).unwrap().is_some());
    assert_eq!(count(&database), 1);
}

#[test]
fn metadata_newer_publication_wins_over_old_arrival_even_with_earlier_wall_clock() {
    let directory = TestDirectory::new();
    let (mut database, pending) = setup(&directory, MIGRATIONS.len());
    database
        .publish_metadata_snapshot(&pending, &capabilities(), reply(), 100)
        .unwrap();
    assert!(matches!(
        database.publish_metadata_snapshot(&pending, &capabilities(), reply(), 101),
        Err(StorageError::Conflict)
    ));
    let next = database
        .capture_metadata_input(&pending.root_id, "location")
        .unwrap();
    let current = database
        .publish_metadata_snapshot(&next, &capabilities(), reply(), 1)
        .unwrap();
    assert_eq!(
        database
            .current_metadata_snapshot("project")
            .unwrap()
            .unwrap()
            .header()
            .id,
        current.id
    );
    assert_eq!(count(&database), 2);
}

#[test]
fn metadata_unchanged_observation_does_not_invalidate_and_revision_overflow_fails_closed() {
    let directory = TestDirectory::new();
    let (mut database, input) = setup(&directory, MIGRATIONS.len());
    database.connection.execute_batch("UPDATE project_file SET byte_size=byte_size, modified_at_ns=modified_at_ns; UPDATE file_location SET byte_size=byte_size, presence=presence;").unwrap();
    database
        .publish_metadata_snapshot(&input, &capabilities(), reply(), 1)
        .unwrap();
    database
        .connection
        .execute("UPDATE file_location SET metadata_revision=?1", [i64::MAX])
        .unwrap();
    assert!(
        database
            .connection
            .execute("UPDATE file_location SET byte_size=byte_size+1", [])
            .is_err()
    );
    let size: i64 = database
        .connection
        .query_row("SELECT byte_size FROM file_location", [], |row| row.get(0))
        .unwrap();
    assert_eq!(size, bytes().len() as i64);
}

#[test]
fn metadata_snapshot_insert_and_latest_pointer_roll_back_together() {
    let directory = TestDirectory::new();
    let (mut database, input) = setup(&directory, MIGRATIONS.len());
    database.connection.execute_batch("CREATE TRIGGER metadata_fail_pointer BEFORE UPDATE OF current_metadata_snapshot_id ON project_file BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;").unwrap();
    assert!(
        database
            .publish_metadata_snapshot(&input, &capabilities(), reply(), 1)
            .is_err()
    );
    assert_eq!(count(&database), 0);
    assert!(
        database
            .current_metadata_snapshot("project")
            .unwrap()
            .is_none()
    );
    database
        .connection
        .execute_batch("DROP TRIGGER metadata_fail_pointer;")
        .unwrap();
    database
        .publish_metadata_snapshot(&input, &capabilities(), reply(), 2)
        .unwrap();
    assert_eq!(count(&database), 1);
}

#[test]
fn metadata_upgrade_v9_backup_and_recovery_preserve_source_and_snapshots() {
    let directory = TestDirectory::new();
    let (mut database, input) = setup(&directory, 9);
    assert_eq!(database.schema_version().unwrap(), MIGRATIONS.len());
    let backups: Vec<_> = fs::read_dir(directory.path().join("storage/backups"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(backups.len(), 1);
    let backup = Connection::open(&backups[0]).unwrap();
    assert_eq!(
        backup
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        9
    );
    assert_eq!(
        backup
            .query_row("SELECT count(*) FROM file_location", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    drop(backup);
    let first = database
        .publish_metadata_snapshot(&input, &capabilities(), reply(), 1)
        .unwrap();
    database.set_startup_view(StartupView::Library).unwrap();
    let backup = database.create_backup().unwrap();
    let restored_directory = TestDirectory::new();
    Database::recover_to(&backup, restored_directory.path()).unwrap();
    let restored = Database::open(restored_directory.path()).unwrap();
    assert_eq!(
        restored
            .current_metadata_snapshot("project")
            .unwrap()
            .unwrap()
            .header()
            .id,
        first.id
    );
    assert_eq!(restored.startup_view().unwrap(), StartupView::Library);
    assert_eq!(count(&restored), 1);
}

#[test]
fn metadata_pagination_is_bounded_stable_and_scoped_to_one_file() {
    let directory = TestDirectory::new();
    let (mut database, original) = setup(&directory, MIGRATIONS.len());
    let mut ids = Vec::new();
    for _ in 0..5 {
        let input = database
            .capture_metadata_input(&original.root_id, "location")
            .unwrap();
        ids.push(
            database
                .publish_metadata_snapshot(&input, &capabilities(), reply(), 1)
                .unwrap()
                .id,
        );
    }
    ids.sort();
    ids.reverse();
    let first = database.metadata_snapshot_page("project", None, 2).unwrap();
    let second = database
        .metadata_snapshot_page("project", first.next.as_ref(), 2)
        .unwrap();
    let last = database
        .metadata_snapshot_page("project", second.next.as_ref(), 2)
        .unwrap();
    let actual: Vec<_> = first
        .items
        .iter()
        .chain(&second.items)
        .chain(&last.items)
        .map(|header| header.id.clone())
        .collect();
    assert_eq!(actual, ids);
    assert!(last.next.is_none());
    assert!(matches!(
        database.metadata_snapshot_page("foreign", first.next.as_ref(), 2),
        Err(StorageError::InvalidCursor)
    ));
    for limit in [0, MAX_METADATA_PAGE_SIZE + 1] {
        assert!(matches!(
            database.metadata_snapshot_page("project", None, limit),
            Err(StorageError::InvalidCursor)
        ));
    }
    let mut query = database.connection.prepare(&format!("EXPLAIN QUERY PLAN SELECT {HEADER_COLUMNS} FROM metadata_snapshot AS snapshot WHERE snapshot.project_file_id=?1 AND (snapshot.parsed_at_ms, snapshot.id COLLATE BINARY)<(?2,?3) ORDER BY snapshot.parsed_at_ms DESC, snapshot.id COLLATE BINARY DESC LIMIT ?4")).unwrap();
    let plan: Vec<String> = query
        .query_map(params!["project", i64::MAX, "z", 2], |row| row.get(3))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert!(
        plan.iter()
            .any(|line| line.contains("metadata_snapshot_file_order"))
    );
    assert!(plan.iter().all(|line| !line.contains("TEMP B-TREE")));
}

#[test]
#[ignore = "subprocess helper, invoked by metadata_interrupted_publication_is_atomic"]
fn metadata_crash_child() {
    let directory =
        std::env::var_os("FRUITBOARD_METADATA_CRASH_TEST").expect("controlled fixture required");
    let mut database = Database::open(std::path::Path::new(&directory)).unwrap();
    let root_id: String = database
        .connection
        .query_row(
            "SELECT scan_root_id FROM file_location WHERE id='location'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let input = database
        .capture_metadata_input(&root_id, "location")
        .unwrap();
    database
        .publish_metadata_observed(&input, &capabilities(), reply(), 2, || {
            println!("metadata-transaction-ready");
            std::io::stdout().flush().unwrap();
            let mut control = String::new();
            std::io::stdin().read_line(&mut control).unwrap();
            assert_eq!(control, "commit\n");
        })
        .unwrap();
}

#[test]
fn metadata_interrupted_publication_is_atomic() {
    let directory = TestDirectory::new();
    let (mut database, input) = setup(&directory, MIGRATIONS.len());
    let first = database
        .publish_metadata_snapshot(&input, &capabilities(), reply(), 1)
        .unwrap();
    drop(database);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "metadata::tests::metadata_crash_child",
            "--ignored",
            "--nocapture",
        ])
        .env("FRUITBOARD_METADATA_CRASH_TEST", directory.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut found = false;
        for line in BufReader::new(stdout)
            .lines()
            .map_while(std::result::Result::ok)
        {
            if line == "metadata-transaction-ready" {
                found = true;
                break;
            }
        }
        let _ = sender.send(found);
    });
    let ready = receiver
        .recv_timeout(std::time::Duration::from_secs(15))
        .unwrap_or(false);
    child.kill().unwrap();
    child.wait().unwrap();
    reader.join().unwrap();
    assert!(ready, "bounded child handshake must precede termination");
    let database = Database::open(directory.path()).unwrap();
    assert_eq!(count(&database), 1);
    assert_eq!(
        database
            .current_metadata_snapshot("project")
            .unwrap()
            .unwrap()
            .header()
            .id,
        first.id
    );
    assert_eq!(
        database
            .capture_metadata_input(&input.root_id, "location")
            .unwrap()
            .publication_revision,
        1
    );
}
