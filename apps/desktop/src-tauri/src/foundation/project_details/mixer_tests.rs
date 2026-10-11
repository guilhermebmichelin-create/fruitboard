//! Approved F18-F21 across parse, immutable publication and authorized reads.
use super::*;
use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::validate_descriptor;
use fruitboard_flp_parser::{ADAPTER_ID, ADAPTER_VERSION, parse_bytes, sha256_hex};
use fruitboard_storage::{LibraryQuery, ScanKind, ScanObservation};
use serde_json::json;

#[test]
fn approved_mixer_records_survive_publication_refresh_restart_and_root_revocation() {
    let expectations: Value = serde_json::from_str(include_str!(
        "../../../../../../fixtures/parser-corpus/mixer-insert-expectations.json"
    ))
    .unwrap();
    for fixture in [
        "FIX-FL2026-MIXER-DEFAULT.flp",
        "FIX-FL2026-MIXER-NAMES.flp",
        "FIX-FL2026-MIXER-DUPLICATE.flp",
        "FIX-FL2026-MIXER-SPECIAL.flp",
    ] {
        let directory =
            std::env::temp_dir().join(format!("fruitboard-mixer-details-{}", uuid::Uuid::now_v7()));
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/parser-corpus")
            .join(fixture);
        let bytes = std::fs::read(&file).unwrap();
        let before = sha256_hex(&bytes);
        let mut db = Database::open(&directory).unwrap();
        let root = db
            .add_scan_root("Approved mixer records", "C:\\Synthetic\\Mixer")
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
                locator_key: "v1:i:mixer.flp".into(),
                relative_path: "Mixer.flp".into(),
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
            "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","channelGeneratorNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars","patternCount","patternNames","patternNoteCounts","mixerInsertCount","mixerInsertNames"],
            "maxFileBytes":67108864,"maxEvents":100000,"maxChannels":256,"maxEventBytes":67108864,"maxPatterns":1024,"maxPlaylistClips":1024,"maxNoteRecordsPerPattern":65536,"maxNoteRecordsTotal":262144,"maxMixerInserts":512,"maxMixerInsertCandidates":512})).unwrap();
        let mut raw = parse_bytes(&bytes);
        raw["inputFingerprint"] = json!({"size":bytes.len(),"modifiedAtMs":42,"hash":{"algorithm":"sha256","value":before}});
        raw["filesystemCreatedAtMs"] =
            json!({"status":"unavailable","reason":"FILESYSTEM_CREATION_TIME_UNAVAILABLE"});
        let expected = expectations["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["file"] == fixture)
            .unwrap();
        assert_eq!(before, expected["sha256"].as_str().unwrap());
        let expected_names = expected["savedSection"]["ordinaryRecords"].clone();
        assert!(expected_names.is_array());
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
            let mixer = &result["data"]["mixerInserts"];
            assert_eq!(mixer["state"], "available");
            assert_eq!(mixer["count"], 16);
            for (index, expected) in expected_names.as_array().unwrap().iter().enumerate() {
                assert_eq!(mixer["items"][index]["savedInsertId"], index + 1);
                let name = &mixer["items"][index]["name"];
                if expected["name"]["status"] == "extracted" {
                    assert_eq!(name["value"], expected["name"]["value"]);
                } else {
                    assert_eq!(
                        name,
                        &json!({"status":"unavailable","reason":"name_not_saved"})
                    );
                }
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
            assert!(result["data"].get("mixerInserts").is_none());
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
        let recorded = format!("{:?}", logs.events());
        for expected in expected_names.as_array().unwrap() {
            if let Some(name) = expected["name"]["value"].as_str() {
                assert!(!recorded.contains(name));
            }
        }
        drop(database);
        assert_eq!(sha256_hex(&std::fs::read(file).unwrap()), before);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn stored_mixer_fields_refuse_malformed_present_data_and_preserve_older_results() {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/parser-corpus/FIX-FL2026-MIXER-NAMES.flp");
    let reply = parse_bytes(&std::fs::read(file).unwrap());
    let pair = json!({"count":reply["mixerInsertCount"],"names":reply["mixerInsertNames"]});
    let project_pair = |value: &Value, build| {
        serde_json::to_value(mixer_inserts::project(Some(value), build).unwrap()).unwrap()
    };
    assert_eq!(
        serde_json::to_value(mixer_inserts::project(None, "26.1.0.5530").unwrap()).unwrap(),
        json!({"state":"unsupported","reason":"not_saved"})
    );
    assert_eq!(project_pair(&pair, "26.1.0.5530")["count"], 16);
    assert!(mixer_inserts::project(Some(&pair), "25.1.3.4922").is_err());
    for (pointer, bad) in [
        ("/count/value", json!(0)),
        ("/count/value", json!(17)),
        ("/count/coverage", json!("all-audible-tracks")),
        ("/names/items/0/savedInsertId", json!(0)),
        ("/names/items/0/savedInsertId", json!(17)),
        ("/names/items/0/savedInsertId", json!(2)),
        ("/names/status", json!("extracted")),
    ] {
        let mut invalid = pair.clone();
        *invalid.pointer_mut(pointer).unwrap() = bad;
        assert!(mixer_inserts::project(Some(&invalid), "26.1.0.5530").is_err());
    }
    for invalid in [
        Value::Null,
        json!({"count":pair["count"]}),
        json!({"count":pair["count"],"names":pair["names"],"slots":[]}),
    ] {
        assert!(mixer_inserts::project(Some(&invalid), "26.1.0.5530").is_err());
    }
    let marker = json!({"status":"unsupported","reason":"MIXER_INSERTS_NOT_ADVERTISED"});
    assert_eq!(
        project_pair(&marker, "26.1.0.5530")["reason"],
        "not_advertised"
    );
    for (status, reason, expected) in [
        (
            "unavailable",
            "MIXER_INSERT_DATA_NOT_STORED",
            "no_saved_section",
        ),
        (
            "unsupported",
            "MIXER_INSERT_LAYOUT_UNVERIFIED",
            "unverified_layout",
        ),
        (
            "unsupported",
            "MIXER_INSERT_BINDING_UNVERIFIED",
            "unverified_binding",
        ),
        (
            "unsupported",
            "MIXER_INSERT_RECORDS_AMBIGUOUS",
            "ambiguous_records",
        ),
        (
            "unsupported",
            "MIXER_INSERT_LIMIT_EXCEEDED",
            "limit_exceeded",
        ),
    ] {
        let field = json!({"status":status,"reason":reason,"coverage":"explicit-saved-mixer-insert-records"});
        let stored = json!({"count":field,"names":field});
        assert_eq!(project_pair(&stored, "26.1.0.5530")["reason"], expected);
    }
}
