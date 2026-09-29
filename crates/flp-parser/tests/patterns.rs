use fruitboard_flp_parser::{MAX_PATTERNS, parse_bytes};

// In-memory protocol cases, not FL Studio GUI fixtures or compatibility proof.
fn stream(version: &str, pattern_ids: &[u16]) -> Vec<u8> {
    let mut events = vec![199, (version.len() + 1) as u8];
    events.extend_from_slice(version.as_bytes());
    events.push(0);
    events.extend_from_slice(&[64, 0, 0]); // One channel.
    for id in pattern_ids {
        events.push(65);
        events.extend_from_slice(&id.to_le_bytes());
    }
    let mut bytes = b"FLhd".to_vec();
    bytes.extend_from_slice(&6_u32.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, 1, 0, 96, 0]);
    bytes.extend_from_slice(b"FLdt");
    bytes.extend_from_slice(&(events.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&events);
    bytes
}

#[test]
fn repeated_and_sparse_pattern_ids_count_once_each() {
    let parsed = parse_bytes(&stream("26.1.0.5530", &[1, 4, 9, 1, 4, 9]));
    assert_eq!(parsed["patternCount"]["status"], "extracted");
    assert_eq!(parsed["patternCount"]["value"], 3);
    assert_eq!(parsed["channelCount"]["value"], 1);
}

#[test]
fn missing_markers_and_unverified_builds_do_not_invent_counts() {
    let missing = parse_bytes(&stream("26.1.0.5530", &[]));
    assert_eq!(missing["patternCount"]["status"], "unavailable");
    assert_eq!(missing["patternCount"]["reason"], "PATTERN_DATA_NOT_STORED");
    for version in ["24.1.0.4225", "25.1.3.4922"] {
        let unverified = parse_bytes(&stream(version, &[1, 2, 3]));
        assert_eq!(unverified["patternCount"]["status"], "unsupported");
        assert_eq!(
            unverified["patternCount"]["reason"],
            "PATTERN_COUNT_UNVERIFIED_BUILD"
        );
        assert_eq!(unverified["channelCount"]["value"], 1);
    }
}

#[test]
fn zero_id_and_unique_pattern_limit_fail_with_typed_results() {
    let invalid = parse_bytes(&stream("26.1.0.5530", &[0]));
    assert_eq!(invalid["code"], "INVALID_PATTERN_ID");
    let mut ids: Vec<u16> = (1..=MAX_PATTERNS as u16).collect();
    ids.push(1); // Repetition at the bound is still permitted.
    assert_eq!(
        parse_bytes(&stream("26.1.0.5530", &ids))["patternCount"]["value"],
        MAX_PATTERNS
    );
    ids.push(MAX_PATTERNS as u16 + 1);
    let limited = parse_bytes(&stream("26.1.0.5530", &ids));
    assert_eq!(limited["code"], "PATTERN_COUNT_LIMIT");
    assert_eq!(limited["patternCount"]["status"], "failed");
}

#[test]
fn approved_three_pattern_corpus_counts_distinct_ids_despite_repetition() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/parser-corpus")
        .join("FIX-FL2026-PATTERNS.flp");
    let before = std::fs::read(&path).expect("approved pattern fixture");
    assert_eq!(before.len(), 47449);
    let parsed = parse_bytes(&before);
    assert_eq!(parsed["outcome"], "complete");
    assert_eq!(parsed["savedVersion"]["value"], "26.1.0.5530");
    assert_eq!(parsed["baseTempoBpm"]["value"], 130.0);
    assert_eq!(parsed["channelCount"]["value"], 1);
    assert_eq!(parsed["patternCount"]["status"], "extracted");
    assert_eq!(parsed["patternCount"]["value"], 3);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(parsed["patternNames"]["status"], "extracted");
    for (index, suffix) in ["A", "B", "C"].iter().enumerate() {
        let item = &parsed["patternNames"]["value"][index];
        assert_eq!(item["patternId"], index + 1);
        assert_eq!(item["name"]["status"], "extracted");
        assert_eq!(item["name"]["value"], format!("Fixture Pattern {suffix}"));
    }
}

fn append(bytes: &mut Vec<u8>, events: &[u8]) {
    bytes.extend_from_slice(events);
    let length = (bytes.len() - 22) as u32;
    bytes[18..22].copy_from_slice(&length.to_le_bytes());
}

fn name_event(name: &str) -> Vec<u8> {
    let data: Vec<u8> = name
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect();
    assert!(data.len() < 128);
    let mut event = vec![193, data.len() as u8];
    event.extend(data);
    event
}

#[test]
fn names_follow_sparse_ids_and_identical_repetitions_are_idempotent() {
    let mut bytes = stream("26.1.0.5530", &[9]);
    append(&mut bytes, &name_event("Música 🎵"));
    append(&mut bytes, &[65, 4, 0]);
    append(&mut bytes, &name_event(""));
    append(&mut bytes, &[65, 9, 0]);
    append(&mut bytes, &name_event("Música 🎵"));
    let parsed = parse_bytes(&bytes);
    assert_eq!(
        parsed["patternNames"]["value"],
        serde_json::json!([
            {"patternId":4,"name":{"status":"extracted","value":""}},
            {"patternId":9,"name":{"status":"extracted","value":"Música 🎵"}}
        ])
    );
    append(&mut bytes, &name_event("different"));
    let failed = parse_bytes(&bytes);
    assert_eq!(failed["code"], "CONFLICTING_PATTERN_NAMES");
    assert_eq!(failed["patternNames"]["status"], "failed");
}

#[test]
fn absent_names_preserve_ids_and_supported_names_without_inventing_defaults() {
    let mut bytes = stream("26.1.0.5530", &[4, 9]);
    append(&mut bytes, &name_event("Nine"));
    let parsed = parse_bytes(&bytes);
    assert_eq!(parsed["patternNames"]["status"], "unavailable");
    assert_eq!(parsed["patternNames"]["items"][0]["patternId"], 4);
    assert_eq!(
        parsed["patternNames"]["items"][0]["name"]["reason"],
        "PATTERN_NAME_NOT_STORED"
    );
    assert_eq!(parsed["patternNames"]["items"][1]["name"]["value"], "Nine");
    assert_eq!(
        parse_bytes(&stream("26.1.0.5530", &[]))["patternNames"]["reason"],
        "PATTERN_DATA_NOT_STORED"
    );
    for version in ["24.1.0.4225", "25.1.3.4922", "99.0.0.0"] {
        assert_eq!(
            parse_bytes(&stream(version, &[1]))["patternNames"]["status"],
            "unsupported"
        );
    }
}

#[test]
fn malformed_names_and_names_outside_pattern_context_fail() {
    for data in [
        vec![1],
        vec![65, 0],
        vec![0, 216, 0, 0],
        vec![0, 0, 0, 0],
        vec![0; 8194],
    ] {
        let mut bytes = stream("26.1.0.5530", &[1]);
        let mut event = vec![193];
        let mut length = data.len();
        while length >= 128 {
            event.push((length as u8 & 127) | 128);
            length >>= 7;
        }
        event.push(length as u8);
        event.extend(data);
        append(&mut bytes, &event);
        assert_eq!(parse_bytes(&bytes)["code"], "INVALID_PATTERN_NAME");
    }
    for boundary in [vec![98, 0, 0], vec![64, 0, 0]] {
        let mut bytes = stream("26.1.0.5530", &[1]);
        append(&mut bytes, &boundary);
        append(&mut bytes, &name_event("Orphan"));
        assert_eq!(parse_bytes(&bytes)["code"], "PATTERN_NAME_WITHOUT_ID");
    }
    let mut bytes = stream("26.1.0.5530", &[]);
    append(&mut bytes, &name_event("Orphan"));
    assert_eq!(parse_bytes(&bytes)["code"], "PATTERN_NAME_WITHOUT_ID");
}
