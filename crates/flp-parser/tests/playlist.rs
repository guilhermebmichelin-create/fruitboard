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

fn append_meter(bytes: &mut Vec<u8>, numerator: u8, denominator: u8) {
    bytes.extend_from_slice(&[17, numerator, 18, denominator]);
    let data_length = (bytes.len() - 22) as u32;
    bytes[18..22].copy_from_slice(&data_length.to_le_bytes());
}

fn append_tempo(bytes: &mut Vec<u8>, millibpm: u32) {
    bytes.push(156);
    bytes.extend_from_slice(&millibpm.to_le_bytes());
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
    assert_eq!(parsed["playlistPatternEndTick"]["status"], "extracted");
    assert_eq!(parsed["playlistPatternEndTick"]["value"], 1536);
    assert_eq!(
        parsed["playlistPatternNominalSeconds"]["status"],
        "inferred"
    );
    assert_eq!(parsed["playlistPatternNominalSeconds"]["confidence"], "low");
    let nominal = parsed["playlistPatternNominalSeconds"]["value"]
        .as_f64()
        .unwrap();
    assert!((nominal - 1536.0 / 96.0 * 60.0 / 130.0).abs() < 1e-10);
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
        assert_eq!(
            parsed["playlistPatternEndTick"]["reason"],
            "NO_PLAYLIST_PATTERN_CLIPS"
        );
        assert_eq!(
            parsed["playlistPatternNominalSeconds"]["reason"],
            "NO_PLAYLIST_PATTERN_CLIPS"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[test]
fn missing_and_unverified_playlist_data_is_labeled() {
    assert_eq!(
        parse_bytes(&stream("26.1.0.5530", &[1]))["playlistPatternClips"]["reason"],
        "PLAYLIST_DATA_NOT_STORED"
    );
    assert_eq!(
        parse_bytes(&stream("26.1.0.5530", &[1]))["playlistPatternEndTick"]["reason"],
        "PLAYLIST_DATA_NOT_STORED"
    );
    for version in ["24.1.0.4225", "25.1.3.4922", "99.0.0.0"] {
        let mut bytes = stream(version, &[1]);
        append_payload(&mut bytes, &clip(0, 1, 384, 499));
        assert_eq!(
            parse_bytes(&bytes)["playlistPatternClips"]["status"],
            "unsupported"
        );
        assert_eq!(
            parse_bytes(&bytes)["playlistPatternEndTick"]["status"],
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
    assert_eq!(
        parse_bytes(&malformed)["playlistPatternNominalSeconds"]["reason"],
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
    assert_eq!(failed["playlistPatternEndTick"]["status"], "failed");
    assert_eq!(failed["playlistPatternNominalSeconds"]["status"], "failed");

    let mut excessive = stream("26.1.0.5530", &[1]);
    append_payload(
        &mut excessive,
        &clip(0, 1, 384, 499).repeat(MAX_PLAYLIST_CLIPS + 1),
    );
    assert_eq!(parse_bytes(&excessive)["code"], "PLAYLIST_CLIP_LIMIT");
}

#[test]
fn timeline_uses_maximum_clip_end_across_overlap_and_gap() {
    let mut bytes = stream("26.1.0.5530", &[1, 2]);
    append_meter(&mut bytes, 4, 4);
    append_tempo(&mut bytes, 130_000);
    let mut records = Vec::new();
    records.extend_from_slice(&clip(0, 1, 768, 499));
    records.extend_from_slice(&clip(384, 2, 384, 498));
    append_payload(&mut bytes, &records);
    let parsed = parse_bytes(&bytes);
    assert_eq!(parsed["playlistPatternEndTick"]["value"], 768);
    assert_eq!(
        parsed["playlistPatternNominalSeconds"]["status"],
        "inferred"
    );
    assert!(
        (parsed["playlistPatternNominalSeconds"]["value"]
            .as_f64()
            .unwrap()
            - 768.0 / 96.0 * 60.0 / 130.0)
            .abs()
            < 1e-10
    );

    let mut gap = stream("26.1.0.5530", &[1, 2]);
    append_meter(&mut gap, 4, 4);
    append_tempo(&mut gap, 130_000);
    records.extend_from_slice(&clip(1536, 1, 384, 499));
    append_payload(&mut gap, &records);
    assert_eq!(parse_bytes(&gap)["playlistPatternEndTick"]["value"], 1920);
}

#[test]
fn nominal_seconds_needs_known_ppq_meter_and_base_tempo() {
    let mut no_tempo = stream("26.1.0.5530", &[1]);
    append_meter(&mut no_tempo, 4, 4);
    append_payload(&mut no_tempo, &clip(0, 1, 384, 499));
    let parsed = parse_bytes(&no_tempo);
    assert_eq!(parsed["playlistPatternEndTick"]["value"], 384);
    assert_eq!(
        parsed["playlistPatternNominalSeconds"]["reason"],
        "BASE_TEMPO_ABSENT"
    );

    let mut changed_ppq = no_tempo.clone();
    changed_ppq[12..14].copy_from_slice(&120_u16.to_le_bytes());
    append_tempo(&mut changed_ppq, 130_000);
    assert_eq!(
        parse_bytes(&changed_ppq)["playlistPatternNominalSeconds"]["reason"],
        "PPQ_UNVERIFIED"
    );

    let mut changed_meter = stream("26.1.0.5530", &[1]);
    append_meter(&mut changed_meter, 3, 4);
    append_tempo(&mut changed_meter, 130_000);
    append_payload(&mut changed_meter, &clip(0, 1, 384, 499));
    assert_eq!(
        parse_bytes(&changed_meter)["playlistPatternNominalSeconds"]["reason"],
        "METER_UNVERIFIED"
    );

    let mut repeated_meter = stream("26.1.0.5530", &[1]);
    append_meter(&mut repeated_meter, 4, 4);
    append_meter(&mut repeated_meter, 4, 4);
    append_tempo(&mut repeated_meter, 130_000);
    append_payload(&mut repeated_meter, &clip(0, 1, 384, 499));
    assert_eq!(
        parse_bytes(&repeated_meter)["playlistPatternNominalSeconds"]["reason"],
        "METER_UNVERIFIED"
    );
}
