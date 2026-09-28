//! Bounded, read-only FLP metadata parser selected under ADR-002.
//! Event interpretation derives from research source commit 080e825.

use serde_json::{Value, json};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::UNIX_EPOCH;

pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_EVENTS: usize = 100_000;
const MAX_CHANNELS: u16 = 256;
const MAX_EVENT_BYTES: usize = 2 * 1024 * 1024;

fn field(status: &str, value: Value, reason: Option<&str>) -> Value {
    match reason {
        Some(reason) => json!({"status":status,"reason":reason}),
        None => json!({"status":status,"value":value}),
    }
}

pub fn failed(code: &str) -> Value {
    json!({
        "outcome":"failed", "code":code,
        "savedVersion":field("failed",Value::Null,Some(code)),
        "baseTempoBpm":field("failed",Value::Null,Some(code)),
        "channelCount":field("failed",Value::Null,Some(code)),
        "channelNames":field("failed",Value::Null,Some(code)),
        "sampleReferences":field("failed",Value::Null,Some(code)),
        "diagnostics":[]
    })
}

fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("checked four-byte slice"))
}

fn three_byte_event_172(version: &str) -> bool {
    let parts: Vec<u32> = version.split('.').filter_map(|s| s.parse().ok()).collect();
    matches!(parts.as_slice(), [major, ..] if *major >= 26)
        || matches!(parts.as_slice(), [25, minor, patch, ..] if *minor > 2 || (*minor == 2 && *patch >= 4))
}

fn known_sampler_default_build(version: Option<&str>) -> bool {
    matches!(version, Some("25.1.3.4922" | "26.1.0.5530"))
}

fn supported_saved_version(version: &str) -> bool {
    matches!(version, "24.1.0.4225" | "25.1.3.4922" | "26.1.0.5530")
}

fn unsupported_version(version: &str) -> Value {
    json!({
        "outcome":"unsupported", "code":"UNSUPPORTED_SAVED_VERSION",
        "savedVersion":field("extracted",json!(version),None),
        "baseTempoBpm":field("unsupported",Value::Null,Some("UNSUPPORTED_SAVED_VERSION")),
        "channelCount":field("unsupported",Value::Null,Some("UNSUPPORTED_SAVED_VERSION")),
        "channelNames":field("unsupported",Value::Null,Some("UNSUPPORTED_SAVED_VERSION")),
        "sampleReferences":field("unsupported",Value::Null,Some("UNSUPPORTED_SAVED_VERSION")),
        "diagnostics":[]
    })
}

fn utf16_text(data: &[u8]) -> Result<String, &'static str> {
    if data.len() < 2 || !data.len().is_multiple_of(2) || data.len() > 8192 {
        return Err("INVALID_TEXT_LENGTH");
    }
    let units: Vec<u16> = data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    if units.last() != Some(&0) || units[..units.len() - 1].contains(&0) {
        return Err("INVALID_TEXT_TERMINATOR");
    }
    String::from_utf16(&units[..units.len() - 1]).map_err(|_| "INVALID_TEXT_ENCODING")
}

pub fn parse_bytes(bytes: &[u8]) -> Value {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return failed("FILE_SIZE_LIMIT");
    }
    if bytes.len() < 22 || &bytes[0..4] != b"FLhd" || le_u32(&bytes[4..8]) != 6 {
        return failed("INVALID_HEADER");
    }
    let header_channels = u16::from_le_bytes([bytes[10], bytes[11]]);
    if header_channels > MAX_CHANNELS {
        return failed("CHANNEL_COUNT_LIMIT");
    }
    if &bytes[14..18] != b"FLdt" {
        return failed("MISSING_DATA_CHUNK");
    }
    let declared_len = le_u32(&bytes[18..22]) as usize;
    if declared_len.checked_add(22) != Some(bytes.len()) {
        return failed("TRUNCATED_DATA_CHUNK");
    }

    let mut cursor = 22usize;
    let mut version: Option<String> = None;
    let mut tempo: Option<f64> = None;
    let mut diagnostics: Vec<Value> = Vec::new();
    let mut names: Vec<Option<String>> = Vec::new();
    let mut channel_kinds: Vec<u16> = Vec::new();
    let mut samples: Vec<Option<String>> = Vec::new();
    let mut current_channel: Option<usize> = None;
    let mut event_count = 0usize;
    while cursor < bytes.len() {
        if event_count == MAX_EVENTS {
            return failed("EVENT_COUNT_LIMIT");
        }
        event_count += 1;
        let id = bytes[cursor];
        cursor += 1;
        if event_count == 1 && id != 199 {
            return failed("MISSING_SAVED_VERSION");
        }
        let width = match id {
            0..=63 => 1,
            64..=127 => 2,
            172 if version.as_deref().is_some_and(three_byte_event_172) => 3,
            128..=191 => 4,
            _ => {
                let mut value = 0usize;
                let mut shift = 0usize;
                loop {
                    if cursor >= bytes.len() || shift > 28 {
                        return failed("MALFORMED_EVENT_LENGTH");
                    }
                    let byte = bytes[cursor];
                    cursor += 1;
                    value |= ((byte & 127) as usize) << shift;
                    if byte < 128 {
                        break;
                    }
                    shift += 7;
                }
                value
            }
        };
        if width > MAX_EVENT_BYTES {
            return failed("EVENT_LENGTH_OUT_OF_BOUNDS");
        }
        let Some(end) = cursor.checked_add(width) else {
            return failed("EVENT_LENGTH_OUT_OF_BOUNDS");
        };
        if end > bytes.len() {
            return failed("EVENT_LENGTH_OUT_OF_BOUNDS");
        }
        let data = &bytes[cursor..end];
        cursor = end;
        match id {
            64 => {
                if names.len() >= MAX_CHANNELS as usize {
                    return failed("CHANNEL_COUNT_LIMIT");
                }
                current_channel = Some(names.len());
                names.push(None);
                channel_kinds.push(u16::from_le_bytes([data[0], data[1]]));
                samples.push(None);
            }
            98 => current_channel = None,
            199 => {
                if version.is_some() || data.len() < 2 || data.last() != Some(&0) {
                    return failed("INVALID_SAVED_VERSION");
                }
                let Ok(s) = std::str::from_utf8(&data[..data.len() - 1]) else {
                    return failed("INVALID_SAVED_VERSION");
                };
                if s.len() > 32
                    || s.is_empty()
                    || !s.bytes().all(|c| c.is_ascii_digit() || c == b'.')
                {
                    return failed("INVALID_SAVED_VERSION");
                }
                if !supported_saved_version(s) {
                    return unsupported_version(s);
                }
                version = Some(s.to_owned());
            }
            156 => {
                if tempo.is_some() {
                    return failed("MULTIPLE_BASE_TEMPOS");
                }
                let bpm = le_u32(data) as f64 / 1000.0;
                if !(1.0..=999.0).contains(&bpm) {
                    return failed("INVALID_BASE_TEMPO");
                }
                tempo = Some(bpm);
            }
            203 if current_channel.is_some() => {
                let index = current_channel.expect("guarded");
                if names[index].is_some() {
                    return failed("MULTIPLE_CHANNEL_NAMES");
                }
                let Ok(name) = utf16_text(data) else {
                    return failed("INVALID_CHANNEL_NAME");
                };
                names[index] = Some(name);
            }
            196 if current_channel.is_some() => {
                let index = current_channel.expect("guarded");
                if samples[index].is_some() {
                    return failed("MULTIPLE_SAMPLE_REFERENCES");
                }
                let Ok(reference) = utf16_text(data) else {
                    return failed("INVALID_SAMPLE_REFERENCE");
                };
                if !reference.is_empty() {
                    samples[index] = Some(reference);
                }
            }
            255 => diagnostics.push(json!({"code":"UNSUPPORTED_EVENT","eventId":255})),
            _ => {}
        }
    }
    if names.len() != header_channels as usize {
        return failed("CHANNEL_COUNT_MISMATCH");
    }
    let infer_sampler_default = known_sampler_default_build(version.as_deref());
    let saved_version = match version {
        Some(value) => field("extracted", json!(value), None),
        None => field("unavailable", Value::Null, Some("SAVED_VERSION_ABSENT")),
    };
    let base_tempo = match tempo {
        Some(value) => field("extracted", json!(value), None),
        None => field("unavailable", Value::Null, Some("BASE_TEMPO_ABSENT")),
    };
    let channel_count = names.len();
    let channel_names = if names.iter().all(Option::is_some) {
        field(
            "extracted",
            json!(names.into_iter().flatten().collect::<Vec<_>>()),
            None,
        )
    } else {
        let items = names
            .into_iter()
            .zip(channel_kinds)
            .map(|(name, kind)| match name {
                Some(value) => field("extracted", json!(value), None),
                None if infer_sampler_default && kind == 0 => json!({
                    "status":"inferred", "value":"Sampler",
                    "method":"sampler-default-for-known-build", "confidence":"high"
                }),
                None => field("unavailable", Value::Null, Some("CHANNEL_NAME_NOT_STORED")),
            })
            .collect::<Vec<_>>();
        if items
            .iter()
            .all(|item| item["status"].as_str() != Some("unavailable"))
        {
            json!({
                "status":"inferred",
                "value":items.iter().map(|item| item["value"].clone()).collect::<Vec<_>>(),
                "method":"sampler-default-for-known-build", "confidence":"high",
                "items":items
            })
        } else {
            json!({
                "status":"unavailable", "reason":"CHANNEL_NAME_NOT_STORED",
                "items":items
            })
        }
    };
    let sample_references = if samples.iter().all(Option::is_some) {
        field(
            "extracted",
            json!(samples.into_iter().flatten().collect::<Vec<_>>()),
            None,
        )
    } else {
        json!({
            "status":"unavailable", "reason":"SAMPLE_REFERENCE_NOT_STORED",
            "items":samples.into_iter().map(|sample| match sample {
                Some(value) => field("extracted",json!(value),None),
                None => field("unavailable",Value::Null,Some("SAMPLE_REFERENCE_NOT_STORED")),
            }).collect::<Vec<_>>()
        })
    };
    json!({
        "outcome":if diagnostics.is_empty() {"complete"} else {"partial"},
        "savedVersion":saved_version,
        "baseTempoBpm":base_tempo,
        "channelCount":field("extracted",json!(channel_count),None),
        "channelNames":channel_names,
        "sampleReferences":sample_references,
        "diagnostics":diagnostics,
        "eventCount":event_count
    })
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedFingerprint {
    pub size: u64,
    pub modified_at_ms: u64,
}

pub fn modified_at_ms(metadata: &std::fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis()
        .try_into()
        .ok()
}

/// Opens only the explicit path supplied by the trusted supervisor. The
/// supervisor owns root authorization and must not pass unvalidated paths.
pub fn parse_file(path: &Path, expected: ExpectedFingerprint) -> Value {
    let Ok(file) = File::open(path) else {
        return failed("INPUT_OPEN_FAILED");
    };
    let Ok(before) = file.metadata() else {
        return failed("INPUT_METADATA_FAILED");
    };
    if !before.is_file() {
        return failed("INPUT_NOT_FILE");
    }
    if before.len() > MAX_FILE_BYTES {
        return failed("FILE_SIZE_LIMIT");
    }
    let Some(before_modified) = modified_at_ms(&before) else {
        return failed("INPUT_FINGERPRINT_UNAVAILABLE");
    };
    if before.len() != expected.size || before_modified != expected.modified_at_ms {
        return failed("INPUT_CHANGED");
    }
    let mut bytes = Vec::with_capacity(before.len() as usize);
    let mut reader = file.take(MAX_FILE_BYTES + 1);
    if reader.read_to_end(&mut bytes).is_err() {
        return failed("INPUT_READ_FAILED");
    }
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return failed("FILE_SIZE_LIMIT");
    }
    let Ok(after) = reader.into_inner().metadata() else {
        return failed("INPUT_METADATA_FAILED");
    };
    let Some(after_modified) = modified_at_ms(&after) else {
        return failed("INPUT_FINGERPRINT_UNAVAILABLE");
    };
    if after.len() != expected.size || after_modified != expected.modified_at_ms {
        return failed("INPUT_CHANGED");
    }
    let mut result = parse_bytes(&bytes);
    result["inputFingerprint"] = json!({
        "size": expected.size,
        "modifiedAtMs": expected.modified_at_ms,
        "hash": null
    });
    result
}
