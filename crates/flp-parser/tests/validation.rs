use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::*;
use fruitboard_flp_parser::{ExpectedFingerprint, failed};
use serde_json::{Value, json};

fn descriptor() -> Value {
    json!({
        "adapter":"rust-flp-parser", "adapterVersion":env!("CARGO_PKG_VERSION"),
        "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","sampleReferences"],
        "maxFileBytes":4194304, "maxEvents":100000, "maxChannels":256,
        "maxEventBytes":2097152, "maxPatterns":1024, "maxPlaylistClips":1024
    })
}

fn context() -> ParseContext {
    ParseContext {
        root_id: "root-a".into(),
        file_id: "file-a".into(),
        root_revision: 7,
        file_revision: 9,
        root_enabled: true,
        expected: ExpectedFingerprint {
            size: 100,
            modified_at_ms: 12345,
        },
        content_sha256: Some("a".repeat(64)),
    }
}

fn reply() -> Value {
    json!({
        "outcome":"complete",
        "savedVersion":{"status":"extracted","value":"26.1.0.5530"},
        "baseTempoBpm":{"status":"extracted","value":137.0},
        "channelCount":{"status":"extracted","value":1},
        "channelNames":{"status":"extracted","value":["Fixture Sample A"]},
        "sampleReferences":{"status":"extracted","value":["sample.wav"]},
        "inputFingerprint":{"size":100,"modifiedAtMs":12345,"hash":{"algorithm":"sha256","value":"a".repeat(64)}},
        "diagnostics":[],"eventCount":5,"parseElapsedMicros":100
    })
}

fn validate(value: Value) -> Result<ValidatedReply, ValidationError> {
    validate_reply(
        ProtocolReply::Result(value),
        &validate_descriptor(&descriptor()).unwrap(),
        &context(),
        &context(),
    )
}

fn metadata(value: Value) -> ValidatedMetadata {
    match validate(value) {
        Ok(ValidatedReply::Metadata(metadata)) => metadata,
        _ => panic!("expected validated initial metadata"),
    }
}

fn sampler(name: &str, confidence: &str) -> Value {
    json!({"status":"inferred","value":name,"method":"sampler-default-for-known-build","confidence":confidence})
}

#[test]
fn capabilities_require_known_identity_initial_fields_and_safe_resource_ceilings() {
    assert!(validate_descriptor(&descriptor()).is_ok());
    for (key, invalid) in [
        ("adapter", json!("another-parser")),
        ("adapterVersion", json!("999.0.0")),
        ("fields", json!(["savedVersion"])),
        (
            "fields",
            json!([
                "savedVersion",
                "baseTempoBpm",
                "channelCount",
                "channelNames",
                "sampleReferences",
                "savedVersion"
            ]),
        ),
        ("maxFileBytes", json!(4194305)),
        ("maxEvents", json!(100001)),
        ("maxChannels", json!(257)),
        ("maxEventBytes", json!(2097153)),
        ("maxPatterns", json!(1025)),
        ("maxPlaylistClips", json!(1025)),
        ("maxChannels", json!(0)),
        ("maxEvents", json!(-1)),
        ("maxFileBytes", json!(1.5)),
    ] {
        let mut value = descriptor();
        value[key] = invalid;
        assert!(
            matches!(
                validate_descriptor(&value),
                Err(ValidationError::InvalidCapabilities)
            ),
            "{key}"
        );
    }
    for name in [
        "adapter",
        "adapterVersion",
        "fields",
        "maxFileBytes",
        "maxEvents",
        "maxChannels",
        "maxEventBytes",
        "maxPatterns",
        "maxPlaylistClips",
    ] {
        let mut value = descriptor();
        value.as_object_mut().unwrap().remove(name);
        assert!(validate_descriptor(&value).is_err(), "{name}");
    }
    let mut future = descriptor();
    future["fields"]
        .as_array_mut()
        .unwrap()
        .push(json!("futureField"));
    future["futureCapability"] = json!({"ignored":true});
    assert!(validate_descriptor(&future).is_ok());
}

#[test]
fn initial_projection_preserves_extracted_unavailable_and_partial_facts() {
    let result = metadata(reply());
    assert_eq!(result.saved_version(), "26.1.0.5530");
    assert_eq!(result.channel_count(), 1);
    assert!(matches!(result.base_tempo_bpm(), Field::Extracted(bpm) if *bpm == 137.0));
    assert_eq!(
        result.channel_names().entries()[0].value(),
        Some("Fixture Sample A")
    );
    assert_eq!(
        result.sample_references().entries()[0].value(),
        Some("sample.wav")
    );
    assert_eq!(result.outcome(), MetadataOutcome::Complete);
    assert_eq!(result.content_sha256(), "a".repeat(64));
    let mut absent = reply();
    absent["baseTempoBpm"] = json!({"status":"unavailable","reason":"BASE_TEMPO_ABSENT"});
    absent["sampleReferences"] = json!({"status":"unavailable","reason":"SAMPLE_REFERENCE_NOT_STORED","items":[{"status":"unavailable","reason":"SAMPLE_REFERENCE_NOT_STORED"}]});
    let absent = metadata(absent);
    assert!(matches!(
        absent.base_tempo_bpm(),
        Field::Unavailable(MissingReason::BaseTempoAbsent)
    ));
    assert_eq!(absent.sample_references().entries()[0].value(), None);
    let mut partial = reply();
    partial["outcome"] = json!("partial");
    partial["diagnostics"] = json!([{"code":"UNSUPPORTED_EVENT","eventId":255}]);
    assert_eq!(
        metadata(partial).diagnostics(),
        &[Diagnostic::UnsupportedEvent255]
    );
}

#[test]
fn inferred_names_keep_per_channel_provenance_and_aggregate_confidence() {
    let mut mixed = reply();
    mixed["channelCount"]["value"] = json!(2);
    mixed["sampleReferences"]["value"] = json!(["a.wav", "b.wav"]);
    mixed["channelNames"] = json!({
        "status":"inferred","value":["Stored","Sampler"],
        "method":"mixed-extracted-and-sampler-default","confidence":"medium",
        "items":[{"status":"extracted","value":"Stored"},sampler("Sampler","high")]
    });
    let result = metadata(mixed.clone());
    assert!(matches!(
        result.channel_names().entries()[0],
        TextField::Extracted(_)
    ));
    assert!(matches!(
        result.channel_names().entries()[1],
        TextField::Inferred {
            confidence: Confidence::High,
            ..
        }
    ));
    assert_eq!(
        result.channel_names().status(),
        ListStatus::Inferred {
            method: NameInference::MixedExtractedAndSamplerDefault,
            confidence: Confidence::Medium
        }
    );
    mixed["channelNames"] = json!({
        "status":"inferred","value":["Sampler","Sampler 2"],
        "method":"sampler-default-for-known-build","confidence":"medium",
        "items":[sampler("Sampler","high"),sampler("Sampler 2","medium")]
    });
    assert!(matches!(
        metadata(mixed.clone()).channel_names().entries()[1],
        TextField::Inferred {
            confidence: Confidence::Medium,
            ..
        }
    ));
    for (pointer, invalid) in [
        ("/channelNames/confidence", json!("high")),
        ("/channelNames/items/1/confidence", json!("high")),
        ("/channelNames/items/1/value", json!("Sampler 99")),
        ("/channelNames/items/1/method", json!("unverified-guess")),
        ("/channelNames/value/1", json!("Different")),
        ("/savedVersion/value", json!("24.1.0.4225")),
    ] {
        let mut invalid_reply = mixed.clone();
        *invalid_reply.pointer_mut(pointer).unwrap() = invalid;
        assert!(
            matches!(validate(invalid_reply), Err(ValidationError::InvalidReply)),
            "{pointer}"
        );
    }
    let mut missing = mixed;
    missing["channelNames"] = json!({"status":"unavailable","reason":"CHANNEL_NAME_NOT_STORED","items":[sampler("Sampler","high"),{"status":"unavailable","reason":"CHANNEL_NAME_NOT_STORED"}]});
    assert!(matches!(
        metadata(missing).channel_names().entries()[0],
        TextField::Inferred { .. }
    ));
}

#[test]
fn malformed_fields_and_cross_field_relationships_fail_closed() {
    for (pointer, invalid) in [
        ("/outcome", json!("future")),
        ("/savedVersion/value", json!("26.1")),
        ("/savedVersion/value", json!("27.1.0.9999")),
        ("/baseTempoBpm/value", json!(0)),
        ("/baseTempoBpm/value", json!(1000)),
        ("/baseTempoBpm/value", json!("NaN")),
        ("/baseTempoBpm/value", Value::Null),
        ("/channelCount/value", json!(-1)),
        ("/channelCount/value", json!(1.5)),
        ("/channelNames/value", json!([])),
        ("/channelNames/value/0", json!(false)),
        ("/channelNames/value/0", json!("a\u{0000}b")),
        ("/sampleReferences/value/0", json!("")),
        ("/diagnostics", Value::Null),
        (
            "/diagnostics",
            json!([{"code":"PRIVATE_DIAGNOSTIC","message":"private"}]),
        ),
        ("/outcome", json!("partial")),
        ("/parseElapsedMicros", json!(60000001)),
    ] {
        let mut value = reply();
        *value.pointer_mut(pointer).unwrap() = invalid;
        assert!(validate(value).is_err(), "{pointer}");
    }
    for field in [
        "savedVersion",
        "baseTempoBpm",
        "channelCount",
        "channelNames",
        "sampleReferences",
    ] {
        let mut missing = reply();
        missing.as_object_mut().unwrap().remove(field);
        assert!(validate(missing).is_err(), "{field}");
        let mut ambiguous = reply();
        ambiguous[field]["reason"] = json!("BASE_TEMPO_ABSENT");
        assert!(validate(ambiguous).is_err(), "{field}");
    }
    let mut missing_without_missing_item = reply();
    missing_without_missing_item["channelNames"] = json!({"status":"unavailable","reason":"CHANNEL_NAME_NOT_STORED","items":[{"status":"extracted","value":"Stored"}]});
    assert!(validate(missing_without_missing_item).is_err());
}

#[test]
fn bounds_follow_descriptor_limits_and_utf16_source_units() {
    let mut maximal = reply();
    maximal["channelNames"]["value"][0] = json!("漢".repeat(4095));
    assert!(
        validate(maximal.clone()).is_ok(),
        "valid UTF-8 can exceed 8192 bytes"
    );
    maximal["channelNames"]["value"][0] = json!("漢".repeat(4096));
    assert!(matches!(
        validate(maximal),
        Err(ValidationError::LimitExceeded)
    ));
    for (pointer, invalid) in [
        ("/channelCount/value", json!(257)),
        ("/eventCount", json!(100001)),
    ] {
        let mut value = reply();
        *value.pointer_mut(pointer).unwrap() = invalid;
        assert!(matches!(
            validate(value),
            Err(ValidationError::LimitExceeded)
        ));
    }
    let mut too_many = reply();
    too_many["outcome"] = json!("partial");
    too_many["eventCount"] = json!(1025);
    too_many["diagnostics"] = json!(vec![
        json!({"code":"UNSUPPORTED_EVENT","eventId":255});
        1025
    ]);
    assert!(matches!(
        validate(too_many),
        Err(ValidationError::LimitExceeded)
    ));
    let mut limited = descriptor();
    limited["maxChannels"] = json!(1);
    let mut value = reply();
    value["channelCount"]["value"] = json!(2);
    assert!(matches!(
        validate_reply(
            ProtocolReply::Result(value),
            &validate_descriptor(&limited).unwrap(),
            &context(),
            &context()
        ),
        Err(ValidationError::LimitExceeded)
    ));
}

#[test]
fn returned_fingerprint_and_independent_digest_must_match() {
    for (pointer, invalid, expected) in [
        (
            "/inputFingerprint/size",
            json!(101),
            ValidationError::FingerprintMismatch,
        ),
        (
            "/inputFingerprint/modifiedAtMs",
            json!(12346),
            ValidationError::FingerprintMismatch,
        ),
        (
            "/inputFingerprint/hash/value",
            json!("b".repeat(64)),
            ValidationError::FingerprintMismatch,
        ),
        (
            "/inputFingerprint/hash/value",
            json!("A".repeat(64)),
            ValidationError::InvalidReply,
        ),
        (
            "/inputFingerprint/hash/value",
            json!("g".repeat(64)),
            ValidationError::InvalidReply,
        ),
        (
            "/inputFingerprint/hash/value",
            json!("a".repeat(63)),
            ValidationError::InvalidReply,
        ),
        (
            "/inputFingerprint/hash/algorithm",
            json!("md5"),
            ValidationError::InvalidReply,
        ),
    ] {
        let mut value = reply();
        *value.pointer_mut(pointer).unwrap() = invalid;
        assert!(
            matches!(validate(value), Err(error) if error == expected),
            "{pointer}"
        );
    }
    let mut unknown_hash = context();
    unknown_hash.content_sha256 = None;
    assert!(
        validate_reply(
            ProtocolReply::Result(reply()),
            &validate_descriptor(&descriptor()).unwrap(),
            &unknown_hash,
            &unknown_hash
        )
        .is_ok()
    );
    let mut current = unknown_hash.clone();
    current.content_sha256 = Some("b".repeat(64));
    assert!(matches!(
        validate_reply(
            ProtocolReply::Result(reply()),
            &validate_descriptor(&descriptor()).unwrap(),
            &unknown_hash,
            &current
        ),
        Err(ValidationError::FingerprintMismatch)
    ));
}

#[test]
fn authoritative_identity_revision_enablement_and_fingerprint_changes_discard_results() {
    let original = context();
    let mut changes = Vec::new();
    let mut changed = original.clone();
    changed.root_id = "root-b".into();
    changes.push(changed);
    let mut changed = original.clone();
    changed.file_id = "file-b".into();
    changes.push(changed);
    let mut changed = original.clone();
    changed.root_revision += 1;
    changes.push(changed);
    let mut changed = original.clone();
    changed.file_revision += 1;
    changes.push(changed);
    let mut changed = original.clone();
    changed.root_enabled = false;
    changes.push(changed);
    let mut changed = original.clone();
    changed.expected.size += 1;
    changes.push(changed);
    let mut changed = original.clone();
    changed.expected.modified_at_ms += 1;
    changes.push(changed);
    let mut changed = original.clone();
    changed.content_sha256 = Some("b".repeat(64));
    changes.push(changed);
    for current in changes {
        assert!(matches!(
            validate_reply(
                ProtocolReply::Result(reply()),
                &validate_descriptor(&descriptor()).unwrap(),
                &original,
                &current
            ),
            Err(ValidationError::StaleInput)
        ));
    }
    let mut disabled = original;
    disabled.root_enabled = false;
    assert!(matches!(
        validate_reply(
            ProtocolReply::Result(reply()),
            &validate_descriptor(&descriptor()).unwrap(),
            &disabled,
            &disabled
        ),
        Err(ValidationError::StaleInput)
    ));
}

#[test]
fn failed_unsupported_and_rejected_outcomes_preserve_safe_categories() {
    assert!(
        matches!(validate(failed("INVALID_HEADER")), Ok(ValidatedReply::Failed(code)) if code.as_str() == "INVALID_HEADER")
    );
    let mut inconsistent = failed("INVALID_HEADER");
    inconsistent["channelNames"]["reason"] = json!("INPUT_CHANGED");
    assert!(matches!(
        validate(inconsistent),
        Err(ValidationError::InvalidReply)
    ));
    assert!(matches!(
        validate(failed("PRIVATE_SECRET")),
        Err(ValidationError::InvalidReply)
    ));
    let mut unsupported = reply();
    unsupported["outcome"] = json!("unsupported");
    unsupported["code"] = json!("UNSUPPORTED_SAVED_VERSION");
    unsupported["savedVersion"]["value"] = json!("27.1.0.9999");
    for field in [
        "baseTempoBpm",
        "channelCount",
        "channelNames",
        "sampleReferences",
    ] {
        unsupported[field] = json!({"status":"unsupported","reason":"UNSUPPORTED_SAVED_VERSION"});
    }
    assert!(
        matches!(validate(unsupported), Ok(ValidatedReply::UnsupportedSavedVersion(version)) if version == "27.1.0.9999")
    );
    let capabilities = validate_descriptor(&descriptor()).unwrap();
    for code in ["INVALID_PATH", "RESPONSE_LIMIT"] {
        assert!(
            matches!(validate_reply(ProtocolReply::Rejected { code:code.into() }, &capabilities, &context(), &context()), Ok(ValidatedReply::Rejected(parsed)) if parsed.as_str() == code)
        );
    }
    assert!(matches!(
        validate_reply(
            ProtocolReply::Rejected {
                code: "PRIVATE_SECRET".into()
            },
            &capabilities,
            &context(),
            &context()
        ),
        Err(ValidationError::InvalidReply)
    ));
}

#[test]
fn unvalidated_extensions_and_unknown_fields_do_not_enter_the_projection() {
    let mut future = reply();
    future["patternCount"] = json!({"status":"future","value":"unvalidated"});
    future["privateExtension"] = json!({"path":"not part of the projection"});
    let projected = metadata(future);
    assert_eq!(projected.channel_count(), 1);
    assert_eq!(projected.saved_version(), "26.1.0.5530");
}
