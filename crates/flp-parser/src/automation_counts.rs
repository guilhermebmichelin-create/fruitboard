use serde_json::{Value, json};

pub const BUILD: &str = "26.1.0.5530";
pub const MAX_OBJECTS: u64 = 2;
pub const MAX_POINTS_PER_OBJECT: u64 = 4;
pub const MAX_POINTS: u64 = 6;
pub const UNVERIFIED_BUILD: &str = "AUTOMATION_COUNTS_UNVERIFIED_BUILD";
pub const UNVERIFIED_LAYOUT: &str = "AUTOMATION_COUNTS_UNVERIFIED_LAYOUT";

const SAMPLER_EVENTS: &[u8] = &[
    64, 21, 201, 212, 155, 128, 41, 0, 209, 138, 139, 89, 97, 48, 69, 86, 71, 83, 74, 75, 76, 85,
    131, 70, 104, 50, 219, 229, 221, 215, 132, 144, 145, 32, 251, 228, 228, 218, 218, 218, 218,
    218, 143, 20, 170, 51,
];
const CLIP_EVENTS: &[u8] = &[
    64, 21, 201, 212, 203, 155, 128, 41, 0, 209, 138, 139, 89, 97, 48, 69, 86, 71, 83, 74, 75, 76,
    85, 131, 70, 104, 50, 219, 229, 221, 215, 132, 144, 145, 234, 32, 228, 228, 218, 218, 218, 218,
    218, 143, 20, 170, 51,
];
const LAST_CHANNEL_EVENTS: &[u8] = &[99, 241, 36];
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

#[derive(Default)]
struct Channel {
    event_index: usize,
    points: Option<u64>,
}

/// Tracks only the demonstrated channel section. The ordinary parser owns
/// framing and bounds. An unrecognized section disables this optional field.
pub struct AutomationCounts {
    declared: usize,
    ids: [u16; 3],
    seen: usize,
    channel: Option<Channel>,
    version_seen: bool,
    playlist_seen: bool,
    points: u64,
    active: bool,
}

impl AutomationCounts {
    pub fn new(format: u16, ppq: u16, channels: u16) -> Self {
        Self {
            declared: usize::from(channels),
            ids: [0; 3],
            seen: 0,
            channel: None,
            version_seen: false,
            playlist_seen: false,
            points: 0,
            active: format == 0 && ppq == 96 && (1..=3).contains(&channels),
        }
    }

    pub fn observe(&mut self, id: u8, payload: &[u8]) {
        if self.active {
            self.active = self.observe_layout(id, payload).is_some();
        }
    }

    fn observe_layout(&mut self, id: u8, payload: &[u8]) -> Option<()> {
        match id {
            199 => {
                if self.version_seen || self.seen != 0 || payload != b"26.1.0.5530\0" {
                    return None;
                }
                self.version_seen = true;
                return Some(());
            }
            172 if payload.len() != 3 => return None,
            233 => {
                if self.playlist_seen || self.channel.is_none() || self.seen != self.declared {
                    return None;
                }
                self.finish_channel()?;
                self.playlist_seen = true;
                return (self.seen != 1 || payload.is_empty()).then_some(());
            }
            65 | 98 | 100 | 236 => return self.playlist_seen.then_some(()),
            64 => {
                self.finish_channel()?;
                let saved_id = u16::from_le_bytes(payload.try_into().ok()?);
                if self.playlist_seen
                    || self.seen >= self.declared
                    || self.ids[..self.seen].contains(&saved_id)
                {
                    return None;
                }
                self.ids[self.seen] = saved_id;
                self.seen += 1;
                self.channel = Some(Channel::default());
            }
            _ => {}
        }

        let Some(channel) = &mut self.channel else {
            return (!matches!(id, 21 | 201 | 234)).then_some(());
        };
        let is_sampler = self.seen == 1;
        let events = if is_sampler {
            SAMPLER_EVENTS
        } else {
            CLIP_EVENTS
        };
        let expected = if channel.event_index < events.len() {
            events.get(channel.event_index)
        } else if self.seen == self.declared {
            LAST_CHANNEL_EVENTS.get(channel.event_index - events.len())
        } else {
            None
        };
        if expected != Some(&id)
            || (id == 21 && payload != [if is_sampler { 0 } else { 5 }])
            || (id == 201 && payload != [0, 0])
        {
            return None;
        }
        if id == 234 {
            channel.points =
                saved_curve_point_count(payload).filter(|&n| n <= MAX_POINTS_PER_OBJECT);
            channel.points?;
        }
        channel.event_index += 1;
        Some(())
    }

    fn finish_channel(&mut self) -> Option<()> {
        let Some(channel) = self.channel.take() else {
            return Some(());
        };
        let is_sampler = self.seen == 1;
        let expected_length = if is_sampler {
            SAMPLER_EVENTS.len()
        } else {
            CLIP_EVENTS.len()
        } + if self.seen == self.declared {
            LAST_CHANNEL_EVENTS.len()
        } else {
            0
        };
        if channel.event_index != expected_length {
            return None;
        }
        if !is_sampler {
            self.points = self.points.checked_add(channel.points?)?;
            if self.points > MAX_POINTS {
                return None;
            }
        }
        Some(())
    }

    pub fn summary(&self, version: Option<&str>) -> Value {
        if version != Some(BUILD) {
            return crate::field("unavailable", Value::Null, Some(UNVERIFIED_BUILD));
        }
        if !self.active || !self.playlist_seen {
            return crate::field("unavailable", Value::Null, Some(UNVERIFIED_LAYOUT));
        }
        crate::field(
            "extracted",
            json!({"objectCount": self.seen - 1, "pointCount": self.points}),
            None,
        )
    }
}

// Recognizing a saved curve for playlist classification does not qualify
// aggregate automation counts for that layout.
pub(crate) fn saved_curve_point_count(payload: &[u8]) -> Option<u64> {
    let count = u32::from_le_bytes(payload.get(17..21)?.try_into().ok()?);
    if count == 0 {
        return None;
    }
    let suffix_offset = (count as usize).checked_mul(24)?.checked_add(21)?;
    (payload.starts_with(CURVE_PREFIX) && payload.get(suffix_offset..) == Some(CURVE_SUFFIX))
        .then_some(u64::from(count))
}
