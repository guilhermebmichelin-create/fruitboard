//! Approved F24-F30 through immutable storage and the authorized details seam.
use super::*;
use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::validate_descriptor;
use fruitboard_flp_parser::{ADAPTER_ID, ADAPTER_VERSION, parse_bytes, sha256_hex};
use fruitboard_storage::{LibraryQuery, ScanKind, ScanObservation};
use serde_json::json;

#[test]
fn approved_note_sequences_reach_details_after_restart_and_disappear_on_root_loss() {
    let expectations: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/parser-corpus/note-sequence-expectations.json"
    )))
    .unwrap();
    for case in expectations["cases"].as_array().unwrap() {
        let directory = std::env::temp_dir().join(format!(
            "fruitboard-note-sequence-details-{}",
            uuid::Uuid::now_v7()
        ));
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/parser-corpus")
            .join(case["file"].as_str().unwrap());
        let bytes = std::fs::read(&file).unwrap();
        assert_eq!(sha256_hex(&bytes), case["sha256"]);
        let mut db = Database::open(&directory).unwrap();
        let root = db
            .add_scan_root("Approved note sequences", "C:\\Synthetic\\Notes")
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
            "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","channelGeneratorNames","sampleReferences",
            "projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips",
            "playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars","patternCount","patternNames",
            "patternNoteCounts","patternNoteSequences"],
            "maxFileBytes":67108864,"maxEvents":100000,"maxChannels":256,"maxEventBytes":67108864,
            "maxPatterns":1024,"maxPlaylistClips":1024,"maxNoteRecordsPerPattern":65536,"maxNoteRecordsTotal":262144,
            "maxDetailedNotesPerPattern":512,"maxDetailedNotesTotal":1024,"maxNoteSequenceBytes":131072})).unwrap();
        let mut raw = parse_bytes(&bytes);
        raw["inputFingerprint"] = json!({"size":bytes.len(),"modifiedAtMs":42,"hash":{"algorithm":"sha256","value":sha256_hex(&bytes)}});
        raw["filesystemCreatedAtMs"] =
            json!({"status":"unavailable","reason":"FILESYSTEM_CREATION_TIME_UNAVAILABLE"});
        let input = db.capture_metadata_input(&root.id, &location.id).unwrap();
        let header = db
            .publish_metadata_snapshot(&input, &caps, ProtocolReply::Result(raw), 100)
            .unwrap();
        let original = db
            .metadata_snapshot(&header.id)
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
        let request = json!({"schemaVersion":1,"rootId":root.id,"locationId":location.id,
            "expectedByteSize":bytes.len().to_string(),"expectedModifiedAt":super::super::scan_console::unix_ns_to_rfc3339(42_000_000)});
        let read = |request| {
            serde_json::to_value(handle_get_project_details(
                &runtime,
                &database,
                Some(request),
                false,
            ))
            .unwrap()
        };
        let expected = &case["expectedNote"];
        let channel = case["channelDeclarations"]
            .as_array()
            .unwrap()
            .iter()
            .position(|decl| decl["savedId"] == expected["savedChannelId"])
            .unwrap();
        for _ in 0..2 {
            let reply = read(request.clone());
            assert_eq!(reply["status"], "ok");
            assert_eq!(reply["data"]["snapshotId"], header.id);
            assert_eq!(
                reply["data"]["patternNoteSequences"],
                json!({"state":"available",
                "coverage":"stored-pattern-note-records","coordinateSystem":"pattern-local-ticks","ppq":96,
                "items":[{"patternId":1,"sequence":{"status":"extracted","records":[{
                    "ordinal":0,"channelIndex":channel,"savedKey":expected["savedKey"],
                    "startTick":expected["position"],"durationTick":expected["storedLength"],
                    "storedVelocityByte":expected["storedVelocityByte"]}]}}]})
            );
            assert!(reply["data"].get("payloadJson").is_none());
        }
        for (key, bad) in [
            ("rootId", json!(other.id)),
            ("expectedByteSize", json!("1")),
        ] {
            let mut foreign = request.clone();
            foreign[key] = bad;
            let reply = read(foreign);
            assert_eq!(reply["data"]["state"], "no_current");
            assert!(reply["data"].get("patternNoteSequences").is_none());
        }
        for pointer in [
            "/patternNoteSequences",
            "/patternNoteSequences/ppq",
            "/patternNoteSequences/patterns/0/records/0/channelIndex",
        ] {
            let mut bad: Value = serde_json::from_str(&original).unwrap();
            *bad.pointer_mut(pointer).unwrap() = Value::Null;
            assert!(project(&bad.to_string()).is_err());
        }
        database
            .lock()
            .unwrap()
            .set_scan_root_enabled(&root.id, false)
            .unwrap();
        let reply = read(request);
        assert_eq!(reply["data"]["state"], "no_current");
        assert!(reply["data"].get("patternNoteSequences").is_none());
        assert_eq!(
            database
                .lock()
                .unwrap()
                .metadata_snapshot(&header.id)
                .unwrap()
                .unwrap()
                .payload_json(),
            Some(original.as_str())
        );
        assert!(
            database
                .lock()
                .unwrap()
                .analysis_status(&location.id)
                .unwrap()
                .is_none()
        );
        assert!(!format!("{:?}", logs.events()).contains("savedKey"));
        drop(database);
        assert_eq!(std::fs::read(file).unwrap(), bytes);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn absent_sequence_details_are_older_results_but_null_is_malformed() {
    let mut raw = super::tests::payload();
    assert_eq!(
        serde_json::to_value(project(&raw.to_string()).unwrap().8).unwrap(),
        json!({"state":"unsupported","reason":"not_saved"})
    );
    raw["patternNoteSequences"] = Value::Null;
    assert!(project(&raw.to_string()).is_err());
}
