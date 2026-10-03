use fruitboard_flp_parser::{MAX_FILE_BYTES, parse_bytes, sha256_hex};
use serde_json::{Value, json};

fn event(id: u8, data: &[u8]) -> Vec<u8> {
    let mut result = vec![id];
    let mut length = data.len();
    loop {
        let byte = (length & 127) as u8;
        length >>= 7;
        result.push(byte | if length == 0 { 0 } else { 128 });
        if length == 0 {
            break;
        }
    }
    result.extend_from_slice(data);
    result
}

// Constructed data, not a new GUI compatibility fixture. The opaque wrapper
// state is deliberately large; only its tiny name/vendor subrecords are read.
fn project(size: usize, event_id: u8) -> Vec<u8> {
    let mut data = event(199, b"26.1.0.5530\0");
    let class = "Fruity Wrapper\0"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>();
    data.extend(event(201, &class));
    let mut wrapper = 8_u32.to_le_bytes().to_vec();
    for (id, text) in [
        (54_u32, b"Synthetic Plugin".as_slice()),
        (56, b"Synthetic Vendor".as_slice()),
    ] {
        wrapper.extend(id.to_le_bytes());
        wrapper.extend((text.len() as u64).to_le_bytes());
        wrapper.extend(text);
    }
    // Payloads here use four varint bytes, including the 64 MiB boundary.
    let state_length = size - 22 - data.len() - 5 - wrapper.len() - 12;
    wrapper.extend(53_u32.to_le_bytes());
    wrapper.extend((state_length as u64).to_le_bytes());
    wrapper.resize(wrapper.len() + state_length, 0xa5);
    data.extend(event(event_id, &wrapper));
    let mut bytes = b"FLhd".to_vec();
    bytes.extend(6_u32.to_le_bytes());
    bytes.extend(0_u16.to_le_bytes());
    bytes.extend(0_u16.to_le_bytes());
    bytes.extend(96_u16.to_le_bytes());
    bytes.extend(b"FLdt");
    bytes.extend((data.len() as u32).to_le_bytes());
    bytes.extend(data);
    assert_eq!(bytes.len(), size);
    bytes
}

#[test]
fn ordinary_projects_and_exact_ceiling_keep_metadata_without_plugin_state() {
    for size in [4_601_596, 25_000_000, MAX_FILE_BYTES as usize] {
        let bytes = project(size, 213);
        let before = sha256_hex(&bytes);
        let result = parse_bytes(&bytes);
        assert_ne!(result["outcome"], "failed", "{size}: {result}");
        assert_eq!(result["savedVersion"]["value"], "26.1.0.5530");
        let plugin = &result["pluginReferences"]["value"][0];
        assert_eq!(plugin["name"]["value"], "Synthetic Plugin");
        assert_eq!(plugin["vendor"]["value"], "Synthetic Vendor");
        assert!(result.to_string().len() < 4096);
        assert!(!result.to_string().contains("state"));
        assert_eq!(sha256_hex(&bytes), before);
    }
}

#[test]
fn larger_files_and_non_state_events_still_fail_closed() {
    assert_eq!(
        parse_bytes(&project(MAX_FILE_BYTES as usize + 1, 213))["code"],
        "FILE_SIZE_LIMIT"
    );
    assert_eq!(
        parse_bytes(&project(4_601_596, 233))["code"],
        "EVENT_LENGTH_OUT_OF_BOUNDS"
    );
    let mut bytes = project(4_601_596, 213);
    bytes.pop();
    let length = (bytes.len() - 22) as u32;
    bytes[18..22].copy_from_slice(&length.to_le_bytes());
    assert_eq!(parse_bytes(&bytes)["code"], "EVENT_LENGTH_OUT_OF_BOUNDS");
}

#[test]
fn capabilities_accept_current_and_lower_ceiling_but_reject_unbounded_claims() {
    let mut descriptor: Value = json!({
        "adapter":fruitboard_flp_parser::ADAPTER_ID,"adapterVersion":fruitboard_flp_parser::ADAPTER_VERSION,
        "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","sampleReferences"],
        "maxFileBytes":MAX_FILE_BYTES,"maxEventBytes":MAX_FILE_BYTES,
        "maxEvents":100000,"maxChannels":256,"maxPatterns":1024,"maxPlaylistClips":1024
    });
    assert!(fruitboard_flp_parser::validation::validate_descriptor(&descriptor).is_ok());
    descriptor["maxFileBytes"] = json!(4 * 1024 * 1024);
    descriptor["maxEventBytes"] = json!(2 * 1024 * 1024);
    assert!(fruitboard_flp_parser::validation::validate_descriptor(&descriptor).is_ok());
    descriptor["maxFileBytes"] = json!(MAX_FILE_BYTES + 1);
    assert!(fruitboard_flp_parser::validation::validate_descriptor(&descriptor).is_err());
}
