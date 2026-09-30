use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::*;
use fruitboard_flp_parser::{
    ExpectedFingerprint, modified_at_ms, parse_bytes, parse_file, sha256_hex,
};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

fn event(id: u8, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![id];
    if id >= 192 {
        let mut size = data.len();
        loop {
            let byte = (size & 127) as u8;
            size >>= 7;
            bytes.push(byte | if size == 0 { 0 } else { 128 });
            if size == 0 {
                break;
            }
        }
    }
    bytes.extend_from_slice(data);
    bytes
}
fn utf16(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}
fn project(events: Vec<Vec<u8>>, channels: u16) -> Vec<u8> {
    let mut data = event(199, b"26.1.0.5530\0");
    for e in events {
        data.extend(e);
    }
    let mut bytes = b"FLhd".to_vec();
    bytes.extend(6_u32.to_le_bytes());
    bytes.extend(0_u16.to_le_bytes());
    bytes.extend(channels.to_le_bytes());
    bytes.extend(96_u16.to_le_bytes());
    bytes.extend(b"FLdt");
    bytes.extend((data.len() as u32).to_le_bytes());
    bytes.extend(data);
    bytes
}
fn info(created: f64, spent: f64) -> Vec<u8> {
    [created.to_le_bytes(), spent.to_le_bytes()].concat()
}
fn context(bytes: &[u8]) -> ParseContext {
    ParseContext {
        root_id: "test".into(),
        file_id: "file".into(),
        root_revision: 1,
        file_revision: 1,
        root_enabled: true,
        expected: ExpectedFingerprint {
            size: bytes.len() as u64,
            modified_at_ms: 42,
        },
        content_sha256: Some(sha256_hex(bytes)),
    }
}
fn descriptor() -> Value {
    json!({"adapter":"rust-flp-parser","adapterVersion":env!("CARGO_PKG_VERSION"),"fields":["savedVersion","baseTempoBpm","channelCount","channelNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars"],"maxFileBytes":4194304,"maxEvents":100000,"maxChannels":256,"maxEventBytes":2097152,"maxPatterns":1024,"maxPlaylistClips":1024})
}
fn reply(bytes: &[u8]) -> Value {
    let mut value = parse_bytes(bytes);
    value["inputFingerprint"] = json!({"size":bytes.len(),"modifiedAtMs":42,"hash":{"algorithm":"sha256","value":sha256_hex(bytes)}});
    value["filesystemCreatedAtMs"] = json!({"status":"extracted","value":1000});
    value
}
fn validate(value: Value, bytes: &[u8]) -> Result<ValidatedProjectReply, ValidationError> {
    validate_project_reply(
        ProtocolReply::Result(value),
        &validate_descriptor(&descriptor()).unwrap(),
        &context(bytes),
        &context(bytes),
    )
}
fn metadata(value: Value, bytes: &[u8]) -> ValidatedProjectMetadata {
    match validate(value, bytes) {
        Ok(ValidatedProjectReply::Metadata(value)) => *value,
        _ => panic!("expected project metadata"),
    }
}

#[test]
fn saved_project_info_keeps_local_time_and_rounds_elapsed_milliseconds() {
    let bytes = project(vec![event(237, &info(46112.5, 90.0 / 1440.0))], 0);
    let result = metadata(reply(&bytes), &bytes);
    assert_eq!(
        result.project_created_local().value().map(String::as_str),
        Some("2026-03-31T12:00:00.000")
    );
    assert_eq!(result.fl_studio_time_spent_ms().value(), Some(&5_400_000));
    assert_eq!(result.file_size_bytes(), bytes.len() as u64);
    assert_eq!(result.file_modified_at_ms(), 42);
    assert_eq!(result.filesystem_created_at_ms().value(), Some(&1000));
    for (days, date) in [
        (2.0, "1900-01-01T00:00:00.000"),
        (61.0, "1900-03-01T00:00:00.000"),
        (36585.0, "2000-02-29T00:00:00.000"),
    ] {
        let bytes = project(vec![event(237, &info(days, 0.0))], 0);
        assert_eq!(parse_bytes(&bytes)["projectCreatedLocal"]["value"], date);
    }
}

#[test]
fn absent_duplicate_malformed_and_nonfinite_info_are_explicit() {
    let absent = project(vec![], 0);
    assert_eq!(
        parse_bytes(&absent)["flStudioTimeSpentMs"]["reason"],
        "PROJECT_INFO_NOT_STORED"
    );
    let duplicate = project(
        vec![
            event(237, &info(46112.0, 0.0)),
            event(237, &info(46112.0, 0.0)),
        ],
        0,
    );
    assert_eq!(
        parse_bytes(&duplicate)["projectCreatedLocal"]["reason"],
        "MULTIPLE_PROJECT_INFO_RECORDS"
    );
    let malformed = project(vec![event(237, &[0; 15])], 0);
    assert_eq!(
        parse_bytes(&malformed)["projectCreatedLocal"]["reason"],
        "PROJECT_INFO_LAYOUT_UNSUPPORTED"
    );
    for bad in [f64::NAN, f64::INFINITY, -1.0, 2_958_466.0] {
        let bytes = project(vec![event(237, &info(bad, bad))], 0);
        let value = parse_bytes(&bytes);
        assert_eq!(value["projectCreatedLocal"]["status"], "unsupported");
        assert_eq!(value["flStudioTimeSpentMs"]["status"], "unsupported");
        assert!(validate(reply(&bytes), &bytes).is_ok());
    }
}

fn chunk(id: u32, value: &[u8]) -> Vec<u8> {
    [
        id.to_le_bytes().as_slice(),
        (value.len() as u64).to_le_bytes().as_slice(),
        value,
    ]
    .concat()
}
fn wrapper() -> Vec<u8> {
    [
        8_u32.to_le_bytes().to_vec(),
        chunk(54, b"Fixture VST"),
        chunk(56, b"Fixture Vendor"),
        chunk(55, b"PRIVATE_PATH_DO_NOT_RETURN"),
        chunk(53, b"OPAQUE_STATE_DO_NOT_RETURN"),
    ]
    .concat()
}

#[test]
fn saved_plugin_names_include_wrapper_metadata_without_paths_or_state() {
    let bytes = project(
        vec![
            event(64, &0_u16.to_le_bytes()),
            event(21, &[0]),
            event(201, &utf16("")),
            event(65, &1_u16.to_le_bytes()),
            event(201, &utf16("Fruity Limiter")),
            event(201, &utf16("Fruity Wrapper")),
            event(213, &wrapper()),
        ],
        1,
    );
    let value = reply(&bytes);
    assert_eq!(
        value["pluginReferences"]["value"].as_array().unwrap().len(),
        3
    );
    assert!(!value.to_string().contains("DO_NOT_RETURN"));
    let result = metadata(value, &bytes);
    assert_eq!(
        result.plugins()[0].name().value().map(String::as_str),
        Some("Sampler")
    );
    assert_eq!(
        result.plugins()[1].name().value().map(String::as_str),
        Some("Fruity Limiter")
    );
    assert_eq!(
        result.plugins()[2].name().value().map(String::as_str),
        Some("Fixture VST")
    );
    assert_eq!(
        result.plugins()[2].vendor().value().map(String::as_str),
        Some("Fixture Vendor")
    );
}

#[test]
fn malformed_wrapper_lengths_duplicates_and_text_do_not_leak_or_panic() {
    let mut oversized = 8_u32.to_le_bytes().to_vec();
    oversized.extend(54_u32.to_le_bytes());
    oversized.extend(u64::MAX.to_le_bytes());
    let duplicate = [wrapper(), chunk(54, b"Another")].concat();
    let nul = [8_u32.to_le_bytes().to_vec(), chunk(54, b"a\0b")].concat();
    for data in [vec![], oversized, duplicate, nul, vec![1; 16]] {
        let bytes = project(
            vec![event(201, &utf16("Fruity Wrapper")), event(213, &data)],
            0,
        );
        let result = metadata(reply(&bytes), &bytes);
        assert!(matches!(
            result.plugins()[0].name(),
            ProjectField::Unsupported(ProjectReason::VstMetadataUnsupported)
        ));
    }
    let bytes = project(
        vec![
            event(64, &0_u16.to_le_bytes()),
            event(21, &[0]),
            event(65, &1_u16.to_le_bytes()),
            event(201, &utf16("")),
        ],
        1,
    );
    assert_eq!(
        metadata(reply(&bytes), &bytes).plugins().len(),
        0,
        "empty mixer/pattern slot must not inherit Sampler identity"
    );
}

#[test]
fn plugin_projection_rejects_inconsistent_identity_and_wrapper_states() {
    let bytes = project(
        vec![event(201, &utf16("Fruity Wrapper")), event(213, &wrapper())],
        0,
    );
    for (field, bad) in [
        (
            "className",
            json!({"status":"unsupported","reason":"PLUGIN_NAME_ENCODING_UNSUPPORTED"}),
        ),
        (
            "className",
            json!({"status":"unavailable","reason":"PLUGIN_NAME_NOT_STORED"}),
        ),
        (
            "name",
            json!({"status":"unsupported","reason":"MULTIPLE_VST_METADATA_RECORDS"}),
        ),
        (
            "vendor",
            json!({"status":"unsupported","reason":"VST_METADATA_UNSUPPORTED"}),
        ),
    ] {
        let mut value = reply(&bytes);
        value["pluginReferences"]["value"][0][field] = bad;
        assert!(validate(value, &bytes).is_err(), "{field}");
    }
    let invalid_name = project(vec![event(201, &[0x00, 0xd8, 0, 0])], 0);
    assert!(validate(reply(&invalid_name), &invalid_name).is_ok());
    let duplicate = project(
        vec![
            event(201, &utf16("Fruity Wrapper")),
            event(213, &wrapper()),
            event(213, &wrapper()),
        ],
        0,
    );
    let parsed = metadata(reply(&duplicate), &duplicate);
    assert!(matches!(
        parsed.plugins()[0].name(),
        ProjectField::Unsupported(ProjectReason::MultipleVstMetadataRecords)
    ));
}

#[test]
fn project_projection_rejects_invalid_dates_counters_and_provenance() {
    let bytes = project(vec![event(237, &info(46112.0, 0.1))], 0);
    for (pointer, bad) in [
        (
            "/projectCreatedLocal/value",
            json!("2026-02-30T00:00:00.000"),
        ),
        ("/projectCreatedLocal/timezone", json!("UTC")),
        ("/flStudioTimeSpentMs/value", json!(-1)),
        ("/flStudioTimeSpentMs/value", json!(1.5)),
        ("/filesystemCreatedAtMs/value", json!("1000")),
    ] {
        let mut value = reply(&bytes);
        *value.pointer_mut(pointer).unwrap() = bad;
        assert!(validate(value, &bytes).is_err(), "{pointer}");
    }
    let mut value = reply(&bytes);
    value["flStudioTimeSpentMs"] =
        json!({"status":"unavailable","reason":"PROJECT_INFO_NOT_STORED"});
    assert!(validate(value, &bytes).is_err());
    let mut capabilities = descriptor();
    capabilities["fields"] = json!([
        "savedVersion",
        "baseTempoBpm",
        "channelCount",
        "channelNames",
        "sampleReferences"
    ]);
    assert!(matches!(
        validate_project_reply(
            ProtocolReply::Result(reply(&bytes)),
            &validate_descriptor(&capabilities).unwrap(),
            &context(&bytes),
            &context(&bytes)
        ),
        Err(ValidationError::InvalidCapabilities)
    ));
}

#[test]
fn approved_corpus_exercises_extended_projection_without_mutating_bytes() {
    let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/parser-corpus");
    for entry in fs::read_dir(corpus).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|value| value != "flp") {
            continue;
        }
        let before = fs::read(&path).unwrap();
        let attrs = fs::metadata(&path).unwrap();
        let mut native = context(&before);
        native.expected.modified_at_ms = modified_at_ms(&attrs).unwrap();
        let result = validate_project_reply(
            ProtocolReply::Result(parse_file(&path, native.expected)),
            &validate_descriptor(&descriptor()).unwrap(),
            &native,
            &native,
        )
        .unwrap();
        if let ValidatedProjectReply::Metadata(value) = result {
            assert!(value.project_created_local().value().is_some());
            assert!(value.fl_studio_time_spent_ms().value().is_some());
            if path.file_name().unwrap() == "FIX-FL2026-PATTERNS.flp" {
                assert_eq!(value.arrangement_end_tick().value(), Some(&1536));
                assert_eq!(value.arrangement_span_bars().value(), Some(&4.0));
                assert!(
                    (value.arrangement_estimated_seconds().value().unwrap() - 7.384615384615385)
                        .abs()
                        < 1e-12
                );
            }
            if path.file_name().unwrap() == "FIX-FL2026-3XOSC.flp" {
                assert_eq!(value.plugins().len(), 2);
                assert_eq!(
                    value.plugins()[1].name().value().map(String::as_str),
                    Some("3x Osc")
                );
            }
        }
        assert_eq!(fs::read(path).unwrap(), before);
    }
}

#[test]
fn arrangement_uses_maximum_clip_end_and_rejects_inconsistent_estimates() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/parser-corpus/FIX-FL2026-PATTERNS.flp");
    let bytes = fs::read(path).unwrap();
    let mut value = reply(&bytes);
    value["playlistPatternClips"]["value"] = json!([{"patternId":1,"startTick":0,"lengthTick":384,"trackToken":499},{"patternId":1,"startTick":192,"lengthTick":384,"trackToken":499},{"patternId":1,"startTick":1152,"lengthTick":384,"trackToken":499}]);
    assert_eq!(
        metadata(value.clone(), &bytes)
            .arrangement_end_tick()
            .value(),
        Some(&1536)
    );
    for (pointer, bad) in [
        ("/playlistPatternEndTick/value", json!(1152)),
        ("/playlistPatternSpanBars/value", json!(3)),
        ("/playlistPatternNominalSeconds/value", json!(42)),
        ("/playlistPatternNominalSeconds/confidence", json!("high")),
        (
            "/playlistPatternClips/value/0/lengthTick",
            json!(4294967295_u64),
        ),
    ] {
        let mut bad_reply = value.clone();
        *bad_reply.pointer_mut(pointer).unwrap() = bad;
        assert!(validate(bad_reply, &bytes).is_err(), "{pointer}");
    }
    value["playlistPatternSpanBars"] = json!({"status":"unsupported","reason":"PPQ_UNVERIFIED"});
    assert!(validate(value, &bytes).is_err());
}

#[test]
fn extended_limits_and_native_freshness_fail_closed() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/parser-corpus/FIX-FL2026-PATTERNS.flp");
    let bytes = fs::read(path).unwrap();
    let mut limits = descriptor();
    limits["maxPlaylistClips"] = json!(1);
    assert!(matches!(
        validate_project_reply(
            ProtocolReply::Result(reply(&bytes)),
            &validate_descriptor(&limits).unwrap(),
            &context(&bytes),
            &context(&bytes)
        ),
        Err(ValidationError::LimitExceeded)
    ));
    let mut current = context(&bytes);
    current.file_revision += 1;
    assert!(matches!(
        validate_project_reply(
            ProtocolReply::Result(reply(&bytes)),
            &validate_descriptor(&descriptor()).unwrap(),
            &context(&bytes),
            &current
        ),
        Err(ValidationError::StaleInput)
    ));
    let reference = json!({"className":{"status":"extracted","value":"Fixture Native"},"name":{"status":"extracted","value":"Fixture Native"},"vendor":{"status":"unavailable","reason":"PLUGIN_VENDOR_NOT_STORED"}});
    let mut too_many = reply(&bytes);
    too_many["pluginReferences"]["value"] = json!(vec![reference; 1025]);
    assert!(matches!(
        validate(too_many, &bytes),
        Err(ValidationError::LimitExceeded)
    ));
    let large = project(
        (0..1025)
            .map(|_| event(201, &utf16("Fixture Native")))
            .collect(),
        0,
    );
    assert_eq!(parse_bytes(&large)["code"], "PLUGIN_REFERENCE_LIMIT");
    let mut name_mismatch = reply(&bytes);
    name_mismatch["pluginReferences"]["value"][0]["name"]["value"] = json!("Fabricated");
    assert!(validate(name_mismatch, &bytes).is_err());
}
