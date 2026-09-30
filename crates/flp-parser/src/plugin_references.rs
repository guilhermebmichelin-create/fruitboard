//! Inert top-level saved plugin references. No plugin state or path is retained.
use crate::{field, le_u32, utf16_text};
use serde_json::{Value, json};

pub(crate) const MAX_PLUGIN_REFERENCES: usize = 1024;

#[derive(Default)]
pub(crate) struct PluginReferences {
    references: Vec<Value>,
    current: Option<Value>,
    wrapper_data_seen: bool,
}

impl PluginReferences {
    pub fn boundary(&mut self) -> Result<(), &'static str> {
        if let Some(reference) = self.current.take() {
            if self.references.len() == MAX_PLUGIN_REFERENCES {
                return Err("PLUGIN_REFERENCE_LIMIT");
            }
            self.references.push(reference);
        }
        self.wrapper_data_seen = false;
        Ok(())
    }

    pub fn name(&mut self, data: &[u8], sampler: bool) -> Result<(), &'static str> {
        self.boundary()?;
        let absent = || field("unavailable", Value::Null, Some("PLUGIN_NAME_NOT_STORED"));
        let (class, name) = match utf16_text(data) {
            Ok(class) if !class.is_empty() => {
                let name = if class == "Fruity Wrapper" {
                    field("unsupported", Value::Null, Some("VST_METADATA_UNSUPPORTED"))
                } else {
                    field("extracted", json!(class), None)
                };
                (field("extracted", json!(class), None), name)
            }
            Ok(_) if sampler => (
                absent(),
                json!({"status":"inferred","value":"Sampler","method":"sampler-default-for-known-build","confidence":"high"}),
            ),
            Ok(_) => return Ok(()), // An empty slot is not a used plugin.
            Err(_) => {
                let invalid = field(
                    "unsupported",
                    Value::Null,
                    Some("PLUGIN_NAME_ENCODING_UNSUPPORTED"),
                );
                (invalid.clone(), invalid)
            }
        };
        self.current = Some(json!({
            "className":class,"name":name,
            "vendor":field("unavailable",Value::Null,Some("PLUGIN_VENDOR_NOT_STORED"))
        }));
        Ok(())
    }

    pub fn data(&mut self, data: &[u8]) {
        let Some(reference) = self.current.as_mut() else {
            return;
        };
        if reference["className"]["value"] != "Fruity Wrapper" {
            return;
        }
        let parsed = if self.wrapper_data_seen {
            Err("MULTIPLE_VST_METADATA_RECORDS")
        } else {
            wrapper_metadata(data)
        };
        self.wrapper_data_seen = true;
        match parsed {
            Ok((name, vendor)) => {
                reference["name"] = name.map_or_else(
                    || field("unavailable", Value::Null, Some("PLUGIN_NAME_NOT_STORED")),
                    |name| field("extracted", json!(name), None),
                );
                reference["vendor"] = vendor.map_or_else(
                    || field("unavailable", Value::Null, Some("PLUGIN_VENDOR_NOT_STORED")),
                    |vendor| field("extracted", json!(vendor), None),
                );
            }
            Err(reason) => {
                reference["name"] = field("unsupported", Value::Null, Some(reason));
                reference["vendor"] = field("unsupported", Value::Null, Some(reason));
            }
        }
    }

    pub fn into_field(mut self) -> Result<Value, &'static str> {
        self.boundary()?;
        Ok(
            json!({"status":"extracted","value":self.references,"coverage":"top-level-saved-references"}),
        )
    }
}

// Only name/vendor subrecords are interpreted; state, paths and IDs are skipped
// by checked lengths and are never copied, loaded, logged or included in JSON.
fn wrapper_metadata(data: &[u8]) -> Result<(Option<String>, Option<String>), &'static str> {
    let invalid = "VST_METADATA_UNSUPPORTED";
    if data.len() < 4 || !matches!(le_u32(&data[..4]), 8 | 10) {
        return Err(invalid);
    }
    let mut cursor = 4_usize;
    let mut records = 0;
    let mut name = None;
    let mut vendor = None;
    while cursor < data.len() {
        records += 1;
        if records > 1024 || data.len() - cursor < 12 {
            return Err(invalid);
        }
        let id = le_u32(&data[cursor..cursor + 4]);
        let length = u64::from_le_bytes(
            data[cursor + 4..cursor + 12]
                .try_into()
                .expect("checked length"),
        );
        cursor += 12;
        let length = usize::try_from(length).map_err(|_| invalid)?;
        let end = cursor
            .checked_add(length)
            .filter(|end| *end <= data.len())
            .ok_or(invalid)?;
        if matches!(id, 54 | 56) {
            let slot = if id == 54 { &mut name } else { &mut vendor };
            if slot.is_some() || length == 0 || length > 4095 * 3 {
                return Err(invalid);
            }
            let value = std::str::from_utf8(&data[cursor..end]).map_err(|_| invalid)?;
            if value.contains('\0') || value.encode_utf16().count() > 4095 {
                return Err(invalid);
            }
            *slot = Some(value.to_owned());
        }
        cursor = end;
    }
    Ok((name, vendor))
}
