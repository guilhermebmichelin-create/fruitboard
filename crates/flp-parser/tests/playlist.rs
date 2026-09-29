use fruitboard_flp_parser::{MAX_PLAYLIST_CLIPS, parse_bytes};
use serde_json::json;

// In-memory boundary cases; only the committed F13 file proves GUI behavior.
fn stream(version: &str, pattern_ids: &[u16]) -> Vec<u8> {
    let mut events = vec![199, (version.len() + 1) as u8];
    events.extend_from_slice(version.as_bytes());
    events.push(0);
    events.extend_from_slice(&[64, 0, 0]);
    for id in pattern_ids {
        events.push(65);
        events.extend_from_slice(&id.to_le_bytes());
    }
    let mut bytes = b"FLhd".to_vec();
    bytes.extend_from_slice(&6_u32.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, 1, 0, 96, 0]);
    bytes.extend_from_slice(b"FLdt");
    bytes.extend_from_slice(&(events.len() as u32).to_le_bytes());
    bytes.extend(events);
    bytes
}

fn append_payload(bytes: &mut Vec<u8>, payload: &[u8]) {
    bytes.push(233);
    let mut length = payload.len();
    while length >= 128 {
        bytes.push((length as u8 & 127) | 128);
        length >>= 7;
    }
    bytes.push(length as u8);
    bytes.extend_from_slice(payload);
    let data_length = (bytes.len() - 22) as u32;
    bytes[18..22].copy_from_slice(&data_length.to_le_bytes());
}

fn clip(start: u32, pattern_id: u16, length: u32, track_token: u32) -> [u8; 88] {
    let mut record = [0; 88];
    record[0..4].copy_from_slice(&start.to_le_bytes());
    record[4..8].copy_from_slice(&((u32::from(0x5000 + pattern_id) << 16) | 0x5000).to_le_bytes());
    record[8..12].copy_from_slice(&length.to_le_bytes());
    record[12..16].copy_from_slice(&track_token.to_le_bytes());
    record
}

#[test]
fn approved_f13_matches_four_gui_registered_placements() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/parser-corpus/FIX-FL2026-PATTERNS.flp");
    let before = std::fs::read(&path).expect("approved F13 fixture");
    assert_eq!(before.len(), 47449);
    let parsed = parse_bytes(&before);
    assert_eq!(parsed["outcome"], "complete");
    assert_eq!(parsed["patternCount"]["value"], 3);
    assert_eq!(parsed["playlistPatternClips"]["status"], "extracted");
    assert_eq!(
        parsed["playlistPatternClips"]["value"],
        json!([
            {"patternId":1,"startTick":0,"lengthTick":384,"trackToken":499},
            {"patternId":1,"startTick":384,"lengthTick":384,"trackToken":499},
            {"patternId":2,"startTick":768,"lengthTick":384,"trackToken":499},
            {"patternId":3,"startTick":1152,"lengthTick":384,"trackToken":499},
        ])
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn empty_saved_2026_arrangements_have_no_placements() {
    for file in ["FIX-FL2026-MIN.flp", "FIX-FL2026-SAMPLE.flp"] {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("fixtures/parser-corpus")
            .join(file);
        let before = std::fs::read(&path).unwrap();
        let parsed = parse_bytes(&before);
        assert_eq!(parsed["playlistPatternClips"]["status"], "extracted");
        assert_eq!(parsed["playlistPatternClips"]["value"], json!([]));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[test]
fn missing_and_unverified_playlist_data_is_labeled() {
    assert_eq!(
        parse_bytes(&stream("26.1.0.5530", &[1]))["playlistPatternClips"]["reason"],
        "PLAYLIST_DATA_NOT_STORED"
    );
    for version in ["24.1.0.4225", "25.1.3.4922", "99.0.0.0"] {
        let mut bytes = stream(version, &[1]);
        append_payload(&mut bytes, &clip(0, 1, 384, 499));
        assert_eq!(
            parse_bytes(&bytes)["playlistPatternClips"]["status"],
            "unsupported"
        );
    }
}

#[test]
fn unknown_layout_kind_reference_and_multiple_arrangements_are_not_guessed() {
    let mut malformed = stream("26.1.0.5530", &[1]);
    append_payload(&mut malformed, &[0; 87]);
    assert_eq!(
        parse_bytes(&malformed)["playlistPatternClips"]["reason"],
        "PLAYLIST_CLIP_LAYOUT_UNVERIFIED"
    );

    let mut unknown_kind = clip(0, 1, 384, 499);
    unknown_kind[5] = 0;
    let mut bytes = stream("26.1.0.5530", &[1]);
    append_payload(&mut bytes, &unknown_kind);
    assert_eq!(
        parse_bytes(&bytes)["playlistPatternClips"]["reason"],
        "PLAYLIST_CLIP_KIND_UNVERIFIED"
    );

    let mut bytes = stream("26.1.0.5530", &[1]);
    append_payload(&mut bytes, &clip(0, 2, 384, 499));
    assert_eq!(
        parse_bytes(&bytes)["playlistPatternClips"]["reason"],
        "PLAYLIST_PATTERN_REFERENCE_UNVERIFIED"
    );

    let mut bytes = stream("26.1.0.5530", &[1]);
    append_payload(&mut bytes, &clip(0, 1, 384, 499));
    append_payload(&mut bytes, &clip(384, 1, 384, 499));
    assert_eq!(
        parse_bytes(&bytes)["playlistPatternClips"]["reason"],
        "MULTIPLE_ARRANGEMENTS_UNVERIFIED"
    );
}

#[test]
fn zero_length_and_overflow_are_bounded() {
    let mut zero = stream("26.1.0.5530", &[1]);
    append_payload(&mut zero, &clip(0, 1, 0, 499));
    assert_eq!(
        parse_bytes(&zero)["playlistPatternClips"]["reason"],
        "ZERO_LENGTH_PLAYLIST_CLIP_UNVERIFIED"
    );

    let mut overflow = stream("26.1.0.5530", &[1]);
    append_payload(&mut overflow, &clip(u32::MAX, 1, 2, 499));
    let failed = parse_bytes(&overflow);
    assert_eq!(failed["code"], "PLAYLIST_POSITION_OVERFLOW");
    assert_eq!(failed["playlistPatternClips"]["status"], "failed");

    let mut excessive = stream("26.1.0.5530", &[1]);
    append_payload(
        &mut excessive,
        &clip(0, 1, 384, 499).repeat(MAX_PLAYLIST_CLIPS + 1),
    );
    assert_eq!(parse_bytes(&excessive)["code"], "PLAYLIST_CLIP_LIMIT");
}
