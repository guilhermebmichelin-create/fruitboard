//! Explicit local display selection; never raw saved JSON or new authority.
use fruitboard_flp_parser::validation::{
    NoteSequence, NoteSequenceReason, PatternNoteSequences, validate_stored_pattern_note_counts,
    validate_stored_pattern_note_sequences, validate_stored_patterns,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(super) enum Summary {
    #[serde(rename_all = "camelCase")]
    Available {
        coverage: &'static str,
        coordinate_system: &'static str,
        ppq: u16,
        items: Vec<Pattern>,
    },
    Unavailable {
        reason: &'static str,
    },
    Unsupported {
        reason: &'static str,
    },
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Pattern {
    pattern_id: u16,
    sequence: Sequence,
}
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Sequence {
    Extracted { records: Vec<Record> },
    Unavailable { reason: &'static str },
    Unsupported { reason: &'static str },
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    ordinal: u16,
    channel_index: u16,
    saved_key: u16,
    start_tick: u32,
    duration_tick: u32,
    stored_velocity_byte: u8,
}
fn reason(value: NoteSequenceReason) -> &'static str {
    match value {
        NoteSequenceReason::Counts(code) => super::pattern_notes::reason(code),
        NoteSequenceReason::UnverifiedBuild => "unverified_build",
        NoteSequenceReason::LimitExceeded => "limit_exceeded",
        NoteSequenceReason::PpqUnverified => "unverified_ppq",
        NoteSequenceReason::ChannelsUnverified => "unverified_channels",
        NoteSequenceReason::RecordUnqualified => "unqualified_record",
    }
}
pub(super) fn project(value: &Value, build: &str, channels: usize) -> Result<Summary, ()> {
    let Some(raw) = value.get("patternNoteSequences") else {
        return Ok(Summary::Unsupported {
            reason: "not_saved",
        });
    };
    let patterns = validate_stored_patterns(&value["patterns"], build).map_err(|_| ())?;
    let counts = validate_stored_pattern_note_counts(&value["patternNoteCounts"], build, &patterns)
        .map_err(|_| ())?;
    let selected = validate_stored_pattern_note_sequences(raw, build, &patterns, &counts, channels)
        .map_err(|_| ())?;
    Ok(match selected {
        PatternNoteSequences::NotAdvertised => return Err(()),
        PatternNoteSequences::Unavailable(code) => Summary::Unavailable {
            reason: reason(code),
        },
        PatternNoteSequences::Unsupported(code) => Summary::Unsupported {
            reason: reason(code),
        },
        PatternNoteSequences::Entries(items) => Summary::Available {
            coverage: "stored-pattern-note-records",
            coordinate_system: "pattern-local-ticks",
            ppq: 96,
            items: items
                .iter()
                .map(|item| Pattern {
                    pattern_id: item.id(),
                    sequence: match item.sequence() {
                        NoteSequence::Extracted(records) => Sequence::Extracted {
                            records: records
                                .iter()
                                .map(|note| Record {
                                    ordinal: note.ordinal(),
                                    channel_index: note.channel_index(),
                                    saved_key: note.saved_key(),
                                    start_tick: note.start_tick(),
                                    duration_tick: note.duration_tick(),
                                    stored_velocity_byte: note.stored_velocity_byte(),
                                })
                                .collect(),
                        },
                        NoteSequence::Unavailable => Sequence::Unavailable {
                            reason: "no_stored_notes",
                        },
                        NoteSequence::Unsupported(code) => Sequence::Unsupported {
                            reason: reason(*code),
                        },
                    },
                })
                .collect(),
        },
    })
}
