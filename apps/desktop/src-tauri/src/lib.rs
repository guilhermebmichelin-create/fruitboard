// MSVC prints routine import-library creation on stdout for the required
// `cdylib` target; rustc otherwise promotes that informational line to a warning.
#![cfg_attr(target_env = "msvc", allow(linker_messages))]

pub mod foundation;

#[cfg(feature = "packaging-smoke")]
use foundation::packaging_smoke::{SmokeMode, SmokeRequest};

use foundation::{
    AppError, COMMAND_SCHEMA_VERSION, Clock, CommandEnvelope, CommandRuntime, DiagnosticCode,
    ErrorCode, IdGenerator, SystemClock, SystemIdGenerator, default_log_sink,
};
use fruitboard_storage::{Database, StartupView, StorageError};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct GetStartupViewRequest {
    schema_version: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SetStartupViewRequest {
    schema_version: u64,
    startup_view: StartupView,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct StartupViewPreference {
    startup_view: StartupView,
}

struct PreferencesService {
    database: Mutex<Database>,
}

impl PreferencesService {
    fn new(database: Database) -> Self {
        Self {
            database: Mutex::new(database),
        }
    }

    fn get_startup_view(&self) -> Result<StartupViewPreference, AppError> {
        let database = self.database.lock().map_err(|_| storage_failed())?;
        let startup_view = database.startup_view().map_err(map_storage_error)?;
        Ok(StartupViewPreference { startup_view })
    }

    fn set_startup_view(
        &self,
        startup_view: StartupView,
    ) -> Result<StartupViewPreference, AppError> {
        let mut database = self.database.lock().map_err(|_| storage_failed())?;
        database
            .set_startup_view(startup_view)
            .map_err(map_storage_error)?;
        Ok(StartupViewPreference { startup_view })
    }
}

struct NativeFoundation {
    commands: CommandRuntime,
    preferences: PreferencesService,
}

impl NativeFoundation {
    fn new(
        log_directory: Option<PathBuf>,
        data_directory: &std::path::Path,
    ) -> Result<Self, StorageError> {
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let ids: Arc<dyn IdGenerator> = Arc::new(SystemIdGenerator);
        let logs = default_log_sink(log_directory, clock.clone());

        Ok(Self {
            commands: CommandRuntime::new(clock, ids, logs),
            preferences: PreferencesService::new(Database::open(data_directory)?),
        })
    }
}

fn invalid_request() -> AppError {
    AppError::invalid_request(DiagnosticCode::RequestSchemaValidationFailed)
}

fn decode_request<Request: DeserializeOwned>(request: Option<Value>) -> Result<Request, AppError> {
    request
        .and_then(|value| serde_json::from_value(value).ok())
        .ok_or_else(invalid_request)
}

fn storage_failed() -> AppError {
    AppError::new(ErrorCode::Internal, DiagnosticCode::StorageFailed)
}

fn map_storage_error(error: StorageError) -> AppError {
    match error {
        StorageError::Busy => AppError::new(ErrorCode::Unavailable, DiagnosticCode::StorageBusy),
        _ => storage_failed(),
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

fn handle_get_startup_view(
    commands: &CommandRuntime,
    preferences: &PreferencesService,
    request: Option<Value>,
) -> CommandEnvelope<StartupViewPreference> {
    commands.execute("get_startup_view", move || {
        let request: GetStartupViewRequest = decode_request(request)?;
        if request.schema_version != COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        preferences.get_startup_view()
    })
}

fn handle_set_startup_view(
    commands: &CommandRuntime,
    preferences: &PreferencesService,
    request: Option<Value>,
) -> CommandEnvelope<StartupViewPreference> {
    commands.execute("set_startup_view", move || {
        let request: SetStartupViewRequest = decode_request(request)?;
        if request.schema_version != COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        preferences.set_startup_view(request.startup_view)
    })
}

#[tauri::command]
fn get_startup_view(
    request: Option<Value>,
    state: tauri::State<'_, NativeFoundation>,
) -> CommandEnvelope<StartupViewPreference> {
    handle_get_startup_view(&state.commands, &state.preferences, request)
}

#[tauri::command]
fn set_startup_view(
    request: Option<Value>,
    state: tauri::State<'_, NativeFoundation>,
) -> CommandEnvelope<StartupViewPreference> {
    handle_set_startup_view(&state.commands, &state.preferences, request)
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

    let builder = tauri::Builder::default();
    #[cfg(feature = "packaging-smoke")]
    let builder = builder.plugin(tauri_plugin_shell::init());

    builder
        .setup(|app| {
            let log_directory = app.path().app_log_dir().ok();
            let data_directory = app.path().app_local_data_dir()?;
            let foundation =
                NativeFoundation::new(log_directory, &data_directory).inspect_err(|error| {
                    // StorageError is a closed set of fixed codes, without a raw
                    // SQLite/io source, SQL statement, or filesystem path.
                    eprintln!("{error}");
                })?;

            #[cfg(feature = "packaging-smoke")]
            let smoke = SmokeRequest::from_environment().map(|request| {
                let storage = foundation
                    .preferences
                    .get_startup_view()
                    .map(|before| {
                        let after = match request.mode {
                            SmokeMode::Seed => foundation
                                .preferences
                                .set_startup_view(StartupView::Library)?,
                            SmokeMode::Verify => foundation.preferences.get_startup_view()?,
                        };
                        if request.mode == SmokeMode::Verify
                            && before.startup_view != StartupView::Library
                        {
                            return Err(storage_failed());
                        }
                        Ok((before.startup_view, after.startup_view))
                    })
                    .and_then(|value| value)
                    .map_err(|_| "storage_preservation_failed");
                (request, storage)
            });

            app.manage(foundation);

            #[cfg(feature = "packaging-smoke")]
            if let Some((request, storage)) = smoke {
                foundation::packaging_smoke::schedule(app.handle().clone(), request, storage);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_health,
            get_startup_view,
            set_startup_view
        ])
        .run(tauri::generate_context!())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
    use crate::foundation::{ErrorCode, LogSink};
    use serde_json::json;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    const PANIC_HOOK_PROBE: &str = "FRUITBOARD_SAFE_PANIC_HOOK_PROBE";
    static NEXT_STORAGE_TEST: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_STORAGE_TEST.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "fruitboard-command-storage-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("test app-data directory should be created");
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

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

    #[test]
    fn startup_view_commands_persist_through_a_database_restart() {
        let directory = TestDirectory::new();

        {
            let preferences = PreferencesService::new(
                Database::open(directory.path()).expect("test database should open"),
            );
            let (runtime, logs) = test_runtime();
            let response = handle_set_startup_view(
                &runtime,
                &preferences,
                Some(json!({
                    "schemaVersion": COMMAND_SCHEMA_VERSION,
                    "startupView": "library",
                })),
            );

            assert_eq!(
                serde_json::to_value(response).expect("set response should serialize"),
                json!({
                    "status": "ok",
                    "schemaVersion": COMMAND_SCHEMA_VERSION,
                    "correlationId": "correlation_00000000000000000000000000000001",
                    "data": { "startupView": "library" },
                })
            );
            assert_eq!(logs.events()[0].operation, "set_startup_view");
        }

        let preferences = PreferencesService::new(
            Database::open(directory.path()).expect("test database should reopen"),
        );
        let (runtime, logs) = test_runtime();
        let response = handle_get_startup_view(
            &runtime,
            &preferences,
            Some(json!({ "schemaVersion": COMMAND_SCHEMA_VERSION })),
        );

        assert_eq!(
            serde_json::to_value(response).expect("get response should serialize"),
            json!({
                "status": "ok",
                "schemaVersion": COMMAND_SCHEMA_VERSION,
                "correlationId": "correlation_00000000000000000000000000000001",
                "data": { "startupView": "library" },
            })
        );
        assert_eq!(logs.events()[0].operation, "get_startup_view");
    }

    #[test]
    fn malformed_preference_requests_fail_closed_without_mutating_storage() {
        let directory = TestDirectory::new();
        let preferences = PreferencesService::new(
            Database::open(directory.path()).expect("test database should open"),
        );
        let malformed_set_requests = [
            None,
            Some(Value::Null),
            Some(json!({ "schemaVersion": COMMAND_SCHEMA_VERSION })),
            Some(json!({
                "schemaVersion": 0,
                "startupView": "library",
            })),
            Some(json!({
                "schemaVersion": COMMAND_SCHEMA_VERSION,
                "startupView": "private/path.flp",
            })),
            Some(json!({
                "schemaVersion": COMMAND_SCHEMA_VERSION,
                "startupView": "library",
                "extra": "Bearer secret",
            })),
        ];

        for request in malformed_set_requests {
            let (runtime, logs) = test_runtime();
            let response = handle_set_startup_view(&runtime, &preferences, request);
            let serialized =
                serde_json::to_value(response).expect("invalid response should serialize");

            assert_eq!(serialized["error"]["code"], "invalid_request");
            assert!(!serialized.to_string().contains("private/path.flp"));
            assert_eq!(logs.events()[0].operation, "set_startup_view");
            assert_eq!(
                logs.events()[0]
                    .diagnostic
                    .as_ref()
                    .map(|diagnostic| diagnostic.as_str()),
                Some("request_schema_validation_failed")
            );
        }

        let (runtime, _) = test_runtime();
        let invalid_get = handle_get_startup_view(
            &runtime,
            &preferences,
            Some(json!({
                "schemaVersion": COMMAND_SCHEMA_VERSION,
                "extra": true,
            })),
        );
        assert_eq!(
            serde_json::to_value(invalid_get).expect("invalid response should serialize")["error"]
                ["code"],
            "invalid_request"
        );
        assert_eq!(
            preferences
                .get_startup_view()
                .expect("preference should remain readable")
                .startup_view,
            StartupView::Home
        );
    }

    #[test]
    fn storage_failures_map_to_fixed_user_and_diagnostic_codes() {
        let busy = map_storage_error(StorageError::Busy);
        assert_eq!(busy.user().code, ErrorCode::Unavailable);
        assert_eq!(busy.diagnostic().diagnostic_code.as_str(), "storage_busy");

        for error in [
            StorageError::Io,
            StorageError::InvalidSchema,
            StorageError::Database,
        ] {
            let mapped = map_storage_error(error);
            assert_eq!(mapped.user().code, ErrorCode::Internal);
            assert_eq!(
                mapped.diagnostic().diagnostic_code.as_str(),
                "storage_failed"
            );
        }
    }
}
