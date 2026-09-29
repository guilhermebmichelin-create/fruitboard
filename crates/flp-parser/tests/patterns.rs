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
}
