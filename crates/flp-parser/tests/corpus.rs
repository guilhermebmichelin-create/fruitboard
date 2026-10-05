use fruitboard_flp_parser::{ExpectedFingerprint, modified_at_ms, parse_bytes, parse_file};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/parser-corpus")
        .join(name)
}

#[test]
fn approved_corpus_keeps_registered_values_and_typed_outcomes() {
    for (name, version, tempo, channel, pattern_status, pattern_value) in [
        (
            "FIX-BASE-MIN.flp",
            "24.1.0.4225",
            120.0,
            "Sampler",
            "unsupported",
            0,
        ),
        (
            "FIX-FL2024-A.flp",
            "24.1.0.4225",
            140.0,
            "Sampler",
            "unsupported",
            0,
        ),
        (
            "FIX-FL2024-B.flp",
            "24.1.0.4225",
            141.0,
            "Sampler",
            "unsupported",
            0,
        ),
        (
            "FIX-FL2025-MIN.flp",
            "25.1.3.4922",
            130.0,
            "Sampler",
            "unsupported",
            0,
        ),
        (
            "FIX-FL2026-MIN.flp",
            "26.1.0.5530",
            130.0,
            "Sampler",
            "unavailable",
            0,
        ),
        (
            "FIX-FL2026-SAMPLE.flp",
            "26.1.0.5530",
            137.0,
            "Fixture Sample A",
            "unavailable",
            0,
        ),
        (
            "FIX-FL2026-PATTERNS.flp",
            "26.1.0.5530",
            130.0,
            "Sampler",
            "extracted",
            3,
        ),
    ] {
        let bytes = fs::read(fixture(name)).expect("approved fixture");
        let parsed = parse_bytes(&bytes);
        assert_eq!(parsed["outcome"], "complete", "{name}");
        assert_eq!(parsed["savedVersion"]["value"], version, "{name}");
        assert_eq!(parsed["baseTempoBpm"]["value"], tempo, "{name}");
        assert_eq!(parsed["channelCount"]["status"], "extracted", "{name}");
        assert_eq!(parsed["channelCount"]["value"], 1, "{name}");
        assert_eq!(parsed["patternCount"]["status"], pattern_status, "{name}");
        if pattern_status == "extracted" {
            assert_eq!(parsed["patternCount"]["value"], pattern_value, "{name}");
        }
        assert_eq!(parsed["channelNames"]["value"][0], channel, "{name}");
        if matches!(
            name,
            "FIX-FL2025-MIN.flp" | "FIX-FL2026-MIN.flp" | "FIX-FL2026-PATTERNS.flp"
        ) {
            assert_eq!(parsed["channelNames"]["status"], "inferred", "{name}");
            assert_eq!(
                parsed["channelNames"]["method"], "sampler-label-for-known-build",
                "{name}"
            );
            assert_eq!(parsed["channelNames"]["confidence"], "high", "{name}");
        } else {
            assert_eq!(parsed["channelNames"]["status"], "extracted", "{name}");
        }
        if name == "FIX-FL2026-SAMPLE.flp" {
            assert_eq!(parsed["sampleReferences"]["status"], "extracted");
            assert_eq!(
                parsed["sampleReferences"]["value"][0],
                r"C:\Users\Public\Documents\FruitboardFixtures\F12\fixture-silence.wav"
            );
        } else {
            assert_eq!(parsed["sampleReferences"]["status"], "unavailable");
        }
        if version == "26.1.0.5530" {
            assert_eq!(
                parsed["channelGeneratorNames"]["status"], "inferred",
                "{name}"
            );
            assert_eq!(
                parsed["channelGeneratorNames"]["value"],
                serde_json::json!(["Sampler"]),
                "{name}"
            );
            assert_eq!(
                parsed["channelGeneratorNames"]["confidence"], "high",
                "{name}"
            );
        } else {
            assert_eq!(
                parsed["channelGeneratorNames"]["status"], "unsupported",
                "{name}"
            );
        }
    }

    for (name, outcome, code) in [
        ("FIX-RB-TRUNC.flp", "failed", "TRUNCATED_DATA_CHUNK"),
        ("FIX-RB-MALFORM.flp", "failed", "EVENT_LENGTH_OUT_OF_BOUNDS"),
        ("FIX-RB-LIMIT.flp", "failed", "CHANNEL_COUNT_LIMIT"),
    ] {
        let parsed = parse_bytes(&fs::read(fixture(name)).expect("approved fixture"));
        assert_eq!(parsed["outcome"], outcome, "{name}");
        assert_eq!(parsed["code"], code, "{name}");
        assert_eq!(parsed["channelCount"]["status"], "failed", "{name}");
        assert_eq!(
            parsed["channelGeneratorNames"]["status"], "failed",
            "{name}"
        );
    }
    let unknown = parse_bytes(&fs::read(fixture("FIX-RB-UNKNOWN.flp")).unwrap());
    assert_eq!(unknown["outcome"], "partial");
    assert_eq!(unknown["diagnostics"][0]["code"], "UNSUPPORTED_EVENT");
    assert_eq!(unknown["diagnostics"][0]["eventId"], 255);
    assert_eq!(unknown["channelCount"]["value"], 1);
}

#[test]
fn approved_three_sampler_gui_save_preserves_duplicate_labels_read_only() {
    // Registered GUI values precede parser inspection; this exact file/hash
    // received owner publication approval on 2026-10-05 (manifest F15).
    let path = fixture("FIX-FL2026-MULTISAMPLER.flp");
    let before = fs::read(&path).unwrap();
    let hash = "d9f09c8f61293ce1ea95ee925af0bdb67b4978668cc273966e553cc6f2e5a71e";
    assert_eq!(fruitboard_flp_parser::sha256_hex(&before), hash);
    let metadata = fs::metadata(&path).unwrap();
    let parsed = parse_file(
        &path,
        ExpectedFingerprint {
            size: metadata.len(),
            modified_at_ms: modified_at_ms(&metadata).unwrap(),
        },
    );
    assert_eq!(parsed["outcome"], "complete");
    assert_eq!(parsed["savedVersion"]["value"], "26.1.0.5530");
    assert_eq!(parsed["baseTempoBpm"]["value"], 130.0);
    assert_eq!(parsed["channelCount"]["value"], 3);
    let labels = serde_json::json!(["Sampler", "Sampler", "Sampler"]);
    assert_eq!(parsed["channelNames"]["value"], labels);
    assert_eq!(parsed["channelNames"]["status"], "inferred");
    assert_eq!(
        parsed["channelNames"]["method"],
        "sampler-label-for-known-build"
    );
    assert_eq!(parsed["channelNames"]["confidence"], "medium");
    for (index, confidence) in ["high", "medium", "medium"].iter().enumerate() {
        assert_eq!(parsed["channelNames"]["items"][index]["value"], "Sampler");
        assert_eq!(
            parsed["channelNames"]["items"][index]["confidence"],
            *confidence
        );
    }
    assert_eq!(parsed["channelGeneratorNames"]["value"], labels);
    assert_eq!(parsed["sampleReferences"]["status"], "unavailable");
    assert_eq!(
        parsed["sampleReferences"]["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    for sample in parsed["sampleReferences"]["items"].as_array().unwrap() {
        assert_eq!(sample["status"], "unavailable");
        assert_eq!(sample["reason"], "SAMPLE_REFERENCE_NOT_STORED");
    }
    assert_eq!(
        parsed["playlistPatternClips"]["value"],
        serde_json::json!([])
    );
    assert_eq!(parsed["inputFingerprint"]["hash"]["value"], hash);
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn read_only_file_parse_checks_expected_fingerprint_and_hides_path() {
    let path = fixture("FIX-FL2026-SAMPLE.flp");
    let bytes_before = fs::read(&path).unwrap();
    let metadata = fs::metadata(&path).unwrap();
    let expected = ExpectedFingerprint {
        size: metadata.len(),
        modified_at_ms: modified_at_ms(&metadata).unwrap(),
    };
    let parsed = parse_file(&path, expected);
    assert_eq!(parsed["outcome"], "complete");
    assert_eq!(parsed["inputFingerprint"]["size"], expected.size);
    // The digest identifies the parsed bytes and an independent bounded
    // read from the same file verifies them after parsing.
    assert_eq!(parsed["inputFingerprint"]["hash"]["algorithm"], "sha256");
    assert_eq!(
        parsed["inputFingerprint"]["hash"]["value"],
        fruitboard_flp_parser::sha256_hex(&bytes_before)
    );
    assert!(
        !parsed
            .to_string()
            .contains(&path.to_string_lossy().to_string())
    );
    assert_eq!(fs::read(&path).unwrap(), bytes_before);

    let changed = parse_file(
        &path,
        ExpectedFingerprint {
            size: expected.size + 1,
            ..expected
        },
    );
    assert_eq!(changed["code"], "INPUT_CHANGED");
    assert_eq!(fs::read(&path).unwrap(), bytes_before);
}

#[test]
fn oversized_buffer_fails_before_event_walk() {
    let bytes = vec![0; fruitboard_flp_parser::MAX_FILE_BYTES as usize + 1];
    let parsed: Value = parse_bytes(&bytes);
    assert_eq!(parsed["code"], "FILE_SIZE_LIMIT");
}

#[test]
fn header_count_must_match_channel_events_before_reporting_a_count() {
    let path = fixture("FIX-BASE-MIN.flp");
    let original = fs::read(&path).unwrap();
    let mut changed = original.clone();
    changed[10..12].copy_from_slice(&2_u16.to_le_bytes());
    let parsed = parse_bytes(&changed);
    assert_eq!(parsed["code"], "CHANNEL_COUNT_MISMATCH");
    assert_eq!(parsed["channelCount"]["status"], "failed");
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn unverified_saved_build_is_explicitly_unsupported() {
    let version = b"27.0.0.1\0";
    let mut bytes = b"FLhd".to_vec();
    bytes.extend_from_slice(&6_u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 6]);
    bytes.extend_from_slice(b"FLdt");
    bytes.extend_from_slice(&((version.len() + 2) as u32).to_le_bytes());
    bytes.push(199);
    bytes.push(version.len() as u8);
    bytes.extend_from_slice(version);
    let parsed = parse_bytes(&bytes);
    assert_eq!(parsed["outcome"], "unsupported");
    assert_eq!(parsed["savedVersion"]["value"], "27.0.0.1");
    assert_eq!(parsed["channelNames"]["status"], "unsupported");
    assert_eq!(parsed["channelCount"]["status"], "unsupported");
    assert_eq!(parsed["patternCount"]["status"], "unsupported");
    assert_eq!(parsed["channelGeneratorNames"]["status"], "unsupported");
}
