//! F24–F30's ordinary-record subset only. Raw saved values are not MIDI values.
use crate::{Channel, ChannelType, MAX_DETAILED_NOTES_PER_PATTERN, MAX_NOTE_SEQUENCE_BYTES};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const COVERAGE: &str = "stored-pattern-note-records";
pub(crate) const UNVERIFIED_BUILD: &str = "PATTERN_NOTE_SEQUENCES_UNVERIFIED_BUILD";
pub(crate) const DETAIL_LIMIT: &str = "PATTERN_NOTE_SEQUENCE_LIMIT_EXCEEDED";
pub(crate) const PPQ_UNVERIFIED: &str = "PATTERN_NOTE_SEQUENCE_PPQ_UNVERIFIED";
pub(crate) const CHANNELS_UNVERIFIED: &str = "PATTERN_NOTE_SEQUENCE_CHANNELS_UNVERIFIED";
pub(crate) const RECORD_UNQUALIFIED: &str = "PATTERN_NOTE_RECORD_UNQUALIFIED";

pub(crate) fn absent(status: &str, reason: &str) -> Value {
    json!({"status":status,"reason":reason,"coverage":COVERAGE})
}

/// Until further genuine-save qualification, accept only the observed raw
/// values. This deliberately does not infer a general key/velocity conversion
/// or support arbitrary timing, articulation or channel layouts.
pub(crate) fn qualified_values(key: u64, start: u64, duration: u64, velocity: u64) -> bool {
    matches!(key, 60 | 64)
        && matches!(start, 0 | 48)
        && matches!(duration, 48 | 96)
        && matches!(velocity, 74 | 89)
}

fn record(data: &[u8], ordinal: usize, channels: &[Channel]) -> Option<Value> {
    let saved_channel = u16::from_le_bytes(data[6..8].try_into().ok()?);
    // Resolve declarations within this save; never use the raw word as an index.
    let index = channels
        .iter()
        .position(|channel| channel.id == saved_channel)?;
    if channels[index].channel_type != ChannelType::Sampler
        || data[4..6] != [0, 64]
        || data[14..21] != [0, 0, 120, 0, 64, 0, 64]
        || data[22..24] != [128, 128]
    {
        return None;
    }
    let start = u32::from_le_bytes(data[0..4].try_into().ok()?);
    let duration = u32::from_le_bytes(data[8..12].try_into().ok()?);
    let key = u16::from_le_bytes(data[12..14].try_into().ok()?);
    let velocity = data[21];
    if !qualified_values(key.into(), start.into(), duration.into(), velocity.into()) {
        return None;
    }
    start.checked_add(duration)?;
    Some(
        json!({"ordinal":ordinal,"channelIndex":index,"savedKey":key,
        "startTick":start,"durationTick":duration,"storedVelocityByte":velocity}),
    )
}

pub(super) fn summary(
    build: Option<&str>,
    ppq: u16,
    counts: &Value,
    payloads: &BTreeMap<u16, &[u8]>,
    channels: &[Channel],
    detail_limit: bool,
) -> Value {
    if build != Some("26.1.0.5530") {
        return absent("unsupported", UNVERIFIED_BUILD);
    }
    if detail_limit {
        return absent("unsupported", DETAIL_LIMIT);
    }
    let items = counts.get("value").or_else(|| counts.get("items"));
    let Some(items) = items.and_then(Value::as_array) else {
        return absent(
            counts["status"].as_str().unwrap_or("unsupported"),
            counts["reason"].as_str().unwrap_or(DETAIL_LIMIT),
        );
    };
    if ppq != 96 {
        return absent("unsupported", PPQ_UNVERIFIED);
    }
    // Sparse/deleted IDs and more than two channels are not independently
    // qualified. Check the set, while still mapping each declaration explicitly.
    let ids = channels
        .iter()
        .map(|channel| channel.id)
        .collect::<BTreeSet<_>>();
    if !(1..=2).contains(&channels.len()) || ids != (0..channels.len() as u16).collect() {
        return absent("unsupported", CHANNELS_UNVERIFIED);
    }
    let patterns = items
        .iter()
        .map(|item| {
            let id = item["patternId"].as_u64().expect("internal pattern ID") as u16;
            let count = &item["noteCount"];
            if count["status"] != "extracted" {
                return json!({"patternId":id,"status":count["status"],"reason":count["reason"]});
            }
            if count["value"].as_u64().unwrap_or(u64::MAX) > MAX_DETAILED_NOTES_PER_PATTERN as u64 {
                return json!({"patternId":id,"status":"unsupported","reason":DETAIL_LIMIT});
            }
            let records = payloads.get(&id).and_then(|payload| {
                payload
                    .chunks_exact(24)
                    .enumerate()
                    .map(|(ordinal, bytes)| record(bytes, ordinal, channels))
                    .collect::<Option<Vec<_>>>()
            });
            match records {
                Some(records) => json!({"patternId":id,"status":"extracted","records":records}),
                None => json!({"patternId":id,"status":"unsupported","reason":RECORD_UNQUALIFIED}),
            }
        })
        .collect::<Vec<_>>();
    let result = json!({"coverage":COVERAGE,"coordinateSystem":"pattern-local-ticks", "ppq":96,"patterns":patterns});
    if serde_json::to_vec(&result).map_or(true, |bytes| bytes.len() > MAX_NOTE_SEQUENCE_BYTES) {
        absent("unsupported", DETAIL_LIMIT)
    } else {
        result
    }
}
