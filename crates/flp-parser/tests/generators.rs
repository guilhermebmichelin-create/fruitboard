use fruitboard_flp_parser::parse_bytes;
use serde_json::json;

struct SyntheticChannel {
    id: u16,
    type_events: Vec<u8>,
    generator_events: Vec<Vec<u8>>,
    editable_name: Option<String>,
}

fn channel(
    id: u16,
    type_events: &[u8],
    generator_events: &[&[u8]],
    editable_name: Option<&str>,
) -> SyntheticChannel {
    SyntheticChannel {
        id,
        type_events: type_events.to_vec(),
        generator_events: generator_events
            .iter()
            .map(|event| event.to_vec())
            .collect(),
        editable_name: editable_name.map(str::to_owned),
    }
}

fn stream(version: &str, channels: &[SyntheticChannel], prefix_type_events: &[u8]) -> Vec<u8> {
    let mut events = vec![199, (version.len() + 1) as u8];
    events.extend_from_slice(version.as_bytes());
    events.push(0);
    events.push(156);
    events.extend_from_slice(&130_000_u32.to_le_bytes());
    for channel_type in prefix_type_events {
        events.extend_from_slice(&[21, *channel_type]);
    }
    for channel in channels {
        events.push(64);
        events.extend_from_slice(&channel.id.to_le_bytes());
        for channel_type in &channel.type_events {
            events.extend_from_slice(&[21, *channel_type]);
        }
        for generator_event in &channel.generator_events {
            events.extend_from_slice(generator_event);
        }
        if let Some(name) = &channel.editable_name {
            text_event(&mut events, 203, name);
        }
    }
    let mut bytes = b"FLhd".to_vec();
    bytes.extend_from_slice(&6_u32.to_le_bytes());
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&(channels.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&96_u16.to_le_bytes());
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
fn channel_id_and_type_are_independent_in_synth_first_sparse_layout() {
    let sampler_class = generator_event("");
    let synth_class = generator_event("3x Osc");
    let channels = [
        channel(0, &[2], &[&synth_class], None),
        channel(511, &[0], &[&sampler_class], None),
        channel(7, &[0], &[&sampler_class], None),
    ];
    let parsed = parse_bytes(&stream("26.1.0.5530", &channels, &[]));

    assert_eq!(parsed["outcome"], "complete");
    assert_eq!(parsed["channelCount"]["value"], 3);
    assert_eq!(parsed["channelNames"]["status"], "unavailable");
    assert_eq!(
        parsed["channelNames"]["items"][0]["reason"],
        "CHANNEL_NAME_NOT_STORED"
    );
    assert_eq!(parsed["channelNames"]["items"][1]["value"], "Sampler");
    assert_eq!(parsed["channelNames"]["items"][2]["value"], "Sampler 2");
    assert_eq!(
        parsed["channelGeneratorNames"]["value"],
        json!(["3x Osc", "Sampler", "Sampler"])
    );
    assert_eq!(parsed["channelGeneratorNames"]["status"], "inferred");
    assert_eq!(parsed["channelGeneratorNames"]["confidence"], "medium");
    assert_eq!(
        parsed["channelGeneratorNames"]["items"][0]["name"],
        json!({"status":"extracted","value":"3x Osc"})
    );
}

#[test]
fn missing_duplicate_and_invalid_type_events_keep_stored_names_and_block_inference() {
    let empty_class = generator_event("");
    let osc_class = generator_event("3x Osc");
    let wrapper_class = generator_event("Fruity Wrapper");
    let channels = [
        channel(12, &[], &[&empty_class], Some("Saved without type")),
        channel(
            80,
            &[0, 0],
            &[&empty_class],
            Some("Saved with duplicate type"),
        ),
        channel(
            3,
            &[255],
            &[&osc_class, &wrapper_class],
            Some("Saved with invalid type"),
        ),
    ];
    let parsed = parse_bytes(&stream("26.1.0.5530", &channels, &[]));

    assert_eq!(parsed["outcome"], "complete");
    assert_eq!(
        parsed["channelNames"],
        json!({"status":"extracted","value":[
            "Saved without type",
            "Saved with duplicate type",
            "Saved with invalid type"
        ]})
    );
    assert_eq!(parsed["channelGeneratorNames"]["status"], "unsupported");
    assert_eq!(
        parsed["channelGeneratorNames"]["items"][0]["name"]["status"],
        "unsupported"
    );
    assert_eq!(
        parsed["channelGeneratorNames"]["items"][1]["name"]["status"],
        "unsupported"
    );
    assert_eq!(
        parsed["channelGeneratorNames"]["items"][2]["name"]["status"],
        "unsupported"
    );
}

#[test]
fn type_events_outside_channel_context_are_ignored() {
    let empty_class = generator_event("");
    let channels = [channel(0, &[], &[&empty_class], None)];
    let parsed = parse_bytes(&stream("26.1.0.5530", &channels, &[0]));

    assert_eq!(parsed["outcome"], "complete");
    assert_eq!(parsed["channelNames"]["status"], "unavailable");
    assert_eq!(parsed["channelNames"]["items"][0]["status"], "unavailable");
    assert_eq!(parsed["channelGeneratorNames"]["status"], "unsupported");
}

#[test]
fn unknown_or_malformed_class_does_not_become_a_plugin_claim() {
    let empty_class = generator_event("");
    let wrapper_class = generator_event("Fruity Wrapper");
    let channels = [
        channel(41, &[0], &[&empty_class], None),
        channel(9, &[2], &[&wrapper_class], Some("Wrapper")),
    ];
    let parsed = parse_bytes(&stream("26.1.0.5530", &channels, &[]));
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

    for malformed in [
        vec![201, 2, b'X', 0], // No UTF-16 terminator.
        vec![201, 2, 0, 0],    // Empty native class.
        Vec::new(),            // Class event not stored.
    ] {
        let channels = [
            channel(41, &[0], &[&empty_class], None),
            channel(9, &[2], &[&malformed], Some("Fixture Synth A")),
        ];
        let parsed = parse_bytes(&stream("26.1.0.5530", &channels, &[]));
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

    let osc_class = generator_event("3x Osc");
    let repeated = [
        channel(41, &[0], &[&empty_class], None),
        channel(9, &[2], &[&osc_class, &osc_class], Some("Fixture Synth A")),
    ];
    let parsed = parse_bytes(&stream("26.1.0.5530", &repeated, &[]));
    assert_eq!(
        parsed["channelGeneratorNames"]["items"][1]["name"]["reason"],
        "MULTIPLE_GENERATOR_EVENTS_UNVERIFIED"
    );
}

#[test]
fn older_saved_builds_stay_unverified_for_generator_names() {
    let sampler_class = generator_event("");
    let osc_class = generator_event("3x Osc");
    let channels = [
        channel(41, &[0], &[&sampler_class], None),
        channel(9, &[2], &[&osc_class], Some("Fixture Synth A")),
    ];
    for version in ["24.1.0.4225", "25.1.3.4922"] {
        let parsed = parse_bytes(&stream(version, &channels, &[]));
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
