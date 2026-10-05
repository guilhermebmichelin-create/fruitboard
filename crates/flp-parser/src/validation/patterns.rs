//! Bounded saved IDs/names, never a GUI default or an execution capability.
use super::{ValidationError, field_shape, scalar, text};
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PatternSupport {
    NotAdvertised,
    UnverifiedBuild,
}
impl PatternSupport {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotAdvertised => "PATTERN_DETAILS_NOT_ADVERTISED",
            Self::UnverifiedBuild => "PATTERN_DETAILS_UNVERIFIED_BUILD",
        }
    }
}
pub struct SavedPattern {
    id: u16,
    name: Option<String>,
}
impl SavedPattern {
    pub fn id(&self) -> u16 {
        self.id
    }
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}
// No Debug/Serialize: saved text leaves this type only through an explicit view.
pub enum SavedPatterns {
    Unsupported(PatternSupport),
    Unavailable,
    Entries(Vec<SavedPattern>),
}

fn absent(value: &Value, status: &str, reason: &str) -> Result<(), ValidationError> {
    field_shape(value, status)?;
    if value["reason"] != reason || value.get("items").is_some() {
        return Err(ValidationError::InvalidReply);
    }
    Ok(())
}
fn entries(items: &[Value], count: usize) -> Result<Vec<SavedPattern>, ValidationError> {
    if items.len() != count {
        return Err(ValidationError::InvalidReply);
    }
    let mut previous = 0;
    let mut result = Vec::with_capacity(count);
    for item in items {
        let id = item["patternId"]
            .as_u64()
            .filter(|id| *id > previous && *id <= u64::from(u16::MAX))
            .ok_or(ValidationError::InvalidReply)?;
        previous = id;
        let name = &item["name"];
        let name = match name["status"].as_str() {
            Some("extracted") => Some(text(scalar(name)?, false)?),
            Some("unavailable") => {
                absent(name, "unavailable", "PATTERN_NAME_NOT_STORED")?;
                None
            }
            _ => return Err(ValidationError::InvalidReply),
        };
        result.push(SavedPattern {
            id: id as u16,
            name,
        });
    }
    Ok(result)
}

pub(super) fn validate(
    count: &Value,
    names: &Value,
    advertised: bool,
    build: &str,
    maximum: usize,
) -> Result<SavedPatterns, ValidationError> {
    if !advertised {
        return Ok(SavedPatterns::Unsupported(PatternSupport::NotAdvertised));
    }
    if build != "26.1.0.5530" {
        absent(count, "unsupported", "PATTERN_COUNT_UNVERIFIED_BUILD")?;
        absent(names, "unsupported", "PATTERN_NAMES_UNVERIFIED_BUILD")?;
        return Ok(SavedPatterns::Unsupported(PatternSupport::UnverifiedBuild));
    }
    if count["status"] == "unavailable" {
        absent(count, "unavailable", "PATTERN_DATA_NOT_STORED")?;
        absent(names, "unavailable", "PATTERN_DATA_NOT_STORED")?;
        return Ok(SavedPatterns::Unavailable);
    }
    let count = scalar(count)?
        .as_u64()
        .filter(|count| (1..=maximum as u64).contains(count))
        .ok_or(ValidationError::InvalidReply)? as usize;
    let status = names["status"]
        .as_str()
        .ok_or(ValidationError::InvalidReply)?;
    field_shape(names, status)?;
    let items = match status {
        "extracted" if names.get("items").is_none() => &names["value"],
        "unavailable" if names["reason"] == "PATTERN_NAME_NOT_STORED" => &names["items"],
        _ => return Err(ValidationError::InvalidReply),
    };
    let result = entries(
        items.as_array().ok_or(ValidationError::InvalidReply)?,
        count,
    )?;
    let has_missing = result.iter().any(|pattern| pattern.name.is_none());
    if has_missing != (status == "unavailable") {
        return Err(ValidationError::InvalidReply);
    }
    Ok(SavedPatterns::Entries(result))
}

/// Validate an additive immutable display projection, not parser/source authority.
/// A caller handles an absent field as an older result; malformed present data
/// never falls back to an older shape or grants filesystem/process authority.
pub fn validate_stored_patterns(
    value: &Value,
    build: &str,
) -> Result<SavedPatterns, ValidationError> {
    let status = value["status"]
        .as_str()
        .ok_or(ValidationError::InvalidReply)?;
    if status != "extracted" {
        if value.get("count").is_some() {
            return Err(ValidationError::InvalidReply);
        }
        match (status, value["reason"].as_str()) {
            ("unsupported", Some("PATTERN_DETAILS_NOT_ADVERTISED")) => {
                absent(value, status, PatternSupport::NotAdvertised.as_str())?;
                return Ok(SavedPatterns::Unsupported(PatternSupport::NotAdvertised));
            }
            ("unsupported", Some("PATTERN_DETAILS_UNVERIFIED_BUILD")) if build != "26.1.0.5530" => {
                absent(value, status, PatternSupport::UnverifiedBuild.as_str())?;
                return Ok(SavedPatterns::Unsupported(PatternSupport::UnverifiedBuild));
            }
            ("unavailable", Some("PATTERN_DATA_NOT_STORED")) if build == "26.1.0.5530" => {
                absent(value, status, "PATTERN_DATA_NOT_STORED")?;
                return Ok(SavedPatterns::Unavailable);
            }
            _ => return Err(ValidationError::InvalidReply),
        }
    }
    // Stored containers carry count/items instead of a raw parser value array.
    if build != "26.1.0.5530"
        || value.get("value").is_some()
        || value.get("reason").is_some()
        || value.get("method").is_some()
        || value.get("confidence").is_some()
    {
        return Err(ValidationError::InvalidReply);
    }
    let count = value["count"]
        .as_u64()
        .filter(|count| (1..=crate::MAX_PATTERNS as u64).contains(count))
        .ok_or(ValidationError::InvalidReply)? as usize;
    Ok(SavedPatterns::Entries(entries(
        value["items"]
            .as_array()
            .ok_or(ValidationError::InvalidReply)?,
        count,
    )?))
}
