//! Approved F13 through publication and the authorized read command. A retained
//! review profile is explicit, fresh, synthetic and never an owner's profile.
use super::*;
use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::validate_descriptor;
use fruitboard_flp_parser::{ADAPTER_ID, ADAPTER_VERSION, parse_bytes, sha256_hex};
use fruitboard_storage::{LibraryQuery, ScanKind, ScanObservation};
use serde_json::json;

#[test]
fn approved_patterns_publish_and_refresh_without_reanalysis_or_cross_root_leaks() {
    exercise(None);
}

#[test]
#[ignore = "creates an explicitly selected fresh review profile; never use an owner profile"]
fn patterns_create_fresh_ui_review_profile() {
    let path = std::path::PathBuf::from(
        std::env::var_os("FRUITBOARD_PATTERNS_REVIEW_PROFILE")
            .expect("explicit fresh profile required"),
    );
    assert!(
        path.is_absolute() && !path.exists(),
        "refuse an existing directory"
    );
    exercise(Some(path));
}

fn exercise(retain: Option<std::path::PathBuf>) {
    let path = retain.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("fruitboard-patterns-{}", uuid::Uuid::now_v7()))
    });
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/parser-corpus/FIX-FL2026-PATTERNS.flp");
    let bytes = std::fs::read(&fixture).unwrap();
    let before = sha256_hex(&bytes);
    let mut db = Database::open(&path).unwrap();
    let root_path = if retain.is_some() {
        let root = path.join("fixture-root");
        std::fs::create_dir(&root).unwrap();
        let file = root.join("Patterns.flp");
        std::fs::write(&file, &bytes).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_times(
                std::fs::FileTimes::new()
                    .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_millis(42)),
            )
            .unwrap();
        root.to_string_lossy().into_owned()
    } else {
        "C:\\Synthetic\\Patterns".into()
    };
    let root = db
        .add_scan_root("Approved pattern fixture", &root_path)
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
            locator_key: "v1:i:patterns.flp".into(),
            relative_path: "Patterns.flp".into(),
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
        "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","channelGeneratorNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars","patternCount","patternNames"],
        "maxFileBytes":67108864,"maxEvents":100000,"maxChannels":256,"maxEventBytes":67108864,"maxPatterns":1024,"maxPlaylistClips":1024})).unwrap();
    let mut raw = parse_bytes(&bytes);
    raw["inputFingerprint"] =
        json!({"size":bytes.len(),"modifiedAtMs":42,"hash":{"algorithm":"sha256","value":before}});
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
    let database = Arc::new(Mutex::new(db));
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
        assert_eq!(
            result["data"]["patterns"],
            json!({"state":"available","count":3,"items":[
            {"patternId":1,"name":{"status":"extracted","value":"Fixture Pattern A"}},
            {"patternId":2,"name":{"status":"extracted","value":"Fixture Pattern B"}},
            {"patternId":3,"name":{"status":"extracted","value":"Fixture Pattern C"}}]})
        );
        assert!(result["data"].get("payloadJson").is_none());
        assert!(!result.to_string().contains("fixture-root"));
    }
    let mut wrong = request.clone();
    wrong["rootId"] = json!(other.id);
    assert_eq!(read(wrong)["data"]["state"], "no_current");
    let mut wrong = request;
    wrong["expectedByteSize"] = json!("1");
    let result = read(wrong);
    assert_eq!(result["data"]["state"], "no_current");
    assert!(result["data"].get("patterns").is_none());
    let db = database.lock().unwrap();
    assert!(db.analysis_status(&location.id).unwrap().is_none());
    assert_eq!(
        db.metadata_snapshot(&saved.id)
            .unwrap()
            .unwrap()
            .payload_json(),
        Some(stored.as_str())
    );
    assert!(!format!("{:?}", logs.events()).contains("Fixture Pattern"));
    drop(db);
    drop(database);
    assert_eq!(sha256_hex(&std::fs::read(fixture).unwrap()), before);
    if retain.is_none() {
        std::fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn older_snapshots_and_absence_are_distinct_from_corrupted_present_patterns() {
    let mut raw = super::tests::payload();
    assert_eq!(
        serde_json::to_value(project(&raw.to_string()).unwrap().5).unwrap(),
        json!({"state":"unsupported","reason":"not_saved"})
    );
    raw["patterns"] = json!({"status":"unavailable","reason":"PATTERN_DATA_NOT_STORED"});
    assert_eq!(
        serde_json::to_value(project(&raw.to_string()).unwrap().5).unwrap(),
        json!({"state":"unavailable","reason":"no_stored_patterns"})
    );
    for invalid in [
        Value::Null,
        json!({"status":"extracted","count":0,"items":[]}),
        json!({"status":"extracted","count":1,"items":[{"patternId":1,"name":{"status":"extracted","value":"bad\u{0000}"}}]}),
        json!({"status":"unsupported","reason":"PATTERN_DETAILS_UNVERIFIED_BUILD"}),
    ] {
        raw["patterns"] = invalid;
        assert!(project(&raw.to_string()).is_err());
    }
}
