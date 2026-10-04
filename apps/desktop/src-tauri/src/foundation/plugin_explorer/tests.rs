use super::*;
use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
use serde_json::json;

fn runtime(logs: Arc<RecordingLogSink>) -> CommandRuntime {
    CommandRuntime::new(
        Arc::new(FakeClock::new(100)),
        Arc::new(FakeIdGenerator::new(1)),
        logs,
    )
}

#[test]
fn explorer_command_rejects_paths_and_unknown_fields_without_logging_private_text() {
    let path = std::env::temp_dir().join(format!("fruitboard-explorer-{}", uuid::Uuid::now_v7()));
    let db = Arc::new(Mutex::new(Database::open(&path).unwrap()));
    let logs = Arc::new(RecordingLogSink::default());
    let commands = runtime(logs.clone());
    for request in [
        json!({"schemaVersion":1,"rootId":"root","path":"C:\\Private\\secret.flp"}),
        json!({"schemaVersion":2,"rootId":"root"}),
        json!({"schemaVersion":1,"rootId":""}),
        json!({"schemaVersion":1,"rootId":"root\u{0}"}),
    ] {
        let result =
            serde_json::to_value(handle_get_plugin_explorer(&commands, &db, Some(request)))
                .unwrap();
        assert_eq!(result["error"]["code"], "invalid_request");
        assert!(!result.to_string().contains("Private"));
    }
    let result = serde_json::to_value(handle_get_plugin_explorer(
        &commands,
        &db,
        Some(json!({"schemaVersion":1,"rootId":"unknown"})),
    ))
    .unwrap();
    assert_eq!(result["status"], "ok");
    #[cfg(not(feature = "analysis-jobs"))]
    assert_eq!(result["data"]["state"], "disabled");
    #[cfg(feature = "analysis-jobs")]
    assert_eq!(result["data"]["state"], "unavailable");
    assert!(!format!("{:?}", logs.events()).contains("Private"));
    drop(db);
    std::fs::remove_dir_all(path).unwrap();
}

#[cfg(feature = "analysis-jobs")]
fn builtin(name: &str) -> Value {
    json!({"className":{"status":"extracted","value":name},"name":{"status":"extracted","value":name},"vendor":{"status":"unavailable","reason":"PLUGIN_VENDOR_NOT_STORED"}})
}

#[cfg(feature = "analysis-jobs")]
#[test]
fn explorer_output_budget_returns_no_partial_aggregate() {
    use fruitboard_storage::{ExplorerEntry, FilePresence, PublishedLocation};
    let path = std::env::temp_dir().join(format!(
        "fruitboard-explorer-budget-{}",
        uuid::Uuid::now_v7()
    ));
    let mut db = Database::open(&path).unwrap();
    let mut root = db
        .add_scan_root("Synthetic", "C:\\Synthetic\\Projects")
        .unwrap();
    root.canonical_path = format!("C:\\Synthetic\\{}", "x".repeat(6_000));
    let entries = (0..500)
        .map(|index| ExplorerEntry {
            location: PublishedLocation {
                id: format!("location-{index}"),
                project_file_id: format!("project-{index}"),
                scan_root_id: Some(root.id.clone()),
                detached_scan_root_id: None,
                locator_key: format!("v1:i:project-{index}.flp"),
                relative_path: format!("project-{index}.flp"),
                byte_size: 1024,
                modified_at_ns: 42_000_000,
                identity: None,
                presence: FilePresence::Present,
                last_seen_scan_run_id: None,
                last_seen_at_ms: None,
            },
            metadata: ExplorerMetadata::NoAnalysis,
        })
        .collect();
    let result =
        serde_json::to_value(project(ExplorerRead::Ready { root, entries }).unwrap()).unwrap();
    assert_eq!(result, json!({"state":"limited"}));
    drop(db);
    std::fs::remove_dir_all(path).unwrap();
}

#[cfg(feature = "analysis-jobs")]
#[test]
fn explorer_real_publication_preserves_exact_groups_duplicates_and_coverage() {
    exercise_publication(None);
}

#[cfg(feature = "analysis-jobs")]
#[test]
#[ignore = "creates an explicitly selected fresh synthetic UI-review profile; never use an owner profile"]
fn explorer_create_synthetic_ui_review_profile() {
    let path = std::path::PathBuf::from(
        std::env::var_os("FRUITBOARD_EXPLORER_REVIEW_PROFILE")
            .expect("fresh synthetic review profile required"),
    );
    assert!(path.is_absolute());
    assert!(
        !path.exists(),
        "refuse any existing app data or other directory"
    );
    exercise_publication(Some(path));
}

#[cfg(feature = "analysis-jobs")]
fn exercise_publication(retain: Option<std::path::PathBuf>) {
    use fruitboard_flp_parser::supervisor::ProtocolReply;
    use fruitboard_flp_parser::validation::validate_descriptor;
    use fruitboard_flp_parser::{ADAPTER_ID, ADAPTER_VERSION, parse_bytes, sha256_hex};
    use fruitboard_storage::{EncodedIdentity, LibraryQuery, ScanKind, ScanObservation};
    let path = retain.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("fruitboard-explorer-{}", uuid::Uuid::now_v7()))
    });
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp"),
    )
    .unwrap();
    let before = sha256_hex(&bytes);
    let mut db = Database::open(&path).unwrap();
    let root_path = if retain.is_some() {
        let fixture_root = path.join("fixture-root");
        std::fs::create_dir(&fixture_root).unwrap();
        for index in 1..=9 {
            let fixture_file = fixture_root.join(format!("Project-{index}.flp"));
            std::fs::write(&fixture_file, &bytes).unwrap();
            std::fs::File::options()
                .write(true)
                .open(&fixture_file)
                .unwrap()
                .set_times(
                    std::fs::FileTimes::new()
                        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_millis(42)),
                )
                .unwrap();
        }
        fixture_root.to_string_lossy().into_owned()
    } else {
        "C:\\Synthetic\\Projects".into()
    };
    let other_path = if retain.is_some() {
        let fixture_root = path.join("other-fixture-root");
        std::fs::create_dir(&fixture_root).unwrap();
        fixture_root.to_string_lossy().into_owned()
    } else {
        "C:\\Synthetic\\Other".into()
    };
    let root = db.add_scan_root("Synthetic", &root_path).unwrap();
    let other = db.add_scan_root("Other", &other_path).unwrap();
    let session = db.start_scan_session(1).unwrap();
    db.enqueue_scan(&root.id, ScanKind::Manual, 2).unwrap();
    let scan = db.lease_next_scan(&session.id, 3, 1000).unwrap().unwrap();
    let observations: Vec<_> = (1..=9)
        .map(|index| ScanObservation {
            locator_key: format!("v1:i:project-{index}.flp"),
            relative_path: format!("Project-{index}.flp"),
            byte_size: bytes.len() as u64,
            modified_at_ns: 42_000_000,
            identity: Some(EncodedIdentity {
                volume_serial: "7".into(),
                file_id: index.to_string(),
            }),
        })
        .collect();
    db.stage_scan_observations(
        &scan.run.id,
        &session.id,
        &scan.run.lease_token,
        4,
        &observations,
    )
    .unwrap();
    db.publish_scan_run(&scan.run.id, &session.id, &scan.run.lease_token, 5)
        .unwrap();
    let locations = db
        .query_library(&LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 10,
            cursor: None,
            snapshot: None,
        })
        .unwrap()
        .locations;
    let capabilities = validate_descriptor(&json!({"adapter":ADAPTER_ID,"adapterVersion":ADAPTER_VERSION,
        "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","channelGeneratorNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars"],
        "maxFileBytes":67108864,"maxEventBytes":67108864,"maxEvents":100000,"maxChannels":256,"maxPatterns":1024,"maxPlaylistClips":1024})).unwrap();
    let publish = |database: &mut Database, location_id: &str, plugins: Vec<Value>, at: i64| {
        let input = database
            .capture_metadata_input(&root.id, location_id)
            .unwrap();
        let mut raw = parse_bytes(&bytes);
        raw["inputFingerprint"] = json!({"size":bytes.len(),"modifiedAtMs":42,"hash":{"algorithm":"sha256","value":before}});
        raw["filesystemCreatedAtMs"] =
            json!({"status":"unavailable","reason":"FILESYSTEM_CREATION_TIME_UNAVAILABLE"});
        raw["eventCount"] = json!(
            raw["eventCount"]
                .as_u64()
                .unwrap()
                .max(plugins.len() as u64)
        );
        raw["pluginReferences"] =
            json!({"status":"extracted","value":plugins,"coverage":"top-level-saved-references"});
        database
            .publish_metadata_snapshot(&input, &capabilities, ProtocolReply::Result(raw), at)
            .unwrap();
    };
    for (index, location) in locations.iter().take(2).enumerate() {
        let mut plugins = vec![builtin("3x Osc"), builtin("3x Osc")];
        if index == 0 {
            plugins.push(builtin("3X Osc"));
            plugins.push(json!({"className":{"status":"extracted","value":"Fruity Wrapper"},"name":{"status":"unavailable","reason":"PLUGIN_NAME_NOT_STORED"},"vendor":{"status":"unavailable","reason":"PLUGIN_VENDOR_NOT_STORED"}}));
        }
        publish(&mut db, &location.id, plugins, 10 + index as i64);
    }
    let logs = Arc::new(RecordingLogSink::default());
    let commands = runtime(logs.clone());
    let db = Arc::new(Mutex::new(db));
    let get = |id: &str| {
        serde_json::to_value(handle_get_plugin_explorer(
            &commands,
            &db,
            Some(json!({"schemaVersion":1,"rootId":id})),
        ))
        .unwrap()
    };
    let result = get(&root.id);
    assert_eq!(result["status"], "ok");
    let data = &result["data"];
    assert_eq!(data["state"], "ready");
    assert_eq!(data["entries"].as_array().unwrap().len(), 9);
    assert_eq!(data["referenceCount"], 6);
    assert_eq!(data["unnamedReferenceCount"], 1);
    assert_eq!(data["groups"].as_array().unwrap().len(), 2);
    let group = data["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["plugin"]["name"]["value"] == "3x Osc")
        .unwrap();
    assert_eq!(group["referenceCount"], 4);
    assert_eq!(group["matches"].as_array().unwrap().len(), 2);
    assert!(
        group["matches"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["referenceCount"] == 2)
    );
    assert_eq!(data["entries"][2]["metadataState"], "no_analysis");
    assert!(
        get(&other.id)["data"]["groups"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    if retain.is_some() {
        db.lock()
            .unwrap()
            .set_startup_view(fruitboard_storage::StartupView::Library)
            .unwrap();
        drop(db);
        return;
    }
    // Constructed valid replies exercise the fixed budgets; this does not
    // establish more GUI compatibility or claim the fixture uses these plugins.
    {
        let mut database = db.lock().unwrap();
        for (index, location) in locations.iter().enumerate() {
            publish(
                &mut database,
                &location.id,
                vec![builtin("Repeated"); 1024],
                100 + index as i64,
            );
        }
    }
    let limited = get(&root.id);
    assert_eq!(limited["data"]["state"], "limited");
    assert!(limited["data"].get("groups").is_none());
    assert!(limited["data"].get("referenceCount").is_none());
    {
        let mut database = db.lock().unwrap();
        for (index, location) in locations.iter().enumerate() {
            let plugins = match index {
                0 => (0..1024)
                    .map(|id| builtin(&format!("Exact {id}")))
                    .collect(),
                1 => vec![builtin("Additional exact group")],
                _ => Vec::new(),
            };
            publish(&mut database, &location.id, plugins, 200 + index as i64);
        }
    }
    assert_eq!(get(&root.id)["data"]["state"], "limited");
    db.lock()
        .unwrap()
        .set_scan_root_enabled_at(&root.id, false, 20)
        .unwrap();
    assert_eq!(get(&root.id)["data"]["state"], "unavailable");
    assert!(!format!("{:?}", logs.events()).contains("3x Osc"));
    assert_eq!(sha256_hex(&bytes), before);
    drop(db);
    std::fs::remove_dir_all(path).unwrap();
}
