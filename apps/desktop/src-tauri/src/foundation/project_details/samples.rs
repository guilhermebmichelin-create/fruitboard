//! Saved reference text for an explicit local view; never filesystem authority.
use serde::Serialize;
use serde_json::Value;

// No Debug: private saved references are returned only by the local details read.
#[derive(Serialize)]
pub(super) struct SampleReference {
    position: usize,
    status: &'static str,
    value: Option<String>,
}

pub(super) fn project(
    field: Option<&Value>,
    count: usize,
) -> Result<Option<Vec<SampleReference>>, ()> {
    let Some(field) = field else { return Ok(None) };
    if count > 256
        || field.get("value").is_some()
        || field.get("method").is_some()
        || field.get("confidence").is_some()
    {
        return Err(());
    }
    let status = field["status"].as_str().ok_or(())?;
    match status {
        "extracted" if field.get("reason").is_none() => {}
        "unavailable" if field["reason"] == "SAMPLE_REFERENCE_NOT_STORED" => {}
        _ => return Err(()),
    }
    let items = field["items"]
        .as_array()
        .filter(|v| v.len() == count)
        .ok_or(())?;
    let mut missing = 0;
    let mut result = Vec::with_capacity(count);
    for (index, item) in items.iter().enumerate() {
        if item.get("items").is_some()
            || item.get("method").is_some()
            || item.get("confidence").is_some()
        {
            return Err(());
        }
        let (status, value) = match item["status"].as_str() {
            Some("extracted") if item.get("reason").is_none() => {
                let text = item["value"]
                    .as_str()
                    .filter(|v| {
                        !v.is_empty()
                            && v.len() <= 12285
                            && v.encode_utf16().count() <= 4095
                            && !v.contains('\0')
                    })
                    .ok_or(())?;
                ("extracted", Some(text.to_owned()))
            }
            Some("unavailable")
                if item["reason"] == "SAMPLE_REFERENCE_NOT_STORED"
                    && item.get("value").is_none() =>
            {
                missing += 1;
                ("unavailable", None)
            }
            _ => return Err(()),
        };
        result.push(SampleReference {
            position: index + 1,
            status,
            value,
        });
    }
    if (status == "unavailable") != (missing > 0) {
        return Err(());
    }
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn saved() -> Value {
        json!({"status":"unavailable","reason":"SAMPLE_REFERENCE_NOT_STORED","items":[
            {"status":"extracted","value":"..\\Samples\\Kick.wav","privateExtension":"discard"},
            {"status":"unavailable","reason":"SAMPLE_REFERENCE_NOT_STORED"},
            {"status":"extracted","value":"..\\Samples\\Kick.wav"}
        ]})
    }

    #[test]
    fn keeps_order_duplicate_references_and_missing_entries_without_extensions() {
        let output = serde_json::to_value(project(Some(&saved()), 3).unwrap()).unwrap();
        assert_eq!(
            output,
            json!([
                {"position":1,"status":"extracted","value":"..\\Samples\\Kick.wav"},
                {"position":2,"status":"unavailable","value":null},
                {"position":3,"status":"extracted","value":"..\\Samples\\Kick.wav"}
            ])
        );
        assert!(project(None, 3).unwrap().is_none());
        assert_eq!(
            serde_json::to_value(
                project(Some(&json!({"status":"extracted","items":[]})), 0).unwrap()
            )
            .unwrap(),
            json!([])
        );
    }

    #[test]
    fn rejects_ambiguous_counts_provenance_and_invalid_reference_text() {
        for (pointer, replacement) in [
            ("/status", json!("extracted")),
            ("/reason", json!("private diagnostic")),
            ("/value", json!([])),
            ("/method", json!("guessed-path")),
            ("/items/0/status", json!("inferred")),
            ("/items/0/reason", json!("private")),
            ("/items/0/value", json!("")),
            ("/items/0/value", json!("nul\u{0}value")),
            ("/items/0/value", json!("x".repeat(4096))),
            ("/items/1/value", Value::Null),
            ("/items/1/reason", json!("SAMPLE_MISSING_ON_DISK")),
        ] {
            let mut value = saved();
            let parts: Vec<_> = pointer.trim_start_matches('/').split('/').collect();
            if parts.len() == 1 {
                value[parts[0]] = replacement
            } else {
                value["items"][parts[1].parse::<usize>().unwrap()][parts[2]] = replacement
            }
            assert!(project(Some(&value), 3).is_err(), "{pointer}");
        }
        assert!(project(Some(&saved()), 2).is_err());
        assert!(project(Some(&saved()), 257).is_err());
        let mut all_present = saved();
        all_present["items"][1] = json!({"status":"extracted","value":"other.wav"});
        assert!(project(Some(&all_present), 3).is_err());
    }

    #[test]
    fn preserves_inert_unicode_path_and_control_text_with_the_existing_limits() {
        let text = format!("%FLStudioData%\\音\\{}\n\u{202e}.wav", "🎵".repeat(2000));
        let field = json!({"status":"extracted","items":[{"status":"extracted","value":text}]});
        let output = serde_json::to_value(project(Some(&field), 1).unwrap()).unwrap();
        assert_eq!(output[0]["value"], text);
    }
}
