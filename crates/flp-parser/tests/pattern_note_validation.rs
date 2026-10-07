use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::*;
use fruitboard_flp_parser::{ExpectedFingerprint, parse_bytes, sha256_hex};
use serde_json::{Value, json};

fn descriptor() -> Value {
    json!({"adapter":"rust-flp-parser","adapterVersion":env!("CARGO_PKG_VERSION"),
        "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","channelGeneratorNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars","patternCount","patternNames","patternNoteCounts"],
        "maxFileBytes":67108864,"maxEvents":100000,"maxChannels":256,"maxEventBytes":67108864,"maxPatterns":1024,"maxPlaylistClips":1024,
        "maxNoteRecordsPerPattern":65536,"maxNoteRecordsTotal":262144})
}
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus")
            .join(name),
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
        _ => panic!("metadata expected"),
    }
}
#[test]
fn approved_counts_and_named_empty_survive_full_typed_validation() {
    let bytes = fixture("FIX-FL2026-NOTES.flp");
    let metadata = validate(reply(&bytes), descriptor(), &bytes).unwrap();
    let PatternNoteCounts::Entries(items) = metadata.pattern_note_counts() else {
        panic!("counts required")
    };
    assert_eq!(
        items
            .iter()
            .map(|item| match item.count() {
                NoteRecordCount::Extracted(n) => (item.id(), *n),
                _ => panic!("qualified count expected"),
            })
            .collect::<Vec<_>>(),
        [(1, 3), (2, 2), (3, 4)]
    );
    let bytes = fixture("FIX-FL2026-NAMED-EMPTY.flp");
    let metadata = validate(reply(&bytes), descriptor(), &bytes).unwrap();
    let PatternNoteCounts::Entries(items) = metadata.pattern_note_counts() else {
        panic!("entry required")
    };
    assert_eq!(items.len(), 1);
    assert!(matches!(items[0].count(), NoteRecordCount::Unavailable));
}
#[test]
fn advertisement_requires_valid_limits_and_pattern_contract_together() {
    for key in ["maxNoteRecordsPerPattern", "maxNoteRecordsTotal"] {
        let mut missing = descriptor();
        missing.as_object_mut().unwrap().remove(key);
        assert!(validate_descriptor(&missing).is_err());
        for value in [
            json!(0),
            json!(-1),
            json!(1.5),
            json!("65536"),
            json!(u64::MAX),
            Value::Null,
        ] {
            let mut bad = descriptor();
            bad[key] = value;
            assert!(validate_descriptor(&bad).is_err());
        }
    }
    for name in ["patternCount", "patternNames"] {
        let mut bad = descriptor();
        bad["fields"]
            .as_array_mut()
            .unwrap()
            .retain(|field| field != name);
        assert!(validate_descriptor(&bad).is_err());
    }
    let mut bad = descriptor();
    bad["adapterVersion"] = json!("0.1.1");
    assert!(validate_descriptor(&bad).is_err());
}
#[test]
fn malformed_advertised_counts_shapes_and_relationships_fail_closed() {
    let bytes = fixture("FIX-FL2026-NOTES.flp");
    let raw = reply(&bytes);
    for (pointer, bad) in [
        ("/patternNoteCounts", Value::Null),
        ("/patternNoteCounts/coverage", json!("audible-notes")),
        ("/patternNoteCounts/status", json!("inferred")),
        ("/patternNoteCounts/value", json!([])),
        (
            "/patternNoteCounts/value",
            json!([raw["patternNoteCounts"]["value"][0].clone()]),
        ),
        ("/patternNoteCounts/value/0/patternId", json!(0)),
        ("/patternNoteCounts/value/0/patternId", json!(65536)),
        ("/patternNoteCounts/value/0/patternId", json!(1.0)),
        ("/patternNoteCounts/value/1/patternId", json!(1)),
        ("/patternNoteCounts/value/0/noteCount/value", json!(0)),
        ("/patternNoteCounts/value/0/noteCount/value", json!(-1)),
        ("/patternNoteCounts/value/0/noteCount/value", json!(1.5)),
        ("/patternNoteCounts/value/0/noteCount/value", json!(65537)),
        (
            "/patternNoteCounts/value/0/noteCount/value",
            json!(u64::MAX),
        ),
        (
            "/patternNoteCounts/value/0/noteCount",
            json!({"status":"unavailable","reason":"PATTERN_NOTES_NOT_STORED"}),
        ),
        (
            "/patternNoteCounts/value/0/noteCount",
            json!({"status":"unsupported","reason":"PATTERN_NOTE_LAYOUT_UNVERIFIED"}),
        ),
    ] {
        let mut bad_raw = raw.clone();
        *bad_raw.pointer_mut(pointer).unwrap() = bad;
        assert!(
            validate(bad_raw, descriptor(), &bytes).is_err(),
            "{pointer}"
        );
    }
    for (pointer, key, value) in [
        ("/patternNoteCounts", "items", json!([])),
        (
            "/patternNoteCounts",
            "reason",
            json!("PATTERN_NOTE_COUNTS_INCOMPLETE"),
        ),
        ("/patternNoteCounts", "method", json!("inference")),
        (
            "/patternNoteCounts/value/0",
            "name",
            json!("private extension"),
        ),
        (
            "/patternNoteCounts/value/0/noteCount",
            "reason",
            json!("PATTERN_NOTES_NOT_STORED"),
        ),
        (
            "/patternNoteCounts/value/0/noteCount",
            "confidence",
            json!("high"),
        ),
        ("/patternNoteCounts/value/0/noteCount", "records", json!([])),
    ] {
        let mut bad = raw.clone();
        bad.pointer_mut(pointer).unwrap()[key] = value;
        assert!(
            validate(bad, descriptor(), &bytes).is_err(),
            "{pointer}/{key}"
        );
    }
    let mut missing = raw.clone();
    missing.as_object_mut().unwrap().remove("patternNoteCounts");
    assert!(validate(missing, descriptor(), &bytes).is_err());
    for (key, limit) in [("maxNoteRecordsPerPattern", 3), ("maxNoteRecordsTotal", 8)] {
        let mut small = descriptor();
        small[key] = json!(limit);
        assert!(validate(raw.clone(), small, &bytes).is_err());
    }
}
#[test]
fn mixed_states_and_global_failures_keep_exact_reasons_and_other_metadata() {
    let bytes = fixture("FIX-FL2026-NOTES.flp");
    let mut raw = reply(&bytes);
    let mut items = raw["patternNoteCounts"]["value"].clone();
    items[0]["noteCount"] = json!({"status":"unavailable","reason":"PATTERN_NOTES_NOT_STORED"});
    raw["patternNoteCounts"] = json!({"status":"unavailable","reason":"PATTERN_NOTE_COUNTS_INCOMPLETE","coverage":"stored-pattern-note-records","items":items});
    assert!(validate(raw.clone(), descriptor(), &bytes).is_ok());
    raw["patternNoteCounts"]["items"][1]["noteCount"] =
        json!({"status":"unsupported","reason":"MULTIPLE_PATTERN_NOTE_PAYLOADS"});
    assert!(validate(raw.clone(), descriptor(), &bytes).is_err());
    raw["patternNoteCounts"]["status"] = json!("unsupported");
    assert!(validate(raw.clone(), descriptor(), &bytes).is_ok());
    raw["patternNoteCounts"]["items"][1]["noteCount"]["reason"] = json!("arbitrary private code");
    assert!(validate(raw, descriptor(), &bytes).is_err());
    for reason in [
        "PATTERN_NOTE_BINDING_UNVERIFIED",
        "PATTERN_NOTE_LIMIT_EXCEEDED",
    ] {
        let mut raw = reply(&bytes);
        raw["patternNoteCounts"] = json!({"status":"unsupported","reason":reason,"coverage":"stored-pattern-note-records"});
        assert!(validate(raw.clone(), descriptor(), &bytes).is_ok());
        raw["patternNoteCounts"]["status"] = json!("unavailable");
        assert!(validate(raw, descriptor(), &bytes).is_err());
    }
    let mut raw = reply(&bytes);
    raw["patternNoteCounts"] = json!({"status":"unavailable","reason":"PATTERN_DATA_NOT_STORED","coverage":"stored-pattern-note-records"});
    assert!(validate(raw, descriptor(), &bytes).is_err());
}
#[test]
fn checked_sum_accepts_total_boundary_and_rejects_the_next_record() {
    let bytes = fixture("FIX-FL2026-NOTES.flp");
    let make = |counts: &[u32]| {
        let mut raw = reply(&bytes);
        raw["patternCount"]["value"] = json!(counts.len());
        raw["patternNames"]["value"] = json!(
            counts
                .iter()
                .enumerate()
                .map(|(i, _)| json!({"patternId":i+1,"name":{"status":"extracted","value":""}}))
                .collect::<Vec<_>>()
        );
        raw["patternNoteCounts"]["value"] = json!(
            counts
                .iter()
                .enumerate()
                .map(|(i, n)| json!({"patternId":i+1,"noteCount":{"status":"extracted","value":n}}))
                .collect::<Vec<_>>()
        );
        raw
    };
    assert!(validate(make(&[65536; 4]), descriptor(), &bytes).is_ok());
    assert!(validate(make(&[65536, 65536, 65536, 65536, 1]), descriptor(), &bytes).is_err());
}
#[test]
fn unadvertised_extension_is_discarded_while_initial_validation_remains_usable() {
    let bytes = fixture("FIX-FL2026-NOTES.flp");
    let mut raw = reply(&bytes);
    raw["patternNoteCounts"] = json!({"private":"untrusted"});
    let mut old = descriptor();
    old["fields"]
        .as_array_mut()
        .unwrap()
        .retain(|field| field != "patternNoteCounts");
    for key in ["maxNoteRecordsPerPattern", "maxNoteRecordsTotal"] {
        old.as_object_mut().unwrap().remove(key);
    }
    assert!(matches!(
        validate(raw.clone(), old.clone(), &bytes)
            .unwrap()
            .pattern_note_counts(),
        PatternNoteCounts::NotAdvertised
    ));
    // Initial-only validation deliberately does not select richer extensions.
    let context = ParseContext {
        root_id: "r".into(),
        file_id: "f".into(),
        root_revision: 1,
        file_revision: 1,
        root_enabled: true,
        expected: ExpectedFingerprint {
            size: bytes.len() as u64,
            modified_at_ms: 42,
        },
        content_sha256: Some(sha256_hex(&bytes)),
    };
    assert!(
        validate_reply(
            ProtocolReply::Result(raw),
            &validate_descriptor(&descriptor()).unwrap(),
            &context,
            &context
        )
        .is_ok()
    );
}
#[test]
fn no_patterns_and_other_saved_builds_validate_only_their_fixed_states() {
    let bytes = fixture("FIX-FL2026-MIN.flp");
    assert!(matches!(
        validate(reply(&bytes), descriptor(), &bytes)
            .unwrap()
            .pattern_note_counts(),
        PatternNoteCounts::Unavailable(PatternNoteReason::DataNotStored)
    ));
    let bytes = fixture("FIX-FL2025-MIN.flp");
    let mut raw = reply(&bytes);
    assert!(matches!(
        validate(raw.clone(), descriptor(), &bytes)
            .unwrap()
            .pattern_note_counts(),
        PatternNoteCounts::Unsupported(PatternNoteReason::UnverifiedBuild)
    ));
    raw["patternNoteCounts"] =
        json!({"status":"extracted","coverage":"stored-pattern-note-records","value":[]});
    assert!(validate(raw, descriptor(), &bytes).is_err());
}
