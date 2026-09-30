//! Research-only, read-only parser for the approved issue #138 corpus.
//! This binary is deliberately outside the production Cargo workspace.

use serde_json::{Value, json};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_EVENTS: usize = 100_000;
const MAX_CHANNELS: u16 = 256;
const MAX_EVENT_BYTES: usize = 2 * 1024 * 1024;

fn field(status: &str, value: Value, reason: Option<&str>) -> Value {
    match reason {
        Some(reason) => json!({"status":status,"reason":reason}),
        None => json!({"status":status,"value":value}),
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ChannelType {
    #[default]
    Missing,
    Sampler,
    Invalid,
    Multiple,
}

impl ChannelType {
    fn from_event(value: u8) -> Self {
        match value {
            0 => Self::Sampler,
            _ => Self::Invalid,
        }
    }
}

struct Channel {
    id: u16,
    channel_type: ChannelType,
    name: Option<String>,
    sample: Option<String>,
}

#[derive(Clone, Copy)]
struct ChannelContext {
    id: u16,
    index: usize,
}

impl ChannelContext {
    fn resolve(self, channels: &[Channel]) -> Option<usize> {
        channels
            .get(self.index)
            .filter(|channel| channel.id == self.id)
            .map(|_| self.index)
    }
}

fn failed(code: &str) -> Value {
    json!({
        "outcome":"failed", "code":code,
        "savedVersion":field("failed",Value::Null,Some(code)),
        "baseTempoBpm":field("failed",Value::Null,Some(code)),
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

fn parse_bytes(bytes: &[u8]) -> Value {
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
    let mut channels: Vec<Channel> = Vec::new();
    let mut current_channel: Option<ChannelContext> = None;
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
                if channels.len() >= MAX_CHANNELS as usize {
                    return failed("CHANNEL_COUNT_LIMIT");
                }
                let id = u16::from_le_bytes([data[0], data[1]]);
                let index = channels.len();
                channels.push(Channel {
                    id,
                    channel_type: ChannelType::Missing,
                    name: None,
                    sample: None,
                });
                current_channel = Some(ChannelContext { id, index });
            }
            21 if current_channel.is_some() => {
                if let Some(index) = current_channel.and_then(|context| context.resolve(&channels))
                {
                    let channel = &mut channels[index];
                    channel.channel_type = match channel.channel_type {
                        ChannelType::Missing => ChannelType::from_event(data[0]),
                        _ => ChannelType::Multiple,
                    };
                }
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
                let Some(index) = current_channel.and_then(|context| context.resolve(&channels))
                else {
                    continue;
                };
                if channels[index].name.is_some() {
                    return failed("MULTIPLE_CHANNEL_NAMES");
                }
                let Ok(name) = utf16_text(data) else {
                    return failed("INVALID_CHANNEL_NAME");
                };
                channels[index].name = Some(name);
            }
            196 if current_channel.is_some() => {
                let Some(index) = current_channel.and_then(|context| context.resolve(&channels))
                else {
                    continue;
                };
                if channels[index].sample.is_some() {
                    return failed("MULTIPLE_SAMPLE_REFERENCES");
                }
                let Ok(reference) = utf16_text(data) else {
                    return failed("INVALID_SAMPLE_REFERENCE");
                };
                if !reference.is_empty() {
                    channels[index].sample = Some(reference);
                }
            }
            255 => diagnostics.push(json!({"code":"UNSUPPORTED_EVENT","eventId":255})),
            _ => {}
        }
    }
    if channels.len() != header_channels as usize {
        return failed("CHANNEL_COUNT_MISMATCH");
    }
    let infer_sampler_default = known_sampler_default_build(version.as_deref());
    let names = channels
        .iter()
        .map(|channel| channel.name.clone())
        .collect::<Vec<_>>();
    let channel_types = channels
        .iter()
        .map(|channel| channel.channel_type)
        .collect::<Vec<_>>();
    let saved_version = match version {
        Some(value) => field("extracted", json!(value), None),
        None => field("unavailable", Value::Null, Some("SAVED_VERSION_ABSENT")),
    };
    let base_tempo = match tempo {
        Some(value) => field("extracted", json!(value), None),
        None => field("unavailable", Value::Null, Some("BASE_TEMPO_ABSENT")),
    };
    let channel_names = if names.iter().all(Option::is_some) {
        field(
            "extracted",
            json!(names.iter().flatten().cloned().collect::<Vec<_>>()),
            None,
        )
    } else {
        let items = names
            .into_iter()
            .zip(channel_types)
            .map(|(name, channel_type)| match name {
                Some(value) => field("extracted", json!(value), None),
                None if infer_sampler_default && channel_type == ChannelType::Sampler => json!({
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
    let samples = channels
        .into_iter()
        .map(|channel| channel.sample)
        .collect::<Vec<_>>();
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
        "channelNames":channel_names,
        "sampleReferences":sample_references,
        "diagnostics":diagnostics,
        "eventCount":event_count
    })
}

fn parse_path(path: &Path) -> Value {
    let Ok(file) = File::open(path) else {
        return failed("INPUT_OPEN_FAILED");
    };
    let Ok(metadata) = file.metadata() else {
        return failed("INPUT_METADATA_FAILED");
    };
    if !metadata.is_file() {
        return failed("INPUT_NOT_FILE");
    }
    if metadata.len() > MAX_FILE_BYTES {
        return failed("FILE_SIZE_LIMIT");
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let Ok(_) = file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes) else {
        return failed("INPUT_READ_FAILED");
    };
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return failed("FILE_SIZE_LIMIT");
    }
    parse_bytes(&bytes)
}

#[cfg(windows)]
fn peak_working_set_bytes() -> Option<u64> {
    use std::ffi::c_void;
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_nonpaged_pool_usage: usize,
        quota_nonpaged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
    }
    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }
    let mut counters = ProcessMemoryCounters {
        cb: std::mem::size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_nonpaged_pool_usage: 0,
        quota_nonpaged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    // The OS fills only this local process counter struct. It opens no project file.
    let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    (ok != 0).then_some(counters.peak_working_set_size as u64)
}

#[cfg(not(windows))]
fn peak_working_set_bytes() -> Option<u64> {
    None
}

fn main() {
    let mut args = std::env::args_os();
    let _program = args.next();
    let mut output = match args.next().and_then(|s| s.into_string().ok()).as_deref() {
        Some("describe") if args.next().is_none() => json!({
            "adapter":"rust-research-138", "protocolVersion":1,
            "fields":["savedVersion","baseTempoBpm","channelNames","sampleReferences"],
            "maxFileBytes":MAX_FILE_BYTES,"maxEvents":MAX_EVENTS,
            "maxChannels":MAX_CHANNELS,"maxEventBytes":MAX_EVENT_BYTES
        }),
        Some("health-check") if args.next().is_none() => json!({"status":"ok"}),
        Some("parse") => match args.next() {
            Some(path) if args.next().is_none() => {
                let started = std::time::Instant::now();
                let mut result = parse_path(Path::new(&path));
                result["parseElapsedMicros"] = json!(started.elapsed().as_micros());
                result
            }
            _ => failed("INVALID_REQUEST"),
        },
        _ => failed("INVALID_REQUEST"),
    };
    if let Some(peak) = peak_working_set_bytes() {
        output["peakWorkingSetBytes"] = json!(peak);
    }
    println!("{output}");
}

#[cfg(test)]
mod tests {
    use super::parse_bytes;

    struct SyntheticChannel {
        id: u16,
        type_events: Vec<u8>,
        name: Option<String>,
    }

    fn channel(id: u16, type_events: &[u8], name: Option<&str>) -> SyntheticChannel {
        SyntheticChannel {
            id,
            type_events: type_events.to_vec(),
            name: name.map(str::to_owned),
        }
    }

    fn text_event(events: &mut Vec<u8>, id: u8, text: &str) {
        let data = text
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        events.extend_from_slice(&[id, data.len() as u8]);
        events.extend(data);
    }

    fn stream(channels: &[SyntheticChannel]) -> Vec<u8> {
        let version = "26.1.0.5530";
        let mut events = vec![199, (version.len() + 1) as u8];
        events.extend_from_slice(version.as_bytes());
        events.push(0);
        events.push(156);
        events.extend_from_slice(&130_000_u32.to_le_bytes());
        for channel in channels {
            events.push(64);
            events.extend_from_slice(&channel.id.to_le_bytes());
            for channel_type in &channel.type_events {
                events.extend_from_slice(&[21, *channel_type]);
            }
            if let Some(name) = &channel.name {
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

    #[test]
    fn sampler_inference_uses_event_21_type_not_event_64_id() {
        let channels = [channel(0, &[2], None), channel(1, &[0], None)];
        let parsed = parse_bytes(&stream(&channels));

        assert_eq!(parsed["outcome"], "complete");
        assert_eq!(parsed["channelNames"]["status"], "unavailable");
        assert_eq!(parsed["channelNames"]["items"][0]["status"], "unavailable");
        assert_eq!(
            parsed["channelNames"]["items"][0]["reason"],
            "CHANNEL_NAME_NOT_STORED"
        );
        assert_eq!(parsed["channelNames"]["items"][1]["status"], "inferred");
        assert_eq!(parsed["channelNames"]["items"][1]["value"], "Sampler");
    }

    #[test]
    fn missing_duplicate_and_unknown_types_do_not_hide_stored_names_or_infer_defaults() {
        let channels = [
            channel(7, &[], None),
            channel(40, &[0, 0], Some("Stored despite duplicate type")),
            channel(1, &[255], None),
        ];
        let parsed = parse_bytes(&stream(&channels));

        assert_eq!(parsed["outcome"], "complete");
        assert_eq!(parsed["channelNames"]["status"], "unavailable");
        assert_eq!(parsed["channelNames"]["items"][0]["status"], "unavailable");
        assert_eq!(parsed["channelNames"]["items"][1]["status"], "extracted");
        assert_eq!(
            parsed["channelNames"]["items"][1]["value"],
            "Stored despite duplicate type"
        );
        assert_eq!(parsed["channelNames"]["items"][2]["status"], "unavailable");
    }
}
