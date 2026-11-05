//! Safe operational explanation only; no job/lease IDs or raw parser text in IPC.
use fruitboard_storage::{AnalysisState, Database, MetadataInput};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Summary {
    state: &'static str,
    attempts: Option<u8>,
    reason: Option<&'static str>,
    #[serde(rename = "savedVersion", skip_serializing_if = "Option::is_none")]
    saved_version: Option<String>,
}
impl Summary {
    pub(super) fn not_current() -> Self {
        Self {
            state: "not_current",
            attempts: None,
            reason: None,
            saved_version: None,
        }
    }
}
pub(super) fn read(db: &Database, input: &MetadataInput) -> Result<Summary, ()> {
    let Some(status) = db.current_analysis_status(input).map_err(|_| ())? else {
        return Ok(Summary {
            state: "not_reported",
            attempts: None,
            reason: None,
            saved_version: None,
        });
    };
    let snapshot = status
        .snapshot_id
        .as_deref()
        .map(|id| db.metadata_snapshot(id).map_err(|_| ())?.ok_or(()))
        .transpose()?;
    let code = status.error_code.as_deref().or_else(|| {
        snapshot
            .as_ref()
            .and_then(|s| s.header().parser_code.as_deref())
    });
    let state = match status.state {
        AnalysisState::Queued => "queued",
        AnalysisState::Running => "running",
        AnalysisState::Complete => "complete",
        AnalysisState::Unsupported => "unsupported",
        AnalysisState::Failed => "failed",
        AnalysisState::Cancelled => "cancelled",
        AnalysisState::Stale => "stale",
    };
    let reason = code
        .map(reason)
        .or(if status.state == AnalysisState::Unsupported {
            Some("unsupported_version")
        } else {
            None
        });
    // current_analysis_status already fences this snapshot to the current
    // location/root revisions, fingerprint, adapter and terminal state. Expose
    // only the stored unsupported-version scalar, never raw parser payloads.
    let saved_version =
        if status.state == AnalysisState::Unsupported && reason == Some("unsupported_version") {
            snapshot
                .as_ref()
                .filter(|snapshot| {
                    snapshot.header().parser_code.as_deref() == Some("UNSUPPORTED_SAVED_VERSION")
                })
                .and_then(|snapshot| snapshot.header().unsupported_saved_version.as_deref())
                .map(validated_saved_version)
                .transpose()?
        } else {
            None
        };
    Ok(Summary {
        state,
        attempts: Some(u8::try_from(status.attempt).map_err(|_| ())?),
        reason,
        saved_version,
    })
}
fn validated_saved_version(value: &str) -> Result<String, ()> {
    if value.len() > 32
        || value.split('.').count() != 4
        || value
            .split('.')
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(());
    }
    Ok(value.to_owned())
}
fn reason(code: &str) -> &'static str {
    match code {
        "INTERRUPTED" => "interrupted",
        "SOURCE_UNAVAILABLE"
        | "INPUT_NOT_FILE"
        | "INPUT_OPEN_FAILED"
        | "INPUT_READ_FAILED"
        | "INPUT_METADATA_FAILED"
        | "INPUT_FINGERPRINT_UNAVAILABLE"
        | "INVALID_PATH"
        | "INPUT_REPARSE_POINT"
        | "INPUT_OFFLINE"
        | "INPUT_NOT_LOCAL"
        | "INPUT_AUTHORIZATION_FAILED" => "source_unavailable",
        "SOURCE_CHANGED" | "INPUT_CHANGED" => "source_changed",
        "PARSER_TRANSPORT" => "parser_unavailable",
        "INVALID_REPLY"
        | "INVALID_REQUEST"
        | "INVALID_REQUEST_ID"
        | "UNSUPPORTED_PROTOCOL_VERSION"
        | "UNSUPPORTED_SCHEMA_VERSION"
        | "UNSUPPORTED_FEATURE"
        | "UNKNOWN_METHOD" => "invalid_result",
        "CANCELLED" => "cancelled",
        "UNSUPPORTED_SAVED_VERSION" => "unsupported_version",
        "CHANNEL_COUNT_LIMIT"
        | "EVENT_COUNT_LIMIT"
        | "FILE_SIZE_LIMIT"
        | "PATTERN_COUNT_LIMIT"
        | "PLAYLIST_CLIP_LIMIT"
        | "PLUGIN_REFERENCE_LIMIT"
        | "RESPONSE_LIMIT" => "resource_limit",
        "CHANNEL_COUNT_MISMATCH"
        | "CONFLICTING_PATTERN_NAMES"
        | "EVENT_LENGTH_OUT_OF_BOUNDS"
        | "INVALID_BASE_TEMPO"
        | "INVALID_CHANNEL_NAME"
        | "INVALID_HEADER"
        | "INVALID_PATTERN_ID"
        | "INVALID_PATTERN_NAME"
        | "INVALID_SAMPLE_REFERENCE"
        | "INVALID_SAVED_VERSION"
        | "MALFORMED_EVENT_LENGTH"
        | "MISSING_DATA_CHUNK"
        | "MISSING_SAVED_VERSION"
        | "MULTIPLE_BASE_TEMPOS"
        | "MULTIPLE_CHANNEL_NAMES"
        | "MULTIPLE_SAMPLE_REFERENCES"
        | "PATTERN_NAME_WITHOUT_ID"
        | "PLAYLIST_POSITION_OVERFLOW"
        | "TRUNCATED_DATA_CHUNK" => "invalid_file",
        _ => "unclassified",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_versions_are_bounded_numeric_display_values_only() {
        assert_eq!(
            validated_saved_version("99.1.0.5530").unwrap(),
            "99.1.0.5530"
        );
        for value in [
            "",
            "26.1.0",
            "26..0.5530",
            "26.1.0.5530\n",
            "C:\\Private\\project.flp",
            "26.1.0.5530<script>",
            "123456789012345678901234567890.1.1.1",
        ] {
            assert!(validated_saved_version(value).is_err());
        }
    }
    #[test]
    fn unknown_private_codes_become_fixed_unclassified_reason() {
        assert_eq!(reason("C:\\Private\\project.flp"), "unclassified");
        assert_eq!(reason("UNSUPPORTED_SAVED_VERSION"), "unsupported_version");
        assert_eq!(reason("PARSER_TRANSPORT"), "parser_unavailable");
    }
}
