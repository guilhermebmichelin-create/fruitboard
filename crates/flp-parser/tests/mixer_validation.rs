use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::*;
use fruitboard_flp_parser::{ExpectedFingerprint, failed, parse_bytes, sha256_hex};
use serde_json::{Value, json};

fn descriptor() -> Value {
    json!({"adapter":"rust-flp-parser","adapterVersion":env!("CARGO_PKG_VERSION"),
        "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","channelGeneratorNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars","patternCount","patternNames","patternNoteCounts","mixerInsertCount","mixerInsertNames"],
        "maxFileBytes":67108864,"maxEvents":100000,"maxChannels":256,"maxEventBytes":67108864,"maxPatterns":1024,"maxPlaylistClips":1024,
        "maxNoteRecordsPerPattern":65536,"maxNoteRecordsTotal":262144,
        "maxMixerInserts":512,"maxMixerInsertCandidates":512})
}
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus")
            .join(name),
    )
    .unwrap()
}
fn context(bytes: &[u8]) -> ParseContext {
    ParseContext {
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
    }
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
) -> Result<ValidatedProjectReply, ValidationError> {
    let context = context(bytes);
    validate_project_reply(
        ProtocolReply::Result(raw),
        &validate_descriptor(&descriptor)?,
        &context,
        &context,
    )
}
fn entries(raw: Value, descriptor: Value, bytes: &[u8]) {
    let ValidatedProjectReply::Metadata(metadata) = validate(raw, descriptor, bytes).unwrap()
    else {
        panic!("metadata expected")
    };
    let SavedMixerInserts::Entries(items) = metadata.mixer_inserts() else {
        panic!("qualified entries expected")
    };
    assert_eq!(items.len(), 16);
    assert_eq!(
        items.iter().map(SavedMixerInsert::id).collect::<Vec<_>>(),
        (1..=16).collect::<Vec<_>>()
    );
}

#[test]
fn pair_advertisement_requires_both_fields_and_positive_ordered_caps() {
    assert!(validate_descriptor(&descriptor()).is_ok());
    for field in ["mixerInsertCount", "mixerInsertNames"] {
        let mut bad = descriptor();
        bad["fields"]
            .as_array_mut()
            .unwrap()
            .retain(|name| name != field);
        assert!(validate_descriptor(&bad).is_err());
    }
    for cap in ["maxMixerInserts", "maxMixerInsertCandidates"] {
        let mut bad = descriptor();
        bad.as_object_mut().unwrap().remove(cap);
        assert!(validate_descriptor(&bad).is_err());
        for value in [
            json!(0),
            json!(-1),
            json!(1.5),
            json!(513),
            json!("512"),
            json!(u64::MAX),
            Value::Null,
        ] {
            let mut bad = descriptor();
            bad[cap] = value;
            assert!(validate_descriptor(&bad).is_err());
        }
    }
    let mut bad = descriptor();
    bad["maxMixerInsertCandidates"] = json!(511);
    assert!(validate_descriptor(&bad).is_err());
    let mut bad = descriptor();
    bad["adapterVersion"] = json!("0.1.2");
    assert!(validate_descriptor(&bad).is_err());
}

#[test]
fn complete_typed_projection_has_positions_and_missing_names_without_inference() {
    let bytes = fixture("FIX-FL2026-MIXER-NAMES.flp");
    entries(reply(&bytes), descriptor(), &bytes);
    let ValidatedProjectReply::Metadata(metadata) =
        validate(reply(&bytes), descriptor(), &bytes).unwrap()
    else {
        panic!()
    };
    let SavedMixerInserts::Entries(items) = metadata.mixer_inserts() else {
        panic!()
    };
    assert!(
        matches!(items[1].name(), MixerInsertName::Extracted(value) if value == "Fixture Mixer A")
    );
    assert!(
        matches!(items[6].name(), MixerInsertName::Extracted(value) if value == "Fixture Mixer B")
    );
    assert!(matches!(items[0].name(), MixerInsertName::Unavailable));
    let bytes = fixture("FIX-FL2026-MIXER-DUPLICATE.flp");
    let ValidatedProjectReply::Metadata(metadata) =
        validate(reply(&bytes), descriptor(), &bytes).unwrap()
    else {
        panic!()
    };
    let SavedMixerInserts::Entries(items) = metadata.mixer_inserts() else {
        panic!()
    };
    for index in [1, 6] {
        assert!(
            matches!(items[index].name(), MixerInsertName::Extracted(value) if value == "Fixture Mixer Same")
        );
    }
    let bytes = fixture("FIX-FL2026-MIXER-SPECIAL.flp");
    let ValidatedProjectReply::Metadata(metadata) =
        validate(reply(&bytes), descriptor(), &bytes).unwrap()
    else {
        panic!()
    };
    let SavedMixerInserts::Entries(items) = metadata.mixer_inserts() else {
        panic!()
    };
    assert!(
        items
            .iter()
            .all(|item| matches!(item.name(), MixerInsertName::Unavailable))
    );
}

#[test]
fn count_ids_shapes_and_name_relationships_fail_entire_advertised_reply() {
    let bytes = fixture("FIX-FL2026-MIXER-NAMES.flp");
    let raw = reply(&bytes);
    for (pointer, bad) in [
        ("/mixerInsertCount", Value::Null),
        ("/mixerInsertNames", Value::Null),
        ("/mixerInsertCount/value", json!(0)),
        ("/mixerInsertCount/value", json!(15)),
        ("/mixerInsertCount/value", json!(17)),
        ("/mixerInsertCount/value", json!(-1)),
        ("/mixerInsertCount/value", json!(16.0)),
        ("/mixerInsertCount/value", json!(u64::MAX)),
        ("/mixerInsertCount/coverage", json!("used-tracks")),
        ("/mixerInsertNames/coverage", json!("used-tracks")),
        ("/mixerInsertNames/items", json!([])),
        ("/mixerInsertNames/items/0/savedInsertId", json!(0)),
        ("/mixerInsertNames/items/0/savedInsertId", json!(17)),
        ("/mixerInsertNames/items/0/savedInsertId", json!(1.0)),
        ("/mixerInsertNames/items/0/savedInsertId", json!(-1)),
        ("/mixerInsertNames/items/0/savedInsertId", json!(65535)),
        ("/mixerInsertNames/items/0/savedInsertId", json!(65536)),
        ("/mixerInsertNames/items/1/savedInsertId", json!(1)),
        ("/mixerInsertNames/items/1/name/value", json!("")),
        ("/mixerInsertNames/items/1/name/value", json!("a\0b")),
        ("/mixerInsertNames/items/1/name/value", json!(false)),
        ("/mixerInsertNames/items/1/name/status", json!("inferred")),
        (
            "/mixerInsertNames/items/0/name/reason",
            json!("MIXER_INSERT_NAMES_INCOMPLETE"),
        ),
        (
            "/mixerInsertNames/reason",
            json!("MIXER_INSERT_NAME_NOT_STORED"),
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
        ("/mixerInsertCount", "items", json!([])),
        ("/mixerInsertCount", "reason", json!("missing")),
        ("/mixerInsertNames", "value", json!([])),
        ("/mixerInsertNames", "method", json!("default")),
        ("/mixerInsertNames/items/1", "category", json!("ordinary")),
        (
            "/mixerInsertNames/items/1/name",
            "confidence",
            json!("high"),
        ),
        ("/mixerInsertNames/items/1/name", "reason", json!("missing")),
        ("/mixerInsertNames/items/0/name", "value", json!("Insert 1")),
    ] {
        let mut bad = raw.clone();
        bad.pointer_mut(pointer).unwrap()[key] = value;
        assert!(
            validate(bad, descriptor(), &bytes).is_err(),
            "{pointer}/{key}"
        );
    }
    for field in ["mixerInsertCount", "mixerInsertNames"] {
        let mut bad = raw.clone();
        bad.as_object_mut().unwrap().remove(field);
        assert!(validate(bad, descriptor(), &bytes).is_err());
    }
    let mut bad = raw.clone();
    bad["mixerInsertNames"]["items"]
        .as_array_mut()
        .unwrap()
        .swap(1, 2);
    assert!(validate(bad, descriptor(), &bytes).is_err());
    let mut bad = raw.clone();
    bad["mixerInsertNames"]["items"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(validate(bad, descriptor(), &bytes).is_err());
}

#[test]
fn extracted_and_incomplete_aggregates_require_matching_entry_states_and_caps() {
    let bytes = fixture("FIX-FL2026-MIXER-NAMES.flp");
    let raw = reply(&bytes);
    let mut complete = raw.clone();
    let mut items = complete["mixerInsertNames"]["items"].clone();
    for entry in items.as_array_mut().unwrap() {
        entry["name"] = json!({"status":"extracted","value":"Synthetic boundary name"});
    }
    complete["mixerInsertNames"] = json!({"status":"extracted","coverage":"explicit-saved-mixer-insert-records","value":items});
    entries(complete.clone(), descriptor(), &bytes);
    let mut bad = complete.clone();
    bad["mixerInsertNames"]["value"][0]["name"] =
        raw["mixerInsertNames"]["items"][0]["name"].clone();
    assert!(validate(bad, descriptor(), &bytes).is_err());
    let mut bad = raw.clone();
    bad["mixerInsertNames"]["items"] = complete["mixerInsertNames"]["value"].clone();
    assert!(validate(bad, descriptor(), &bytes).is_err());
    for (key, limit) in [("maxMixerInserts", 15), ("maxMixerInsertCandidates", 17)] {
        let mut desc = descriptor();
        desc[key] = json!(limit);
        if key == "maxMixerInsertCandidates" {
            desc["maxMixerInserts"] = json!(16);
        }
        assert!(matches!(
            validate(raw.clone(), desc, &bytes),
            Err(ValidationError::LimitExceeded)
        ));
    }
    for text in [
        "a".repeat(4096),
        "\u{0800}".repeat(4096),
        "\u{1f600}".repeat(2048),
    ] {
        let mut bad = raw.clone();
        bad["mixerInsertNames"]["items"][1]["name"]["value"] = json!(text);
        assert!(matches!(
            validate(bad, descriptor(), &bytes),
            Err(ValidationError::LimitExceeded)
        ));
    }
    let mut valid = raw.clone();
    valid["mixerInsertNames"]["items"][1]["name"]["value"] = json!("\u{0800}".repeat(4095));
    entries(valid, descriptor(), &bytes);
    for entry in complete["mixerInsertNames"]["value"]
        .as_array_mut()
        .unwrap()
    {
        entry["name"]["value"] = json!("\u{0001}".repeat(4095));
    }
    assert!(matches!(
        validate(complete, descriptor(), &bytes),
        Err(ValidationError::LimitExceeded)
    ));
}

#[test]
fn global_states_agree_and_other_build_is_unverified() {
    let bytes = fixture("FIX-FL2026-MIXER-DEFAULT.flp");
    for (status, reason) in [
        ("unavailable", "MIXER_INSERT_DATA_NOT_STORED"),
        ("unsupported", "MIXER_INSERT_LAYOUT_UNVERIFIED"),
        ("unsupported", "MIXER_INSERT_BINDING_UNVERIFIED"),
        ("unsupported", "MIXER_INSERT_RECORDS_AMBIGUOUS"),
        ("unsupported", "MIXER_INSERT_LIMIT_EXCEEDED"),
    ] {
        let mut raw = reply(&bytes);
        let state = json!({"status":status,"reason":reason,"coverage":"explicit-saved-mixer-insert-records"});
        raw["mixerInsertCount"] = state.clone();
        raw["mixerInsertNames"] = state;
        assert!(validate(raw.clone(), descriptor(), &bytes).is_ok());
        let mut bad = raw.clone();
        bad["mixerInsertNames"]["reason"] = json!("MIXER_INSERT_UNVERIFIED_BUILD");
        assert!(validate(bad, descriptor(), &bytes).is_err());
        let mut bad = raw.clone();
        bad["mixerInsertNames"]["items"] = json!([]);
        assert!(validate(bad, descriptor(), &bytes).is_err());
        let mut bad = raw.clone();
        bad["mixerInsertCount"]["value"] = json!(0);
        assert!(validate(bad, descriptor(), &bytes).is_err());
    }
    let bytes = fixture("FIX-FL2025-MIN.flp");
    let raw = reply(&bytes);
    let ValidatedProjectReply::Metadata(metadata) =
        validate(raw.clone(), descriptor(), &bytes).unwrap()
    else {
        panic!()
    };
    assert!(matches!(
        metadata.mixer_inserts(),
        SavedMixerInserts::Unsupported(MixerInsertReason::UnverifiedBuild)
    ));
    let mut bad = raw;
    bad["mixerInsertCount"] =
        json!({"status":"extracted","value":16,"coverage":"explicit-saved-mixer-insert-records"});
    assert!(validate(bad, descriptor(), &bytes).is_err());
}

#[test]
fn unadvertised_and_initial_only_compatibility_discards_unselected_data() {
    let bytes = fixture("FIX-FL2026-MIXER-NAMES.flp");
    let mut raw = reply(&bytes);
    raw["mixerInsertNames"] = json!({"arbitrary":"ignored"});
    raw["mixerInsertCount"] = Value::Null;
    let mut desc = descriptor();
    desc["fields"]
        .as_array_mut()
        .unwrap()
        .retain(|name| name != "mixerInsertCount" && name != "mixerInsertNames");
    desc.as_object_mut().unwrap().remove("maxMixerInserts");
    desc.as_object_mut()
        .unwrap()
        .remove("maxMixerInsertCandidates");
    let ValidatedProjectReply::Metadata(metadata) = validate(raw.clone(), desc, &bytes).unwrap()
    else {
        panic!()
    };
    assert!(matches!(
        metadata.mixer_inserts(),
        SavedMixerInserts::NotAdvertised
    ));
    let caps = validate_descriptor(&descriptor()).unwrap();
    let context = context(&bytes);
    assert!(matches!(
        validate_reply(
            ProtocolReply::Result(raw.clone()),
            &caps,
            &context,
            &context
        )
        .unwrap(),
        ValidatedReply::Metadata(_)
    ));
    assert!(validate(raw, descriptor(), &bytes).is_err());
}

#[test]
fn failed_pair_is_strict_and_rejected_requests_do_not_require_result_fields() {
    let bytes = fixture("FIX-FL2026-MIXER-NAMES.flp");
    let raw = failed("INVALID_HEADER");
    assert!(matches!(
        validate(raw.clone(), descriptor(), &bytes).unwrap(),
        ValidatedProjectReply::Failed(_)
    ));
    for (pointer, value) in [
        ("/mixerInsertCount", Value::Null),
        ("/mixerInsertNames/reason", json!("FILE_SIZE_LIMIT")),
    ] {
        let mut bad = raw.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(validate(bad, descriptor(), &bytes).is_err());
    }
    let mut bad = raw;
    bad["mixerInsertNames"]["coverage"] = json!("explicit-saved-mixer-insert-records");
    assert!(validate(bad, descriptor(), &bytes).is_err());
    let context = context(&bytes);
    let caps = validate_descriptor(&descriptor()).unwrap();
    assert!(matches!(
        validate_project_reply(
            ProtocolReply::Rejected {
                code: "INVALID_PATH".into()
            },
            &caps,
            &context,
            &context
        )
        .unwrap(),
        ValidatedProjectReply::Rejected(_)
    ));
}
