//! Initial metadata projection for the native application boundary.
//!
//! Consume only envelope-checked supervisor replies. The native caller supplies
//! captured and current identities; these must come from its own authoritative
//! state, never from the parser. Publication still needs a transactional recheck.
//! Unvalidated extension fields and arbitrary diagnostics are never retained.

use crate::supervisor::ProtocolReply;
use crate::{ExpectedFingerprint, MAX_FILE_BYTES};
use serde_json::Value;
use std::collections::BTreeSet;
mod project_facts;
pub use project_facts::*;
mod channel_generators;
pub use channel_generators::*;

const INITIAL_FIELDS: [&str; 5] = [
    "savedVersion",
    "baseTempoBpm",
    "channelCount",
    "channelNames",
    "sampleReferences",
];
const MAX_DIAGNOSTICS: usize = 1024;
// The parser accepts 8192 UTF-16 bytes including the terminating NUL. UTF-8
// output can occupy three bytes per remaining code unit.
const MAX_TEXT_BYTES: usize = 4095 * 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationError {
    InvalidCapabilities,
    InvalidReply,
    LimitExceeded,
    FingerprintMismatch,
    StaleInput,
}

/// Limits can only be constructed by validating the selected descriptor.
pub struct ParserCapabilities {
    max_file_bytes: u64,
    max_events: u64,
    max_channels: usize,
    max_playlist_clips: usize,
    project_facts: bool,
    channel_generators: bool,
}

pub fn validate_descriptor(value: &Value) -> Result<ParserCapabilities, ValidationError> {
    let invalid = ValidationError::InvalidCapabilities;
    if value["adapter"] != crate::ADAPTER_ID || value["adapterVersion"] != crate::ADAPTER_VERSION {
        return Err(invalid);
    }
    let fields = value["fields"].as_array().ok_or(invalid)?;
    if fields.len() > 64 {
        return Err(invalid);
    }
    let mut names = BTreeSet::new();
    for field in fields {
        let name = field.as_str().ok_or(invalid)?;
        if name.is_empty() || name.len() > 64 || !names.insert(name) {
            return Err(invalid);
        }
    }
    if INITIAL_FIELDS.iter().any(|field| !names.contains(field)) {
        return Err(invalid);
    }
    let limit = |name: &str, ceiling: u64| {
        value[name]
            .as_u64()
            .filter(|number| (1..=ceiling).contains(number))
            .ok_or(invalid)
    };
    let max_file_bytes = limit("maxFileBytes", MAX_FILE_BYTES)?;
    let max_events = limit("maxEvents", crate::MAX_EVENTS as u64)?;
    let max_channels = limit("maxChannels", u64::from(crate::MAX_CHANNELS))? as usize;
    limit("maxEventBytes", crate::MAX_EVENT_BYTES as u64)?;
    limit("maxPatterns", crate::MAX_PATTERNS as u64)?;
    let max_playlist_clips = limit("maxPlaylistClips", crate::MAX_PLAYLIST_CLIPS as u64)? as usize;
    Ok(ParserCapabilities {
        max_file_bytes,
        max_events,
        max_channels,
        max_playlist_clips,
        project_facts: project_facts::FIELDS
            .iter()
            .all(|field| names.contains(field)),
        channel_generators: names.contains("channelGeneratorNames"),
    })
}

/// A native observation associated with one parse. No paths are needed here.
/// A digest, when present, must be independently known by the native caller.
#[derive(Clone)]
pub struct ParseContext {
    pub root_id: String,
    pub file_id: String,
    pub root_revision: u64,
    pub file_revision: u64,
    pub root_enabled: bool,
    pub expected: ExpectedFingerprint,
    pub content_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingReason {
    BaseTempoAbsent,
    ChannelNameNotStored,
    SampleReferenceNotStored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameInference {
    SamplerDefaultForKnownBuild,
    MixedExtractedAndSamplerDefault,
}

pub enum Field<T> {
    Extracted(T),
    Unavailable(MissingReason),
}

pub enum TextField {
    Extracted(String),
    Inferred {
        value: String,
        method: NameInference,
        confidence: Confidence,
    },
    Unavailable(MissingReason),
}

impl TextField {
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Extracted(value) | Self::Inferred { value, .. } => Some(value),
            Self::Unavailable(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListStatus {
    Extracted,
    Inferred {
        method: NameInference,
        confidence: Confidence,
    },
    Unavailable(MissingReason),
}

pub struct TextList {
    status: ListStatus,
    entries: Vec<TextField>,
}

impl TextList {
    pub fn status(&self) -> ListStatus {
        self.status
    }

    pub fn entries(&self) -> &[TextField] {
        &self.entries
    }
}

/// An allowlisted parser category, never arbitrary subprocess text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParserCode(&'static str);

impl ParserCode {
    pub fn as_str(self) -> &'static str {
        self.0
    }
}

fn parser_code(value: &str) -> Result<ParserCode, ValidationError> {
    const CODES: &[&str] = &[
        "CHANNEL_COUNT_LIMIT",
        "CHANNEL_COUNT_MISMATCH",
        "CONFLICTING_PATTERN_NAMES",
        "EVENT_COUNT_LIMIT",
        "EVENT_LENGTH_OUT_OF_BOUNDS",
        "FILE_SIZE_LIMIT",
        "INPUT_CHANGED",
        "INPUT_FINGERPRINT_UNAVAILABLE",
        "INPUT_METADATA_FAILED",
        "INPUT_NOT_FILE",
        "INPUT_OPEN_FAILED",
        "INPUT_READ_FAILED",
        "INVALID_BASE_TEMPO",
        "INVALID_CHANNEL_NAME",
        "INVALID_HEADER",
        "INVALID_PATH",
        "INVALID_PATTERN_ID",
        "INVALID_PATTERN_NAME",
        "INVALID_SAMPLE_REFERENCE",
        "INVALID_SAVED_VERSION",
        "MALFORMED_EVENT_LENGTH",
        "MISSING_DATA_CHUNK",
        "MISSING_SAVED_VERSION",
        "MULTIPLE_BASE_TEMPOS",
        "MULTIPLE_CHANNEL_NAMES",
        "MULTIPLE_SAMPLE_REFERENCES",
        "PATTERN_COUNT_LIMIT",
        "PATTERN_NAME_WITHOUT_ID",
        "PLAYLIST_CLIP_LIMIT",
        "PLAYLIST_POSITION_OVERFLOW",
        "PLUGIN_REFERENCE_LIMIT",
        "TRUNCATED_DATA_CHUNK",
        "RESPONSE_LIMIT",
        "INVALID_REQUEST",
        "INVALID_REQUEST_ID",
        "UNSUPPORTED_PROTOCOL_VERSION",
        "UNSUPPORTED_SCHEMA_VERSION",
        "UNSUPPORTED_FEATURE",
        "UNKNOWN_METHOD",
    ];
    CODES
        .iter()
        .find(|code| **code == value)
        .copied()
        .map(ParserCode)
        .ok_or(ValidationError::InvalidReply)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetadataOutcome {
    Complete,
    Partial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Diagnostic {
    UnsupportedEvent255,
}

/// No Debug/Serialize implementation: these fields can contain private names
/// and raw sample locators. Only the validated initial projection is exposed.
pub struct ValidatedMetadata {
    outcome: MetadataOutcome,
    saved_version: String,
    base_tempo_bpm: Field<f64>,
    channel_count: u16,
    channel_names: TextList,
    sample_references: TextList,
    content_sha256: String,
    diagnostics: Vec<Diagnostic>,
}

impl ValidatedMetadata {
    pub fn outcome(&self) -> MetadataOutcome {
        self.outcome
    }
    pub fn saved_version(&self) -> &str {
        &self.saved_version
    }
    pub fn base_tempo_bpm(&self) -> &Field<f64> {
        &self.base_tempo_bpm
    }
    pub fn channel_count(&self) -> u16 {
        self.channel_count
    }
    pub fn channel_names(&self) -> &TextList {
        &self.channel_names
    }
    pub fn sample_references(&self) -> &TextList {
        &self.sample_references
    }
    pub fn content_sha256(&self) -> &str {
        &self.content_sha256
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

pub enum ValidatedReply {
    Metadata(ValidatedMetadata),
    Failed(ParserCode),
    UnsupportedSavedVersion(String),
    Rejected(ParserCode),
}

fn fresh(captured: &ParseContext, current: &ParseContext) -> Result<(), ValidationError> {
    if captured.root_id.is_empty()
        || captured.file_id.is_empty()
        || !captured.root_enabled
        || !current.root_enabled
        || captured.root_id != current.root_id
        || captured.file_id != current.file_id
        || captured.root_revision != current.root_revision
        || captured.file_revision != current.file_revision
        || captured.expected.size != current.expected.size
        || captured.expected.modified_at_ms != current.expected.modified_at_ms
        || matches!((&captured.content_sha256, &current.content_sha256),
            (Some(left), Some(right)) if left != right)
    {
        return Err(ValidationError::StaleInput);
    }
    Ok(())
}

fn hash_shape(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn fingerprint(
    value: &Value,
    capabilities: &ParserCapabilities,
    captured: &ParseContext,
    current: &ParseContext,
) -> Result<String, ValidationError> {
    let size = value["size"]
        .as_u64()
        .ok_or(ValidationError::InvalidReply)?;
    let modified = value["modifiedAtMs"]
        .as_u64()
        .ok_or(ValidationError::InvalidReply)?;
    let hash = value["hash"]["value"]
        .as_str()
        .ok_or(ValidationError::InvalidReply)?;
    if value["hash"]["algorithm"] != "sha256" || !hash_shape(hash) {
        return Err(ValidationError::InvalidReply);
    }
    if size > capabilities.max_file_bytes {
        return Err(ValidationError::LimitExceeded);
    }
    if size != captured.expected.size
        || modified != captured.expected.modified_at_ms
        || [&captured.content_sha256, &current.content_sha256]
            .iter()
            .any(|known| {
                known
                    .as_ref()
                    .is_some_and(|known| !hash_shape(known) || known != hash)
            })
    {
        return Err(ValidationError::FingerprintMismatch);
    }
    Ok(hash.to_owned())
}

fn field_shape(value: &Value, status: &str) -> Result<(), ValidationError> {
    let object = value.as_object().ok_or(ValidationError::InvalidReply)?;
    if value["status"] != status {
        return Err(ValidationError::InvalidReply);
    }
    let has_value = status == "extracted" || status == "inferred";
    if object.contains_key("value") != has_value
        || object.contains_key("reason") == has_value
        || (status != "inferred"
            && (object.contains_key("method") || object.contains_key("confidence")))
    {
        return Err(ValidationError::InvalidReply);
    }
    Ok(())
}

fn scalar(value: &Value) -> Result<&Value, ValidationError> {
    field_shape(value, "extracted")?;
    if value.get("items").is_some() {
        return Err(ValidationError::InvalidReply);
    }
    Ok(&value["value"])
}

fn text(value: &Value, sample: bool) -> Result<String, ValidationError> {
    let value = value.as_str().ok_or(ValidationError::InvalidReply)?;
    if value.len() > MAX_TEXT_BYTES || value.encode_utf16().count() > 4095 {
        return Err(ValidationError::LimitExceeded);
    }
    if value.contains('\0') || (sample && value.is_empty()) {
        return Err(ValidationError::InvalidReply);
    }
    Ok(value.to_owned())
}

fn version(value: &Value) -> Result<String, ValidationError> {
    let value = value.as_str().ok_or(ValidationError::InvalidReply)?;
    if value.len() > 32
        || value.split('.').count() != 4
        || value
            .split('.')
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(ValidationError::InvalidReply);
    }
    Ok(value.to_owned())
}

fn unavailable(value: &Value, reason: &str) -> Result<(), ValidationError> {
    field_shape(value, "unavailable")?;
    if value["reason"] != reason {
        return Err(ValidationError::InvalidReply);
    }
    Ok(())
}

fn text_list(
    value: &Value,
    count: usize,
    build: &str,
    sample: bool,
) -> Result<TextList, ValidationError> {
    let reason = if sample {
        "SAMPLE_REFERENCE_NOT_STORED"
    } else {
        "CHANNEL_NAME_NOT_STORED"
    };
    let missing = if sample {
        MissingReason::SampleReferenceNotStored
    } else {
        MissingReason::ChannelNameNotStored
    };
    let status = value["status"]
        .as_str()
        .ok_or(ValidationError::InvalidReply)?;
    field_shape(value, status)?;
    if status == "extracted" {
        if value.get("items").is_some() {
            return Err(ValidationError::InvalidReply);
        }
        let values = value["value"]
            .as_array()
            .ok_or(ValidationError::InvalidReply)?;
        if values.len() != count {
            return Err(ValidationError::InvalidReply);
        }
        let entries = values
            .iter()
            .map(|value| text(value, sample).map(TextField::Extracted))
            .collect::<Result<_, _>>()?;
        return Ok(TextList {
            status: ListStatus::Extracted,
            entries,
        });
    }
    if status != "unavailable" && (status != "inferred" || sample) {
        return Err(ValidationError::InvalidReply);
    }
    if status == "unavailable" {
        unavailable(value, reason)?;
    }
    let items = value["items"]
        .as_array()
        .ok_or(ValidationError::InvalidReply)?;
    if items.len() != count {
        return Err(ValidationError::InvalidReply);
    }
    let mut entries = Vec::with_capacity(count);
    let mut inferred = 0;
    let mut missing_count = 0;
    let mut extracted = 0;
    for item in items {
        if item.get("items").is_some() {
            return Err(ValidationError::InvalidReply);
        }
        match item["status"].as_str() {
            Some("extracted") => {
                extracted += 1;
                entries.push(TextField::Extracted(text(scalar(item)?, sample)?));
            }
            Some("unavailable") => {
                unavailable(item, reason)?;
                missing_count += 1;
                entries.push(TextField::Unavailable(missing));
            }
            Some("inferred") if !sample && matches!(build, "25.1.3.4922" | "26.1.0.5530") => {
                field_shape(item, "inferred")?;
                inferred += 1;
                let expected_name = if inferred == 1 {
                    "Sampler".to_owned()
                } else {
                    format!("Sampler {inferred}")
                };
                let confidence = if inferred == 1 {
                    Confidence::High
                } else {
                    Confidence::Medium
                };
                if item["value"] != expected_name
                    || item["method"] != "sampler-default-for-known-build"
                    || item["confidence"] != if inferred == 1 { "high" } else { "medium" }
                {
                    return Err(ValidationError::InvalidReply);
                }
                entries.push(TextField::Inferred {
                    value: expected_name,
                    method: NameInference::SamplerDefaultForKnownBuild,
                    confidence,
                });
            }
            _ => return Err(ValidationError::InvalidReply),
        }
    }
    let list_status = if status == "unavailable" {
        if missing_count == 0 {
            return Err(ValidationError::InvalidReply);
        }
        ListStatus::Unavailable(missing)
    } else {
        if inferred == 0 || missing_count != 0 {
            return Err(ValidationError::InvalidReply);
        }
        let values = value["value"]
            .as_array()
            .ok_or(ValidationError::InvalidReply)?;
        if values.len() != count
            || values
                .iter()
                .zip(&entries)
                .any(|(value, entry)| value.as_str() != entry.value())
        {
            return Err(ValidationError::InvalidReply);
        }
        let (method, method_name) = if extracted > 0 {
            (
                NameInference::MixedExtractedAndSamplerDefault,
                "mixed-extracted-and-sampler-default",
            )
        } else {
            (
                NameInference::SamplerDefaultForKnownBuild,
                "sampler-default-for-known-build",
            )
        };
        let confidence = if extracted > 0 || inferred > 1 {
            Confidence::Medium
        } else {
            Confidence::High
        };
        if value["method"] != method_name
            || value["confidence"]
                != if confidence == Confidence::High {
                    "high"
                } else {
                    "medium"
                }
        {
            return Err(ValidationError::InvalidReply);
        }
        ListStatus::Inferred { method, confidence }
    };
    Ok(TextList {
        status: list_status,
        entries,
    })
}

/// Validate a parse reply, retaining only the initial five fields. Extension
/// metadata (patterns, generators and playlist/timing) is deliberately dropped.
pub fn validate_reply(
    reply: ProtocolReply,
    capabilities: &ParserCapabilities,
    captured: &ParseContext,
    current: &ParseContext,
) -> Result<ValidatedReply, ValidationError> {
    fresh(captured, current)?;
    let value = match reply {
        ProtocolReply::Rejected { code } => {
            return Ok(ValidatedReply::Rejected(parser_code(&code)?));
        }
        ProtocolReply::Result(value) => value,
    };
    let outcome = value["outcome"]
        .as_str()
        .ok_or(ValidationError::InvalidReply)?;
    if outcome == "failed" {
        let code = parser_code(
            value["code"]
                .as_str()
                .ok_or(ValidationError::InvalidReply)?,
        )?;
        for name in INITIAL_FIELDS {
            field_shape(&value[name], "failed")?;
            if value[name]["reason"] != code.as_str() || value[name].get("items").is_some() {
                return Err(ValidationError::InvalidReply);
            }
        }
        empty_diagnostics(&value)?;
        if let Some(input) = value.get("inputFingerprint") {
            fingerprint(input, capabilities, captured, current)?;
        }
        return Ok(ValidatedReply::Failed(code));
    }
    let saved_version = version(scalar(&value["savedVersion"])?)?;
    let content_sha256 = fingerprint(&value["inputFingerprint"], capabilities, captured, current)?;
    if outcome == "unsupported" {
        if value["code"] != "UNSUPPORTED_SAVED_VERSION"
            || crate::supported_saved_version(&saved_version)
        {
            return Err(ValidationError::InvalidReply);
        }
        for name in INITIAL_FIELDS.into_iter().skip(1) {
            field_shape(&value[name], "unsupported")?;
            if value[name]["reason"] != "UNSUPPORTED_SAVED_VERSION"
                || value[name].get("items").is_some()
            {
                return Err(ValidationError::InvalidReply);
            }
        }
        empty_diagnostics(&value)?;
        return Ok(ValidatedReply::UnsupportedSavedVersion(saved_version));
    }
    if !matches!(outcome, "complete" | "partial")
        || value.get("code").is_some()
        || !crate::supported_saved_version(&saved_version)
    {
        return Err(ValidationError::InvalidReply);
    }
    let count = scalar(&value["channelCount"])?
        .as_u64()
        .ok_or(ValidationError::InvalidReply)?;
    if count > capabilities.max_channels as u64 {
        return Err(ValidationError::LimitExceeded);
    }
    let event_count = value["eventCount"]
        .as_u64()
        .ok_or(ValidationError::InvalidReply)?;
    if event_count == 0 || event_count > capabilities.max_events {
        return Err(ValidationError::LimitExceeded);
    }
    if value
        .get("parseElapsedMicros")
        .is_some_and(|elapsed| elapsed.as_u64().is_none_or(|number| number > 60_000_000))
    {
        return Err(ValidationError::InvalidReply);
    }
    let base_tempo_bpm = if value["baseTempoBpm"]["status"] == "extracted" {
        let bpm = scalar(&value["baseTempoBpm"])?
            .as_f64()
            .filter(|number| number.is_finite() && (1.0..=999.0).contains(number))
            .ok_or(ValidationError::InvalidReply)?;
        Field::Extracted(bpm)
    } else {
        unavailable(&value["baseTempoBpm"], "BASE_TEMPO_ABSENT")?;
        if value["baseTempoBpm"].get("items").is_some() {
            return Err(ValidationError::InvalidReply);
        }
        Field::Unavailable(MissingReason::BaseTempoAbsent)
    };
    let channel_names = text_list(
        &value["channelNames"],
        count as usize,
        &saved_version,
        false,
    )?;
    let sample_references = text_list(
        &value["sampleReferences"],
        count as usize,
        &saved_version,
        true,
    )?;
    let raw_diagnostics = value["diagnostics"]
        .as_array()
        .ok_or(ValidationError::InvalidReply)?;
    if raw_diagnostics.len() > MAX_DIAGNOSTICS || raw_diagnostics.len() as u64 > event_count {
        return Err(ValidationError::LimitExceeded);
    }
    let mut diagnostics = Vec::with_capacity(raw_diagnostics.len());
    for diagnostic in raw_diagnostics {
        if diagnostic["code"] != "UNSUPPORTED_EVENT" || diagnostic["eventId"].as_u64() != Some(255)
        {
            return Err(ValidationError::InvalidReply);
        }
        diagnostics.push(Diagnostic::UnsupportedEvent255);
    }
    if (outcome == "complete") != diagnostics.is_empty() {
        return Err(ValidationError::InvalidReply);
    }
    Ok(ValidatedReply::Metadata(ValidatedMetadata {
        outcome: if diagnostics.is_empty() {
            MetadataOutcome::Complete
        } else {
            MetadataOutcome::Partial
        },
        saved_version,
        base_tempo_bpm,
        channel_count: count as u16,
        channel_names,
        sample_references,
        content_sha256,
        diagnostics,
    }))
}

fn empty_diagnostics(value: &Value) -> Result<(), ValidationError> {
    if value["diagnostics"]
        .as_array()
        .is_none_or(|items| !items.is_empty())
    {
        return Err(ValidationError::InvalidReply);
    }
    Ok(())
}
