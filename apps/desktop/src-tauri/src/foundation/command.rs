use super::clock::Clock;
use super::errors::{AppError, ErrorCode, UserFacingError};
use super::identifiers::{IdGenerator, IdKind, OpaqueId};
use super::logging::{LogEventKind, LogLevel, LogSink, OperationalLogEvent};
use serde::Serialize;
use std::panic::{AssertUnwindSafe, UnwindSafe, catch_unwind};
use std::sync::Arc;

pub const COMMAND_SCHEMA_VERSION: u64 = 1;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CommandEnvelope<T> {
    Ok {
        #[serde(rename = "schemaVersion")]
        schema_version: u64,
        #[serde(rename = "correlationId")]
        correlation_id: OpaqueId,
        data: T,
    },
    Error {
        #[serde(rename = "schemaVersion")]
        schema_version: u64,
        #[serde(rename = "correlationId")]
        correlation_id: OpaqueId,
        error: UserFacingError,
    },
}

pub struct CommandRuntime {
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
    logs: Arc<dyn LogSink>,
}

struct CommandLog {
    level: LogLevel,
    kind: LogEventKind,
    operation: &'static str,
    correlation_id: OpaqueId,
    error_code: Option<ErrorCode>,
    diagnostic: Option<String>,
}

impl CommandRuntime {
    pub(crate) fn new(
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
        logs: Arc<dyn LogSink>,
    ) -> Self {
        Self { clock, ids, logs }
    }

    pub fn execute<T, Operation>(
        &self,
        operation_name: &'static str,
        operation: Operation,
    ) -> CommandEnvelope<T>
    where
        Operation: FnOnce() -> Result<T, AppError> + UnwindSafe,
    {
        let correlation_id = self.ids.next_id(IdKind::Correlation);
        let outcome = catch_unwind(operation);

        match outcome {
            Ok(Ok(data)) => {
                self.write_log(CommandLog {
                    level: LogLevel::Info,
                    kind: LogEventKind::Completed,
                    operation: operation_name,
                    correlation_id: correlation_id.clone(),
                    error_code: None,
                    diagnostic: None,
                });
                CommandEnvelope::Ok {
                    schema_version: COMMAND_SCHEMA_VERSION,
                    correlation_id,
                    data,
                }
            }
            Ok(Err(error)) => {
                let code = error.user().code;
                let rejected = matches!(code, ErrorCode::InvalidRequest | ErrorCode::Cancelled);
                self.write_log(CommandLog {
                    level: if rejected {
                        LogLevel::Warning
                    } else {
                        LogLevel::Error
                    },
                    kind: if rejected {
                        LogEventKind::Rejected
                    } else {
                        LogEventKind::Failed
                    },
                    operation: operation_name,
                    correlation_id: correlation_id.clone(),
                    error_code: Some(code),
                    diagnostic: Some(error.diagnostic().summary.clone()),
                });
                CommandEnvelope::Error {
                    schema_version: COMMAND_SCHEMA_VERSION,
                    correlation_id,
                    error: error.into_user(),
                }
            }
            Err(_) => {
                let error = AppError::unknown("panic caught at the native command boundary");
                self.write_log(CommandLog {
                    level: LogLevel::Error,
                    kind: LogEventKind::Panicked,
                    operation: operation_name,
                    correlation_id: correlation_id.clone(),
                    error_code: Some(ErrorCode::Internal),
                    diagnostic: Some(error.diagnostic().summary.clone()),
                });
                CommandEnvelope::Error {
                    schema_version: COMMAND_SCHEMA_VERSION,
                    correlation_id,
                    error: error.into_user(),
                }
            }
        }
    }

    fn write_log(&self, context: CommandLog) {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let event = OperationalLogEvent {
                timestamp_millis: self.clock.now_millis(),
                level: context.level,
                kind: context.kind,
                operation: context.operation,
                correlation_id: context.correlation_id,
                job_id: None,
                error_code: context.error_code,
                diagnostic: context.diagnostic,
            };
            let _ = self.logs.write(&event);
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::test_support::{
        FakeClock, FakeError, FakeIdGenerator, RecordingLogSink,
    };
    use serde_json::json;
    use std::io;

    #[derive(Debug)]
    struct FailingLogSink;

    impl LogSink for FailingLogSink {
        fn write(&self, _event: &OperationalLogEvent) -> io::Result<()> {
            Err(io::Error::other("synthetic log failure"))
        }
    }

    fn runtime() -> (CommandRuntime, Arc<RecordingLogSink>) {
        let logs = Arc::new(RecordingLogSink::default());
        (
            CommandRuntime::new(
                Arc::new(FakeClock::new(1_234)),
                Arc::new(FakeIdGenerator::new(1)),
                logs.clone(),
            ),
            logs,
        )
    }

    #[test]
    fn serializes_a_stable_success_envelope() {
        let (runtime, logs) = runtime();
        let response = runtime.execute("test_command", || Ok(json!({ "value": 7 })));

        assert_eq!(
            serde_json::to_value(response).expect("command response should serialize"),
            json!({
                "status": "ok",
                "schemaVersion": COMMAND_SCHEMA_VERSION,
                "correlationId": "correlation_00000000000000000000000000000001",
                "data": { "value": 7 },
            })
        );
        assert_eq!(logs.events()[0].kind, LogEventKind::Completed);
    }

    #[test]
    fn serializes_only_the_user_safe_error() {
        let (runtime, logs) = runtime();
        let response = runtime.execute::<(), _>("test_command", || {
            Err(AppError::new(
                ErrorCode::Unavailable,
                "access_token=secret C:\\Users\\person\\private.flp",
            ))
        });
        let serialized = serde_json::to_value(response).expect("command error should serialize");

        assert_eq!(
            serialized,
            json!({
                "status": "error",
                "schemaVersion": COMMAND_SCHEMA_VERSION,
                "correlationId": "correlation_00000000000000000000000000000001",
                "error": {
                    "code": "unavailable",
                    "message": "The requested service is temporarily unavailable.",
                    "retryable": true,
                },
            })
        );
        assert!(!serialized.to_string().contains("secret"));
        assert_eq!(logs.events()[0].kind, LogEventKind::Failed);
    }

    #[test]
    fn maps_unknown_errors_to_internal_without_exposing_details() {
        let (runtime, _) = runtime();
        let response = runtime.execute::<(), _>("test_command", || {
            Err(AppError::unknown(FakeError::new("private adapter detail")))
        });
        let serialized = serde_json::to_value(response).expect("command error should serialize");

        assert_eq!(serialized["error"]["code"], "internal");
        assert_eq!(
            serialized["error"]["message"],
            "Fruitboard could not complete the request."
        );
        assert!(!serialized.to_string().contains("private adapter detail"));
    }

    #[test]
    fn catches_panics_without_crashing_the_command_host() {
        let (runtime, logs) = runtime();
        let response = runtime.execute::<(), _>("test_command", || {
            panic!("untrusted panic detail must not cross the boundary")
        });
        let serialized = serde_json::to_value(response).expect("panic error should serialize");

        assert_eq!(serialized["status"], "error");
        assert_eq!(serialized["error"]["code"], "internal");
        assert!(!serialized.to_string().contains("untrusted panic detail"));
        assert_eq!(logs.events()[0].kind, LogEventKind::Panicked);
        assert_eq!(
            logs.events()[0].diagnostic.as_deref(),
            Some("panic caught at the native command boundary")
        );
    }

    #[test]
    fn logging_failure_does_not_fail_a_successful_command() {
        let runtime = CommandRuntime::new(
            Arc::new(FakeClock::new(1_234)),
            Arc::new(FakeIdGenerator::new(1)),
            Arc::new(FailingLogSink),
        );

        let response = runtime.execute("test_command", || Ok(json!({ "value": 7 })));

        assert_eq!(
            serde_json::to_value(response).expect("command response should serialize")["status"],
            "ok"
        );
    }
}
