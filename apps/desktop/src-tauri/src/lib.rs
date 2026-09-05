// MSVC prints routine import-library creation on stdout for the required
// `cdylib` target; rustc otherwise promotes that informational line to a warning.
#![cfg_attr(target_env = "msvc", allow(linker_messages))]

pub mod foundation;

use foundation::{
    AppError, COMMAND_SCHEMA_VERSION, Clock, CommandEnvelope, CommandRuntime, DiagnosticCode,
    IdGenerator, SystemClock, SystemIdGenerator, default_log_sink,
};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::Manager;

const SAFE_NATIVE_PANIC_MESSAGE: &str = "Fruitboard contained an unexpected native failure.";

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppHealth {
    status: &'static str,
    runtime: &'static str,
    version: &'static str,
}

struct NativeFoundation {
    commands: CommandRuntime,
}

impl NativeFoundation {
    fn new(log_directory: Option<PathBuf>) -> Self {
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let ids: Arc<dyn IdGenerator> = Arc::new(SystemIdGenerator);
        let logs = default_log_sink(log_directory, clock.clone());

        Self {
            commands: CommandRuntime::new(clock, ids, logs),
        }
    }
}

fn handle_get_app_health(
    commands: &CommandRuntime,
    request: Option<Value>,
) -> CommandEnvelope<AppHealth> {
    commands.execute("get_app_health", move || {
        let valid_request = request.is_some_and(|value| {
            value.as_object().is_some_and(|object| {
                object.len() == 1
                    && object.get("schemaVersion").and_then(Value::as_u64)
                        == Some(COMMAND_SCHEMA_VERSION)
            })
        });

        if !valid_request {
            return Err(AppError::invalid_request(
                DiagnosticCode::RequestSchemaValidationFailed,
            ));
        }

        Ok(AppHealth {
            status: "ok",
            runtime: "desktop",
            version: env!("CARGO_PKG_VERSION"),
        })
    })
}

#[tauri::command]
fn get_app_health(
    request: Option<Value>,
    state: tauri::State<'_, NativeFoundation>,
) -> CommandEnvelope<AppHealth> {
    handle_get_app_health(&state.commands, request)
}

fn install_safe_panic_hook() {
    static INSTALL: std::sync::Once = std::sync::Once::new();

    INSTALL.call_once(|| {
        std::panic::set_hook(Box::new(|_| {
            eprintln!("{SAFE_NATIVE_PANIC_MESSAGE}");
        }));
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    install_safe_panic_hook();

    tauri::Builder::default()
        .setup(|app| {
            let log_directory = app.path().app_log_dir().ok();
            app.manage(NativeFoundation::new(log_directory));
            let data_directory = app.path().app_local_data_dir()?;
            let storage =
                fruitboard_storage::Database::open(&data_directory).inspect_err(|error| {
                    // StorageError is a closed set of fixed codes, without a raw
                    // SQLite/io source, SQL statement, or filesystem path.
                    eprintln!("{error}");
                })?;
            app.manage(Mutex::new(storage));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_app_health])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
    use crate::foundation::{ErrorCode, LogSink};
    use serde_json::json;

    const PANIC_HOOK_PROBE: &str = "FRUITBOARD_SAFE_PANIC_HOOK_PROBE";

    fn test_runtime() -> (CommandRuntime, Arc<RecordingLogSink>) {
        let logs = Arc::new(RecordingLogSink::default());
        let log_sink: Arc<dyn LogSink> = logs.clone();
        (
            CommandRuntime::new(
                Arc::new(FakeClock::new(1_234)),
                Arc::new(FakeIdGenerator::new(1)),
                log_sink,
            ),
            logs,
        )
    }

    #[test]
    fn health_command_matches_the_client_envelope() {
        let (runtime, _) = test_runtime();
        let response = handle_get_app_health(
            &runtime,
            Some(json!({ "schemaVersion": COMMAND_SCHEMA_VERSION })),
        );

        assert_eq!(
            serde_json::to_value(response).expect("health response should serialize"),
            json!({
                "status": "ok",
                "schemaVersion": COMMAND_SCHEMA_VERSION,
                "correlationId": "correlation_00000000000000000000000000000001",
                "data": {
                    "status": "ok",
                    "runtime": "desktop",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            })
        );
    }

    #[test]
    fn safe_panic_hook_omits_the_panic_payload() {
        if std::env::var_os(PANIC_HOOK_PROBE).is_some() {
            install_safe_panic_hook();
            let outcome = std::panic::catch_unwind(|| {
                panic!("Bearer secret C:\\Users\\producer\\private.flp")
            });
            assert!(outcome.is_err());
            return;
        }

        let output = std::process::Command::new(
            std::env::current_exe().expect("the test executable should have a path"),
        )
        .args([
            "--exact",
            "tests::safe_panic_hook_omits_the_panic_payload",
            "--nocapture",
        ])
        .env(PANIC_HOOK_PROBE, "1")
        .output()
        .expect("the panic-hook probe should run");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        assert!(output.status.success(), "probe failed: {combined}");
        assert!(combined.contains(SAFE_NATIVE_PANIC_MESSAGE));
        assert!(!combined.contains("Bearer secret"));
        assert!(!combined.contains("private.flp"));
    }

    #[test]
    fn malformed_health_requests_fail_closed_without_logging_input() {
        let malformed = [
            None,
            Some(Value::Null),
            Some(json!({})),
            Some(json!({ "schemaVersion": 0 })),
            Some(json!({ "schemaVersion": 1, "extra": true })),
            Some(json!({
                "access_token": "secret",
                "path": "C:\\Users\\producer\\private.flp",
                "payload": [0, 1, 2, 3],
            })),
        ];

        for request in malformed {
            let (runtime, logs) = test_runtime();
            let response = handle_get_app_health(&runtime, request);
            let serialized = serde_json::to_value(response).expect("health error should serialize");

            assert_eq!(serialized["status"], "error");
            assert_eq!(serialized["error"]["code"], "invalid_request");
            assert_eq!(logs.events()[0].error_code, Some(ErrorCode::InvalidRequest));
            let events = logs.events();
            let diagnostic = events[0]
                .diagnostic
                .as_ref()
                .expect("rejected request should have a safe diagnostic");
            assert_eq!(diagnostic.as_str(), "request_schema_validation_failed");
        }
    }
}
