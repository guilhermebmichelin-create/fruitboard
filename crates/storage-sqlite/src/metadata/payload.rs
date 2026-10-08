//! Explicit private projection for local persistence; no automatic metadata
//! Debug/Serialize and no raw parser extensions or diagnostics are retained.
use super::MAX_METADATA_JSON_BYTES;
use crate::{Result, StorageError};
use fruitboard_flp_parser::validation::*;
use serde::Serialize;
use serde_json::{Value, json};
use std::io::{self, Write};

struct BoundedJson(Vec<u8>);
impl Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > MAX_METADATA_JSON_BYTES {
            return Err(io::Error::other("metadata projection limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn check_reply_size(value: &Value) -> Result<()> {
    serde_json::to_writer(BoundedJson(Vec::new()), value).map_err(|_| StorageError::InvalidSchema)
}

fn missing(reason: MissingReason) -> &'static str {
    match reason {
        MissingReason::BaseTempoAbsent => "BASE_TEMPO_ABSENT",
        MissingReason::ChannelNameNotStored => "CHANNEL_NAME_NOT_STORED",
        MissingReason::SampleReferenceNotStored => "SAMPLE_REFERENCE_NOT_STORED",
    }
}
fn inference(method: NameInference) -> &'static str {
    match method {
        NameInference::SamplerLabelForKnownBuild => "sampler-label-for-known-build",
        NameInference::MixedExtractedAndSamplerLabel => "mixed-extracted-and-sampler-label",
    }
}
fn confidence(value: Confidence) -> &'static str {
    match value {
        Confidence::High => "high",
        Confidence::Medium => "medium",
    }
}
fn project_reason(reason: ProjectReason) -> &'static str {
    match reason {
        ProjectReason::ProjectInfoNotStored => "PROJECT_INFO_NOT_STORED",
        ProjectReason::ProjectInfoLayoutUnsupported => "PROJECT_INFO_LAYOUT_UNSUPPORTED",
        ProjectReason::MultipleProjectInfoRecords => "MULTIPLE_PROJECT_INFO_RECORDS",
        ProjectReason::ProjectCreationDateInvalid => "PROJECT_CREATION_DATE_INVALID",
        ProjectReason::ProjectTimeSpentInvalid => "PROJECT_TIME_SPENT_INVALID",
        ProjectReason::FilesystemCreationTimeUnavailable => "FILESYSTEM_CREATION_TIME_UNAVAILABLE",
        ProjectReason::PluginNameNotStored => "PLUGIN_NAME_NOT_STORED",
        ProjectReason::PluginVendorNotStored => "PLUGIN_VENDOR_NOT_STORED",
        ProjectReason::PluginNameEncodingUnsupported => "PLUGIN_NAME_ENCODING_UNSUPPORTED",
        ProjectReason::VstMetadataUnsupported => "VST_METADATA_UNSUPPORTED",
        ProjectReason::MultipleVstMetadataRecords => "MULTIPLE_VST_METADATA_RECORDS",
        ProjectReason::PlaylistClipsUnverifiedBuild => "PLAYLIST_CLIPS_UNVERIFIED_BUILD",
        ProjectReason::MultipleArrangementsUnverified => "MULTIPLE_ARRANGEMENTS_UNVERIFIED",
        ProjectReason::PlaylistDataNotStored => "PLAYLIST_DATA_NOT_STORED",
        ProjectReason::PlaylistClipLayoutUnverified => "PLAYLIST_CLIP_LAYOUT_UNVERIFIED",
        ProjectReason::PlaylistClipKindUnverified => "PLAYLIST_CLIP_KIND_UNVERIFIED",
        ProjectReason::PlaylistPatternReferenceUnverified => {
            "PLAYLIST_PATTERN_REFERENCE_UNVERIFIED"
        }
        ProjectReason::ZeroLengthPlaylistClipUnverified => "ZERO_LENGTH_PLAYLIST_CLIP_UNVERIFIED",
        ProjectReason::NoPlaylistPatternClips => "NO_PLAYLIST_PATTERN_CLIPS",
        ProjectReason::PpqUnverified => "PPQ_UNVERIFIED",
        ProjectReason::MeterUnverified => "METER_UNVERIFIED",
        ProjectReason::BaseTempoAbsent => "BASE_TEMPO_ABSENT",
    }
}
fn field<T: Serialize>(field: &Field<T>) -> Value {
    match field {
        Field::Extracted(value) => json!({"status":"extracted","value":value}),
        Field::Unavailable(reason) => json!({"status":"unavailable","reason":missing(*reason)}),
    }
}
fn text(field: &TextField) -> Value {
    match field {
        TextField::Extracted(value) => json!({"status":"extracted","value":value}),
        TextField::Inferred {
            value,
            method,
            confidence: level,
        } => {
            json!({"status":"inferred","value":value,"method":inference(*method),"confidence":confidence(*level)})
        }
        TextField::Unavailable(reason) => json!({"status":"unavailable","reason":missing(*reason)}),
    }
}
fn list(list: &TextList) -> Value {
    let mut value = match list.status() {
        ListStatus::Extracted => json!({"status":"extracted"}),
        ListStatus::Inferred {
            method,
            confidence: level,
        } => json!({"status":"inferred","method":inference(method),"confidence":confidence(level)}),
        ListStatus::Unavailable(reason) => json!({"status":"unavailable","reason":missing(reason)}),
    };
    value["items"] = Value::Array(list.entries().iter().map(text).collect());
    value
}
fn project<T: Serialize>(field: &ProjectField<T>) -> Value {
    match field {
        ProjectField::Extracted(value) => json!({"status":"extracted","value":value}),
        ProjectField::Unavailable(reason) => {
            json!({"status":"unavailable","reason":project_reason(*reason)})
        }
        ProjectField::Unsupported(reason) => {
            json!({"status":"unsupported","reason":project_reason(*reason)})
        }
        ProjectField::Inferred { value, method } => {
            let (method, confidence, assumptions) = match method {
                ProjectInference::SamplerDefault => {
                    ("sampler-default-for-known-build", "high", None)
                }
                ProjectInference::ConstantBaseTempo => (
                    "constant-base-tempo-over-pattern-clips",
                    "low",
                    Some([
                        "tempo remains at base BPM",
                        "only verified pattern clips define span",
                    ]),
                ),
                ProjectInference::VerifiedMeter => {
                    ("pattern-clip-span-at-verified-meter", "low", None)
                }
            };
            let mut result =
                json!({"status":"inferred","value":value,"method":method,"confidence":confidence});
            if let Some(assumptions) = assumptions {
                result["assumptions"] = json!(assumptions);
            }
            result
        }
    }
}

pub(super) fn encode(metadata: &ValidatedProjectMetadata) -> Result<String> {
    let initial = metadata.initial();
    let plugins: Vec<Value> = metadata.plugins().iter().map(|plugin| json!({
        "className":project(plugin.class_name()), "name":project(plugin.name()), "vendor":project(plugin.vendor()),
    })).collect();
    let diagnostics: Vec<&str> = initial
        .diagnostics()
        .iter()
        .map(|diagnostic| match diagnostic {
            Diagnostic::UnsupportedEvent255 => "UNSUPPORTED_EVENT_255",
        })
        .collect();
    let value = json!({
        "projectionVersion":1,
        "contentSha256":initial.content_sha256(),
        "savedVersion":initial.saved_version(),
        "baseTempoBpm":field(initial.base_tempo_bpm()),
        "channelCount":initial.channel_count(),
        "channelNames":list(initial.channel_names()),
        "channelGeneratorNames":generators(metadata.channel_generators()),
        "patterns":patterns(metadata.patterns()),
        "patternNoteCounts":pattern_notes(metadata.pattern_note_counts()),
        "sampleReferences":list(initial.sample_references()),
        "projectCreatedLocal":project(metadata.project_created_local()),
        "flStudioTimeSpentMs":project(metadata.fl_studio_time_spent_ms()),
        "filesystemCreatedAtMs":project(metadata.filesystem_created_at_ms()),
        "pluginReferences":{"coverage":"top-level-saved-references","items":plugins},
        "playlistPatternEndTick":project(metadata.arrangement_end_tick()),
        "playlistPatternSpanBars":project(metadata.arrangement_span_bars()),
        "playlistPatternNominalSeconds":project(metadata.arrangement_estimated_seconds()),
        "diagnostics":diagnostics,
    });
    let mut writer = BoundedJson(Vec::new());
    serde_json::to_writer(&mut writer, &value).map_err(|_| StorageError::InvalidSchema)?;
    String::from_utf8(writer.0).map_err(|_| StorageError::InvalidSchema)
}

fn pattern_notes(value: &PatternNoteCounts) -> Value {
    let coverage = "stored-pattern-note-records";
    match value {
        PatternNoteCounts::NotAdvertised => {
            json!({"status":"unsupported","reason":"PATTERN_NOTES_NOT_ADVERTISED","coverage":coverage})
        }
        PatternNoteCounts::Unavailable(reason) => {
            json!({"status":"unavailable","reason":reason.as_str(),"coverage":coverage})
        }
        PatternNoteCounts::Unsupported(reason) => {
            json!({"status":"unsupported","reason":reason.as_str(),"coverage":coverage})
        }
        PatternNoteCounts::Entries(items) => {
            let mut missing = false;
            let mut unsupported = false;
            let entries: Vec<Value> = items
                .iter()
                .map(|item| {
                    let count = match item.count() {
                        NoteRecordCount::Extracted(count) => {
                            json!({"status":"extracted","value":count})
                        }
                        NoteRecordCount::Unavailable => {
                            missing = true;
                            json!({"status":"unavailable","reason":"PATTERN_NOTES_NOT_STORED"})
                        }
                        NoteRecordCount::Unsupported(reason) => {
                            unsupported = true;
                            json!({"status":"unsupported","reason":reason.as_str()})
                        }
                    };
                    json!({"patternId":item.id(),"noteCount":count})
                })
                .collect();
            if unsupported || missing {
                json!({"status":if unsupported {"unsupported"} else {"unavailable"},"reason":"PATTERN_NOTE_COUNTS_INCOMPLETE","coverage":coverage,"items":entries})
            } else {
                json!({"status":"extracted","coverage":coverage,"value":entries})
            }
        }
    }
}

fn generators(value: &ChannelGenerators) -> Value {
    match value {
        ChannelGenerators::Unsupported(reason) => {
            json!({"status":"unsupported","reason":reason.as_str()})
        }
        ChannelGenerators::Items(items) => json!({"items":items.iter().map(|item| match item {
            ChannelGenerator::ExtractedOsc => json!({"status":"extracted","value":"3x Osc"}),
            ChannelGenerator::InferredSampler => json!({"status":"inferred","value":"Sampler","method":"sampler-generator-default-for-known-build","confidence":"high"}),
            ChannelGenerator::Unsupported(reason) => json!({"status":"unsupported","reason":reason.as_str()}),
        }).collect::<Vec<_>>()}),
    }
}

fn patterns(value: &SavedPatterns) -> Value {
    match value {
        SavedPatterns::Unsupported(reason) => {
            json!({"status":"unsupported","reason":reason.as_str()})
        }
        SavedPatterns::Unavailable => {
            json!({"status":"unavailable","reason":"PATTERN_DATA_NOT_STORED"})
        }
        SavedPatterns::Entries(items) => {
            json!({"status":"extracted","count":items.len(),"items":items.iter().map(|item| json!({
            "patternId":item.id(), "name":match item.name() {
                Some(name) => json!({"status":"extracted","value":name}),
                None => json!({"status":"unavailable","reason":"PATTERN_NAME_NOT_STORED"}),
            },
        })).collect::<Vec<_>>()})
        }
    }
}
