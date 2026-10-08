//! Typed counts and fixed states only; no raw note data or automatic projection.
use super::{SavedPatterns, ValidationError};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternNoteReason {
    DataNotStored,
    NotesNotStored,
    UnverifiedBuild,
    LayoutUnverified,
    MultiplePayloads,
    BindingUnverified,
    LimitExceeded,
}
impl PatternNoteReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DataNotStored => "PATTERN_DATA_NOT_STORED",
            Self::NotesNotStored => "PATTERN_NOTES_NOT_STORED",
            Self::UnverifiedBuild => "PATTERN_NOTES_UNVERIFIED_BUILD",
            Self::LayoutUnverified => "PATTERN_NOTE_LAYOUT_UNVERIFIED",
            Self::MultiplePayloads => "MULTIPLE_PATTERN_NOTE_PAYLOADS",
            Self::BindingUnverified => "PATTERN_NOTE_BINDING_UNVERIFIED",
            Self::LimitExceeded => "PATTERN_NOTE_LIMIT_EXCEEDED",
        }
    }
}
pub enum NoteRecordCount {
    Extracted(u32),
    Unavailable,
    Unsupported(PatternNoteReason),
}
pub struct SavedPatternNoteCount {
    id: u16,
    count: NoteRecordCount,
}
impl SavedPatternNoteCount {
    pub fn id(&self) -> u16 {
        self.id
    }
    pub fn count(&self) -> &NoteRecordCount {
        &self.count
    }
}
pub enum PatternNoteCounts {
    NotAdvertised,
    Unavailable(PatternNoteReason),
    Unsupported(PatternNoteReason),
    Entries(Vec<SavedPatternNoteCount>),
}
fn shape(value: &Value, keys: &[&str]) -> Result<(), ValidationError> {
    let object = value.as_object().ok_or(ValidationError::InvalidReply)?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(ValidationError::InvalidReply);
    }
    Ok(())
}

/// Revalidate a selected immutable projection against its saved patterns.
/// Fixed current ceilings apply; this grants no parser or filesystem authority.
pub fn validate_stored_pattern_note_counts(
    value: &Value,
    build: &str,
    patterns: &SavedPatterns,
) -> Result<PatternNoteCounts, ValidationError> {
    if value["status"] == "unsupported" && value["reason"] == "PATTERN_NOTES_NOT_ADVERTISED" {
        shape(value, &["status", "reason", "coverage"])?;
        if value["coverage"] != crate::pattern_notes::COVERAGE {
            return Err(ValidationError::InvalidReply);
        }
        return Ok(PatternNoteCounts::NotAdvertised);
    }
    validate(
        value,
        Some((
            crate::MAX_NOTE_RECORDS_PER_PATTERN as u64,
            crate::MAX_NOTE_RECORDS_TOTAL as u64,
        )),
        build,
        patterns,
    )
}
pub(super) fn validate(
    value: &Value,
    limits: Option<(u64, u64)>,
    build: &str,
    patterns: &SavedPatterns,
) -> Result<PatternNoteCounts, ValidationError> {
    let Some((per_pattern, total_limit)) = limits else {
        return Ok(PatternNoteCounts::NotAdvertised);
    };
    let invalid = ValidationError::InvalidReply;
    if value["coverage"] != crate::pattern_notes::COVERAGE {
        return Err(invalid);
    }
    let status = value["status"].as_str().ok_or(invalid)?;
    if build != "26.1.0.5530" {
        shape(value, &["status", "reason", "coverage"])?;
        return if status == "unsupported" && value["reason"] == "PATTERN_NOTES_UNVERIFIED_BUILD" {
            Ok(PatternNoteCounts::Unsupported(
                PatternNoteReason::UnverifiedBuild,
            ))
        } else {
            Err(invalid)
        };
    }
    if value.get("items").is_none() && status != "extracted" {
        shape(value, &["status", "reason", "coverage"])?;
        return match (status, value["reason"].as_str()) {
            ("unavailable", Some("PATTERN_DATA_NOT_STORED"))
                if matches!(patterns, SavedPatterns::Unavailable) =>
            {
                Ok(PatternNoteCounts::Unavailable(
                    PatternNoteReason::DataNotStored,
                ))
            }
            ("unsupported", Some("PATTERN_NOTE_BINDING_UNVERIFIED")) => Ok(
                PatternNoteCounts::Unsupported(PatternNoteReason::BindingUnverified),
            ),
            ("unsupported", Some("PATTERN_NOTE_LIMIT_EXCEEDED")) => Ok(
                PatternNoteCounts::Unsupported(PatternNoteReason::LimitExceeded),
            ),
            _ => Err(invalid),
        };
    }
    let items = if status == "extracted" {
        shape(value, &["status", "coverage", "value"])?;
        &value["value"]
    } else {
        shape(value, &["status", "reason", "coverage", "items"])?;
        if !matches!(status, "unavailable" | "unsupported")
            || value["reason"] != "PATTERN_NOTE_COUNTS_INCOMPLETE"
        {
            return Err(invalid);
        }
        &value["items"]
    }
    .as_array()
    .ok_or(invalid)?;
    let SavedPatterns::Entries(patterns) = patterns else {
        return Err(invalid);
    };
    if items.len() != patterns.len() || items.is_empty() {
        return Err(invalid);
    }
    let mut total = 0u64;
    let mut missing = false;
    let mut unsupported = false;
    let mut result = Vec::with_capacity(items.len());
    for (item, pattern) in items.iter().zip(patterns) {
        shape(item, &["patternId", "noteCount"])?;
        if item["patternId"].as_u64() != Some(u64::from(pattern.id())) {
            return Err(invalid);
        }
        let count = &item["noteCount"];
        let count = match count["status"].as_str() {
            Some("extracted") => {
                shape(count, &["status", "value"])?;
                // No explicit-zero saved payload has been qualified.
                let number = count["value"]
                    .as_u64()
                    .filter(|n| (1..=per_pattern).contains(n))
                    .ok_or(invalid)?;
                total = total
                    .checked_add(number)
                    .filter(|n| *n <= total_limit)
                    .ok_or(ValidationError::LimitExceeded)?;
                NoteRecordCount::Extracted(number as u32)
            }
            Some("unavailable") => {
                shape(count, &["status", "reason"])?;
                if count["reason"] != "PATTERN_NOTES_NOT_STORED" {
                    return Err(invalid);
                }
                missing = true;
                NoteRecordCount::Unavailable
            }
            Some("unsupported") => {
                shape(count, &["status", "reason"])?;
                let reason = match count["reason"].as_str() {
                    Some("PATTERN_NOTE_LAYOUT_UNVERIFIED") => PatternNoteReason::LayoutUnverified,
                    Some("MULTIPLE_PATTERN_NOTE_PAYLOADS") => PatternNoteReason::MultiplePayloads,
                    _ => return Err(invalid),
                };
                unsupported = true;
                NoteRecordCount::Unsupported(reason)
            }
            _ => return Err(invalid),
        };
        result.push(SavedPatternNoteCount {
            id: pattern.id(),
            count,
        });
    }
    let expected = if unsupported {
        "unsupported"
    } else if missing {
        "unavailable"
    } else {
        "extracted"
    };
    if status != expected {
        return Err(invalid);
    }
    Ok(PatternNoteCounts::Entries(result))
}
