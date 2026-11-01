use fruitboard_flp_parser::supervisor::ProtocolReply;
use fruitboard_flp_parser::validation::*;
use fruitboard_flp_parser::{ExpectedFingerprint, parse_bytes, sha256_hex};
use serde_json::{Value, json};

const RECORD: [u8; 24] = [
    0, 0, 0, 0, 0, 64, 0, 0, 48, 0, 0, 0, 60, 0, 0, 0, 120, 0, 64, 0, 64, 89, 128, 128,
];
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus")
            .join(name),
    )
    .unwrap()
}
fn descriptor() -> Value {
    json!({"adapter":"rust-flp-parser","adapterVersion":env!("CARGO_PKG_VERSION"),
        "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","sampleReferences",
            "projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences",
            "playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds",
            "playlistPatternSpanBars","patternCount","patternNames","patternNoteCounts","patternNoteSequences"],
        "maxFileBytes":67108864,"maxEvents":100000,"maxChannels":256,"maxEventBytes":67108864,
        "maxPatterns":1024,"maxPlaylistClips":1024,"maxNoteRecordsPerPattern":65536,"maxNoteRecordsTotal":262144,
        "maxDetailedNotesPerPattern":512,"maxDetailedNotesTotal":1024,"maxNoteSequenceBytes":131072})
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
fn event(id: u8, data: &[u8]) -> Vec<u8> {
    let mut result = vec![id];
    if id >= 192 {
        let mut size = data.len();
        loop {
            let rest = size >> 7;
            result.push((size & 127) as u8 | if rest == 0 { 0 } else { 128 });
            size = rest;
            if size == 0 {
                break;
            }
        }
    }
    result.extend_from_slice(data);
    result
}
fn stream(patterns: &[(u16, Vec<u8>)], channel_ids: &[u16]) -> Vec<u8> {
    let mut contents = event(199, b"26.1.0.5530\0");
    for (id, notes) in patterns {
        contents.extend(event(65, &id.to_le_bytes()));
        contents.extend(event(224, notes));
    }
    for id in channel_ids {
        contents.extend(event(64, &id.to_le_bytes()));
        contents.extend(event(21, &[0]));
        contents.extend(event(201, &[0, 0]));
    }
    let mut bytes = b"FLhd\x06\0\0\0\0\0\0\0\x60\0FLdt".to_vec();
    bytes[10..12].copy_from_slice(&(channel_ids.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&(contents.len() as u32).to_le_bytes());
    bytes.extend(contents);
    bytes
}

#[test]
fn ordinary_note_pilot_preserves_counts_and_current_qualification_boundary() {
    let registered: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parser-corpus/ordinary-note-coverage-expectations.json"
    ))
    .unwrap();
    let cases = registered["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 8);
    for case in cases {
        let filename = case["file"].as_str().unwrap();
        let before = fixture(filename);
        assert_eq!(before.len() as u64, case["bytes"].as_u64().unwrap());
        assert_eq!(sha256_hex(&before), case["sha256"]);
        let raw = reply(&before);
        assert_eq!(raw["outcome"], "complete", "{filename}");
        assert_eq!(raw["savedVersion"]["value"], registered["savedBuild"]);
        assert_eq!(raw["baseTempoBpm"]["value"], 130.0);
        assert_eq!(
            raw["channelNames"]["value"],
            case["independentGui"]["channelOrder"]
        );
        assert_eq!(raw["sampleReferences"]["status"], "unavailable");
        assert_eq!(
            raw["patternNoteCounts"]["value"][0]["noteCount"]["value"],
            case["savedRecordCount"]
        );
        let pattern = &raw["patternNoteSequences"]["patterns"][0];
        assert_eq!(pattern["patternId"], 1);
        if case["currentParser"]["patternStatus"] == "unsupported" {
            // Includes N15's otherwise supported sibling: never expose a
            // partial sequence when another record has unexplained bytes.
            assert_eq!(
                pattern,
                &json!({"patternId":1,"status":"unsupported",
                    "reason":"PATTERN_NOTE_RECORD_UNQUALIFIED"}),
                "{filename}"
            );
        } else {
            assert_eq!(
                pattern,
                &json!({"patternId":1,"status":"extracted","records":[{
                    "ordinal":0,"channelIndex":0,"savedKey":60,"startTick":0,
                    "durationTick":48,"storedVelocityByte":89
                }]}),
                "{filename}"
            );
        }
        assert!(validate(raw, descriptor(), &before).is_ok(), "{filename}");
        assert_eq!(fixture(filename), before);
    }
}

#[test]
fn all_seven_approved_cases_extract_and_validate_against_independent_expectations() {
    let registered: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parser-corpus/note-sequence-expectations.json"
    ))
    .unwrap();
    for case in registered["cases"].as_array().unwrap() {
        let filename = case["file"].as_str().unwrap();
        let bytes = fixture(filename);
        assert_eq!(sha256_hex(&bytes), case["sha256"]);
        let expected = &case["expectedNote"];
        let channel = case["channelDeclarations"]
            .as_array()
            .unwrap()
            .iter()
            .position(|decl| decl["savedId"] == expected["savedChannelId"])
            .unwrap();
        let raw = reply(&bytes);
        assert_eq!(
            raw["patternNoteSequences"]["patterns"][0],
            json!({"patternId":1,"status":"extracted",
            "records":[{"ordinal":0,"channelIndex":channel,"savedKey":expected["savedKey"],
                "startTick":expected["position"],"durationTick":expected["storedLength"],
                "storedVelocityByte":expected["storedVelocityByte"]}]}),
            "{filename}"
        );
        let metadata = validate(raw, descriptor(), &bytes).unwrap();
        let PatternNoteSequences::Entries(patterns) = metadata.pattern_note_sequences() else {
            panic!("entries expected");
        };
        assert_eq!(patterns[0].id(), 1);
        let NoteSequence::Extracted(notes) = patterns[0].sequence() else {
            panic!("qualified notes expected");
        };
        assert_eq!(notes.len(), 1);
        assert_eq!(u64::from(notes[0].saved_key()), expected["savedKey"]);
        assert_eq!(u64::from(notes[0].start_tick()), expected["position"]);
        assert_eq!(
            u64::from(notes[0].duration_tick()),
            expected["storedLength"]
        );
        assert_eq!(
            u64::from(notes[0].stored_velocity_byte()),
            expected["storedVelocityByte"]
        );
        assert_eq!(notes[0].channel_index() as usize, channel);
        assert_eq!(notes[0].ordinal(), 0);
        assert_eq!(fixture(filename), bytes);
    }
}

#[test]
fn unknown_records_refuse_whole_pattern_without_losing_counts_or_valid_siblings() {
    // Constructed defenses, not expanded genuine-save compatibility.
    for (offset, value) in [
        (4, 8),
        (8, 0),
        (12, 61),
        (21, 90),
        (16, 119),
        (22, 127),
        (6, 2),
    ] {
        let mut unknown = RECORD;
        unknown[offset] = value;
        let mut mixed = RECORD.to_vec();
        mixed.extend(unknown);
        let bytes = stream(&[(1, mixed), (2, RECORD.to_vec())], &[0]);
        let raw = reply(&bytes);
        assert_eq!(
            raw["patternNoteCounts"]["value"][0]["noteCount"]["value"],
            2
        );
        assert_eq!(
            raw["patternNoteSequences"]["patterns"][0],
            json!({"patternId":1,"status":"unsupported","reason":"PATTERN_NOTE_RECORD_UNQUALIFIED"})
        );
        assert_eq!(
            raw["patternNoteSequences"]["patterns"][1]["records"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(validate(raw, descriptor(), &bytes).is_ok());
    }
}

#[test]
fn note_order_duplicates_and_explicit_declaration_mapping_are_preserved() {
    let mut on_second_decl = RECORD;
    on_second_decl[6] = 0;
    let bytes = stream(&[(2, [on_second_decl, on_second_decl].concat())], &[1, 0]);
    let raw = reply(&bytes);
    let notes = raw["patternNoteSequences"]["patterns"][0]["records"]
        .as_array()
        .unwrap();
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0]["ordinal"], 0);
    assert_eq!(notes[1]["ordinal"], 1);
    assert_eq!(notes[0]["channelIndex"], 1);
    assert!(validate(raw, descriptor(), &bytes).is_ok());
}

#[test]
fn ambiguous_sparse_and_unqualified_channel_counts_and_ppq_do_not_guess() {
    for channels in [vec![0, 0], vec![1], vec![0, 2], vec![0, 1, 2]] {
        let bytes = stream(&[(1, RECORD.to_vec())], &channels);
        let raw = reply(&bytes);
        assert_eq!(
            raw["patternNoteSequences"]["reason"],
            "PATTERN_NOTE_SEQUENCE_CHANNELS_UNVERIFIED"
        );
        assert!(validate(raw, descriptor(), &bytes).is_ok());
    }
    let mut bytes = stream(&[(1, RECORD.to_vec())], &[0]);
    bytes[12..14].copy_from_slice(&192u16.to_le_bytes());
    let raw = reply(&bytes);
    assert_eq!(
        raw["patternNoteSequences"]["reason"],
        "PATTERN_NOTE_SEQUENCE_PPQ_UNVERIFIED"
    );
    assert!(validate(raw, descriptor(), &bytes).is_ok());
}

#[test]
fn missing_malformed_duplicate_and_late_payloads_preserve_exact_count_states() {
    let bytes = fixture("FIX-FL2026-NAMED-EMPTY.flp");
    let raw = reply(&bytes);
    assert_eq!(
        raw["patternNoteSequences"]["patterns"][0],
        json!({"patternId":1,"status":"unavailable","reason":"PATTERN_NOTES_NOT_STORED"})
    );
    assert!(validate(raw, descriptor(), &bytes).is_ok());
    for payload in [vec![], vec![0], vec![0; 25]] {
        let bytes = stream(&[(1, payload)], &[0]);
        let raw = reply(&bytes);
        assert_eq!(
            raw["patternNoteSequences"]["patterns"][0]["reason"],
            "PATTERN_NOTE_LAYOUT_UNVERIFIED"
        );
        assert!(validate(raw, descriptor(), &bytes).is_ok());
    }
    let bytes = stream(&[(1, RECORD.to_vec()), (1, RECORD.to_vec())], &[0]);
    let raw = reply(&bytes);
    assert_eq!(
        raw["patternNoteSequences"]["patterns"][0]["reason"],
        "MULTIPLE_PATTERN_NOTE_PAYLOADS"
    );
    assert!(validate(raw, descriptor(), &bytes).is_ok());
    let mut bytes = stream(&[], &[0]);
    bytes.extend(event(65, &1u16.to_le_bytes()));
    bytes.extend(event(224, &RECORD));
    let size = (bytes.len() - 22) as u32;
    bytes[18..22].copy_from_slice(&size.to_le_bytes());
    let raw = reply(&bytes);
    assert_eq!(
        raw["patternNoteSequences"]["reason"],
        "PATTERN_NOTE_BINDING_UNVERIFIED"
    );
    assert!(validate(raw, descriptor(), &bytes).is_ok());
}

#[test]
fn detail_limits_are_complete_or_refused_and_do_not_reduce_count_limits() {
    for count in [512, 513] {
        let bytes = stream(&[(1, RECORD.repeat(count))], &[0]);
        let raw = reply(&bytes);
        assert_eq!(
            raw["patternNoteCounts"]["value"][0]["noteCount"]["value"],
            count
        );
        if count == 512 {
            assert_eq!(
                raw["patternNoteSequences"]["patterns"][0]["records"]
                    .as_array()
                    .unwrap()
                    .len(),
                count
            );
        } else {
            assert_eq!(
                raw["patternNoteSequences"]["patterns"][0]["reason"],
                "PATTERN_NOTE_SEQUENCE_LIMIT_EXCEEDED"
            );
        }
        assert!(validate(raw, descriptor(), &bytes).is_ok());
    }
    let bytes = stream(&[(1, RECORD.repeat(512)), (2, RECORD.repeat(512))], &[0]);
    assert!(validate(reply(&bytes), descriptor(), &bytes).is_ok());
    let bytes = stream(
        &[
            (1, RECORD.repeat(512)),
            (2, RECORD.repeat(512)),
            (3, vec![0]),
        ],
        &[0],
    );
    let raw = reply(&bytes);
    assert_eq!(
        raw["patternNoteSequences"]["reason"],
        "PATTERN_NOTE_SEQUENCE_LIMIT_EXCEEDED"
    );
    assert!(validate(raw, descriptor(), &bytes).is_ok());
    let bytes = stream(
        &(1..=1024)
            .map(|id| (id, RECORD.to_vec()))
            .collect::<Vec<_>>(),
        &[0],
    );
    let raw = reply(&bytes);
    assert_eq!(
        raw["patternNoteSequences"]["reason"],
        "PATTERN_NOTE_SEQUENCE_LIMIT_EXCEEDED"
    );
    assert!(validate(raw, descriptor(), &bytes).is_ok());
}

#[test]
fn advertisement_requires_the_complete_contract_and_bounded_caps() {
    for key in [
        "maxDetailedNotesPerPattern",
        "maxDetailedNotesTotal",
        "maxNoteSequenceBytes",
    ] {
        let mut bad = descriptor();
        bad.as_object_mut().unwrap().remove(key);
        assert!(validate_descriptor(&bad).is_err());
        for value in [
            json!(0),
            json!(-1),
            json!(1.5),
            json!("512"),
            json!(u64::MAX),
            Value::Null,
        ] {
            let mut bad = descriptor();
            bad[key] = value;
            assert!(validate_descriptor(&bad).is_err());
        }
    }
    for field in [
        "patternNoteCounts",
        "patternCount",
        "patternNames",
        "pluginReferences",
        "patternNoteSequences",
    ] {
        let mut bad = descriptor();
        bad["fields"]
            .as_array_mut()
            .unwrap()
            .retain(|name| name != field);
        assert!(validate_descriptor(&bad).is_err());
    }
    let mut bad = descriptor();
    bad["maxDetailedNotesTotal"] = json!(511);
    assert!(validate_descriptor(&bad).is_err());
}

#[test]
fn malformed_advertised_notes_never_fall_back_to_older_metadata() {
    let bytes = fixture("FIX-FL2026-NOTE-BASELINE.flp");
    let raw = reply(&bytes);
    for (pointer, value) in [
        ("/patternNoteSequences", Value::Null),
        ("/patternNoteSequences/coverage", json!("MIDI")),
        ("/patternNoteSequences/ppq", json!(96.0)),
        ("/patternNoteSequences/ppq", json!(192)),
        (
            "/patternNoteSequences/coordinateSystem",
            json!("song-seconds"),
        ),
        ("/patternNoteSequences/patterns", json!([])),
        ("/patternNoteSequences/patterns/0/patternId", json!(2)),
        ("/patternNoteSequences/patterns/0/records", json!([])),
        (
            "/patternNoteSequences/patterns/0/records/0/ordinal",
            json!(1),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/channelIndex",
            json!(1),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/savedKey",
            json!(61),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/startTick",
            json!(-1),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/startTick",
            json!(u64::MAX),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/durationTick",
            json!(0),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/storedVelocityByte",
            json!("89"),
        ),
        (
            "/patternNoteSequences/patterns/0/records/0/storedVelocityByte",
            json!(89.0),
        ),
        ("/patternNoteCounts/value/0/noteCount/value", json!(2)),
    ] {
        let mut bad = raw.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(validate(bad, descriptor(), &bytes).is_err(), "{pointer}");
    }
    for pointer in [
        "/patternNoteSequences",
        "/patternNoteSequences/patterns/0",
        "/patternNoteSequences/patterns/0/records/0",
    ] {
        let mut bad = raw.clone();
        bad.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!("private"));
        assert!(validate(bad, descriptor(), &bytes).is_err());
    }
    let mut missing = raw.clone();
    missing
        .as_object_mut()
        .unwrap()
        .remove("patternNoteSequences");
    assert!(validate(missing, descriptor(), &bytes).is_err());
    let mut cap = descriptor();
    cap["maxNoteSequenceBytes"] = json!(1);
    assert!(validate(raw, cap, &bytes).is_err());
}

#[test]
fn unadvertised_details_are_discarded_and_other_builds_remain_explicit() {
    let bytes = fixture("FIX-FL2026-NOTE-BASELINE.flp");
    let mut old = descriptor();
    old["fields"]
        .as_array_mut()
        .unwrap()
        .retain(|field| field != "patternNoteSequences");
    for key in [
        "maxDetailedNotesPerPattern",
        "maxDetailedNotesTotal",
        "maxNoteSequenceBytes",
    ] {
        old.as_object_mut().unwrap().remove(key);
    }
    let mut raw = reply(&bytes);
    raw["patternNoteSequences"] = json!({"private":"untrusted"});
    assert!(matches!(
        validate(raw, old, &bytes).unwrap().pattern_note_sequences(),
        PatternNoteSequences::NotAdvertised
    ));
    for filename in ["FIX-FL2024-A.flp", "FIX-FL2025-MIN.flp"] {
        let bytes = fixture(filename);
        assert!(matches!(
            validate(reply(&bytes), descriptor(), &bytes)
                .unwrap()
                .pattern_note_sequences(),
            PatternNoteSequences::Unsupported(NoteSequenceReason::UnverifiedBuild)
        ));
    }
    let bytes = fixture("FIX-FL2026-MIN.flp");
    assert!(matches!(
        validate(reply(&bytes), descriptor(), &bytes)
            .unwrap()
            .pattern_note_sequences(),
        PatternNoteSequences::Unavailable(NoteSequenceReason::Counts(
            PatternNoteReason::DataNotStored
        ))
    ));
}
