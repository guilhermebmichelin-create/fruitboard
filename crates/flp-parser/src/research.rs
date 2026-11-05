//! Explicit, unqualified observations for separately reviewed compatibility
//! experiments. No file IO, native protocol method or production policy change.
use serde_json::{Value, json};

/// Never inferred from successful parsing. A caller must register its choice.
#[derive(Clone, Copy, Debug)]
pub enum Event172Width {
    One,
    Three,
    Four,
}

impl Event172Width {
    pub fn bytes(self) -> usize {
        match self {
            Self::One => 1,
            Self::Three => 3,
            Self::Four => 4,
        }
    }
}

/// Exact four-component build only; deliberately has no wire deserializer.
pub struct Profile<'a> {
    saved_build: &'a str,
    event_172_width: Event172Width,
}

impl<'a> Profile<'a> {
    pub fn new(saved_build: &'a str, event_172_width: Event172Width) -> Result<Self, &'static str> {
        if saved_build.len() > 32 {
            return Err("RESEARCH_EXACT_BUILD_REQUIRED");
        }
        let parts: Vec<_> = saved_build.split('.').collect();
        if parts.len() != 4
            || parts.iter().any(|part| {
                part.is_empty()
                    || !part.bytes().all(|b| b.is_ascii_digit())
                    || part.parse::<u32>().is_err()
                    || (part.len() > 1 && part.starts_with('0'))
            })
        {
            return Err("RESEARCH_EXACT_BUILD_REQUIRED");
        }
        Ok(Self {
            saved_build,
            event_172_width,
        })
    }
}

/// Inspect bounded bytes with the SAME decoder, changing only exact-build
/// admission and event172 width. Existing field-specific build restrictions,
/// malformed-input handling and resource caps still apply. This is neither a
/// native metadata envelope nor evidence that its observations are correct.
pub fn inspect(bytes: &[u8], profile: &Profile<'_>) -> Value {
    const MAX_PREFIX: usize = 32;
    let mut prefix = Vec::new();
    let mut observed = 0usize;
    let parsed = super::parse_bytes_with_policy(
        bytes,
        |version| version == profile.saved_build,
        |_| profile.event_172_width.bytes(),
        |id, event_offset, payload_offset, payload_bytes| {
            observed += 1;
            if prefix.len() < MAX_PREFIX {
                prefix.push(json!({
                    "id":id, "eventOffset":event_offset,
                    "payloadOffset":payload_offset,"payloadBytes":payload_bytes
                }));
            }
        },
    );
    let fields = [
        "savedVersion",
        "baseTempoBpm",
        "channelCount",
        "channelNames",
        "sampleReferences",
    ]
    .into_iter()
    .map(|name| (name.to_owned(), parsed[name].clone()))
    .collect::<serde_json::Map<_, _>>();
    json!({
        "researchOnly":true,
        "qualified":false,
        "profile":{"exactSavedBuild":profile.saved_build,"event172Bytes":profile.event_172_width.bytes()},
        "decoderOutcome":parsed["outcome"],
        "decoderCode":parsed.get("code"),
        "fieldObservations":fields,
        "observedEvents":observed,
        "framingPrefix":prefix,
        "framingPrefixTruncated":observed > MAX_PREFIX
    })
}
