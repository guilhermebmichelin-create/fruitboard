//! Optional, explicitly selected note data. No automatic logging/serialization.
use super::{
    NoteRecordCount, PatternNoteCounts, PatternNoteReason, SavedPatterns, ValidationError,
};
use crate::pattern_note_sequences as grammar;
use serde_json::Value;

#[derive(Clone, Copy)]
pub(super) struct NoteSequenceLimits {
    pub per_pattern: usize,
    pub total: usize,
    pub bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteSequenceReason {
    Counts(PatternNoteReason),
    UnverifiedBuild,
    LimitExceeded,
    PpqUnverified,
    ChannelsUnverified,
    RecordUnqualified,
}
impl NoteSequenceReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Counts(reason) => reason.as_str(),
            Self::UnverifiedBuild => grammar::UNVERIFIED_BUILD,
            Self::LimitExceeded => grammar::DETAIL_LIMIT,
            Self::PpqUnverified => grammar::PPQ_UNVERIFIED,
            Self::ChannelsUnverified => grammar::CHANNELS_UNVERIFIED,
            Self::RecordUnqualified => grammar::RECORD_UNQUALIFIED,
        }
    }
}

/// Numeric saved fields only. Neither saved_key nor stored_velocity_byte is
/// advertised as MIDI, and no seconds, pitch names or velocity % are inferred.
pub struct SavedNoteRecord {
    ordinal: u16,
    channel_index: u16,
    saved_key: u16,
    start_tick: u32,
    duration_tick: u32,
    stored_velocity_byte: u8,
}
impl SavedNoteRecord {
    pub fn ordinal(&self) -> u16 {
        self.ordinal
    }
    pub fn channel_index(&self) -> u16 {
        self.channel_index
    }
    pub fn saved_key(&self) -> u16 {
        self.saved_key
    }
    pub fn start_tick(&self) -> u32 {
        self.start_tick
    }
    pub fn duration_tick(&self) -> u32 {
        self.duration_tick
    }
    pub fn stored_velocity_byte(&self) -> u8 {
        self.stored_velocity_byte
    }
}
pub enum NoteSequence {
    Extracted(Vec<SavedNoteRecord>),
    Unavailable,
    Unsupported(NoteSequenceReason),
}
pub struct SavedPatternNoteSequence {
    id: u16,
    sequence: NoteSequence,
}
impl SavedPatternNoteSequence {
    pub fn id(&self) -> u16 {
        self.id
    }
    pub fn sequence(&self) -> &NoteSequence {
        &self.sequence
    }
}
pub enum PatternNoteSequences {
    NotAdvertised,
    Unavailable(NoteSequenceReason),
    Unsupported(NoteSequenceReason),
    /// Pattern-local ticks at the independently qualified 96 PPQ only.
    Entries(Vec<SavedPatternNoteSequence>),
}

fn shape(value: &Value, keys: &[&str]) -> Result<(), ValidationError> {
    let object = value.as_object().ok_or(ValidationError::InvalidReply)?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(ValidationError::InvalidReply);
    }
    Ok(())
}

pub(super) fn terminal(value: &Value, status: &str, reason: &str) -> Result<(), ValidationError> {
    shape(value, &["status", "reason"])?;
    if value["status"] != status || value["reason"] != reason {
        return Err(ValidationError::InvalidReply);
    }
    Ok(())
}

pub(super) fn validate(
    value: &Value,
    limits: Option<NoteSequenceLimits>,
    build: &str,
    patterns: &SavedPatterns,
    counts: &PatternNoteCounts,
    channels: usize,
) -> Result<PatternNoteSequences, ValidationError> {
    let Some(limits) = limits else {
        return Ok(PatternNoteSequences::NotAdvertised);
    };
    let invalid = ValidationError::InvalidReply;
    if serde_json::to_vec(value).map_err(|_| invalid)?.len() > limits.bytes {
        return Err(ValidationError::LimitExceeded);
    }
    if value["coverage"] != grammar::COVERAGE {
        return Err(invalid);
    }
    if value.get("patterns").is_none() {
        shape(value, &["status", "reason", "coverage"])?;
        let status = value["status"].as_str().ok_or(invalid)?;
        let reason = value["reason"].as_str().ok_or(invalid)?;
        if build != "26.1.0.5530" {
            return if status == "unsupported" && reason == grammar::UNVERIFIED_BUILD {
                Ok(PatternNoteSequences::Unsupported(
                    NoteSequenceReason::UnverifiedBuild,
                ))
            } else {
                Err(invalid)
            };
        }
        let reason = match (status, reason) {
            ("unavailable", "PATTERN_DATA_NOT_STORED")
                if matches!(patterns, SavedPatterns::Unavailable)
                    && matches!(
                        counts,
                        PatternNoteCounts::Unavailable(PatternNoteReason::DataNotStored)
                    ) =>
            {
                return Ok(PatternNoteSequences::Unavailable(
                    NoteSequenceReason::Counts(PatternNoteReason::DataNotStored),
                ));
            }
            ("unsupported", "PATTERN_NOTE_BINDING_UNVERIFIED")
                if matches!(
                    counts,
                    PatternNoteCounts::Unsupported(PatternNoteReason::BindingUnverified)
                ) =>
            {
                NoteSequenceReason::Counts(PatternNoteReason::BindingUnverified)
            }
            ("unsupported", grammar::DETAIL_LIMIT) => NoteSequenceReason::LimitExceeded,
            ("unsupported", grammar::PPQ_UNVERIFIED)
                if matches!(counts, PatternNoteCounts::Entries(_)) =>
            {
                NoteSequenceReason::PpqUnverified
            }
            ("unsupported", grammar::CHANNELS_UNVERIFIED)
                if matches!(counts, PatternNoteCounts::Entries(_)) =>
            {
                NoteSequenceReason::ChannelsUnverified
            }
            _ => return Err(invalid),
        };
        return Ok(PatternNoteSequences::Unsupported(reason));
    }
    shape(value, &["coverage", "coordinateSystem", "ppq", "patterns"])?;
    if build != "26.1.0.5530"
        || value["coordinateSystem"] != "pattern-local-ticks"
        || value["ppq"].as_u64() != Some(96)
        || !(1..=2).contains(&channels)
    {
        return Err(invalid);
    }
    let SavedPatterns::Entries(patterns) = patterns else {
        return Err(invalid);
    };
    let PatternNoteCounts::Entries(counts) = counts else {
        return Err(invalid);
    };
    let items = value["patterns"].as_array().ok_or(invalid)?;
    if items.is_empty() || items.len() != patterns.len() || items.len() != counts.len() {
        return Err(invalid);
    }
    let mut total = 0usize;
    let mut selected = Vec::with_capacity(items.len());
    for ((item, pattern), count) in items.iter().zip(patterns).zip(counts) {
        if item["patternId"].as_u64() != Some(u64::from(pattern.id())) || pattern.id() != count.id()
        {
            return Err(invalid);
        }
        let sequence = match count.count() {
            NoteRecordCount::Unavailable => {
                shape(item, &["patternId", "status", "reason"])?;
                if item["status"] != "unavailable" || item["reason"] != "PATTERN_NOTES_NOT_STORED" {
                    return Err(invalid);
                }
                NoteSequence::Unavailable
            }
            NoteRecordCount::Unsupported(reason) => {
                shape(item, &["patternId", "status", "reason"])?;
                if item["status"] != "unsupported" || item["reason"] != reason.as_str() {
                    return Err(invalid);
                }
                NoteSequence::Unsupported(NoteSequenceReason::Counts(*reason))
            }
            NoteRecordCount::Extracted(number) => {
                total = total
                    .checked_add(*number as usize)
                    .filter(|n| *n <= limits.total)
                    .ok_or(ValidationError::LimitExceeded)?;
                if item["status"] == "unsupported" {
                    shape(item, &["patternId", "status", "reason"])?;
                    let reason = if *number as usize > limits.per_pattern {
                        NoteSequenceReason::LimitExceeded
                    } else {
                        NoteSequenceReason::RecordUnqualified
                    };
                    if item["reason"] != reason.as_str() {
                        return Err(invalid);
                    }
                    NoteSequence::Unsupported(reason)
                } else {
                    shape(item, &["patternId", "status", "records"])?;
                    if item["status"] != "extracted" || *number as usize > limits.per_pattern {
                        return Err(invalid);
                    }
                    let records = item["records"].as_array().ok_or(invalid)?;
                    if records.len() != *number as usize {
                        return Err(invalid);
                    }
                    let records = records
                        .iter()
                        .enumerate()
                        .map(|(ordinal, note)| {
                            shape(
                                note,
                                &[
                                    "ordinal",
                                    "channelIndex",
                                    "savedKey",
                                    "startTick",
                                    "durationTick",
                                    "storedVelocityByte",
                                ],
                            )?;
                            let integer = |key: &str| note[key].as_u64().ok_or(invalid);
                            let channel = integer("channelIndex")?;
                            let key = integer("savedKey")?;
                            let start = integer("startTick")?;
                            let duration = integer("durationTick")?;
                            let velocity = integer("storedVelocityByte")?;
                            if integer("ordinal")? != ordinal as u64
                                || channel >= channels as u64
                                || !grammar::qualified_values(key, start, duration, velocity)
                                || start
                                    .checked_add(duration)
                                    .is_none_or(|end| end > u32::MAX.into())
                            {
                                return Err(invalid);
                            }
                            Ok(SavedNoteRecord {
                                ordinal: ordinal as u16,
                                channel_index: channel as u16,
                                saved_key: key as u16,
                                start_tick: start as u32,
                                duration_tick: duration as u32,
                                stored_velocity_byte: velocity as u8,
                            })
                        })
                        .collect::<Result<Vec<_>, ValidationError>>()?;
                    NoteSequence::Extracted(records)
                }
            }
        };
        selected.push(SavedPatternNoteSequence {
            id: pattern.id(),
            sequence,
        });
    }
    Ok(PatternNoteSequences::Entries(selected))
}
