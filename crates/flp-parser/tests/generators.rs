use fruitboard_flp_parser::parse_bytes;
use serde_json::json;

// In-memory boundary cases complement the approved matching-build GUI fixture.
fn stream(version: &str, generator_event: &[u8]) -> Vec<u8> {
    let mut events = vec![199, (version.len() + 1) as u8];
    events.extend_from_slice(version.as_bytes());
    events.push(0);
    events.push(156);
    events.extend_from_slice(&130_000_u32.to_le_bytes());
    events.extend_from_slice(&[64, 0, 0]); // Sampler kind.
    events.extend_from_slice(&[201, 2, 0, 0]); // Stored empty class name.
    events.extend_from_slice(&[64, 1, 0]); // Native generator kind.
    events.extend_from_slice(generator_event);
    text_event(&mut events, 203, "Fixture Synth A"); // Editable channel label.
    let mut bytes = b"FLhd".to_vec();
    bytes.extend_from_slice(&6_u32.to_le_bytes());
    bytes.extend_from_slice(&[0, 0, 2, 0, 96, 0]);
    bytes.extend_from_slice(b"FLdt");
    bytes.extend_from_slice(&(events.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&events);
    bytes
}

fn text_event(events: &mut Vec<u8>, id: u8, text: &str) {
    let data = text
        .encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    assert!(data.len() < 128);
    events.extend_from_slice(&[id, data.len() as u8]);
    events.extend(data);
}

fn generator_event(text: &str) -> Vec<u8> {
    let mut event = Vec::new();
    text_event(&mut event, 201, text);
    event
}

#[test]
fn class_identity_is_separate_from_renamed_channel() {
    let parsed = parse_bytes(&stream("26.1.0.5530", &generator_event("3x Osc")));
    assert_eq!(parsed["outcome"], "complete");
    assert_eq!(parsed["channelCount"]["value"], 2);
    assert_eq!(
        parsed["channelNames"]["value"],
        json!(["Sampler", "Fixture Synth A"])
    );
    assert_eq!(
        parsed["channelGeneratorNames"]["value"],
        json!(["Sampler", "3x Osc"])
    );
    assert_eq!(parsed["channelGeneratorNames"]["status"], "inferred");
    assert_eq!(
        parsed["channelGeneratorNames"]["method"],
        "mixed-extracted-and-sampler-generator-default"
    );
    assert_eq!(parsed["channelGeneratorNames"]["confidence"], "medium");
    assert_eq!(
        parsed["channelGeneratorNames"]["items"][0]["name"]["status"],
        "inferred"
    );
    assert_eq!(
        parsed["channelGeneratorNames"]["items"][1]["name"],
        json!({"status":"extracted","value":"3x Osc"})
    );
}

#[test]
fn unknown_or_malformed_class_does_not_become_a_plugin_claim() {
    for event in [
        generator_event("Fruity Wrapper"),
        vec![201, 2, b'X', 0], // No UTF-16 terminator.
        vec![201, 2, 0, 0],    // Empty native class.
        Vec::new(),            // Class event not stored.
    ] {
        let parsed = parse_bytes(&stream("26.1.0.5530", &event));
        assert_eq!(parsed["outcome"], "complete");
        assert_eq!(parsed["channelGeneratorNames"]["status"], "unsupported");
        assert_eq!(
            parsed["channelGeneratorNames"]["items"][1]["name"]["status"],
            "unsupported"
        );
        assert_eq!(
            parsed["channelGeneratorNames"]["items"][1]["channelIndex"],
            1
        );
    }
    let mut repeated = generator_event("3x Osc");
    repeated.extend(generator_event("3x Osc"));
    let parsed = parse_bytes(&stream("26.1.0.5530", &repeated));
    assert_eq!(
        parsed["channelGeneratorNames"]["items"][1]["name"]["reason"],
        "MULTIPLE_GENERATOR_EVENTS_UNVERIFIED"
    );
}

#[test]
fn older_saved_builds_stay_unverified_for_generator_names() {
    for version in ["24.1.0.4225", "25.1.3.4922"] {
        let parsed = parse_bytes(&stream(version, &generator_event("3x Osc")));
        assert_eq!(parsed["channelGeneratorNames"]["status"], "unsupported");
        assert_eq!(
            parsed["channelGeneratorNames"]["reason"],
            "GENERATOR_NAMES_UNVERIFIED_BUILD"
        );
    }
}

#[test]
fn approved_two_channel_fixture_separates_generator_and_label_without_writing() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/parser-corpus/FIX-FL2026-3XOSC.flp");
    let before = std::fs::read(&path).expect("approved F14 fixture");
    assert_eq!(before.len(), 47_881);
    let parsed = parse_bytes(&before);
    assert_eq!(parsed["outcome"], "complete");
    assert_eq!(parsed["savedVersion"]["value"], "26.1.0.5530");
    assert_eq!(parsed["baseTempoBpm"]["value"], 130.0);
    assert_eq!(parsed["channelCount"]["value"], 2);
    assert_eq!(
        parsed["channelNames"]["value"],
        json!(["Sampler", "Fixture Synth A"])
    );
    assert_eq!(parsed["channelNames"]["confidence"], "medium");
    assert_eq!(
        parsed["channelGeneratorNames"]["value"],
        json!(["Sampler", "3x Osc"])
    );
    assert_eq!(parsed["channelGeneratorNames"]["confidence"], "medium");
    assert_eq!(
        parsed["channelGeneratorNames"]["items"][0]["name"]["status"],
        "inferred"
    );
    assert_eq!(
        parsed["channelGeneratorNames"]["items"][1]["name"],
        json!({"status":"extracted","value":"3x Osc"})
    );
    assert_eq!(parsed["patternCount"]["status"], "unavailable");
    assert_eq!(parsed["sampleReferences"]["status"], "unavailable");
    assert_eq!(std::fs::read(path).unwrap(), before);
}
