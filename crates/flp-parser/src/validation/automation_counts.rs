use super::ValidationError;
use crate::automation_counts::{
    BUILD, MAX_OBJECTS, MAX_POINTS, MAX_POINTS_PER_OBJECT, UNVERIFIED_BUILD, UNVERIFIED_LAYOUT,
};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavedAutomationCounts {
    NotAdvertised,
    UnverifiedBuild,
    UnverifiedLayout,
    Extracted { object_count: u64, point_count: u64 },
}

pub(super) fn terminal(value: &Value, status: &str, reason: &str) -> Result<(), ValidationError> {
    if value.as_object().is_some_and(|object| object.len() == 2)
        && value["status"] == status
        && value["reason"] == reason
    {
        Ok(())
    } else {
        Err(ValidationError::InvalidReply)
    }
}

pub(super) fn validate(
    value: &Value,
    advertised: bool,
    version: &str,
    channel_count: u16,
) -> Result<SavedAutomationCounts, ValidationError> {
    if !advertised {
        return Ok(SavedAutomationCounts::NotAdvertised);
    }
    if version != BUILD {
        terminal(value, "unavailable", UNVERIFIED_BUILD)?;
        return Ok(SavedAutomationCounts::UnverifiedBuild);
    }
    if value["status"] == "unavailable" {
        terminal(value, "unavailable", UNVERIFIED_LAYOUT)?;
        return Ok(SavedAutomationCounts::UnverifiedLayout);
    }
    let counts = &value["value"];
    if value["status"] != "extracted"
        || !value.as_object().is_some_and(|object| object.len() == 2)
        || !counts.as_object().is_some_and(|object| object.len() == 2)
    {
        return Err(ValidationError::InvalidReply);
    }
    let object_count = counts["objectCount"]
        .as_u64()
        .ok_or(ValidationError::InvalidReply)?;
    let point_count = counts["pointCount"]
        .as_u64()
        .ok_or(ValidationError::InvalidReply)?;
    if object_count > MAX_OBJECTS
        || object_count + 1 != u64::from(channel_count)
        || !(object_count..=(object_count * MAX_POINTS_PER_OBJECT).min(MAX_POINTS))
            .contains(&point_count)
    {
        return Err(ValidationError::InvalidReply);
    }
    Ok(SavedAutomationCounts::Extracted {
        object_count,
        point_count,
    })
}

/// Read the optional additive field in an immutable metadata projection.
pub fn validate_stored_automation_counts(
    value: Option<&Value>,
    version: &str,
    channel_count: u16,
) -> Result<SavedAutomationCounts, ValidationError> {
    match value {
        Some(value) => validate(value, true, version, channel_count),
        None => Ok(SavedAutomationCounts::NotAdvertised),
    }
}
