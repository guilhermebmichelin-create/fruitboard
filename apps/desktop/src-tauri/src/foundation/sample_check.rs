//! Explicit saved-path command. Renderer inputs are identities, never paths.
use super::{AppError, CommandEnvelope, CommandRuntime};
use crate::{decode_request, invalid_request};
use fruitboard_storage::Database;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[cfg(feature = "analysis-jobs")]
mod host;

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Request {
    schema_version: u64,
    root_id: String,
    location_id: String,
    snapshot_id: String,
    request_id: String,
    expected_byte_size: String,
    expected_modified_at: String,
}
fn decode(value: Option<Value>) -> Result<Request, AppError> {
    if value
        .as_ref()
        .and_then(|v| serde_json::to_vec(v).ok())
        .is_none_or(|v| v.len() > 4096)
    {
        return Err(invalid_request());
    }
    let request: Request = decode_request(value)?;
    let valid_id = |id: &str| {
        !id.is_empty()
            && id.len() <= 128
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    };
    if request.schema_version != super::COMMAND_SCHEMA_VERSION
        || [
            &request.root_id,
            &request.location_id,
            &request.snapshot_id,
            &request.request_id,
        ]
        .iter()
        .any(|id| !valid_id(id))
        || request
            .expected_byte_size
            .parse::<u64>()
            .ok()
            .is_none_or(|v| v.to_string() != request.expected_byte_size)
        || request.expected_modified_at.is_empty()
        || request.expected_modified_at.len() > 40
    {
        return Err(invalid_request());
    }
    Ok(request)
}

#[derive(Serialize)]
pub(crate) struct Response {
    request: Request,
    state: &'static str,
    #[cfg(feature = "analysis-jobs")]
    reason: Option<fruitboard_sample_presence::UncheckedReason>,
    #[cfg(not(feature = "analysis-jobs"))]
    reason: Option<&'static str>,
    #[cfg(feature = "analysis-jobs")]
    report: Option<fruitboard_sample_presence::Report>,
    #[cfg(not(feature = "analysis-jobs"))]
    report: Option<()>,
}
impl Response {
    fn state(request: Request, state: &'static str) -> Self {
        Self {
            request,
            state,
            reason: None,
            report: None,
        }
    }
}

pub(crate) enum Prepared {
    Immediate(Response),
    #[cfg(feature = "analysis-jobs")]
    Worker(host::Ticket),
}
pub(crate) struct Service {
    #[cfg(feature = "analysis-jobs")]
    host: Arc<host::Host>,
}
impl Service {
    pub(crate) fn new(database: Arc<Mutex<Database>>) -> Self {
        #[cfg(not(feature = "analysis-jobs"))]
        let _ = database;
        Self {
            #[cfg(feature = "analysis-jobs")]
            host: host::Host::new(database),
        }
    }
    pub(crate) fn begin(&self, value: Option<Value>, now: u64) -> Result<Prepared, AppError> {
        let request = decode(value)?;
        #[cfg(not(feature = "analysis-jobs"))]
        {
            let _ = now;
            Ok(Prepared::Immediate(Response::state(request, "disabled")))
        }
        #[cfg(feature = "analysis-jobs")]
        self.host.begin(request, now)
    }
    pub(crate) fn cancel(
        &self,
        commands: &CommandRuntime,
        value: Option<Value>,
    ) -> CommandEnvelope<Response> {
        commands.execute("cancel_sample_check", || {
            let request = decode(value)?;
            #[cfg(not(feature = "analysis-jobs"))]
            let state = "disabled";
            #[cfg(feature = "analysis-jobs")]
            let state = {
                self.host.cancel(&request);
                "cancelled"
            };
            Ok(Response::state(request, state))
        })
    }
    pub(crate) fn shutdown(&self) {
        #[cfg(feature = "analysis-jobs")]
        self.host.shutdown();
    }
}

#[cfg(test)]
mod request_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn strict_request_rejects_paths_extensions_oversize_and_noncanonical_identity() {
        let valid = json!({"schemaVersion":1,"rootId":"root","locationId":"location","snapshotId":"snapshot",
            "requestId":"request","expectedByteSize":"123","expectedModifiedAt":"2026-01-02T00:00:00Z"});
        assert!(decode(Some(valid.clone())).is_ok());
        for (key, value) in [
            ("path", json!("C:\\Private\\sample.wav")),
            ("references", json!([])),
            ("requestId", json!("request/path")),
            ("expectedByteSize", json!("01")),
            ("schemaVersion", json!(2)),
            ("rootId", json!("x".repeat(4100))),
        ] {
            let mut request = valid.clone();
            request[key] = value;
            assert!(decode(Some(request)).is_err());
        }
    }
}
