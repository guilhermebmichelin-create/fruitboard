//! Extended projection for the owner's project-facts request (#232).
use super::*;

pub(super) const FIELDS: [&str; 8] = [
    "projectCreatedLocal",
    "flStudioTimeSpentMs",
    "filesystemCreatedAtMs",
    "pluginReferences",
    "playlistPatternClips",
    "playlistPatternEndTick",
    "playlistPatternNominalSeconds",
    "playlistPatternSpanBars",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectReason {
    ProjectInfoNotStored,
    ProjectInfoLayoutUnsupported,
    MultipleProjectInfoRecords,
    ProjectCreationDateInvalid,
    ProjectTimeSpentInvalid,
    FilesystemCreationTimeUnavailable,
    PluginNameNotStored,
    PluginVendorNotStored,
    PluginNameEncodingUnsupported,
    VstMetadataUnsupported,
    MultipleVstMetadataRecords,
    PlaylistClipsUnverifiedBuild,
    MultipleArrangementsUnverified,
    PlaylistDataNotStored,
    PlaylistClipLayoutUnverified,
    PlaylistClipKindUnverified,
    PlaylistPatternReferenceUnverified,
    ZeroLengthPlaylistClipUnverified,
    NoPlaylistPatternClips,
    PpqUnverified,
    MeterUnverified,
    BaseTempoAbsent,
}

fn reason(value: &Value) -> Result<ProjectReason, ValidationError> {
    Ok(match value.as_str().ok_or(ValidationError::InvalidReply)? {
        "PROJECT_INFO_NOT_STORED" => ProjectReason::ProjectInfoNotStored,
        "PROJECT_INFO_LAYOUT_UNSUPPORTED" => ProjectReason::ProjectInfoLayoutUnsupported,
        "MULTIPLE_PROJECT_INFO_RECORDS" => ProjectReason::MultipleProjectInfoRecords,
        "PROJECT_CREATION_DATE_INVALID" => ProjectReason::ProjectCreationDateInvalid,
        "PROJECT_TIME_SPENT_INVALID" => ProjectReason::ProjectTimeSpentInvalid,
        "FILESYSTEM_CREATION_TIME_UNAVAILABLE" => ProjectReason::FilesystemCreationTimeUnavailable,
        "PLUGIN_NAME_NOT_STORED" => ProjectReason::PluginNameNotStored,
        "PLUGIN_VENDOR_NOT_STORED" => ProjectReason::PluginVendorNotStored,
        "PLUGIN_NAME_ENCODING_UNSUPPORTED" => ProjectReason::PluginNameEncodingUnsupported,
        "VST_METADATA_UNSUPPORTED" => ProjectReason::VstMetadataUnsupported,
        "MULTIPLE_VST_METADATA_RECORDS" => ProjectReason::MultipleVstMetadataRecords,
        "PLAYLIST_CLIPS_UNVERIFIED_BUILD" => ProjectReason::PlaylistClipsUnverifiedBuild,
        "MULTIPLE_ARRANGEMENTS_UNVERIFIED" => ProjectReason::MultipleArrangementsUnverified,
        "PLAYLIST_DATA_NOT_STORED" => ProjectReason::PlaylistDataNotStored,
        "PLAYLIST_CLIP_LAYOUT_UNVERIFIED" => ProjectReason::PlaylistClipLayoutUnverified,
        "PLAYLIST_CLIP_KIND_UNVERIFIED" => ProjectReason::PlaylistClipKindUnverified,
        "PLAYLIST_PATTERN_REFERENCE_UNVERIFIED" => {
            ProjectReason::PlaylistPatternReferenceUnverified
        }
        "ZERO_LENGTH_PLAYLIST_CLIP_UNVERIFIED" => ProjectReason::ZeroLengthPlaylistClipUnverified,
        "NO_PLAYLIST_PATTERN_CLIPS" => ProjectReason::NoPlaylistPatternClips,
        "PPQ_UNVERIFIED" => ProjectReason::PpqUnverified,
        "METER_UNVERIFIED" => ProjectReason::MeterUnverified,
        "BASE_TEMPO_ABSENT" => ProjectReason::BaseTempoAbsent,
        _ => return Err(ValidationError::InvalidReply),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectInference {
    SamplerDefault,
    ConstantBaseTempo,
    VerifiedMeter,
}

pub enum ProjectField<T> {
    Extracted(T),
    Inferred { value: T, method: ProjectInference },
    Unavailable(ProjectReason),
    Unsupported(ProjectReason),
}

impl<T> ProjectField<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Extracted(value) | Self::Inferred { value, .. } => Some(value),
            _ => None,
        }
    }
}

pub struct PluginReference {
    class_name: ProjectField<String>,
    name: ProjectField<String>,
    vendor: ProjectField<String>,
}
impl PluginReference {
    pub fn class_name(&self) -> &ProjectField<String> {
        &self.class_name
    }
    pub fn name(&self) -> &ProjectField<String> {
        &self.name
    }
    pub fn vendor(&self) -> &ProjectField<String> {
        &self.vendor
    }
}

/// All private text is accessible explicitly; no automatic Debug/Serialize.
pub struct ValidatedProjectMetadata {
    initial: ValidatedMetadata,
    file_size_bytes: u64,
    file_modified_at_ms: u64,
    filesystem_created_at_ms: ProjectField<u64>,
    project_created_local: ProjectField<String>,
    fl_studio_time_spent_ms: ProjectField<u64>,
    plugins: Vec<PluginReference>,
    channel_generators: ChannelGenerators,
    patterns: SavedPatterns,
    arrangement_end_tick: ProjectField<u32>,
    arrangement_span_bars: ProjectField<f64>,
    arrangement_estimated_seconds: ProjectField<f64>,
}
impl ValidatedProjectMetadata {
    pub fn initial(&self) -> &ValidatedMetadata {
        &self.initial
    }
    pub fn file_size_bytes(&self) -> u64 {
        self.file_size_bytes
    }
    pub fn file_modified_at_ms(&self) -> u64 {
        self.file_modified_at_ms
    }
    pub fn filesystem_created_at_ms(&self) -> &ProjectField<u64> {
        &self.filesystem_created_at_ms
    }
    /// Saved local wall time, with no invented UTC offset.
    pub fn project_created_local(&self) -> &ProjectField<String> {
        &self.project_created_local
    }
    /// FL Studio's saved counter, not independently tracked work.
    pub fn fl_studio_time_spent_ms(&self) -> &ProjectField<u64> {
        &self.fl_studio_time_spent_ms
    }
    /// Top-level saved references; not an exhaustive nested/installed inventory.
    pub fn plugins(&self) -> &[PluginReference] {
        &self.plugins
    }
    pub fn channel_generators(&self) -> &ChannelGenerators {
        &self.channel_generators
    }
    pub fn patterns(&self) -> &SavedPatterns {
        &self.patterns
    }
    pub fn arrangement_end_tick(&self) -> &ProjectField<u32> {
        &self.arrangement_end_tick
    }
    pub fn arrangement_span_bars(&self) -> &ProjectField<f64> {
        &self.arrangement_span_bars
    }
    pub fn arrangement_estimated_seconds(&self) -> &ProjectField<f64> {
        &self.arrangement_estimated_seconds
    }
}

pub enum ValidatedProjectReply {
    Metadata(Box<ValidatedProjectMetadata>),
    Failed(ParserCode),
    UnsupportedSavedVersion(String),
    Rejected(ParserCode),
}

fn absent<T>(
    value: &Value,
    unavailable: &[ProjectReason],
    unsupported: &[ProjectReason],
) -> Result<ProjectField<T>, ValidationError> {
    let status = value["status"]
        .as_str()
        .ok_or(ValidationError::InvalidReply)?;
    field_shape(value, status)?;
    if value.get("items").is_some() {
        return Err(ValidationError::InvalidReply);
    }
    let code = reason(&value["reason"])?;
    match status {
        "unavailable" if unavailable.contains(&code) => Ok(ProjectField::Unavailable(code)),
        "unsupported" if unsupported.contains(&code) => Ok(ProjectField::Unsupported(code)),
        _ => Err(ValidationError::InvalidReply),
    }
}

fn integer(
    value: &Value,
    maximum: u64,
    unavailable: &[ProjectReason],
    unsupported: &[ProjectReason],
) -> Result<ProjectField<u64>, ValidationError> {
    if value["status"] != "extracted" {
        return absent(value, unavailable, unsupported);
    }
    let number = scalar(value)?
        .as_u64()
        .filter(|value| *value <= maximum)
        .ok_or(ValidationError::InvalidReply)?;
    Ok(ProjectField::Extracted(number))
}

fn local_date_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 23
        || !bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            10 => *byte == b'T',
            13 | 16 => *byte == b':',
            19 => *byte == b'.',
            _ => byte.is_ascii_digit(),
        })
    {
        return false;
    }
    let number = |start, end| value[start..end].parse::<u32>().unwrap_or(u32::MAX);
    let year = number(0, 4);
    let month = number(5, 7);
    let day = number(8, 10);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let limit = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    (1900..=9999).contains(&year)
        && day > 0
        && day <= limit
        && number(11, 13) < 24
        && number(14, 16) < 60
        && number(17, 19) < 60
}

fn project_created(value: &Value) -> Result<ProjectField<String>, ValidationError> {
    if value["status"] != "extracted" {
        return absent(
            value,
            &[ProjectReason::ProjectInfoNotStored],
            &[
                ProjectReason::ProjectInfoLayoutUnsupported,
                ProjectReason::MultipleProjectInfoRecords,
                ProjectReason::ProjectCreationDateInvalid,
            ],
        );
    }
    let date = scalar(value)?
        .as_str()
        .filter(|value| local_date_shape(value))
        .ok_or(ValidationError::InvalidReply)?;
    if value["timezone"] != "unspecified-local" {
        return Err(ValidationError::InvalidReply);
    }
    Ok(ProjectField::Extracted(date.to_owned()))
}

fn plugin_text(
    value: &Value,
    vendor: bool,
    build: &str,
) -> Result<ProjectField<String>, ValidationError> {
    if value["status"] == "extracted" {
        return Ok(ProjectField::Extracted(text(scalar(value)?, true)?));
    }
    if !vendor && value["status"] == "inferred" {
        field_shape(value, "inferred")?;
        if !matches!(build, "25.1.3.4922" | "26.1.0.5530")
            || value["value"] != "Sampler"
            || value["method"] != "sampler-default-for-known-build"
            || value["confidence"] != "high"
            || value.get("items").is_some()
        {
            return Err(ValidationError::InvalidReply);
        }
        return Ok(ProjectField::Inferred {
            value: "Sampler".into(),
            method: ProjectInference::SamplerDefault,
        });
    }
    absent(
        value,
        &[if vendor {
            ProjectReason::PluginVendorNotStored
        } else {
            ProjectReason::PluginNameNotStored
        }],
        &[
            ProjectReason::PluginNameEncodingUnsupported,
            ProjectReason::VstMetadataUnsupported,
            ProjectReason::MultipleVstMetadataRecords,
        ],
    )
}

fn plugins(
    value: &Value,
    build: &str,
    event_count: u64,
) -> Result<Vec<PluginReference>, ValidationError> {
    let values = scalar(value)?
        .as_array()
        .ok_or(ValidationError::InvalidReply)?;
    if value["coverage"] != "top-level-saved-references" {
        return Err(ValidationError::InvalidReply);
    }
    if values.len() > crate::plugin_references::MAX_PLUGIN_REFERENCES
        || values.len() as u64 > event_count
    {
        return Err(ValidationError::LimitExceeded);
    }
    values
        .iter()
        .map(|value| {
            let class_name = plugin_text(&value["className"], false, build)?;
            if matches!(class_name, ProjectField::Inferred { .. }) {
                return Err(ValidationError::InvalidReply);
            }
            let name = plugin_text(&value["name"], false, build)?;
            let vendor = plugin_text(&value["vendor"], true, build)?;
            let vendor_absent = matches!(
                vendor,
                ProjectField::Unavailable(ProjectReason::PluginVendorNotStored)
            );
            let consistent = match &class_name {
                ProjectField::Extracted(class) if class == "Fruity Wrapper" => {
                    match (&name, &vendor) {
                        (ProjectField::Extracted(_), ProjectField::Extracted(_))
                        | (ProjectField::Extracted(_), ProjectField::Unavailable(_))
                        | (ProjectField::Unavailable(_), ProjectField::Extracted(_))
                        | (ProjectField::Unavailable(_), ProjectField::Unavailable(_)) => true,
                        (
                            ProjectField::Unsupported(ProjectReason::VstMetadataUnsupported),
                            ProjectField::Unavailable(_),
                        ) => true, // Wrapper name with no saved metadata record.
                        (ProjectField::Unsupported(left), ProjectField::Unsupported(right)) => {
                            left == right
                                && matches!(
                                    left,
                                    ProjectReason::VstMetadataUnsupported
                                        | ProjectReason::MultipleVstMetadataRecords
                                )
                        }
                        _ => false,
                    }
                }
                ProjectField::Extracted(class) => {
                    matches!(&name, ProjectField::Extracted(name) if name == class) && vendor_absent
                }
                ProjectField::Unavailable(ProjectReason::PluginNameNotStored) => {
                    matches!(name, ProjectField::Inferred { .. }) && vendor_absent
                }
                ProjectField::Unsupported(ProjectReason::PluginNameEncodingUnsupported) => {
                    matches!(
                        name,
                        ProjectField::Unsupported(ProjectReason::PluginNameEncodingUnsupported)
                    ) && vendor_absent
                }
                _ => false,
            };
            if !consistent {
                return Err(ValidationError::InvalidReply);
            }
            Ok(PluginReference {
                class_name,
                name,
                vendor,
            })
        })
        .collect()
}

const PLAYLIST_UNSUPPORTED: [ProjectReason; 7] = [
    ProjectReason::PlaylistClipsUnverifiedBuild,
    ProjectReason::MultipleArrangementsUnverified,
    ProjectReason::PlaylistClipLayoutUnverified,
    ProjectReason::PlaylistClipKindUnverified,
    ProjectReason::PlaylistPatternReferenceUnverified,
    ProjectReason::ZeroLengthPlaylistClipUnverified,
    ProjectReason::MeterUnverified,
];

fn arrangement_end(value: &Value, maximum: usize) -> Result<ProjectField<u32>, ValidationError> {
    let clips = &value["playlistPatternClips"];
    if clips["status"] != "extracted" {
        let state: ProjectField<u32> = absent(
            clips,
            &[ProjectReason::PlaylistDataNotStored],
            &PLAYLIST_UNSUPPORTED,
        )?;
        let end: ProjectField<u32> = absent(
            &value["playlistPatternEndTick"],
            &[ProjectReason::PlaylistDataNotStored],
            &PLAYLIST_UNSUPPORTED,
        )?;
        if std::mem::discriminant(&state) != std::mem::discriminant(&end)
            || clips["reason"] != value["playlistPatternEndTick"]["reason"]
        {
            return Err(ValidationError::InvalidReply);
        }
        return Ok(end);
    }
    let clips = scalar(clips)?
        .as_array()
        .ok_or(ValidationError::InvalidReply)?;
    if clips.len() > maximum {
        return Err(ValidationError::LimitExceeded);
    }
    let mut end = None;
    for clip in clips {
        let integer = |name| {
            clip[name]
                .as_u64()
                .and_then(|number| u32::try_from(number).ok())
                .ok_or(ValidationError::InvalidReply)
        };
        let id = integer("patternId")?;
        let start = integer("startTick")?;
        let length = integer("lengthTick")?;
        integer("trackToken")?;
        if id == 0 || id > u16::MAX as u32 || length == 0 {
            return Err(ValidationError::InvalidReply);
        }
        let position = start
            .checked_add(length)
            .ok_or(ValidationError::InvalidReply)?;
        end = Some(end.map_or(position, |previous: u32| previous.max(position)));
    }
    match end {
        Some(end) if scalar(&value["playlistPatternEndTick"])?.as_u64() == Some(u64::from(end)) => {
            Ok(ProjectField::Extracted(end))
        }
        Some(_) => Err(ValidationError::InvalidReply),
        None => absent(
            &value["playlistPatternEndTick"],
            &[ProjectReason::NoPlaylistPatternClips],
            &[],
        ),
    }
}

fn estimate(
    value: &Value,
    end: &ProjectField<u32>,
    tempo: &Field<f64>,
    bars: bool,
) -> Result<ProjectField<f64>, ValidationError> {
    if value["status"] == "inferred" {
        field_shape(value, "inferred")?;
        let end = *end.value().ok_or(ValidationError::InvalidReply)?;
        let expected = if bars {
            f64::from(end) / 384.0
        } else {
            let Field::Extracted(bpm) = tempo else {
                return Err(ValidationError::InvalidReply);
            };
            f64::from(end) / 96.0 * 60.0 / bpm
        };
        let number = value["value"]
            .as_f64()
            .filter(|n| n.is_finite() && (*n - expected).abs() <= expected.abs().max(1.0) * 1e-12)
            .ok_or(ValidationError::InvalidReply)?;
        let method = if bars {
            "pattern-clip-span-at-verified-meter"
        } else {
            "constant-base-tempo-over-pattern-clips"
        };
        if value["method"] != method || value["confidence"] != "low" || value.get("items").is_some()
        {
            return Err(ValidationError::InvalidReply);
        }
        if !bars
            && value["assumptions"]
                != serde_json::json!([
                    "tempo remains at base BPM",
                    "only verified pattern clips define span"
                ])
        {
            return Err(ValidationError::InvalidReply);
        }
        return Ok(ProjectField::Inferred {
            value: number,
            method: if bars {
                ProjectInference::VerifiedMeter
            } else {
                ProjectInference::ConstantBaseTempo
            },
        });
    }
    let field = absent(
        value,
        &[
            ProjectReason::PlaylistDataNotStored,
            ProjectReason::NoPlaylistPatternClips,
            ProjectReason::BaseTempoAbsent,
        ],
        &[
            ProjectReason::PlaylistClipsUnverifiedBuild,
            ProjectReason::MultipleArrangementsUnverified,
            ProjectReason::PlaylistClipLayoutUnverified,
            ProjectReason::PlaylistClipKindUnverified,
            ProjectReason::PlaylistPatternReferenceUnverified,
            ProjectReason::ZeroLengthPlaylistClipUnverified,
            ProjectReason::PpqUnverified,
            ProjectReason::MeterUnverified,
        ],
    )?;
    match (&field, end) {
        (ProjectField::Unavailable(left), ProjectField::Unavailable(right))
        | (ProjectField::Unsupported(left), ProjectField::Unsupported(right))
            if left == right => {}
        (
            ProjectField::Unsupported(
                ProjectReason::PpqUnverified | ProjectReason::MeterUnverified,
            ),
            ProjectField::Extracted(_),
        ) => {}
        (ProjectField::Unavailable(ProjectReason::BaseTempoAbsent), ProjectField::Extracted(_))
            if !bars && matches!(tempo, Field::Unavailable(MissingReason::BaseTempoAbsent)) => {}
        _ => return Err(ValidationError::InvalidReply),
    }
    Ok(field)
}

/// The full projection requires a descriptor advertising every project field.
/// Initial-only callers can continue using validate_reply unchanged.
pub fn validate_project_reply(
    reply: ProtocolReply,
    capabilities: &ParserCapabilities,
    captured: &ParseContext,
    current: &ParseContext,
) -> Result<ValidatedProjectReply, ValidationError> {
    if !capabilities.project_facts {
        return Err(ValidationError::InvalidCapabilities);
    }
    // Keep the original JSON only within this call; neither returned type carries it.
    let raw = match &reply {
        ProtocolReply::Result(value) => Some(value.clone()),
        _ => None,
    };
    let initial = match validate_reply(reply, capabilities, captured, current)? {
        ValidatedReply::Metadata(value) => value,
        ValidatedReply::Failed(code) => return Ok(ValidatedProjectReply::Failed(code)),
        ValidatedReply::UnsupportedSavedVersion(value) => {
            return Ok(ValidatedProjectReply::UnsupportedSavedVersion(value));
        }
        ValidatedReply::Rejected(code) => return Ok(ValidatedProjectReply::Rejected(code)),
    };
    let raw = raw.ok_or(ValidationError::InvalidReply)?;
    let channel_generators = channel_generators::validate(
        &raw["channelGeneratorNames"],
        capabilities.channel_generators,
        initial.saved_version(),
        initial.channel_count() as usize,
    )?;
    let project_created_local = project_created(&raw["projectCreatedLocal"])?;
    let patterns = patterns::validate(
        &raw["patternCount"],
        &raw["patternNames"],
        capabilities.patterns,
        initial.saved_version(),
        capabilities.max_patterns,
    )?;
    let fl_studio_time_spent_ms = integer(
        &raw["flStudioTimeSpentMs"],
        255_611_462_399_999,
        &[ProjectReason::ProjectInfoNotStored],
        &[
            ProjectReason::ProjectInfoLayoutUnsupported,
            ProjectReason::MultipleProjectInfoRecords,
            ProjectReason::ProjectTimeSpentInvalid,
        ],
    )?;
    for code in [
        ProjectReason::ProjectInfoNotStored,
        ProjectReason::ProjectInfoLayoutUnsupported,
        ProjectReason::MultipleProjectInfoRecords,
    ] {
        let created = matches!(&project_created_local,ProjectField::Unavailable(value)|ProjectField::Unsupported(value) if *value==code);
        let spent = matches!(&fl_studio_time_spent_ms,ProjectField::Unavailable(value)|ProjectField::Unsupported(value) if *value==code);
        if created != spent {
            return Err(ValidationError::InvalidReply);
        }
    }
    let filesystem_created_at_ms = integer(
        &raw["filesystemCreatedAtMs"],
        u64::MAX,
        &[ProjectReason::FilesystemCreationTimeUnavailable],
        &[],
    )?;
    let plugins = plugins(
        &raw["pluginReferences"],
        initial.saved_version(),
        raw["eventCount"]
            .as_u64()
            .ok_or(ValidationError::InvalidReply)?,
    )?;
    let arrangement_end_tick = arrangement_end(&raw, capabilities.max_playlist_clips)?;
    if capabilities.patterns && raw["playlistPatternClips"]["status"] == "extracted" {
        let clips = raw["playlistPatternClips"]["value"]
            .as_array()
            .ok_or(ValidationError::InvalidReply)?;
        for clip in clips {
            let SavedPatterns::Entries(items) = &patterns else {
                return Err(ValidationError::InvalidReply);
            };
            let id = clip["patternId"]
                .as_u64()
                .ok_or(ValidationError::InvalidReply)?;
            if !items.iter().any(|pattern| u64::from(pattern.id()) == id) {
                return Err(ValidationError::InvalidReply);
            }
        }
    }
    if !matches!(initial.saved_version(), "26.1.0.5530")
        && !matches!(
            arrangement_end_tick,
            ProjectField::Unsupported(ProjectReason::PlaylistClipsUnverifiedBuild)
        )
    {
        return Err(ValidationError::InvalidReply);
    }
    let arrangement_span_bars = estimate(
        &raw["playlistPatternSpanBars"],
        &arrangement_end_tick,
        initial.base_tempo_bpm(),
        true,
    )?;
    let arrangement_estimated_seconds = estimate(
        &raw["playlistPatternNominalSeconds"],
        &arrangement_end_tick,
        initial.base_tempo_bpm(),
        false,
    )?;
    if let ProjectField::Unsupported(
        ProjectReason::PpqUnverified | ProjectReason::MeterUnverified,
    ) = &arrangement_span_bars
    {
        if raw["playlistPatternSpanBars"]["reason"]
            != raw["playlistPatternNominalSeconds"]["reason"]
        {
            return Err(ValidationError::InvalidReply);
        }
    } else if matches!(
        arrangement_estimated_seconds,
        ProjectField::Unsupported(ProjectReason::PpqUnverified | ProjectReason::MeterUnverified)
    ) {
        return Err(ValidationError::InvalidReply);
    }
    Ok(ValidatedProjectReply::Metadata(Box::new(
        ValidatedProjectMetadata {
            initial,
            file_size_bytes: captured.expected.size,
            file_modified_at_ms: captured.expected.modified_at_ms,
            filesystem_created_at_ms,
            project_created_local,
            fl_studio_time_spent_ms,
            plugins,
            channel_generators,
            patterns,
            arrangement_end_tick,
            arrangement_span_bars,
            arrangement_estimated_seconds,
        },
    )))
}
