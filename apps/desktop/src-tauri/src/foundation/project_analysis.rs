//! One explicit local analysis request. No renderer path or parser selection.
#[cfg(feature = "analysis-jobs")]
use super::AppError;
use super::{CommandEnvelope, CommandRuntime};
use crate::{decode_request, invalid_request};
use fruitboard_storage::Database;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg(feature = "analysis-jobs")]
pub(super) struct RequestInfo {
    state: &'static str,
    request_key: Option<String>,
}
#[cfg(feature = "analysis-jobs")]
impl RequestInfo {
    pub(super) fn blocked(state: &'static str) -> Self {
        Self {
            state,
            request_key: None,
        }
    }
}
#[cfg(feature = "analysis-jobs")]
pub(super) fn info(
    db: &Database,
    input: &fruitboard_storage::MetadataInput,
    runtime_available: bool,
) -> Result<RequestInfo, ()> {
    if !runtime_available {
        return Ok(RequestInfo::blocked("runtime_unavailable"));
    }
    Ok(match db.analysis_request(input).map_err(|_| ())? {
        fruitboard_storage::AnalysisRequest::Ready { key } => RequestInfo {
            state: "ready",
            request_key: Some(key),
        },
        fruitboard_storage::AnalysisRequest::Blocked(blocked) => {
            RequestInfo::blocked(block_name(blocked))
        }
    })
}
#[cfg(feature = "analysis-jobs")]
fn block_name(block: fruitboard_storage::AnalysisRequestBlock) -> &'static str {
    use fruitboard_storage::AnalysisRequestBlock::*;
    match block {
        Pending => "pending",
        AttemptLimit => "attempt_limit",
        Unsupported => "unsupported",
        SourceChanged => "source_changed",
        UnqualifiedSource => "unqualified_source",
        TooLarge => "too_large",
        QueueFull => "queue_full",
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Request {
    schema_version: u64,
    root_id: String,
    location_id: String,
    expected_byte_size: String,
    expected_modified_at: String,
    request_key: String,
}
// No Debug: the action key must not be retained in diagnostics.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Response {
    root_id: String,
    location_id: String,
    state: &'static str,
    reason: Option<&'static str>,
}
pub(crate) fn handle_request_project_analysis(
    commands: &CommandRuntime,
    database: &Arc<Mutex<Database>>,
    request: Option<Value>,
    runtime_available: bool,
) -> CommandEnvelope<Response> {
    commands.execute("request_project_analysis", || {
        let request: Request = decode_request(request)?;
        if request.schema_version != super::COMMAND_SCHEMA_VERSION
            || request.root_id.is_empty()
            || request.root_id.len() > 128
            || request.location_id.is_empty()
            || request.location_id.len() > 128
            || request
                .expected_byte_size
                .parse::<u64>()
                .ok()
                .is_none_or(|v| v.to_string() != request.expected_byte_size)
            || request.expected_modified_at.is_empty()
            || request.expected_modified_at.len() > 40
            || request.request_key.len() != 64
            || !request
                .request_key
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid_request());
        }
        #[cfg(not(feature = "analysis-jobs"))]
        let (state, reason) = {
            let _ = (database, runtime_available);
            ("disabled", None)
        };
        #[cfg(feature = "analysis-jobs")]
        let (state, reason) = {
            if !runtime_available {
                return Ok(Response {
                    root_id: request.root_id,
                    location_id: request.location_id,
                    state: "blocked",
                    reason: Some("runtime_unavailable"),
                });
            }
            let mut db = database.lock().map_err(|_| crate::storage_failed())?;
            let input = db
                .capture_metadata_input(&request.root_id, &request.location_id)
                .map_err(crate::map_storage_error)?;
            if input.byte_size().to_string() != request.expected_byte_size
                || super::scan_console::unix_ns_to_rfc3339(input.modified_at_ns())
                    != request.expected_modified_at
            {
                return Err(AppError::new(
                    super::ErrorCode::Conflict,
                    super::DiagnosticCode::ScanRootConflict,
                ));
            }
            let now = i64::try_from(commands.now_millis()).map_err(|_| crate::storage_failed())?;
            match db
                .request_analysis_job(&input, &request.request_key, now)
                .map_err(crate::map_storage_error)?
            {
                fruitboard_storage::AnalysisRequestOutcome::Queued => ("queued", None),
                fruitboard_storage::AnalysisRequestOutcome::Blocked(blocked) => {
                    ("blocked", Some(block_name(blocked)))
                }
            }
        };
        Ok(Response {
            root_id: request.root_id,
            location_id: request.location_id,
            state,
            reason,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
    use serde_json::json;
    #[test]
    fn request_rejects_paths_raw_parameters_and_invalid_keys_without_logging_values() {
        let directory = std::env::temp_dir().join(format!(
            "fruitboard-analysis-request-{}",
            uuid::Uuid::now_v7()
        ));
        let database = Arc::new(Mutex::new(Database::open(&directory).unwrap()));
        let logs = Arc::new(RecordingLogSink::default());
        let runtime = CommandRuntime::new(
            Arc::new(FakeClock::new(100)),
            Arc::new(FakeIdGenerator::new(1)),
            logs.clone(),
        );
        let valid = json!({"schemaVersion":1,"rootId":"unknown","locationId":"unknown","expectedByteSize":"1024","expectedModifiedAt":"2026-01-02T00:00:00Z","requestKey":"a".repeat(64)});
        for (field, value) in [
            ("path", json!("C:\\Private\\project.flp")),
            ("parser", json!("other.exe")),
            ("requestKey", json!("private token")),
            ("expectedByteSize", json!("01")),
            ("schemaVersion", json!(2)),
        ] {
            let mut request = valid.clone();
            request[field] = value;
            let result = serde_json::to_value(handle_request_project_analysis(
                &runtime,
                &database,
                Some(request),
                true,
            ))
            .unwrap();
            assert_eq!(result["error"]["code"], "invalid_request");
            assert!(!result.to_string().contains("Private"));
        }
        let result = serde_json::to_value(handle_request_project_analysis(
            &runtime,
            &database,
            Some(valid),
            false,
        ))
        .unwrap();
        #[cfg(not(feature = "analysis-jobs"))]
        assert_eq!(result["data"]["state"], "disabled");
        #[cfg(feature = "analysis-jobs")]
        assert_eq!(result["data"]["reason"], "runtime_unavailable");
        let events = format!("{:?}", logs.events());
        assert!(!events.contains("Private"));
        assert!(!events.contains("private token"));
        assert!(!events.contains(&"a".repeat(64)));
        drop(database);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
