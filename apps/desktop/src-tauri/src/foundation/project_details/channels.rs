//! Treat immutable saved projections as untrusted local display data.
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub(super) struct Channel {
    position: usize,
    name: Detail,
    instrument: Detail,
}
#[derive(Serialize)]
struct Detail {
    status: &'static str,
    value: Option<String>,
    explanation: Option<&'static str>,
}
fn detail(status: &'static str, value: Option<&str>, explanation: Option<&'static str>) -> Detail {
    Detail {
        status,
        value: value.map(str::to_owned),
        explanation,
    }
}
fn has_value(field: &Value, status: &str) -> Result<(), ()> {
    if field["status"] != status
        || field.get("reason").is_some()
        || field.get("items").is_some()
        || (status == "extracted"
            && (field.get("method").is_some() || field.get("confidence").is_some()))
    {
        return Err(());
    }
    Ok(())
}
fn absent(field: &Value) -> Result<(), ()> {
    if field.get("value").is_some()
        || field.get("method").is_some()
        || field.get("confidence").is_some()
    {
        return Err(());
    }
    Ok(())
}
pub(super) fn project(value: &Value, build: &str, count: usize) -> Result<Vec<Channel>, ()> {
    if count > 256 {
        return Err(());
    }
    let names = &value["channelNames"];
    let items = names["items"]
        .as_array()
        .filter(|v| v.len() == count)
        .ok_or(())?;
    let (mut extracted, mut inferred, mut missing) = (0, 0, 0);
    let mut result = Vec::with_capacity(count);
    for (index, field) in items.iter().enumerate() {
        let name = match field["status"].as_str() {
            Some("extracted") => {
                has_value(field, "extracted")?;
                let text = field["value"]
                    .as_str()
                    .filter(|text| {
                        text.len() <= 12285
                            && text.encode_utf16().count() <= 4095
                            && !text.contains('\0')
                    })
                    .ok_or(())?;
                extracted += 1;
                detail("extracted", Some(text), None)
            }
            Some("inferred") if matches!(build, "25.1.3.4922" | "26.1.0.5530") => {
                has_value(field, "inferred")?;
                inferred += 1;
                let expected = if inferred == 1 {
                    "Sampler".to_owned()
                } else {
                    format!("Sampler {inferred}")
                };
                if field["value"] != expected
                    || field["method"] != "sampler-default-for-known-build"
                    || field["confidence"] != if inferred == 1 { "high" } else { "medium" }
                {
                    return Err(());
                }
                detail(
                    "inferred",
                    Some(&expected),
                    Some(if inferred == 1 {
                        "High confidence. FL Studio's default Sampler label for this verified build; no channel label was stored."
                    } else {
                        "Medium confidence. Default Sampler numbering follows FL Studio's display convention; no channel label was stored."
                    }),
                )
            }
            Some("unavailable") => {
                absent(field)?;
                if field["reason"] != "CHANNEL_NAME_NOT_STORED" || field.get("items").is_some() {
                    return Err(());
                }
                missing += 1;
                detail("unavailable", None, Some("No channel label was stored."))
            }
            _ => return Err(()),
        };
        result.push(Channel {
            position: index + 1,
            name,
            instrument: instrument(value.get("channelGeneratorNames"), build, count, index)?,
        });
    }
    match names["status"].as_str() {
        Some("extracted") if inferred == 0 && missing == 0 => {
            if names.get("reason").is_some()
                || names.get("method").is_some()
                || names.get("confidence").is_some()
            {
                return Err(());
            }
        }
        Some("unavailable") if missing > 0 => {
            absent(names)?;
            if names["reason"] != "CHANNEL_NAME_NOT_STORED" {
                return Err(());
            }
        }
        Some("inferred") if inferred > 0 && missing == 0 => {
            let method = if extracted > 0 {
                "mixed-extracted-and-sampler-default"
            } else {
                "sampler-default-for-known-build"
            };
            let confidence = if extracted > 0 || inferred > 1 {
                "medium"
            } else {
                "high"
            };
            if names["method"] != method
                || names["confidence"] != confidence
                || names.get("reason").is_some()
            {
                return Err(());
            }
        }
        _ => return Err(()),
    }
    // A zero-channel result still validates the optional instrument container.
    if count == 0 {
        instrument(value.get("channelGeneratorNames"), build, count, 0)?;
    }
    Ok(result)
}
fn instrument(
    field: Option<&Value>,
    build: &str,
    count: usize,
    index: usize,
) -> Result<Detail, ()> {
    let Some(field) = field else {
        return Ok(detail(
            "unsupported",
            None,
            Some("Instrument details were not saved with this result. A new analysis is needed."),
        ));
    };
    if field["status"] == "unsupported" {
        absent(field)?;
        if field.get("items").is_some() {
            return Err(());
        }
        let explanation = match field["reason"].as_str() {
            Some("GENERATOR_NAMES_NOT_ADVERTISED") => {
                "The parser did not advertise instrument details for this result."
            }
            Some("GENERATOR_NAMES_UNVERIFIED_BUILD") if build != "26.1.0.5530" => {
                "Instrument names are not verified for this saved FL Studio build."
            }
            Some("ZERO_CHANNEL_PROJECT_UNVERIFIED") if build == "26.1.0.5530" && count == 0 => {
                "Instrument names are not verified for a project with no channels."
            }
            _ => return Err(()),
        };
        return Ok(detail("unsupported", None, Some(explanation)));
    }
    if build != "26.1.0.5530"
        || count == 0
        || field.get("status").is_some()
        || field.get("value").is_some()
        || field.get("reason").is_some()
    {
        return Err(());
    }
    let items = field["items"]
        .as_array()
        .filter(|v| v.len() == count)
        .ok_or(())?;
    let item = &items[index];
    match item["status"].as_str() {
        Some("extracted") => {
            has_value(item, "extracted")?;
            if item["value"] != "3x Osc" {
                return Err(());
            }
            Ok(detail("extracted", Some("3x Osc"), None))
        }
        Some("inferred") => {
            has_value(item, "inferred")?;
            if item["value"] != "Sampler"
                || item["method"] != "sampler-generator-default-for-known-build"
                || item["confidence"] != "high"
            {
                return Err(());
            }
            Ok(detail(
                "inferred",
                Some("Sampler"),
                Some(
                    "High confidence. Built-in Sampler inferred from the verified saved channel type and empty generator class.",
                ),
            ))
        }
        Some("unsupported") => {
            absent(item)?;
            if item.get("items").is_some() {
                return Err(());
            }
            let explanation = match item["reason"].as_str() {
                Some("GENERATOR_CLASS_UNVERIFIED") => {
                    "This instrument class is not verified. The channel label does not identify its plugin."
                }
                Some("GENERATOR_NAME_ENCODING_UNVERIFIED") => {
                    "The saved instrument name encoding is not verified."
                }
                Some("MULTIPLE_GENERATOR_EVENTS_UNVERIFIED") => {
                    "Multiple saved instrument records prevent a verified instrument name."
                }
                _ => return Err(()),
            };
            Ok(detail("unsupported", None, Some(explanation)))
        }
        _ => Err(()),
    }
}
