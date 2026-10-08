//! Selected counts and fixed states only, validated against the same snapshot.
use fruitboard_flp_parser::validation::{
    NoteRecordCount, PatternNoteCounts, PatternNoteReason, PatternSupport, SavedPatterns,
    validate_stored_pattern_note_counts, validate_stored_patterns,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(super) enum Summary {
    Available { items: Vec<Pattern> },
    Unavailable { reason: &'static str },
    Unsupported { reason: &'static str },
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Pattern {
    pattern_id: u16,
    note_count: Count,
}
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Count {
    Extracted { value: u32 },
    Unavailable { reason: &'static str },
    Unsupported { reason: &'static str },
}
fn reason(value: PatternNoteReason) -> &'static str {
    match value {
        PatternNoteReason::DataNotStored => "no_stored_patterns",
        PatternNoteReason::NotesNotStored => "no_stored_notes",
        PatternNoteReason::UnverifiedBuild => "unverified_build",
        PatternNoteReason::LayoutUnverified => "unverified_layout",
        PatternNoteReason::MultiplePayloads => "multiple_payloads",
        PatternNoteReason::BindingUnverified => "unverified_binding",
        PatternNoteReason::LimitExceeded => "limit_exceeded",
    }
}
pub(super) fn project(value: &Value, build: &str) -> Result<Summary, ()> {
    let Some(notes) = value.get("patternNoteCounts") else {
        return Ok(Summary::Unsupported {
            reason: "not_saved",
        });
    };
    let patterns = value
        .get("patterns")
        .map(|value| validate_stored_patterns(value, build))
        .transpose()
        .map_err(|_| ())?
        .unwrap_or(SavedPatterns::Unsupported(PatternSupport::NotAdvertised));
    Ok(
        match validate_stored_pattern_note_counts(notes, build, &patterns).map_err(|_| ())? {
            PatternNoteCounts::NotAdvertised => Summary::Unsupported {
                reason: "not_advertised",
            },
            PatternNoteCounts::Unavailable(code) => Summary::Unavailable {
                reason: reason(code),
            },
            PatternNoteCounts::Unsupported(code) => Summary::Unsupported {
                reason: reason(code),
            },
            PatternNoteCounts::Entries(items) => Summary::Available {
                items: items
                    .iter()
                    .map(|item| Pattern {
                        pattern_id: item.id(),
                        note_count: match item.count() {
                            NoteRecordCount::Extracted(value) => Count::Extracted { value: *value },
                            NoteRecordCount::Unavailable => Count::Unavailable {
                                reason: "no_stored_notes",
                            },
                            NoteRecordCount::Unsupported(code) => Count::Unsupported {
                                reason: reason(*code),
                            },
                        },
                    })
                    .collect(),
            },
        },
    )
}
