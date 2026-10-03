//! Allowlisted saved references for local display, never plugin authority.
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub(super) struct PluginReferences {
    coverage: &'static str,
    items: Vec<PluginReference>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginReference {
    position: usize,
    class_name: Detail,
    name: Detail,
    vendor: Detail,
}
// No Debug: values are private display data, not diagnostic text.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Detail {
    Extracted {
        value: String,
    },
    Inferred {
        value: &'static str,
        method: &'static str,
        confidence: &'static str,
    },
    Unavailable {
        value: Option<String>,
        reason: &'static str,
    },
    Unsupported {
        value: Option<String>,
        reason: &'static str,
    },
}
#[derive(Clone, Copy, PartialEq)]
enum Role {
    Class,
    Name,
    Vendor,
}

fn field(value: &Value, role: Role, build: &str) -> Result<Detail, ()> {
    if value.get("items").is_some() || value.get("assumptions").is_some() {
        return Err(());
    }
    match value["status"].as_str() {
        Some("extracted") => {
            if value.get("reason").is_some()
                || value.get("method").is_some()
                || value.get("confidence").is_some()
            {
                return Err(());
            }
            let text = value["value"]
                .as_str()
                .filter(|text| {
                    !text.is_empty()
                        && text.len() <= 12285
                        && text.encode_utf16().count() <= 4095
                        && !text.contains('\0')
                })
                .ok_or(())?;
            Ok(Detail::Extracted {
                value: text.to_owned(),
            })
        }
        Some("inferred")
            if role == Role::Name && matches!(build, "25.1.3.4922" | "26.1.0.5530") =>
        {
            if value["value"] != "Sampler"
                || value["method"] != "sampler-default-for-known-build"
                || value["confidence"] != "high"
                || value.get("reason").is_some()
            {
                return Err(());
            }
            Ok(Detail::Inferred {
                value: "Sampler",
                method: "sampler-default-for-known-build",
                confidence: "high",
            })
        }
        Some(status @ ("unavailable" | "unsupported")) => {
            if value.get("value").is_some()
                || value.get("method").is_some()
                || value.get("confidence").is_some()
            {
                return Err(());
            }
            let reason = match (status, value["reason"].as_str()) {
                ("unavailable", Some("PLUGIN_NAME_NOT_STORED")) if role != Role::Vendor => {
                    "PLUGIN_NAME_NOT_STORED"
                }
                ("unavailable", Some("PLUGIN_VENDOR_NOT_STORED")) if role == Role::Vendor => {
                    "PLUGIN_VENDOR_NOT_STORED"
                }
                ("unsupported", Some("PLUGIN_NAME_ENCODING_UNSUPPORTED")) => {
                    "PLUGIN_NAME_ENCODING_UNSUPPORTED"
                }
                ("unsupported", Some("VST_METADATA_UNSUPPORTED")) => "VST_METADATA_UNSUPPORTED",
                ("unsupported", Some("MULTIPLE_VST_METADATA_RECORDS")) => {
                    "MULTIPLE_VST_METADATA_RECORDS"
                }
                _ => return Err(()),
            };
            Ok(if status == "unavailable" {
                Detail::Unavailable {
                    value: None,
                    reason,
                }
            } else {
                Detail::Unsupported {
                    value: None,
                    reason,
                }
            })
        }
        _ => Err(()),
    }
}
fn consistent(class: &Detail, name: &Detail, vendor: &Detail) -> bool {
    let vendor_absent = matches!(
        vendor,
        Detail::Unavailable {
            reason: "PLUGIN_VENDOR_NOT_STORED",
            ..
        }
    );
    match class {
        Detail::Extracted { value } if value == "Fruity Wrapper" => match (name, vendor) {
            (
                Detail::Extracted { .. } | Detail::Unavailable { .. },
                Detail::Extracted { .. } | Detail::Unavailable { .. },
            ) => true,
            (
                Detail::Unsupported {
                    reason: "VST_METADATA_UNSUPPORTED",
                    ..
                },
                Detail::Unavailable { .. },
            ) => true,
            (
                Detail::Unsupported { reason: left, .. },
                Detail::Unsupported { reason: right, .. },
            ) => {
                left == right
                    && matches!(
                        *left,
                        "VST_METADATA_UNSUPPORTED" | "MULTIPLE_VST_METADATA_RECORDS"
                    )
            }
            _ => false,
        },
        Detail::Extracted { value } => {
            matches!(name, Detail::Extracted { value: name } if name == value) && vendor_absent
        }
        Detail::Unavailable {
            reason: "PLUGIN_NAME_NOT_STORED",
            ..
        } => matches!(name, Detail::Inferred { .. }) && vendor_absent,
        Detail::Unsupported {
            reason: "PLUGIN_NAME_ENCODING_UNSUPPORTED",
            ..
        } => {
            matches!(
                name,
                Detail::Unsupported {
                    reason: "PLUGIN_NAME_ENCODING_UNSUPPORTED",
                    ..
                }
            ) && vendor_absent
        }
        _ => false,
    }
}
pub(super) fn project(value: Option<&Value>, build: &str) -> Result<Option<PluginReferences>, ()> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value["coverage"] != "top-level-saved-references"
        || ["value", "status", "reason", "method", "confidence"]
            .iter()
            .any(|key| value.get(key).is_some())
    {
        return Err(());
    }
    let items = value["items"]
        .as_array()
        .filter(|items| items.len() <= 1024)
        .ok_or(())?;
    let mut result = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let class_name = field(&item["className"], Role::Class, build)?;
        let name = field(&item["name"], Role::Name, build)?;
        let vendor = field(&item["vendor"], Role::Vendor, build)?;
        if !consistent(&class_name, &name, &vendor) {
            return Err(());
        }
        result.push(PluginReference {
            position: index + 1,
            class_name,
            name,
            vendor,
        });
    }
    let result = PluginReferences {
        coverage: "top-level-saved-references",
        items: result,
    };
    if serde_json::to_vec(&result).map_err(|_| ())?.len()
        > fruitboard_storage::MAX_METADATA_JSON_BYTES
    {
        return Err(());
    }
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn builtin() -> Value {
        json!({"className":{"status":"extracted","value":"3x Osc"},"name":{"status":"extracted","value":"3x Osc"},"vendor":{"status":"unavailable","reason":"PLUGIN_VENDOR_NOT_STORED"}})
    }
    fn inferred() -> Value {
        json!({"className":{"status":"unavailable","reason":"PLUGIN_NAME_NOT_STORED"},"name":{"status":"inferred","value":"Sampler","method":"sampler-default-for-known-build","confidence":"high"},"vendor":{"status":"unavailable","reason":"PLUGIN_VENDOR_NOT_STORED"}})
    }
    fn wrapper() -> Value {
        json!({"className":{"status":"extracted","value":"Fruity Wrapper"},"name":{"status":"extracted","value":"Synthetic Synth"},"vendor":{"status":"extracted","value":"Synthetic Vendor"}})
    }
    fn saved(items: Vec<Value>) -> Value {
        json!({"coverage":"top-level-saved-references","items":items})
    }
    fn output(value: &Value) -> Result<Value, ()> {
        serde_json::to_value(project(Some(value), "26.1.0.5530")?).map_err(|_| ())
    }
    #[test]
    fn preserves_order_duplicates_provenance_and_only_allowlisted_data() {
        let mut raw = saved(vec![builtin(), inferred(), wrapper(), builtin()]);
        raw["privateExtension"] = json!("discard");
        raw["items"][0]["path"] = json!("private.dll");
        raw["items"][0]["name"]["extra"] = json!("discard");
        let value = output(&raw).unwrap();
        assert_eq!(value["items"][0]["name"], value["items"][3]["name"]);
        assert_eq!(value["items"][1]["name"]["confidence"], "high");
        assert_eq!(value["items"][2]["vendor"]["value"], "Synthetic Vendor");
        assert_eq!(value["items"][3]["position"], 4);
        assert!(!value.to_string().contains("private"));
        assert!(!value.to_string().contains("discard"));
        assert!(project(None, "26.1.0.5530").unwrap().is_none());
        assert_eq!(output(&saved(vec![])).unwrap()["items"], json!([]));
    }
    #[test]
    fn rejects_contradictory_fields_and_unsupported_inference_authority() {
        for (pointer, replacement) in [
            ("/items/0/name/value", json!("different")),
            ("/items/0/vendor/reason", json!("private error")),
            ("/items/0/vendor/value", Value::Null),
            ("/items/0/name/reason", json!("private error")),
            ("/items/0/name/value", json!("")),
            ("/items/0/name/value", json!("x\u{0}y")),
            ("/items/0/name/value", json!("x".repeat(4096))),
            ("/items/0/name/method", json!("guess")),
            ("/items/1/name/confidence", json!("medium")),
            ("/items/1/name/method", json!("guess")),
            ("/items/1/className/status", json!("inferred")),
            ("/items/1/vendor/status", json!("inferred")),
            ("/items/1/name/value", json!("Sampler 2")),
            ("/coverage", json!("installed-plugins")),
        ] {
            let mut raw = saved(vec![builtin(), inferred()]);
            let parts: Vec<_> = pointer.trim_start_matches('/').split('/').collect();
            if parts.len() == 1 {
                raw[parts[0]] = replacement;
            } else {
                raw["items"][parts[1].parse::<usize>().unwrap()][parts[2]][parts[3]] = replacement;
            }
            assert!(output(&raw).is_err(), "{pointer}");
        }
        assert!(project(Some(&saved(vec![inferred()])), "20.0.0.1").is_err());
        assert!(project(Some(&saved(vec![inferred()])), "25.1.3.4922").is_ok());
    }
    #[test]
    fn checks_wrapper_missing_and_unsupported_relationships() {
        for reason in ["VST_METADATA_UNSUPPORTED", "MULTIPLE_VST_METADATA_RECORDS"] {
            let mut item = wrapper();
            item["name"] = json!({"status":"unsupported","reason":reason});
            item["vendor"] = json!({"status":"unsupported","reason":reason});
            assert!(output(&saved(vec![item.clone()])).is_ok());
            item["vendor"]["reason"] = json!("PLUGIN_NAME_ENCODING_UNSUPPORTED");
            assert!(output(&saved(vec![item])).is_err());
        }
        let mut item = wrapper();
        item["name"] = json!({"status":"unavailable","reason":"PLUGIN_NAME_NOT_STORED"});
        assert!(output(&saved(vec![item.clone()])).is_ok());
        item["name"] = json!({"status":"unsupported","reason":"VST_METADATA_UNSUPPORTED"});
        item["vendor"] = json!({"status":"unavailable","reason":"PLUGIN_VENDOR_NOT_STORED"});
        assert!(output(&saved(vec![item])).is_ok());
        let invalid = json!({"className":{"status":"unsupported","reason":"PLUGIN_NAME_ENCODING_UNSUPPORTED"},"name":{"status":"unsupported","reason":"PLUGIN_NAME_ENCODING_UNSUPPORTED"},"vendor":{"status":"unavailable","reason":"PLUGIN_VENDOR_NOT_STORED"}});
        assert!(output(&saved(vec![invalid])).is_ok());
    }
    #[test]
    fn keeps_text_inert_and_enforces_entry_and_total_byte_limits() {
        let mut item = wrapper();
        item["name"]["value"] = json!(format!("<img src=x>\n\u{202e}{}", "🎵".repeat(1900)));
        assert_eq!(
            output(&saved(vec![item.clone()])).unwrap()["items"][0]["name"]["value"],
            item["name"]["value"]
        );
        assert!(output(&saved(vec![builtin(); 1024])).is_ok());
        assert!(output(&saved(vec![builtin(); 1025])).is_err());
        assert!(output(&saved(vec![item; 40])).is_err());
    }
}
