//! Actual approved F13 plus constructed protocol cases; no wider compatibility claim.
use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::*;
use fruitboard_flp_parser::{ExpectedFingerprint, parse_bytes, sha256_hex};
use serde_json::{Value, json};

fn descriptor() -> Value {
    json!({"adapter":"rust-flp-parser","adapterVersion":env!("CARGO_PKG_VERSION"),
        "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","channelGeneratorNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars","patternCount","patternNames"],
        "maxFileBytes":67108864,"maxEvents":100000,"maxChannels":256,"maxEventBytes":67108864,"maxPatterns":1024,"maxPlaylistClips":1024})
}
fn fixture() -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus/FIX-FL2026-PATTERNS.flp"),
    )
    .unwrap()
}
fn reply(bytes: &[u8]) -> Value {
    let mut raw = parse_bytes(bytes);
    raw["inputFingerprint"] = json!({"size":bytes.len(),"modifiedAtMs":42,"hash":{"algorithm":"sha256","value":sha256_hex(bytes)}});
    raw["filesystemCreatedAtMs"] = json!({"status":"extracted","value":1000});
    raw
}
fn validate(
    raw: Value,
    descriptor: Value,
    bytes: &[u8],
) -> Result<ValidatedProjectMetadata, ValidationError> {
    let context = ParseContext {
        root_id: "root".into(),
        file_id: "file".into(),
        root_revision: 1,
        file_revision: 1,
        root_enabled: true,
        expected: ExpectedFingerprint {
            size: bytes.len() as u64,
            modified_at_ms: 42,
        },
        content_sha256: Some(sha256_hex(bytes)),
    };
    match validate_project_reply(
        ProtocolReply::Result(raw),
        &validate_descriptor(&descriptor)?,
        &context,
        &context,
    )? {
        ValidatedProjectReply::Metadata(metadata) => Ok(*metadata),
        _ => panic!("expected metadata"),
    }
}

#[test]
fn approved_f13_keeps_three_ids_names_and_four_placements_separate() {
    let bytes = fixture();
    let before = sha256_hex(&bytes);
    let raw = reply(&bytes);
    assert_eq!(
        raw["playlistPatternClips"]["value"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let metadata = validate(raw, descriptor(), &bytes).unwrap();
    let SavedPatterns::Entries(patterns) = metadata.patterns() else {
        panic!("saved patterns required")
    };
    assert_eq!(
        patterns
            .iter()
            .map(|p| (p.id(), p.name().unwrap()))
            .collect::<Vec<_>>(),
        [
            (1, "Fixture Pattern A"),
            (2, "Fixture Pattern B"),
            (3, "Fixture Pattern C")
        ]
    );
    assert_eq!(
        before,
        "227b44e0a409e067f59d0e21da7c187537e242e45c48ad015e9396aadc005c2a"
    );
    assert_eq!(sha256_hex(&fixture()), before);
}

#[test]
fn invalid_counts_ids_names_and_aggregate_states_are_rejected() {
    let bytes = fixture();
    let raw = reply(&bytes);
    for (pointer, bad) in [
        ("/patternCount/value", json!(0)),
        ("/patternCount/value", json!(1025)),
        ("/patternCount/value", json!(2)),
        ("/patternCount/value", json!(3.5)),
        ("/playlistPatternClips/value/0/patternId", json!(4)),
        ("/patternNames/value/0/patternId", json!(0)),
        ("/patternNames/value/0/patternId", json!(65536)),
        ("/patternNames/value/1/patternId", json!(1)),
        ("/patternNames/value/0/patternId", json!(4)),
        ("/patternNames/value/0/name/value", json!("bad\u{0000}")),
        ("/patternNames/value/0/name/value", json!("x".repeat(4096))),
        ("/patternNames/value/0/name/value", json!("😀".repeat(2048))),
        ("/patternNames/value/0/name/status", json!("inferred")),
        (
            "/patternNames/value/0/name",
            json!({"status":"unavailable","reason":"PATTERN_NAME_NOT_STORED"}),
        ),
        (
            "/patternCount",
            json!({"status":"unavailable","reason":"PATTERN_DATA_NOT_STORED"}),
        ),
        (
            "/patternNames",
            json!({"status":"unsupported","reason":"PATTERN_NAMES_UNVERIFIED_BUILD"}),
        ),
    ] {
        let mut invalid = raw.clone();
        *invalid.pointer_mut(pointer).unwrap() = bad;
        assert!(
            validate(invalid, descriptor(), &bytes).is_err(),
            "{pointer}"
        );
    }
    let mut small = descriptor();
    small["maxPatterns"] = json!(2);
    assert!(validate(raw, small, &bytes).is_err());
}

#[test]
fn missing_empty_unicode_names_sparse_ids_and_unknown_default_are_explicit() {
    let bytes = fixture();
    let mut raw = reply(&bytes);
    raw["patternNames"] = json!({"status":"unavailable","reason":"PATTERN_NAME_NOT_STORED","items":[
        {"patternId":1,"name":{"status":"extracted","value":""}},
        {"patternId":2,"name":{"status":"unavailable","reason":"PATTERN_NAME_NOT_STORED"}},
        {"patternId":65535,"name":{"status":"extracted","value":"合😀\u{202e}\n"}}]});
    for clip in raw["playlistPatternClips"]["value"].as_array_mut().unwrap() {
        if clip["patternId"] == 3 {
            clip["patternId"] = json!(65535);
        }
    }
    let metadata = validate(raw.clone(), descriptor(), &bytes).unwrap();
    let SavedPatterns::Entries(items) = metadata.patterns() else {
        panic!("entries required")
    };
    assert_eq!(items[0].name(), Some(""));
    assert_eq!(items[1].name(), None);
    assert_eq!(items[2].id(), 65535);
    raw["patternNames"]["items"][1]["name"] = json!({"status":"extracted","value":"Present"});
    assert!(validate(raw, descriptor(), &bytes).is_err());
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus/FIX-FL2026-MIN.flp"),
    )
    .unwrap();
    let raw = reply(&bytes);
    assert!(matches!(
        validate(raw, descriptor(), &bytes).unwrap().patterns(),
        SavedPatterns::Unavailable
    ));
}

#[test]
fn unadvertised_fields_are_discarded_and_older_builds_cannot_claim_patterns() {
    let bytes = fixture();
    let mut raw = reply(&bytes);
    raw["patternCount"] = json!("private arbitrary extension");
    raw["patternNames"] = json!({"path":"C:\\Private\\secret"});
    let mut old_descriptor = descriptor();
    old_descriptor["fields"]
        .as_array_mut()
        .unwrap()
        .retain(|field| field != "patternNames");
    assert!(matches!(
        validate(raw, old_descriptor, &bytes).unwrap().patterns(),
        SavedPatterns::Unsupported(PatternSupport::NotAdvertised)
    ));
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus/FIX-FL2025-MIN.flp"),
    )
    .unwrap();
    let mut raw = reply(&bytes);
    assert!(matches!(
        validate(raw.clone(), descriptor(), &bytes)
            .unwrap()
            .patterns(),
        SavedPatterns::Unsupported(PatternSupport::UnverifiedBuild)
    ));
    raw["patternCount"] = json!({"status":"extracted","value":1});
    assert!(validate(raw, descriptor(), &bytes).is_err());
}

#[test]
fn stored_projection_rejects_present_corruption_and_build_contradictions() {
    let valid = json!({"status":"extracted","count":1,"items":[{"patternId":65535,"name":{"status":"extracted","value":"😀".repeat(2047)}}]});
    assert!(validate_stored_patterns(&valid, "26.1.0.5530").is_ok());
    assert!(validate_stored_patterns(&valid, "25.1.3.4922").is_err());
    for (key, bad) in [
        ("count", json!(0)),
        ("count", json!(2)),
        ("value", json!([])),
        ("reason", json!("private")),
        ("items", json!([])),
    ] {
        let mut invalid = valid.clone();
        invalid[key] = bad;
        assert!(validate_stored_patterns(&invalid, "26.1.0.5530").is_err());
    }
    for value in [
        Value::Null,
        json!({"status":"unavailable","reason":"PATTERN_DATA_NOT_STORED","count":0}),
        json!({"status":"unsupported","reason":"PATTERN_DETAILS_UNVERIFIED_BUILD"}),
    ] {
        assert!(validate_stored_patterns(&value, "26.1.0.5530").is_err());
    }
}
