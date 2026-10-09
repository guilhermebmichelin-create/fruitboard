use fruitboard_flp_parser::{MAX_MIXER_INSERT_CANDIDATES, parse_bytes, sha256_hex};
use serde_json::{Value, json};

const COVERAGE: &str = "explicit-saved-mixer-insert-records";
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus")
            .join(name),
    )
    .unwrap()
}
fn expectations() -> Value {
    let text = include_bytes!("../../../fixtures/parser-corpus/mixer-insert-expectations.json");
    assert_eq!(
        sha256_hex(text),
        "cb89b238d9c7b836a6a5ddebaf044ae3f95559b0943364b4faa098fac1cb9d8e"
    );
    serde_json::from_slice(text).unwrap()
}

#[test]
fn approved_cases_match_independent_expectations_and_preserve_all_bytes() {
    for case in expectations()["cases"].as_array().unwrap() {
        let name = case["file"].as_str().unwrap();
        let bytes = fixture(name);
        assert_eq!(bytes.len() as u64, case["bytes"].as_u64().unwrap());
        assert_eq!(sha256_hex(&bytes), case["sha256"].as_str().unwrap());
        let parsed = parse_bytes(&bytes);
        assert_eq!(parsed["outcome"], "complete", "{name}");
        assert_eq!(
            parsed["mixerInsertCount"],
            json!({"status":"extracted","value":16,"coverage":COVERAGE})
        );
        let registered = case["savedSection"]["ordinaryRecords"].as_array().unwrap();
        let mapped = registered.iter().map(|entry| json!({
            "savedInsertId":entry["sectionRecordOrdinal"],
            "name":if entry["name"]["status"] == "extracted" { entry["name"].clone() }
                else { json!({"status":"unavailable","reason":"MIXER_INSERT_NAME_NOT_STORED"}) }
        })).collect::<Vec<_>>();
        assert_eq!(
            parsed["mixerInsertNames"],
            json!({"status":"unavailable","reason":"MIXER_INSERT_NAMES_INCOMPLETE","coverage":COVERAGE,"items":mapped})
        );
        assert_eq!(fixture(name), bytes);
    }
}

// Test-only independent framing for mutations of approved bytes. Mutations
// exercise refusal/bounds; they do not qualify new real-world layouts.
type Event = (u8, Vec<u8>);
fn events() -> Vec<Event> {
    let bytes = fixture("FIX-FL2026-MIXER-DEFAULT.flp");
    let mut cursor = 22;
    let mut events = Vec::new();
    while cursor < bytes.len() {
        let id = bytes[cursor];
        cursor += 1;
        let size = match id {
            0..=63 => 1,
            64..=127 => 2,
            172 => 3, // This helper reads only the pinned genuine 26.1 fixture.
            128..=191 => 4,
            _ => {
                let mut length = 0;
                let mut shift = 0;
                loop {
                    assert!(shift < 35);
                    let byte = bytes[cursor];
                    cursor += 1;
                    length |= usize::from(byte & 127) << shift;
                    if byte & 128 == 0 {
                        break;
                    }
                    shift += 7;
                }
                length
            }
        };
        let end = cursor.checked_add(size).unwrap();
        assert!(end <= bytes.len());
        events.push((id, bytes[cursor..end].to_vec()));
        cursor = end;
    }
    assert_eq!(cursor, bytes.len());
    events
}
fn stream(events: &[Event]) -> Vec<u8> {
    let mut body = Vec::new();
    for (id, data) in events {
        body.push(*id);
        if *id >= 192 {
            let mut size = data.len();
            loop {
                let next = size >> 7;
                body.push((size & 127) as u8 | if next == 0 { 0 } else { 128 });
                size = next;
                if size == 0 {
                    break;
                }
            }
        }
        body.extend_from_slice(data);
    }
    let mut bytes = b"FLhd\x06\0\0\0\0\0\0\0\x60\0FLdt".to_vec();
    let channels = events.iter().filter(|(id, _)| *id == 64).count() as u16;
    bytes[10..12].copy_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&body);
    bytes
}
fn entry(events: &[Event]) -> usize {
    events.iter().position(|(id, _)| *id == 103).unwrap()
}
fn text(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}
fn assert_pair(events: &[Event], status: &str, reason: &str) {
    let parsed = parse_bytes(&stream(events));
    assert!(
        matches!(parsed["outcome"].as_str(), Some("complete" | "partial")),
        "{parsed}"
    );
    let expected = json!({"status":status,"reason":reason,"coverage":COVERAGE});
    assert_eq!(parsed["mixerInsertCount"], expected);
    assert_eq!(parsed["mixerInsertNames"], expected);
}

#[test]
fn absent_and_other_builds_never_invent_zero_or_defaults() {
    assert_pair(
        &[(199, b"26.1.0.5530\0".to_vec())],
        "unavailable",
        "MIXER_INSERT_DATA_NOT_STORED",
    );
    for name in ["FIX-FL2024-A.flp", "FIX-FL2025-MIN.flp"] {
        let raw = parse_bytes(&fixture(name));
        assert_eq!(
            raw["mixerInsertCount"]["reason"],
            "MIXER_INSERT_UNVERIFIED_BUILD"
        );
        assert_eq!(raw["mixerInsertCount"], raw["mixerInsertNames"]);
    }
    let raw = parse_bytes(&stream(&[(199, b"99.1.0.1\0".to_vec())]));
    assert_eq!(raw["outcome"], "unsupported");
    assert_eq!(
        raw["mixerInsertNames"]["reason"],
        "MIXER_INSERT_UNVERIFIED_BUILD"
    );
}

#[test]
fn guard_order_termination_role_and_sparse_or_other_totals_are_unqualified() {
    let original = events();
    let entry = entry(&original);
    for total in [0u16, 16, 17, 19, 65535] {
        let mut bad = original.clone();
        bad[entry].1 = total.to_le_bytes().to_vec();
        assert_pair(&bad, "unsupported", "MIXER_INSERT_BINDING_UNVERIFIED");
    }
    for index in [
        entry - 7,
        entry - 4,
        entry - 1,
        entry + 1,
        original.len() - 1,
    ] {
        let mut bad = original.clone();
        bad.remove(index);
        assert_pair(&bad, "unsupported", "MIXER_INSERT_BINDING_UNVERIFIED");
    }
    let mut bad = original.clone();
    let first_body = bad.iter().position(|(id, _)| *id == 236).unwrap();
    bad[first_body].1[4] = 0x4c; // Master cannot masquerade as ordinary.
    assert_pair(&bad, "unsupported", "MIXER_INSERT_BINDING_UNVERIFIED");
    let mut bad = original.clone();
    let ordinary = bad
        .iter()
        .enumerate()
        .filter(|(_, (id, _))| *id == 236)
        .nth(1)
        .unwrap()
        .0;
    bad[ordinary].1[4] = 0x0c;
    assert_pair(&bad, "unsupported", "MIXER_INSERT_BINDING_UNVERIFIED");
    let mut bad = original.clone();
    bad[first_body].1.pop();
    assert_pair(&bad, "unsupported", "MIXER_INSERT_LAYOUT_UNVERIFIED");
    let mut bad = original.clone();
    bad.insert(entry + 2, (255, vec![]));
    assert_pair(&bad, "unsupported", "MIXER_INSERT_BINDING_UNVERIFIED");
    let mut bad = original.clone();
    bad.insert(entry + 2, (203, text("Channel context")));
    assert_pair(&bad, "unsupported", "MIXER_INSERT_BINDING_UNVERIFIED");
    let mut bad = original.clone();
    bad.push((0, vec![0]));
    assert_pair(&bad, "unsupported", "MIXER_INSERT_BINDING_UNVERIFIED");
    assert_pair(
        &original[..entry],
        "unsupported",
        "MIXER_INSERT_BINDING_UNVERIFIED",
    );
}

#[test]
fn repeated_sections_and_name_assignments_are_ambiguous() {
    let original = events();
    let entry = entry(&original);
    let mut repeated = original.clone();
    repeated.extend_from_slice(&original[entry - 7..]);
    assert_pair(&repeated, "unsupported", "MIXER_INSERT_RECORDS_AMBIGUOUS");
    let mut duplicate = original.clone();
    duplicate.splice(
        entry + 2..entry + 2,
        [(204, text("One")), (204, text("One"))],
    );
    assert_pair(&duplicate, "unsupported", "MIXER_INSERT_RECORDS_AMBIGUOUS");
    // Another assignment after the body remains ambiguous, never last-wins.
    let mut duplicate = original.clone();
    duplicate.insert(entry + 3, (204, text("One")));
    assert_pair(&duplicate, "unsupported", "MIXER_INSERT_RECORDS_AMBIGUOUS");
}

#[test]
fn charge_candidates_before_special_filter_and_after_rejection() {
    let original = events();
    // Eighteen original candidates already include both excluded special tracks.
    for (additional, reason) in [
        (
            MAX_MIXER_INSERT_CANDIDATES - 18,
            "MIXER_INSERT_BINDING_UNVERIFIED",
        ),
        (
            MAX_MIXER_INSERT_CANDIDATES - 17,
            "MIXER_INSERT_LIMIT_EXCEEDED",
        ),
    ] {
        let mut bad = original.clone();
        bad.extend(std::iter::repeat_n((42, vec![0]), additional));
        assert_pair(&bad, "unsupported", reason);
    }
}

#[test]
fn malformed_empty_and_oversize_names_including_specials_fail_the_pair() {
    let original = events();
    let entry = entry(&original);
    for (payload, reason) in [
        (vec![], "MIXER_INSERT_LAYOUT_UNVERIFIED"),
        (vec![0, 0], "MIXER_INSERT_LAYOUT_UNVERIFIED"),
        (vec![1], "MIXER_INSERT_LAYOUT_UNVERIFIED"),
        (vec![0, 0, 65, 0, 0, 0], "MIXER_INSERT_LAYOUT_UNVERIFIED"),
        (vec![0, 216, 0, 0], "MIXER_INSERT_LAYOUT_UNVERIFIED"),
        (vec![65, 0], "MIXER_INSERT_LAYOUT_UNVERIFIED"),
        (text(&"a".repeat(4096)), "MIXER_INSERT_LIMIT_EXCEEDED"),
    ] {
        let mut bad = original.clone();
        bad.insert(entry + 2, (204, payload));
        assert_pair(&bad, "unsupported", reason);
    }
}

fn all_named(value: &str) -> Vec<Event> {
    let mut events = events();
    let starts = events
        .iter()
        .enumerate()
        .filter(|(_, (id, _))| *id == 42)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    for &i in starts[1..17].iter().rev() {
        events.insert(i + 1, (204, text(value)));
    }
    events
}
#[test]
fn maximum_unicode_and_escaped_selected_output_remain_bounded() {
    // Constructed text bounds, not additional compatibility qualification.
    let events = all_named(&"\u{0800}".repeat(4095));
    let raw = parse_bytes(&stream(&events));
    assert_eq!(raw["mixerInsertNames"]["status"], "extracted");
    assert_eq!(
        raw["mixerInsertNames"]["value"][0]["name"]["value"]
            .as_str()
            .unwrap()
            .len(),
        12285
    );
    assert_eq!(
        raw["mixerInsertNames"]["value"].as_array().unwrap().len(),
        16
    );
    assert_pair(
        &all_named(&"\u{0001}".repeat(4095)),
        "unsupported",
        "MIXER_INSERT_LIMIT_EXCEEDED",
    );
    let mut bad = events.clone();
    let body = bad.iter().position(|(id, _)| *id == 236).unwrap();
    bad[body].1 = vec![0; 2 * 1024 * 1024 + 1];
    let failed = parse_bytes(&stream(&bad));
    assert_eq!(failed["code"], "EVENT_LENGTH_OUT_OF_BOUNDS");
    for key in ["mixerInsertCount", "mixerInsertNames"] {
        assert_eq!(
            failed[key],
            json!({"status":"failed","reason":"EVENT_LENGTH_OUT_OF_BOUNDS"})
        );
    }
}
