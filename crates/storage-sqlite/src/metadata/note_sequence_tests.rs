//! Approved-fixture persistence plus constructed publication/read defenses.
use super::tests::setup;
use super::*;
use crate::MIGRATIONS;
use crate::tests::TestDirectory;
use fruitboard_flp_parser::validation::validate_descriptor;
use fruitboard_flp_parser::{parse_bytes, sha256_hex};
use serde_json::{Value, json};
use std::fs;

fn descriptor() -> Value {
    json!({"adapter":ADAPTER_ID,"adapterVersion":ADAPTER_VERSION,
        "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","sampleReferences",
            "projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences",
            "playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds",
            "playlistPatternSpanBars","patternCount","patternNames","patternNoteCounts","patternNoteSequences"],
        "maxFileBytes":67108864,"maxEvents":100000,"maxChannels":256,"maxEventBytes":67108864,
        "maxPatterns":1024,"maxPlaylistClips":1024,"maxNoteRecordsPerPattern":65536,"maxNoteRecordsTotal":262144,
        "maxDetailedNotesPerPattern":512,"maxDetailedNotesTotal":1024,"maxNoteSequenceBytes":131072})
}

fn expectations() -> Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/parser-corpus/note-sequence-expectations.json"
    )))
    .unwrap()
}

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus")
            .join(name),
    )
    .unwrap()
}

fn seeded(directory: &TestDirectory, bytes: &[u8]) -> (Database, MetadataInput, Value) {
    let (db, initial) = setup(directory, MIGRATIONS.len());
    // Only the disposable database observation changes; approved input never does.
    db.connection
        .execute("UPDATE project_file SET byte_size=?1", [bytes.len() as i64])
        .unwrap();
    db.connection
        .execute(
            "UPDATE file_location SET byte_size=?1",
            [bytes.len() as i64],
        )
        .unwrap();
    let input = db
        .capture_metadata_input(&initial.root_id, "location")
        .unwrap();
    let mut raw = parse_bytes(bytes);
    raw["inputFingerprint"] = json!({"size":bytes.len(),"modifiedAtMs":42,
        "hash":{"algorithm":"sha256","value":sha256_hex(bytes)}});
    raw["filesystemCreatedAtMs"] = json!({"status":"extracted","value":1000});
    (db, input, raw)
}

fn baseline(directory: &TestDirectory) -> (Database, MetadataInput, Value) {
    let name = expectations()["cases"][0]["file"]
        .as_str()
        .unwrap()
        .to_owned();
    seeded(directory, &fixture(&name))
}

fn saved(db: &Database, id: &str) -> Value {
    serde_json::from_str(
        db.metadata_snapshot(id)
            .unwrap()
            .unwrap()
            .payload_json()
            .unwrap(),
    )
    .unwrap()
}

fn publish(db: &mut Database, input: &MetadataInput, raw: Value) -> MetadataSnapshotHeader {
    db.publish_metadata_snapshot(
        input,
        &validate_descriptor(&descriptor()).unwrap(),
        ProtocolReply::Result(raw),
        100,
    )
    .unwrap()
}

#[test]
fn note_sequences_approved_cases_survive_close_and_reopen_with_independent_values() {
    for case in expectations()["cases"].as_array().unwrap() {
        let name = case["file"].as_str().unwrap();
        let bytes = fixture(name);
        assert_eq!(sha256_hex(&bytes), case["sha256"]);
        let directory = TestDirectory::new();
        let (mut db, input, mut raw) = seeded(&directory, &bytes);
        raw["unselectedNoteState"] = json!("private extension must not persist");
        let header = publish(&mut db, &input, raw);
        let original = db
            .metadata_snapshot(&header.id)
            .unwrap()
            .unwrap()
            .payload_json()
            .unwrap()
            .to_owned();
        drop(db);
        let db = Database::open(directory.path()).unwrap();
        let current = db.current_metadata_snapshot("project").unwrap().unwrap();
        assert_eq!(current.header().id, header.id);
        assert_eq!(current.payload_json(), Some(original.as_str()));
        let stored = saved(&db, &header.id);
        let expected = &case["expectedNote"];
        let channel = case["channelDeclarations"]
            .as_array()
            .unwrap()
            .iter()
            .position(|decl| decl["savedId"] == expected["savedChannelId"])
            .unwrap();
        assert_eq!(
            stored["patternNoteSequences"],
            json!({
            "coverage":"stored-pattern-note-records","coordinateSystem":"pattern-local-ticks","ppq":96,
            "patterns":[{"patternId":1,"status":"extracted","records":[{
                "ordinal":0,"channelIndex":channel,"savedKey":expected["savedKey"],
                "startTick":expected["position"],"durationTick":expected["storedLength"],
                "storedVelocityByte":expected["storedVelocityByte"]}]}]}),
            "{name}"
        );
        assert_eq!(stored["projectionVersion"], 1);
        assert!(original.len() <= MAX_METADATA_JSON_BYTES);
        assert!(
            !original.contains("unselectedNoteState") && !original.contains("private extension")
        );
        assert_eq!(fixture(name), bytes);
    }
}

#[test]
fn note_sequences_unadvertised_and_older_snapshots_stay_absent_and_immutable() {
    let directory = TestDirectory::new();
    let (mut db, input, mut raw) = baseline(&directory);
    let mut old = descriptor();
    old["fields"]
        .as_array_mut()
        .unwrap()
        .retain(|v| v != "patternNoteSequences");
    for key in [
        "maxDetailedNotesPerPattern",
        "maxDetailedNotesTotal",
        "maxNoteSequenceBytes",
    ] {
        old.as_object_mut().unwrap().remove(key);
    }
    // Even malformed unadvertised extension data cannot be selected into storage.
    raw["patternNoteSequences"] = json!({"private":"discard"});
    let first = db
        .publish_metadata_snapshot(
            &input,
            &validate_descriptor(&old).unwrap(),
            ProtocolReply::Result(raw),
            10,
        )
        .unwrap();
    let original = db
        .metadata_snapshot(&first.id)
        .unwrap()
        .unwrap()
        .payload_json()
        .unwrap()
        .to_owned();
    assert!(saved(&db, &first.id).get("patternNoteSequences").is_none());
    assert!(!original.contains("discard"));
    drop(db);
    let mut db = Database::open(directory.path()).unwrap();
    let input = db
        .capture_metadata_input(&input.root_id, "location")
        .unwrap();
    let bytes = fixture(expectations()["cases"][0]["file"].as_str().unwrap());
    let mut raw = parse_bytes(&bytes);
    raw["inputFingerprint"] = json!({"size":bytes.len(),"modifiedAtMs":42,
        "hash":{"algorithm":"sha256","value":sha256_hex(&bytes)}});
    raw["filesystemCreatedAtMs"] = json!({"status":"extracted","value":1000});
    let next = publish(&mut db, &input, raw);
    assert!(saved(&db, &next.id).get("patternNoteSequences").is_some());
    assert_eq!(
        db.metadata_snapshot(&first.id)
            .unwrap()
            .unwrap()
            .payload_json(),
        Some(original.as_str())
    );
    assert_eq!(
        db.current_metadata_snapshot("project")
            .unwrap()
            .unwrap()
            .header()
            .id,
        next.id
    );
}

#[test]
fn note_sequences_malformed_advertised_replacements_write_nothing() {
    let directory = TestDirectory::new();
    let (mut db, input, raw) = baseline(&directory);
    let first = publish(&mut db, &input, raw.clone());
    let input = db
        .capture_metadata_input(&input.root_id, "location")
        .unwrap();
    for (pointer, bad) in [
        ("/patternNoteSequences", Value::Null),
        ("/patternNoteSequences/ppq", json!(192)),
        ("/patternNoteSequences/coverage", json!("song")),
        ("/patternNoteSequences/patterns/0/patternId", json!(2)),
        ("/patternNoteSequences/patterns/0/records", json!([])),
        (
            "/patternNoteSequences/patterns/0/records/0/ordinal",
            json!(1),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/channelIndex",
            json!(1),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/savedKey",
            json!(61),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/startTick",
            json!(-1),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/durationTick",
            json!(0),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/storedVelocityByte",
            json!(89.5),
        ),
        ("/patternNoteCounts/value/0/noteCount/value", json!(2)),
    ] {
        let mut invalid = raw.clone();
        *invalid.pointer_mut(pointer).unwrap() = bad;
        assert!(
            matches!(
                db.publish_metadata_snapshot(
                    &input,
                    &validate_descriptor(&descriptor()).unwrap(),
                    ProtocolReply::Result(invalid),
                    101
                ),
                Err(StorageError::InvalidSchema)
            ),
            "{pointer}"
        );
        assert_eq!(
            db.current_metadata_snapshot("project")
                .unwrap()
                .unwrap()
                .header()
                .id,
            first.id
        );
        assert_eq!(
            db.capture_metadata_input(&input.root_id, "location")
                .unwrap()
                .publication_revision,
            input.publication_revision
        );
        assert_eq!(
            db.connection
                .query_row("SELECT count(*) FROM metadata_snapshot", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
}

fn repeat_records(raw: &mut Value, patterns: usize, notes: usize) {
    // Constructed duplicates test order and safety caps, not new GUI semantics.
    let record = raw["patternNoteSequences"]["patterns"][0]["records"][0].clone();
    raw["patternCount"] = json!({"status":"extracted","value":patterns});
    raw["patternNames"] = json!({"status":"extracted","value":(1..=patterns).map(|id|
        json!({"patternId":id,"name":{"status":"extracted","value":"Constructed"}})).collect::<Vec<_>>()});
    raw["patternNoteCounts"] = json!({"status":"extracted","coverage":"stored-pattern-note-records",
        "value":(1..=patterns).map(|id| json!({"patternId":id,"noteCount":{"status":"extracted","value":notes}})).collect::<Vec<_>>()});
    raw["patternNoteSequences"]["patterns"] = json!(
        (1..=patterns)
            .map(|id| {
                let records: Vec<Value> = (0..notes)
                    .map(|ordinal| {
                        let mut note = record.clone();
                        note["ordinal"] = json!(ordinal);
                        note
                    })
                    .collect();
                json!({"patternId":id,"status":"extracted","records":records})
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn note_sequences_preserve_source_order_duplicates_and_exact_detail_boundaries() {
    let directory = TestDirectory::new();
    let (mut db, input, mut raw) = baseline(&directory);
    repeat_records(&mut raw, 2, 512);
    let header = publish(&mut db, &input, raw.clone());
    assert_eq!(
        saved(&db, &header.id)["patternNoteSequences"],
        raw["patternNoteSequences"]
    );
    let input = db
        .capture_metadata_input(&input.root_id, "location")
        .unwrap();
    repeat_records(&mut raw, 1, 513);
    raw["patternNoteSequences"]["patterns"][0] = json!({"patternId":1,"status":"unsupported","reason":"PATTERN_NOTE_SEQUENCE_LIMIT_EXCEEDED"});
    let limited = publish(&mut db, &input, raw.clone());
    assert_eq!(
        saved(&db, &limited.id)["patternNoteSequences"],
        raw["patternNoteSequences"]
    );
    let input = db
        .capture_metadata_input(&input.root_id, "location")
        .unwrap();
    raw["patternNoteCounts"]["value"][0]["noteCount"]["value"] = json!(1025);
    raw["patternNoteSequences"] = json!({"status":"unsupported","coverage":"stored-pattern-note-records","reason":"PATTERN_NOTE_SEQUENCE_LIMIT_EXCEEDED"});
    let total = publish(&mut db, &input, raw.clone());
    assert_eq!(
        saved(&db, &total.id)["patternNoteSequences"],
        raw["patternNoteSequences"]
    );
    assert_eq!(
        saved(&db, &header.id)["patternNoteSequences"]["patterns"][1]["records"]
            .as_array()
            .unwrap()
            .len(),
        512
    );
}

#[test]
fn note_sequences_keep_supported_siblings_missing_data_and_refusals_distinct() {
    let directory = TestDirectory::new();
    let (mut db, input, mut raw) = baseline(&directory);
    repeat_records(&mut raw, 3, 1);
    raw["patternNoteCounts"] = json!({"status":"unavailable","reason":"PATTERN_NOTE_COUNTS_INCOMPLETE",
        "coverage":"stored-pattern-note-records","items":[
            {"patternId":1,"noteCount":{"status":"extracted","value":1}},
            {"patternId":2,"noteCount":{"status":"unavailable","reason":"PATTERN_NOTES_NOT_STORED"}},
            {"patternId":3,"noteCount":{"status":"extracted","value":1}}]});
    raw["patternNoteSequences"]["patterns"][1] =
        json!({"patternId":2,"status":"unavailable","reason":"PATTERN_NOTES_NOT_STORED"});
    raw["patternNoteSequences"]["patterns"][2] =
        json!({"patternId":3,"status":"unsupported","reason":"PATTERN_NOTE_RECORD_UNQUALIFIED"});
    let header = publish(&mut db, &input, raw.clone());
    assert_eq!(
        saved(&db, &header.id)["patternNoteSequences"],
        raw["patternNoteSequences"]
    );
    for name in [
        "FIX-FL2026-MIN.flp",
        "FIX-FL2026-NAMED-EMPTY.flp",
        "FIX-FL2025-MIN.flp",
    ] {
        // Approved corpus supplies genuine whole-section unavailable/unsupported cases.
        let directory = TestDirectory::new();
        let (mut db, input, raw) = seeded(&directory, &fixture(name));
        let expected = raw["patternNoteSequences"].clone();
        let header = publish(&mut db, &input, raw);
        assert_eq!(saved(&db, &header.id)["patternNoteSequences"], expected);
    }
}

#[test]
fn note_sequences_corrupt_stored_data_fails_current_and_historical_reads() {
    let directory = TestDirectory::new();
    let (mut db, input, raw) = baseline(&directory);
    let header = publish(&mut db, &input, raw);
    let original = saved(&db, &header.id);
    db.connection
        .execute_batch("DROP TRIGGER metadata_snapshot_no_update")
        .unwrap();
    for (pointer, bad) in [
        ("/patternNoteSequences", Value::Null),
        ("/patternNoteSequences/patterns/0/records", json!([])),
        (
            "/patternNoteSequences/patterns/0/records/0/savedKey",
            json!(61),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/private",
            json!("reject"),
        ),
        (
            "/patternNoteSequences/patterns/0/reason",
            json!("PATTERN_NOTE_RECORD_UNQUALIFIED"),
        ),
        ("/patternNoteSequences/ppq", json!(0)),
        ("/patternNoteCounts/value/0/noteCount/value", json!(2)),
        ("/patterns/items/0/patternId", json!(2)),
        ("/channelCount", json!(0)),
        ("/channelCount", json!(257)),
        ("/channelCount", json!(u64::MAX)),
        ("/savedVersion", json!("25.1.3.4922")),
    ] {
        let mut corrupt = original.clone();
        // Value indexing also inserts unknown keys to test strict shape rejection.
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        if let Some(target) = corrupt.pointer_mut(pointer) {
            *target = bad;
        } else {
            corrupt.pointer_mut(parent).unwrap()[key] = bad;
        }
        db.connection
            .execute(
                "UPDATE metadata_snapshot SET payload_json=?1 WHERE id=?2",
                params![corrupt.to_string(), header.id],
            )
            .unwrap();
        assert!(
            matches!(
                db.metadata_snapshot(&header.id),
                Err(StorageError::InvalidSchema)
            ),
            "{pointer}"
        );
        assert!(
            matches!(
                db.current_metadata_snapshot("project"),
                Err(StorageError::InvalidSchema)
            ),
            "{pointer}"
        );
    }
}

#[test]
fn note_sequences_negative_results_and_source_changes_preserve_historical_records() {
    for change in [
        "UPDATE file_location SET presence='missing'",
        "UPDATE file_location SET byte_size=byte_size+1",
        "UPDATE scan_root SET enabled=0",
    ] {
        let directory = TestDirectory::new();
        let (mut db, input, raw) = baseline(&directory);
        let first = publish(&mut db, &input, raw.clone());
        let original = saved(&db, &first.id);
        let pending = db
            .capture_metadata_input(&input.root_id, "location")
            .unwrap();
        db.publish_metadata_snapshot(
            &pending,
            &validate_descriptor(&descriptor()).unwrap(),
            ProtocolReply::Result(parse_bytes(b"invalid")),
            101,
        )
        .unwrap();
        assert_eq!(
            db.current_metadata_snapshot("project")
                .unwrap()
                .unwrap()
                .header()
                .id,
            first.id
        );
        let pending = db
            .capture_metadata_input(&input.root_id, "location")
            .unwrap();
        db.connection.execute_batch(change).unwrap();
        assert!(matches!(
            db.publish_metadata_snapshot(
                &pending,
                &validate_descriptor(&descriptor()).unwrap(),
                ProtocolReply::Result(raw),
                102
            ),
            Err(StorageError::Conflict)
        ));
        assert!(db.current_metadata_snapshot("project").unwrap().is_none());
        assert_eq!(saved(&db, &first.id), original);
    }
}
