//! Approved F16/F17 across parse, immutable publication and authorized reads.
use super::*;
use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::validate_descriptor;
use fruitboard_flp_parser::{ADAPTER_ID, ADAPTER_VERSION, parse_bytes, sha256_hex};
use fruitboard_storage::{LibraryQuery, ScanKind, ScanObservation};
use serde_json::json;

#[test]
fn approved_note_records_survive_publication_refresh_restart_and_root_revocation() {
    for (fixture, counts) in [
        ("FIX-FL2026-NOTES.flp", Some([3, 2, 4])),
        ("FIX-FL2026-NAMED-EMPTY.flp", None),
    ] {
        let directory =
            std::env::temp_dir().join(format!("fruitboard-note-details-{}", uuid::Uuid::now_v7()));
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/parser-corpus")
            .join(fixture);
        let bytes = std::fs::read(&file).unwrap();
        let before = sha256_hex(&bytes);
        let mut db = Database::open(&directory).unwrap();
        let root = db
            .add_scan_root("Approved notes", "C:\\Synthetic\\Notes")
            .unwrap();
        let other = db.add_scan_root("Other", "C:\\Synthetic\\Other").unwrap();
        let session = db.start_scan_session(1).unwrap();
        db.enqueue_scan(&root.id, ScanKind::Manual, 2).unwrap();
        let scan = db.lease_next_scan(&session.id, 3, 1000).unwrap().unwrap();
        db.stage_scan_observations(
            &scan.run.id,
            &session.id,
            &scan.run.lease_token,
            4,
            &[ScanObservation {
                locator_key: "v1:i:notes.flp".into(),
                relative_path: "Notes.flp".into(),
                byte_size: bytes.len() as u64,
                modified_at_ns: 42_000_000,
                identity: None,
            }],
        )
        .unwrap();
        db.publish_scan_run(&scan.run.id, &session.id, &scan.run.lease_token, 5)
            .unwrap();
        let location = db
            .query_library(&LibraryQuery {
                scan_root_id: root.id.clone(),
                page_size: 10,
                cursor: None,
                snapshot: None,
            })
            .unwrap()
            .locations
            .remove(0);
        let caps = validate_descriptor(&json!({"adapter":ADAPTER_ID,"adapterVersion":ADAPTER_VERSION,
            "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","channelGeneratorNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars","patternCount","patternNames","patternNoteCounts"],
            "maxFileBytes":67108864,"maxEvents":100000,"maxChannels":256,"maxEventBytes":67108864,"maxPatterns":1024,"maxPlaylistClips":1024,"maxNoteRecordsPerPattern":65536,"maxNoteRecordsTotal":262144})).unwrap();
        let mut raw = parse_bytes(&bytes);
        raw["inputFingerprint"] = json!({"size":bytes.len(),"modifiedAtMs":42,"hash":{"algorithm":"sha256","value":before}});
        raw["filesystemCreatedAtMs"] =
            json!({"status":"unavailable","reason":"FILESYSTEM_CREATION_TIME_UNAVAILABLE"});
        let input = db.capture_metadata_input(&root.id, &location.id).unwrap();
        let saved = db
            .publish_metadata_snapshot(&input, &caps, ProtocolReply::Result(raw), 100)
            .unwrap();
        let stored = db
            .metadata_snapshot(&saved.id)
            .unwrap()
            .unwrap()
            .payload_json()
            .unwrap()
            .to_owned();
        drop(db);
        let database = Arc::new(Mutex::new(Database::open(&directory).unwrap()));
        let logs = Arc::new(RecordingLogSink::default());
        let runtime = CommandRuntime::new(
            Arc::new(FakeClock::new(100)),
            Arc::new(FakeIdGenerator::new(1)),
            logs.clone(),
        );
        let request = json!({"schemaVersion":1,"rootId":root.id,"locationId":location.id,"expectedByteSize":bytes.len().to_string(),"expectedModifiedAt":super::super::scan_console::unix_ns_to_rfc3339(42_000_000)});
        let read = |request| {
            serde_json::to_value(handle_get_project_details(
                &runtime,
                &database,
                Some(request),
                false,
            ))
            .unwrap()
        };
        for _ in 0..2 {
            let result = read(request.clone());
            assert_eq!(result["status"], "ok");
            assert_eq!(result["data"]["snapshotId"], saved.id);
            let notes = &result["data"]["patternNoteCounts"];
            assert_eq!(notes["state"], "available");
            if let Some(counts) = counts {
                for (index, count) in counts.iter().enumerate() {
                    assert_eq!(notes["items"][index]["patternId"], index + 1);
                    assert_eq!(
                        notes["items"][index]["noteCount"],
                        json!({"status":"extracted","value":count})
                    );
                }
            } else {
                assert_eq!(
                    notes["items"][0]["noteCount"],
                    json!({"status":"unavailable","reason":"no_stored_notes"})
                );
                assert_eq!(
                    result["data"]["patterns"]["items"][0]["name"]["value"],
                    "Fixture Named Empty"
                );
            }
            assert!(result["data"].get("payloadJson").is_none());
        }
        for (key, value) in [
            ("rootId", json!(other.id)),
            ("expectedByteSize", json!("1")),
        ] {
            let mut wrong = request.clone();
            wrong[key] = value;
            let result = read(wrong);
            assert_eq!(result["data"]["state"], "no_current");
            assert!(result["data"].get("patternNoteCounts").is_none());
        }
        database
            .lock()
            .unwrap()
            .set_scan_root_enabled(&root.id, false)
            .unwrap();
        assert_eq!(read(request)["data"]["state"], "no_current");
        assert_eq!(
            database
                .lock()
                .unwrap()
                .metadata_snapshot(&saved.id)
                .unwrap()
                .unwrap()
                .payload_json(),
            Some(stored.as_str())
        );
        assert!(
            database
                .lock()
                .unwrap()
                .analysis_status(&location.id)
                .unwrap()
                .is_none()
        );
        assert!(!format!("{:?}", logs.events()).contains("Fixture Notes"));
        drop(database);
        assert_eq!(sha256_hex(&std::fs::read(file).unwrap()), before);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn stored_note_counts_distinguish_older_missing_unsupported_and_malformed_results() {
    let mut raw = super::tests::payload();
    assert_eq!(
        serde_json::to_value(project(&raw.to_string()).unwrap().6).unwrap(),
        json!({"state":"unsupported","reason":"not_saved"})
    );
    raw["patterns"] = json!({"status":"extracted","count":1,"items":[{"patternId":1,"name":{"status":"extracted","value":"Synthetic"}}]});
    let notes = json!({"status":"extracted","coverage":"stored-pattern-note-records","value":[{"patternId":1,"noteCount":{"status":"extracted","value":3}}]});
    raw["patternNoteCounts"] = notes.clone();
    assert_eq!(
        serde_json::to_value(project(&raw.to_string()).unwrap().6).unwrap()["items"][0]["noteCount"]
            ["value"],
        3
    );
    for (pointer, bad) in [
        ("", Value::Null),
        ("/coverage", json!("all-musical-notes")),
        ("/value/0/patternId", json!(2)),
        ("/value/0/noteCount/value", json!(0)),
        ("/value/0/noteCount/value", json!(65537)),
        ("/value/0/noteCount/value", json!(1.5)),
        ("/value/0/noteCount/pitches", json!([60])),
        ("/value", json!([])),
    ] {
        let mut invalid = notes.clone();
        if pointer.is_empty() {
            invalid = bad;
        } else if pointer.ends_with("/pitches") {
            invalid["value"][0]["noteCount"]["pitches"] = bad;
        } else {
            *invalid.pointer_mut(pointer).unwrap() = bad;
        }
        raw["patternNoteCounts"] = invalid;
        assert!(project(&raw.to_string()).is_err(), "{pointer}");
    }
    for (reason, expected) in [
        ("PATTERN_NOTE_LIMIT_EXCEEDED", "limit_exceeded"),
        ("PATTERN_NOTE_BINDING_UNVERIFIED", "unverified_binding"),
    ] {
        raw["patternNoteCounts"] = json!({"status":"unsupported","reason":reason,"coverage":"stored-pattern-note-records"});
        assert_eq!(
            serde_json::to_value(project(&raw.to_string()).unwrap().6).unwrap(),
            json!({"state":"unsupported","reason":expected})
        );
    }
}

#[test]
fn saved_note_projection_rechecks_total_bounds_and_state_relationships() {
    let mut raw = json!({"patterns":{"status":"extracted","count":5,"items":(1..=5).map(|id| json!({"patternId":id,"name":{"status":"unavailable","reason":"PATTERN_NAME_NOT_STORED"}})).collect::<Vec<_>>()},
        "patternNoteCounts":{"status":"extracted","coverage":"stored-pattern-note-records","value":(1..=5).map(|id| json!({"patternId":id,"noteCount":{"status":"extracted","value":65536}})).collect::<Vec<_>>()}});
    assert!(pattern_notes::project(&raw, "26.1.0.5530").is_err());
    raw["patterns"]["count"] = json!(4);
    raw["patterns"]["items"].as_array_mut().unwrap().pop();
    raw["patternNoteCounts"]["value"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(pattern_notes::project(&raw, "26.1.0.5530").is_ok());
    raw["patternNoteCounts"] = json!({"status":"unavailable","reason":"PATTERN_DATA_NOT_STORED","coverage":"stored-pattern-note-records"});
    assert!(pattern_notes::project(&raw, "26.1.0.5530").is_err());
    raw["patterns"] = json!({"status":"unavailable","reason":"PATTERN_DATA_NOT_STORED"});
    assert!(pattern_notes::project(&raw, "26.1.0.5530").is_ok());
    raw.as_object_mut().unwrap().remove("patterns");
    assert!(pattern_notes::project(&raw, "26.1.0.5530").is_err());
    raw["patternNoteCounts"] = json!({"status":"unsupported","reason":"PATTERN_NOTES_UNVERIFIED_BUILD","coverage":"stored-pattern-note-records"});
    assert!(pattern_notes::project(&raw, "26.1.0.5530").is_err());
    assert!(pattern_notes::project(&raw, "26.1.0.5531").is_ok());
}
