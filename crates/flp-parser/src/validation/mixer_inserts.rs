//! Typed selected identities/names; no raw state or automatic Debug/Serialize.
use super::ValidationError;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MixerInsertReason {
    DataNotStored,
    UnverifiedBuild,
    LayoutUnverified,
    BindingUnverified,
    RecordsAmbiguous,
    LimitExceeded,
}
impl MixerInsertReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DataNotStored => "MIXER_INSERT_DATA_NOT_STORED",
            Self::UnverifiedBuild => "MIXER_INSERT_UNVERIFIED_BUILD",
            Self::LayoutUnverified => "MIXER_INSERT_LAYOUT_UNVERIFIED",
            Self::BindingUnverified => "MIXER_INSERT_BINDING_UNVERIFIED",
            Self::RecordsAmbiguous => "MIXER_INSERT_RECORDS_AMBIGUOUS",
            Self::LimitExceeded => "MIXER_INSERT_LIMIT_EXCEEDED",
        }
    }
}
pub enum MixerInsertName {
    Extracted(String),
    Unavailable,
}
pub struct SavedMixerInsert {
    id: u16,
    name: MixerInsertName,
}
impl SavedMixerInsert {
    /// Section position in the qualified save, not a persistent identity.
    pub fn id(&self) -> u16 {
        self.id
    }
    pub fn name(&self) -> &MixerInsertName {
        &self.name
    }
}
pub enum SavedMixerInserts {
    NotAdvertised,
    Unavailable(MixerInsertReason),
    Unsupported(MixerInsertReason),
    Entries(Vec<SavedMixerInsert>),
}
fn shape(value: &Value, keys: &[&str]) -> Result<(), ValidationError> {
    let object = value.as_object().ok_or(ValidationError::InvalidReply)?;
    if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
        return Err(ValidationError::InvalidReply);
    }
    Ok(())
}
pub(super) fn validate_failed(
    count: &Value,
    names: &Value,
    code: &str,
) -> Result<(), ValidationError> {
    for field in [count, names] {
        shape(field, &["status", "reason"])?;
        if field["status"] != "failed" || field["reason"] != code {
            return Err(ValidationError::InvalidReply);
        }
    }
    Ok(())
}
pub(super) fn validate(
    count: &Value,
    names: &Value,
    limits: Option<(usize, usize)>,
    build: &str,
) -> Result<SavedMixerInserts, ValidationError> {
    let Some((returned, candidates)) = limits else {
        return Ok(SavedMixerInserts::NotAdvertised);
    };
    let invalid = ValidationError::InvalidReply;
    if count["coverage"] != crate::mixer_inserts::COVERAGE
        || names["coverage"] != crate::mixer_inserts::COVERAGE
    {
        return Err(invalid);
    }
    if !crate::mixer_inserts::selected_fits(count, names) {
        return Err(ValidationError::LimitExceeded);
    }
    if count["status"] != "extracted" {
        for value in [count, names] {
            shape(value, &["status", "reason", "coverage"])?;
        }
        if count != names {
            return Err(invalid);
        }
        let reason = match (count["status"].as_str(), count["reason"].as_str()) {
            (Some("unavailable"), Some("MIXER_INSERT_DATA_NOT_STORED")) => {
                MixerInsertReason::DataNotStored
            }
            (Some("unsupported"), Some("MIXER_INSERT_UNVERIFIED_BUILD")) => {
                MixerInsertReason::UnverifiedBuild
            }
            (Some("unsupported"), Some("MIXER_INSERT_LAYOUT_UNVERIFIED")) => {
                MixerInsertReason::LayoutUnverified
            }
            (Some("unsupported"), Some("MIXER_INSERT_BINDING_UNVERIFIED")) => {
                MixerInsertReason::BindingUnverified
            }
            (Some("unsupported"), Some("MIXER_INSERT_RECORDS_AMBIGUOUS")) => {
                MixerInsertReason::RecordsAmbiguous
            }
            (Some("unsupported"), Some("MIXER_INSERT_LIMIT_EXCEEDED")) => {
                MixerInsertReason::LimitExceeded
            }
            _ => return Err(invalid),
        };
        if (build != crate::mixer_inserts::BUILD) != (reason == MixerInsertReason::UnverifiedBuild)
        {
            return Err(invalid);
        }
        return Ok(if reason == MixerInsertReason::DataNotStored {
            SavedMixerInserts::Unavailable(reason)
        } else {
            SavedMixerInserts::Unsupported(reason)
        });
    }
    shape(count, &["status", "value", "coverage"])?;
    // Limits are envelopes, not authority to accept any other count or IDs.
    if build != crate::mixer_inserts::BUILD || count["value"].as_u64() != Some(16) {
        return Err(invalid);
    }
    if returned < 16 || candidates < 18 {
        return Err(ValidationError::LimitExceeded);
    }
    let incomplete = names["status"] == "unavailable";
    let collection = if incomplete {
        shape(names, &["status", "reason", "coverage", "items"])?;
        if names["reason"] != "MIXER_INSERT_NAMES_INCOMPLETE" {
            return Err(invalid);
        }
        &names["items"]
    } else {
        shape(names, &["status", "coverage", "value"])?;
        if names["status"] != "extracted" {
            return Err(invalid);
        }
        &names["value"]
    };
    let entries = collection
        .as_array()
        .filter(|items| items.len() == 16)
        .ok_or(invalid)?;
    let mut selected = Vec::with_capacity(16);
    let mut missing = false;
    for (index, entry) in entries.iter().enumerate() {
        shape(entry, &["savedInsertId", "name"])?;
        let id = entry["savedInsertId"]
            .as_u64()
            .filter(|id| *id == (index + 1) as u64)
            .ok_or(invalid)?;
        let name = &entry["name"];
        let name = match name["status"].as_str() {
            Some("extracted") => {
                shape(name, &["status", "value"])?;
                let value = super::text(&name["value"], false)?;
                if value.is_empty() {
                    return Err(invalid);
                }
                MixerInsertName::Extracted(value)
            }
            Some("unavailable") => {
                shape(name, &["status", "reason"])?;
                if name["reason"] != "MIXER_INSERT_NAME_NOT_STORED" {
                    return Err(invalid);
                }
                missing = true;
                MixerInsertName::Unavailable
            }
            _ => return Err(invalid),
        };
        selected.push(SavedMixerInsert {
            id: id as u16,
            name,
        });
    }
    if missing != incomplete {
        return Err(invalid);
    }
    Ok(SavedMixerInserts::Entries(selected))
}
