//! Bounded, read-only FLP metadata parser selected under ADR-002.
//! Event interpretation derives from research source commit 080e825.

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::UNIX_EPOCH;

pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_EVENTS: usize = 100_000;
const MAX_CHANNELS: u16 = 256;
const MAX_EVENT_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_PATTERNS: usize = 1024;
pub const MAX_PLAYLIST_CLIPS: usize = 1024;

fn known_pattern_build(version: Option<&str>) -> bool {
    version == Some("26.1.0.5530")
}

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
        "patternCount":field("failed",Value::Null,Some(code)),
        "patternNames":field("failed",Value::Null,Some(code)),
        "playlistPatternClips":field("failed",Value::Null,Some(code)),
        "playlistPatternEndTick":field("failed",Value::Null,Some(code)),
        "playlistPatternNominalSeconds":field("failed",Value::Null,Some(code)),
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
        "patternCount":field("unsupported",Value::Null,Some("UNSUPPORTED_SAVED_VERSION")),
        "patternNames":field("unsupported",Value::Null,Some("UNSUPPORTED_SAVED_VERSION")),
        "playlistPatternClips":field("unsupported",Value::Null,Some("UNSUPPORTED_SAVED_VERSION")),
        "playlistPatternEndTick":field("unsupported",Value::Null,Some("UNSUPPORTED_SAVED_VERSION")),
        "playlistPatternNominalSeconds":field("unsupported",Value::Null,Some("UNSUPPORTED_SAVED_VERSION")),
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

fn playlist_pattern_clips(
    version: Option<&str>,
    payload: Option<&[u8]>,
    multiple_payloads: bool,
    pattern_ids: &BTreeSet<u16>,
) -> Result<(Value, Option<u32>), &'static str> {
    if !known_pattern_build(version) {
        return Ok((
            field(
                "unsupported",
                Value::Null,
                Some("PLAYLIST_CLIPS_UNVERIFIED_BUILD"),
            ),
            None,
        ));
    }
    if multiple_payloads {
        return Ok((
            field(
                "unsupported",
                Value::Null,
                Some("MULTIPLE_ARRANGEMENTS_UNVERIFIED"),
            ),
            None,
        ));
    }
    let Some(payload) = payload else {
        return Ok((
            field("unavailable", Value::Null, Some("PLAYLIST_DATA_NOT_STORED")),
            None,
        ));
    };
    // The approved F13 GUI save has four 88-byte event-233 records. The
    // 2026 minimal saves have an empty event-233 payload. Other layouts and
    // clip kinds stay unsupported rather than being interpreted as patterns.
    if !payload.len().is_multiple_of(88) {
        return Ok((
            field(
                "unsupported",
                Value::Null,
                Some("PLAYLIST_CLIP_LAYOUT_UNVERIFIED"),
            ),
            None,
        ));
    }
    if payload.len() / 88 > MAX_PLAYLIST_CLIPS {
        return Err("PLAYLIST_CLIP_LIMIT");
    }
    let mut clips = Vec::with_capacity(payload.len() / 88);
    let mut end_tick: Option<u32> = None;
    for record in payload.as_chunks::<88>().0 {
        let item = le_u32(&record[4..8]);
        let family = item as u16;
        let encoded_id = (item >> 16) as u16;
        if family != 0x5000 || encoded_id <= 0x5000 {
            return Ok((
                field(
                    "unsupported",
                    Value::Null,
                    Some("PLAYLIST_CLIP_KIND_UNVERIFIED"),
                ),
                None,
            ));
        }
        let pattern_id = encoded_id - 0x5000;
        if !pattern_ids.contains(&pattern_id) {
            return Ok((
                field(
                    "unsupported",
                    Value::Null,
                    Some("PLAYLIST_PATTERN_REFERENCE_UNVERIFIED"),
                ),
                None,
            ));
        }
        let start_tick = le_u32(&record[0..4]);
        let length_tick = le_u32(&record[8..12]);
        if length_tick == 0 {
            return Ok((
                field(
                    "unsupported",
                    Value::Null,
                    Some("ZERO_LENGTH_PLAYLIST_CLIP_UNVERIFIED"),
                ),
                None,
            ));
        }
        let clip_end = start_tick
            .checked_add(length_tick)
            .ok_or("PLAYLIST_POSITION_OVERFLOW")?;
        end_tick = Some(end_tick.map_or(clip_end, |previous| previous.max(clip_end)));
        clips.push(json!({
            "patternId":pattern_id,
            "startTick":start_tick,
            "lengthTick":length_tick,
            "trackToken":le_u32(&record[12..16]),
        }));
    }
    Ok((field("extracted", json!(clips), None), end_tick))
}

pub fn parse_bytes(bytes: &[u8]) -> Value {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return failed("FILE_SIZE_LIMIT");
    }
    if bytes.len() < 22 || &bytes[0..4] != b"FLhd" || le_u32(&bytes[4..8]) != 6 {
        return failed("INVALID_HEADER");
    }
    let header_channels = u16::from_le_bytes([bytes[10], bytes[11]]);
    let header_ppq = u16::from_le_bytes([bytes[12], bytes[13]]);
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
    let mut timing_numerator: Option<u8> = None;
    let mut timing_denominator: Option<u8> = None;
    let mut multiple_timing_events = false;
    let mut diagnostics: Vec<Value> = Vec::new();
    let mut names: Vec<Option<String>> = Vec::new();
    let mut channel_kinds: Vec<u16> = Vec::new();
    let mut samples: Vec<Option<String>> = Vec::new();
    let mut current_channel: Option<usize> = None;
    let mut pattern_ids = BTreeSet::new();
    let mut pattern_names = BTreeMap::new();
    let mut current_pattern = None;
    let mut playlist_payload: Option<Vec<u8>> = None;
    let mut multiple_playlist_payloads = false;
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
            17 if known_pattern_build(version.as_deref()) => {
                multiple_timing_events |= timing_numerator.replace(data[0]).is_some();
            }
            18 if known_pattern_build(version.as_deref()) => {
                multiple_timing_events |= timing_denominator.replace(data[0]).is_some();
            }
            64 => {
                current_pattern = None;
                if names.len() >= MAX_CHANNELS as usize {
                    return failed("CHANNEL_COUNT_LIMIT");
                }
                current_channel = Some(names.len());
                names.push(None);
                channel_kinds.push(u16::from_le_bytes([data[0], data[1]]));
                samples.push(None);
            }
            // The GUI-verified 2026 save repeats each pattern ID for note and
            // property sections. Count unique IDs, never marker occurrences.
            65 if known_pattern_build(version.as_deref()) => {
                let id = u16::from_le_bytes([data[0], data[1]]);
                if id == 0 {
                    return failed("INVALID_PATTERN_ID");
                }
                if !pattern_ids.contains(&id) && pattern_ids.len() == MAX_PATTERNS {
                    return failed("PATTERN_COUNT_LIMIT");
                }
                pattern_ids.insert(id);
                current_pattern = Some(id);
                current_channel = None;
            }
            98 => {
                current_channel = None;
                current_pattern = None;
            }
            // Approved F13 stores UTF-16 names after repeated event-65 IDs.
            // A channel/arrangement boundary ends that pattern context.
            193 if known_pattern_build(version.as_deref()) => {
                let Some(pattern_id) = current_pattern else {
                    return failed("PATTERN_NAME_WITHOUT_ID");
                };
                let Ok(name) = utf16_text(data) else {
                    return failed("INVALID_PATTERN_NAME");
                };
                if pattern_names
                    .get(&pattern_id)
                    .is_some_and(|old| old != &name)
                {
                    return failed("CONFLICTING_PATTERN_NAMES");
                }
                pattern_names.insert(pattern_id, name);
            }
            233 if known_pattern_build(version.as_deref()) => {
                if playlist_payload.is_some() {
                    multiple_playlist_payloads = true;
                } else {
                    playlist_payload = Some(data.to_vec());
                }
                current_pattern = None;
                current_channel = None;
            }
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
    let pattern_count = if !known_pattern_build(version.as_deref()) {
        field(
            "unsupported",
            Value::Null,
            Some("PATTERN_COUNT_UNVERIFIED_BUILD"),
        )
    } else if pattern_ids.is_empty() {
        // FL Studio can display an implicit empty Pattern 1 without saving a
        // pattern marker. Its GUI default is not evidence for a stored count.
        field("unavailable", Value::Null, Some("PATTERN_DATA_NOT_STORED"))
    } else {
        field("extracted", json!(pattern_ids.len()), None)
    };
    let pattern_names = if !known_pattern_build(version.as_deref()) {
        field(
            "unsupported",
            Value::Null,
            Some("PATTERN_NAMES_UNVERIFIED_BUILD"),
        )
    } else if pattern_ids.is_empty() {
        field("unavailable", Value::Null, Some("PATTERN_DATA_NOT_STORED"))
    } else {
        let items = pattern_ids
            .iter()
            .map(|id| {
                let name = match pattern_names.get(id) {
                    Some(name) => field("extracted", json!(name), None),
                    None => field("unavailable", Value::Null, Some("PATTERN_NAME_NOT_STORED")),
                };
                json!({"patternId":id,"name":name})
            })
            .collect::<Vec<_>>();
        if pattern_names.len() == pattern_ids.len() {
            field("extracted", json!(items), None)
        } else {
            json!({"status":"unavailable","reason":"PATTERN_NAME_NOT_STORED","items":items})
        }
    };
    let (playlist_clips, playlist_end_tick) = match playlist_pattern_clips(
        version.as_deref(),
        playlist_payload.as_deref(),
        multiple_playlist_payloads,
        &pattern_ids,
    ) {
        Ok(value) => value,
        Err(code) => return failed(code),
    };
    let playlist_end = match playlist_end_tick {
        Some(value) => field("extracted", json!(value), None),
        None if playlist_clips["status"] == "extracted" => field(
            "unavailable",
            Value::Null,
            Some("NO_PLAYLIST_PATTERN_CLIPS"),
        ),
        None => field(
            playlist_clips["status"].as_str().expect("field status"),
            Value::Null,
            playlist_clips["reason"].as_str(),
        ),
    };
    let playlist_nominal_seconds = match playlist_end_tick {
        None => playlist_end.clone(),
        Some(_) if header_ppq != 96 => field("unsupported", Value::Null, Some("PPQ_UNVERIFIED")),
        Some(_)
            if timing_numerator != Some(4)
                || timing_denominator != Some(4)
                || multiple_timing_events =>
        {
            field("unsupported", Value::Null, Some("METER_UNVERIFIED"))
        }
        Some(end_tick) => match tempo {
            Some(bpm) => json!({
                "status":"inferred",
                "value":f64::from(end_tick) / 96.0 * 60.0 / bpm,
                "method":"constant-base-tempo-over-pattern-clips",
                "confidence":"low",
                "assumptions":["tempo remains at base BPM", "only verified pattern clips define span"]
            }),
            None => field("unavailable", Value::Null, Some("BASE_TEMPO_ABSENT")),
        },
    };
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
        "patternCount":pattern_count,
        "patternNames":pattern_names,
        "playlistPatternClips":playlist_clips,
        "playlistPatternEndTick":playlist_end,
        "playlistPatternNominalSeconds":playlist_nominal_seconds,
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
