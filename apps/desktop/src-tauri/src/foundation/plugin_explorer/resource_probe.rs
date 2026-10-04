//! Explicitly selected, retained synthetic data. Not a normal CI timing test.
use super::*;
use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
use fruitboard_flp_parser::supervisor::{
    CancellationToken, ParserRequest, ParserSupervisor, ProtocolReply, SupervisorLimits,
};
use fruitboard_flp_parser::validation::validate_descriptor;
use fruitboard_flp_parser::{parse_bytes, sha256_hex};
use fruitboard_storage::{EncodedIdentity, LibraryQuery, ScanKind, ScanObservation};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[path = "../../../../../../scripts/native-resource-probe.rs"]
mod probe;

#[test]
#[ignore = "requires a fresh explicit synthetic database, release parser and exclusive idle measurement window"]
fn explorer_resource_probe() {
    let directory = PathBuf::from(
        std::env::var_os("FRUITBOARD_RESOURCE_EXPLORER_DIRECTORY")
            .expect("fresh owned probe directory required"),
    );
    let count: usize = std::env::var("FRUITBOARD_RESOURCE_EXPLORER_ENTRIES")
        .expect("explicit entry count required")
        .parse()
        .unwrap();
    assert!(matches!(count, 100 | 1_000 | 2_000 | 2_001));
    assert!(
        directory.is_absolute() && !directory.exists(),
        "refuse existing data"
    );
    let parser = PathBuf::from(
        std::env::var_os("FRUITBOARD_RESOURCE_PARSER")
            .expect("independently retained release parser required"),
    );
    let mut supervisor = ParserSupervisor::new(parser, SupervisorLimits::default()).unwrap();
    let ProtocolReply::Result(descriptor) = supervisor
        .request(ParserRequest::Describe, &CancellationToken::default())
        .unwrap()
    else {
        panic!("selected parser descriptor required")
    };
    let capabilities = validate_descriptor(&descriptor).unwrap();
    supervisor.shutdown().unwrap();
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp"),
    )
    .unwrap();
    let digest = sha256_hex(&bytes);
    let mut raw = parse_bytes(&bytes);
    raw["inputFingerprint"] = json!({"size":bytes.len(),"modifiedAtMs":42,
        "hash":{"algorithm":"sha256","value":digest}});
    raw["filesystemCreatedAtMs"] =
        json!({"status":"unavailable","reason":"FILESYSTEM_CREATION_TIME_UNAVAILABLE"});
    let mut db = Database::open(&directory).unwrap();
    let root = db
        .add_scan_root("Synthetic resources", "C:\\Synthetic\\MetadataResources")
        .unwrap();
    let session = db.start_scan_session(1).unwrap();
    db.enqueue_scan(&root.id, ScanKind::Manual, 2).unwrap();
    let scan = db.lease_next_scan(&session.id, 3, 1_000).unwrap().unwrap();
    let observations: Vec<_> = (0..count)
        .map(|index| {
            let relative = format!("fixture-{index:04}.flp");
            ScanObservation {
                locator_key: format!("v1:i:{relative}"),
                relative_path: relative,
                byte_size: bytes.len() as u64,
                modified_at_ns: 42_000_000,
                identity: Some(EncodedIdentity {
                    volume_serial: "7".into(),
                    file_id: (index + 1).to_string(),
                }),
            }
        })
        .collect();
    for batch in observations.chunks(128) {
        db.stage_scan_observations(&scan.run.id, &session.id, &scan.run.lease_token, 4, batch)
            .unwrap();
    }
    db.publish_scan_run(&scan.run.id, &session.id, &scan.run.lease_token, 5)
        .unwrap();
    let mut cursor = None;
    let mut snapshot = None;
    loop {
        let page = db
            .query_library(&LibraryQuery {
                scan_root_id: root.id.clone(),
                page_size: 200,
                cursor,
                snapshot,
            })
            .unwrap();
        for location in &page.locations {
            let input = db.capture_metadata_input(&root.id, &location.id).unwrap();
            db.publish_metadata_snapshot(
                &input,
                &capabilities,
                ProtocolReply::Result(raw.clone()),
                10,
            )
            .unwrap();
        }
        if !page.has_more {
            break;
        }
        snapshot = Some(page.snapshot);
        cursor = page.next_cursor;
        assert!(cursor.is_some());
    }
    let db = Arc::new(Mutex::new(db));
    let commands = CommandRuntime::new(
        Arc::new(FakeClock::new(100)),
        Arc::new(FakeIdGenerator::new(1)),
        Arc::new(RecordingLogSink::default()),
    );
    let baseline = probe::memory(None).unwrap();
    let mut runs = Vec::new();
    let mut samples = Vec::new();
    for index in 0..12 {
        let start = Instant::now();
        let response = handle_get_plugin_explorer(
            &commands,
            &db,
            Some(json!({"schemaVersion":1,"rootId":root.id})),
        );
        let serialized = serde_json::to_vec(&response).unwrap();
        let elapsed = start.elapsed().as_secs_f64() * 1_000.0;
        let value: Value = serde_json::from_slice(&serialized).unwrap();
        assert_eq!(value["status"], "ok");
        let data = &value["data"];
        if count <= 2_000 {
            assert_eq!(data["state"], "ready");
            assert_eq!(data["entries"].as_array().unwrap().len(), count);
            assert_eq!(data["referenceCount"], count);
            assert_eq!(data["groups"].as_array().unwrap().len(), 1);
            assert_eq!(
                data["groups"][0]["matches"].as_array().unwrap().len(),
                count
            );
        } else {
            assert_eq!(data["state"], "limited");
            assert!(
                data.get("entries").is_none()
                    && data.get("groups").is_none()
                    && data.get("referenceCount").is_none()
            );
        }
        runs.push(json!({"phase":match index {0=>"first_run",1=>"warm_up",_=>"measured"},"commandAndSerializationMs":elapsed,"responseBytes":serialized.len(),"memory":probe::memory(None).unwrap()}));
        if index >= 2 {
            samples.push(elapsed);
        }
    }
    let mut report = json!({"schema":"fruitboard/native-resource-result/1","probe":"plugin-explorer","entries":count,"state":if count<=2_000 {"ready"}else{"limited"},"baselineMemory":baseline,"runs":runs,"warmCommandAndSerialization":probe::warm_summary(&samples),"savedQueryComparisonMs":200,"completeCountsOrNoPartialResults":true});
    if std::env::var_os("FRUITBOARD_RESOURCE_EXPLORER_PROFILE").is_some() {
        // A separate subsequent series: never perturb or replace the full
        // command samples above. Projection includes its response-budget check.
        let mut rows = Vec::new();
        let mut storage_samples = Vec::new();
        let mut projection_samples = Vec::new();
        let mut serialization_samples = Vec::new();
        for index in 0..12 {
            let start = Instant::now();
            let read = db.lock().unwrap().read_plugin_explorer(&root.id).unwrap();
            let storage_ms = start.elapsed().as_secs_f64() * 1_000.0;
            let start = Instant::now();
            let content = project(read).unwrap();
            let projection_ms = start.elapsed().as_secs_f64() * 1_000.0;
            let envelope = commands.execute("get_plugin_explorer", || {
                Ok(Response {
                    root_id: root.id.clone(),
                    content,
                })
            });
            let start = Instant::now();
            let serialized = serde_json::to_vec(&envelope).unwrap();
            let serialization_ms = start.elapsed().as_secs_f64() * 1_000.0;
            let value: Value = serde_json::from_slice(&serialized).unwrap();
            assert_eq!(value["status"], "ok");
            assert_eq!(value["data"]["state"], report["state"]);
            rows.push(json!({"phase":match index {0=>"first_profile",1=>"warm_up",_=>"measured"},"storageMs":storage_ms,"projectionAndBudgetSerializationMs":projection_ms,"envelopeSerializationMs":serialization_ms}));
            if index >= 2 {
                storage_samples.push(storage_ms);
                projection_samples.push(projection_ms);
                serialization_samples.push(serialization_ms);
            }
        }
        report["subsequentComponentProfile"] = json!({"runs":rows,"warmStorage":probe::warm_summary(&storage_samples),"warmProjectionAndBudgetSerialization":probe::warm_summary(&projection_samples),"warmEnvelopeSerialization":probe::warm_summary(&serialization_samples)});
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("result.json"))
        .unwrap();
    file.write_all(serde_json::to_string_pretty(&report).unwrap().as_bytes())
        .unwrap();
    assert_eq!(
        sha256_hex(
            &std::fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp")
            )
            .unwrap()
        ),
        digest
    );
}
