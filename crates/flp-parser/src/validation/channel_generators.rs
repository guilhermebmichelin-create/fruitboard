//! Verified built-in classes, independent of editable channel labels. No raw
//! plugin text, channel IDs, automatic Debug or Serialize leaves this boundary.
use super::{ValidationError, field_shape};
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GeneratorReason {
    NotAdvertised,
    UnverifiedBuild,
    ZeroChannels,
    UnverifiedClass,
    UnverifiedEncoding,
    MultipleEvents,
}
impl GeneratorReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotAdvertised => "GENERATOR_NAMES_NOT_ADVERTISED",
            Self::UnverifiedBuild => "GENERATOR_NAMES_UNVERIFIED_BUILD",
            Self::ZeroChannels => "ZERO_CHANNEL_PROJECT_UNVERIFIED",
            Self::UnverifiedClass => "GENERATOR_CLASS_UNVERIFIED",
            Self::UnverifiedEncoding => "GENERATOR_NAME_ENCODING_UNVERIFIED",
            Self::MultipleEvents => "MULTIPLE_GENERATOR_EVENTS_UNVERIFIED",
        }
    }
}
pub enum ChannelGenerator {
    ExtractedOsc,
    InferredSampler,
    Unsupported(GeneratorReason),
}
pub enum ChannelGenerators {
    Unsupported(GeneratorReason),
    /// Dense parser order, not FL Studio's saved channel ID.
    Items(Vec<ChannelGenerator>),
}

fn unsupported(value: &Value, reason: GeneratorReason) -> Result<(), ValidationError> {
    field_shape(value, "unsupported")?;
    if value["reason"] != reason.as_str() {
        return Err(ValidationError::InvalidReply);
    }
    Ok(())
}

pub(super) fn validate(
    value: &Value,
    advertised: bool,
    build: &str,
    count: usize,
) -> Result<ChannelGenerators, ValidationError> {
    // A descriptor without this extension grants no authority to retain it.
    if !advertised {
        return Ok(ChannelGenerators::Unsupported(
            GeneratorReason::NotAdvertised,
        ));
    }
    if build != "26.1.0.5530" || count == 0 {
        let reason = if build != "26.1.0.5530" {
            GeneratorReason::UnverifiedBuild
        } else {
            GeneratorReason::ZeroChannels
        };
        unsupported(value, reason)?;
        if value.get("items").is_some() {
            return Err(ValidationError::InvalidReply);
        }
        return Ok(ChannelGenerators::Unsupported(reason));
    }
    let status = value["status"]
        .as_str()
        .ok_or(ValidationError::InvalidReply)?;
    field_shape(value, status)?;
    if status == "extracted" {
        let names = value["value"]
            .as_array()
            .ok_or(ValidationError::InvalidReply)?;
        if names.len() != count
            || names.iter().any(|v| v != "3x Osc")
            || value.get("items").is_some()
        {
            return Err(ValidationError::InvalidReply);
        }
        return Ok(ChannelGenerators::Items(
            (0..count).map(|_| ChannelGenerator::ExtractedOsc).collect(),
        ));
    }
    if !matches!(status, "inferred" | "unsupported") {
        return Err(ValidationError::InvalidReply);
    }
    let items = value["items"]
        .as_array()
        .ok_or(ValidationError::InvalidReply)?;
    if items.len() != count {
        return Err(ValidationError::InvalidReply);
    }
    let mut entries = Vec::with_capacity(count);
    let mut names = Vec::with_capacity(count);
    let (mut extracted, mut inferred, mut unknown) = (0, 0, 0);
    for (index, item) in items.iter().enumerate() {
        if item["channelIndex"].as_u64() != Some(index as u64) {
            return Err(ValidationError::InvalidReply);
        }
        let name = &item["name"];
        if name.get("items").is_some() {
            return Err(ValidationError::InvalidReply);
        }
        let entry = match name["status"].as_str() {
            Some("extracted") => {
                field_shape(name, "extracted")?;
                if name["value"] != "3x Osc" {
                    return Err(ValidationError::InvalidReply);
                }
                extracted += 1;
                names.push(Value::from("3x Osc"));
                ChannelGenerator::ExtractedOsc
            }
            Some("inferred") => {
                field_shape(name, "inferred")?;
                if name["value"] != "Sampler"
                    || name["method"] != "sampler-generator-default-for-known-build"
                    || name["confidence"] != "high"
                {
                    return Err(ValidationError::InvalidReply);
                }
                inferred += 1;
                names.push(Value::from("Sampler"));
                ChannelGenerator::InferredSampler
            }
            Some("unsupported") => {
                let reason = match name["reason"].as_str() {
                    Some("GENERATOR_CLASS_UNVERIFIED") => GeneratorReason::UnverifiedClass,
                    Some("GENERATOR_NAME_ENCODING_UNVERIFIED") => {
                        GeneratorReason::UnverifiedEncoding
                    }
                    Some("MULTIPLE_GENERATOR_EVENTS_UNVERIFIED") => GeneratorReason::MultipleEvents,
                    _ => return Err(ValidationError::InvalidReply),
                };
                unsupported(name, reason)?;
                unknown += 1;
                ChannelGenerator::Unsupported(reason)
            }
            _ => return Err(ValidationError::InvalidReply),
        };
        entries.push(entry);
    }
    if status == "unsupported" {
        unsupported(value, GeneratorReason::UnverifiedClass)?;
        if unknown == 0 {
            return Err(ValidationError::InvalidReply);
        }
    } else {
        let method = if extracted > 0 {
            "mixed-extracted-and-sampler-generator-default"
        } else {
            "sampler-generator-default-for-known-build"
        };
        let confidence = if extracted > 0 { "medium" } else { "high" };
        if unknown > 0
            || inferred == 0
            || value["value"] != Value::Array(names)
            || value["method"] != method
            || value["confidence"] != confidence
        {
            return Err(ValidationError::InvalidReply);
        }
    }
    Ok(ChannelGenerators::Items(entries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn saved() -> Value {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/parser-corpus/FIX-FL2026-3XOSC.flp"),
        )
        .unwrap();
        crate::parse_bytes(&bytes)["channelGeneratorNames"].clone()
    }
    #[test]
    fn verified_fixture_distinguishes_default_sampler_and_saved_osc_class() {
        let ChannelGenerators::Items(items) = validate(&saved(), true, "26.1.0.5530", 2).unwrap()
        else {
            panic!("verified items")
        };
        assert!(matches!(items[0], ChannelGenerator::InferredSampler));
        assert!(matches!(items[1], ChannelGenerator::ExtractedOsc));
        assert!(matches!(
            validate(&Value::Null, false, "26.1.0.5530", 2).unwrap(),
            ChannelGenerators::Unsupported(GeneratorReason::NotAdvertised)
        ));
        for (build, count, reason) in [
            ("25.1.3.4922", 2, GeneratorReason::UnverifiedBuild),
            ("26.1.0.5530", 0, GeneratorReason::ZeroChannels),
        ] {
            assert!(
                validate(
                    &json!({"status":"unsupported","reason":reason.as_str()}),
                    true,
                    build,
                    count
                )
                .is_ok()
            );
            assert!(validate(&saved(), true, build, count).is_err());
        }
    }
    #[test]
    fn rejects_wrong_order_counts_claims_inferences_and_aggregate_values() {
        for (pointer, value) in [
            ("/items/0/channelIndex", json!(1)),
            ("/items/1/channelIndex", json!(0)),
            ("/items/0/name/value", json!("3x Osc")),
            ("/items/1/name/value", json!("Private Plugin")),
            ("/items/0/name/confidence", json!("medium")),
            ("/items/0/name/method", json!("guess")),
            ("/value/1", json!("Wrong")),
            ("/confidence", json!("high")),
            (
                "/method",
                json!("sampler-generator-default-for-known-build"),
            ),
        ] {
            let mut raw = saved();
            *raw.pointer_mut(pointer).unwrap() = value;
            assert!(validate(&raw, true, "26.1.0.5530", 2).is_err(), "{pointer}");
        }
        assert!(validate(&saved(), true, "26.1.0.5530", 1).is_err());
        assert!(
            validate(
                &json!({"status":"extracted","value":["Sampler"]}),
                true,
                "26.1.0.5530",
                1
            )
            .is_err()
        );
        assert!(
            validate(
                &json!({"status":"extracted","value":["3x Osc"]}),
                true,
                "26.1.0.5530",
                1
            )
            .is_ok()
        );
    }
    #[test]
    fn partial_unknown_class_retains_verified_sibling_without_claiming_unknown_name() {
        let mut raw = saved();
        raw["status"] = json!("unsupported");
        raw["reason"] = json!("GENERATOR_CLASS_UNVERIFIED");
        for key in ["value", "method", "confidence"] {
            raw.as_object_mut().unwrap().remove(key);
        }
        raw["items"][1]["name"] = json!({"status":"unsupported","reason":"GENERATOR_CLASS_UNVERIFIED","privateExtension":"must not escape"});
        let ChannelGenerators::Items(items) = validate(&raw, true, "26.1.0.5530", 2).unwrap()
        else {
            panic!("partial items")
        };
        assert!(matches!(items[0], ChannelGenerator::InferredSampler));
        assert!(matches!(
            items[1],
            ChannelGenerator::Unsupported(GeneratorReason::UnverifiedClass)
        ));
        raw["items"][1]["name"] = json!({"status":"extracted","value":"3x Osc"});
        assert!(validate(&raw, true, "26.1.0.5530", 2).is_err());
    }
}
