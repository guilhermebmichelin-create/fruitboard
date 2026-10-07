use fruitboard_flp_parser::{
    MAX_NOTE_RECORDS_PER_PATTERN, MAX_NOTE_RECORDS_TOTAL, parse_bytes, sha256_hex,
};
use serde_json::{Value, json};

// Constructed defenses exercise bounds/binding, not GUI compatibility.
fn event(id: u8, data: &[u8]) -> Vec<u8> {
    let mut out = vec![id];
    if id >= 192 {
        let mut size = data.len();
        loop {
            let next = size >> 7;
            out.push((size & 127) as u8 | if next == 0 { 0 } else { 128 });
            size = next;
            if size == 0 {
                break;
            }
        }
    }
    out.extend_from_slice(data);
    out
}
fn marker(id: u16) -> Vec<u8> {
    event(65, &id.to_le_bytes())
}
fn stream(events: &[Vec<u8>]) -> Vec<u8> {
    let mut contents = event(199, b"26.1.0.5530\0");
    for event in events {
        contents.extend_from_slice(event);
    }
    let mut bytes = b"FLhd\x06\0\0\0\0\0\0\0\x60\0FLdt".to_vec();
    let channels = events
        .iter()
        .filter(|event| event.first() == Some(&64))
        .count() as u16;
    bytes[10..12].copy_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&(contents.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&contents);
    bytes
}
fn summary(events: &[Vec<u8>]) -> Value {
    parse_bytes(&stream(events))["patternNoteCounts"].clone()
}
fn notes(n: usize) -> Vec<u8> {
    event(224, &vec![0; n * 24])
}
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus")
            .join(name),
    )
    .unwrap()
}

#[test]
fn approved_f16_matches_independent_record_counts_without_playlist_multiplication() {
    let registered: Value = serde_json::from_str(include_str!(
        "../../../fixtures/parser-corpus/pattern-note-expectations.json"
    ))
    .unwrap();
    let bytes = fixture(registered["F16"]["filename"].as_str().unwrap());
    let parsed = parse_bytes(&bytes);
    assert_eq!(sha256_hex(&bytes), registered["F16"]["sha256"]);
    let expected = registered["F16"]["patterns"].as_array().unwrap().iter().map(|p|
        json!({"patternId":p["patternId"],"noteCount":{"status":"extracted","value":p["savedRecordCount"]}})).collect::<Vec<_>>();
    assert_eq!(
        parsed["patternNoteCounts"],
        json!({"status":"extracted","coverage":"stored-pattern-note-records","value":expected})
    );
    assert_eq!(
        parsed["playlistPatternClips"]["value"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        fixture(registered["F16"]["filename"].as_str().unwrap()),
        bytes
    );
}
#[test]
fn approved_f17_missing_data_and_older_builds_never_invent_zero() {
    let bytes = fixture("FIX-FL2026-NAMED-EMPTY.flp");
    let parsed = parse_bytes(&bytes);
    assert_eq!(
        parsed["patternNoteCounts"],
        json!({"status":"unavailable","reason":"PATTERN_NOTE_COUNTS_INCOMPLETE","coverage":"stored-pattern-note-records","items":[{"patternId":1,"noteCount":{"status":"unavailable","reason":"PATTERN_NOTES_NOT_STORED"}}]})
    );
    assert_eq!(
        sha256_hex(&bytes),
        "32cee5af3969d61c54f6bb6703fc1f3406be509d33a81e57b571d1c83cf7f78b"
    );
    assert_eq!(fixture("FIX-FL2026-NAMED-EMPTY.flp"), bytes);
    assert_eq!(summary(&[])["reason"], "PATTERN_DATA_NOT_STORED");
    for name in ["FIX-FL2024-A.flp", "FIX-FL2025-MIN.flp"] {
        assert_eq!(
            parse_bytes(&fixture(name))["patternNoteCounts"]["reason"],
            "PATTERN_NOTES_UNVERIFIED_BUILD"
        );
    }
}
#[test]
fn sparse_out_of_order_ids_sort_and_repeated_properties_do_not_multiply() {
    let result = summary(&[
        marker(65535),
        notes(2),
        marker(4),
        notes(1),
        event(226, &[]),
        marker(4),
        marker(65535),
    ]);
    assert_eq!(result["status"], "extracted");
    assert_eq!(
        result["value"],
        json!([{ "patternId":4,"noteCount":{"status":"extracted","value":1}}, {"patternId":65535,"noteCount":{"status":"extracted","value":2}}])
    );
}
#[test]
fn zero_and_misaligned_payloads_are_unqualified_but_do_not_block_other_patterns() {
    for size in [0, 1, 23, 25] {
        let result = summary(&[marker(1), event(224, &vec![0; size]), marker(2), notes(1)]);
        assert_eq!(result["status"], "unsupported");
        assert_eq!(
            result["items"][0]["noteCount"]["reason"],
            "PATTERN_NOTE_LAYOUT_UNVERIFIED"
        );
        assert_eq!(result["items"][1]["noteCount"]["value"], 1);
    }
}
#[test]
fn duplicate_payloads_are_unsupported_including_identical_and_adjacent_copies() {
    for middle in [Vec::new(), marker(1)] {
        let result = summary(&[marker(1), notes(1), middle, notes(1)]);
        assert_eq!(
            result["items"][0]["noteCount"]["reason"],
            "MULTIPLE_PATTERN_NOTE_PAYLOADS"
        );
    }
}
#[test]
fn boundaries_unknown_headers_and_later_markers_cannot_restore_note_authority() {
    for boundary in [
        event(64, &[0, 0]),
        event(98, &[0, 0]),
        event(99, &[0, 0]),
        event(100, &[0, 0]),
        event(233, &[]),
        event(226, &[]),
        event(42, &[0]),
    ] {
        assert_eq!(
            summary(&[marker(1), boundary.clone(), marker(1), notes(1)])["reason"],
            "PATTERN_NOTE_BINDING_UNVERIFIED"
        );
        assert_eq!(
            summary(&[boundary, marker(1), notes(1)])["reason"],
            "PATTERN_NOTE_BINDING_UNVERIFIED"
        );
    }
    assert_eq!(
        summary(&[notes(1)])["reason"],
        "PATTERN_NOTE_BINDING_UNVERIFIED"
    );
    assert_eq!(
        summary(&[marker(1), marker(2), notes(1)])["reason"],
        "PATTERN_NOTE_BINDING_UNVERIFIED"
    );
    assert_eq!(
        summary(&[marker(1), event(17, &[4]), notes(1)])["reason"],
        "PATTERN_NOTE_BINDING_UNVERIFIED"
    );
}
#[test]
fn record_bounds_charge_every_candidate_without_truncation() {
    assert_eq!(
        summary(&[marker(1), notes(MAX_NOTE_RECORDS_PER_PATTERN)])["value"][0]["noteCount"]["value"],
        MAX_NOTE_RECORDS_PER_PATTERN
    );
    assert_eq!(
        summary(&[marker(1), notes(MAX_NOTE_RECORDS_PER_PATTERN + 1)])["reason"],
        "PATTERN_NOTE_LIMIT_EXCEEDED"
    );
    let mut events = Vec::new();
    for id in 1..=4 {
        events.extend([marker(id), notes(MAX_NOTE_RECORDS_PER_PATTERN)]);
    }
    let result = summary(&events);
    assert_eq!(result["status"], "extracted");
    assert_eq!(MAX_NOTE_RECORDS_TOTAL, 4 * MAX_NOTE_RECORDS_PER_PATTERN);
    events.extend([marker(5), event(224, &[0])]);
    assert_eq!(summary(&events)["reason"], "PATTERN_NOTE_LIMIT_EXCEEDED");
    for bound in [true, false] {
        let mut events = vec![];
        if bound {
            events.push(marker(1));
        }
        for _ in 0..5 {
            events.push(notes(MAX_NOTE_RECORDS_PER_PATTERN));
        }
        assert_eq!(summary(&events)["reason"], "PATTERN_NOTE_LIMIT_EXCEEDED");
    }
}
#[test]
fn truncated_payload_is_a_whole_parse_failure() {
    let mut bytes = stream(&[marker(1), notes(1)]);
    bytes.pop();
    assert_eq!(parse_bytes(&bytes)["outcome"], "failed");
}

#[test]
fn genuine_save_registration_header_is_skipped_without_returning_its_bytes() {
    // Constructed innocuous stand-in; never use private registration bytes.
    let result = parse_bytes(&stream(&[event(200, &[0x55; 16]), marker(1), notes(1)]));
    assert_eq!(
        result["patternNoteCounts"]["value"][0]["noteCount"]["value"],
        1
    );
    assert!(result.get("registration").is_none());
    assert_eq!(
        summary(&[marker(1), event(200, &[0x55; 16]), notes(1)])["reason"],
        "PATTERN_NOTE_BINDING_UNVERIFIED"
    );
}
