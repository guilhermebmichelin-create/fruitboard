use crate::{Channel, ChannelType, GeneratorEvent};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub const BUILD: &str = "26.1.0.5530";
const POINT_BYTES: usize = 24;
const POINTS_OFFSET: usize = 21;
pub const MAX_POINTS_PER_OBJECT: u64 =
    ((crate::MAX_INTERPRETED_EVENT_BYTES - POINTS_OFFSET - CURVE_SUFFIX.len()) / POINT_BYTES)
        as u64;
pub const MAX_POINTS: u64 = crate::MAX_FILE_BYTES / POINT_BYTES as u64;
pub const UNVERIFIED_BUILD: &str = "AUTOMATION_COUNTS_UNVERIFIED_BUILD";
pub const UNVERIFIED_LAYOUT: &str = "AUTOMATION_COUNTS_UNVERIFIED_LAYOUT";

const CURVE_PREFIX: &[u8] = &[
    0x01, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
    0x00,
];
const CURVE_SUFFIX: &[u8] = &[
    0x01, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0x80, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x80, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0x3f,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfb, 0xb2, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[derive(Clone, Copy, Default)]
pub(crate) enum Curve {
    #[default]
    Missing,
    Verified(u64),
    Unverified,
}

impl Curve {
    pub(crate) fn observe(&mut self, channel_type: ChannelType, payload: &[u8]) {
        *self = if matches!(self, Self::Missing) && channel_type == ChannelType::AutomationClip {
            saved_curve_point_count(payload).map_or(Self::Unverified, Self::Verified)
        } else {
            Self::Unverified
        };
    }
}

pub(super) fn summary(
    version: Option<&str>,
    channels: &[Channel],
    rack_complete: bool,
    empty_playlist: bool,
) -> Value {
    let unavailable = |reason| crate::field("unavailable", Value::Null, Some(reason));
    if version != Some(BUILD) {
        return unavailable(UNVERIFIED_BUILD);
    }
    if !rack_complete {
        return unavailable(UNVERIFIED_LAYOUT);
    }
    let mut ids = BTreeSet::new();
    let (mut objects, mut points) = (0, 0);
    for channel in channels {
        if !ids.insert(channel.id) {
            return unavailable(UNVERIFIED_LAYOUT);
        }
        match (channel.channel_type, channel.automation_curve) {
            (ChannelType::AutomationClip, Curve::Verified(count)) if matches!(&channel.generator_event, GeneratorEvent::Name(name) if name.is_empty()) =>
            {
                objects += 1;
                points += count;
            }
            (
                ChannelType::Sampler | ChannelType::NativeGenerator | ChannelType::AudioClip,
                Curve::Missing,
            ) => {}
            _ => return unavailable(UNVERIFIED_LAYOUT),
        }
    }
    // A zero count is qualified for the empty, one-Sampler control layout.
    if objects == 0
        && !(empty_playlist
            && matches!(channels, [channel] if channel.channel_type == ChannelType::Sampler
                && channel.sample.is_none()
                && matches!(&channel.generator_event, GeneratorEvent::Name(name) if name.is_empty())))
    {
        return unavailable(UNVERIFIED_LAYOUT);
    }
    crate::field(
        "extracted",
        json!({"objectCount":objects,"pointCount":points}),
        None,
    )
}

pub(crate) fn saved_curve_point_count(payload: &[u8]) -> Option<u64> {
    let count = u32::from_le_bytes(payload.get(17..21)?.try_into().ok()?);
    if count == 0 {
        return None;
    }
    let suffix_offset = (count as usize)
        .checked_mul(POINT_BYTES)?
        .checked_add(POINTS_OFFSET)?;
    (payload.starts_with(CURVE_PREFIX) && payload.get(suffix_offset..) == Some(CURVE_SUFFIX))
        .then_some(u64::from(count))
}
