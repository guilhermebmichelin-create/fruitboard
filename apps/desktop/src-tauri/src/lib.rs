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
use fruitboard_storage::{Database, ScanRoot, StartupView, StorageError};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

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
    database: Arc<Mutex<Database>>,
}

impl PreferencesService {
    fn new(database: Arc<Mutex<Database>>) -> Self {
        Self { database }
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

struct ScanRootsService {
    database: Arc<Mutex<Database>>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanRootSelection {
    selected_path: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanRootRemoved {
    id: String,
}

impl ScanRootsService {
    fn new(database: Arc<Mutex<Database>>) -> Self {
        Self { database }
    }

    fn list_scan_roots(&self) -> Result<Vec<ScanRoot>, AppError> {
        let database = self.database.lock().map_err(|_| storage_failed())?;
        database.list_scan_roots().map_err(map_storage_error)
    }

    fn add_scan_root(&self, display_name: String, path: String) -> Result<ScanRoot, AppError> {
        let display_name = display_name.trim().to_owned();
        if display_name.is_empty() {
            return Err(invalid_request());
        }
        let canonical = canonical_scan_root(&path).ok_or_else(invalid_request)?;
        let mut database = self.database.lock().map_err(|_| storage_failed())?;
        for existing in database.list_scan_roots().map_err(map_storage_error)? {
            if roots_overlap(&existing.canonical_path, &canonical) {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    DiagnosticCode::ScanRootConflict,
                ));
            }
        }
        database
            .add_scan_root(&display_name, &canonical)
            .map_err(map_storage_error)
    }

    fn remove_scan_root(&self, id: String) -> Result<ScanRootRemoved, AppError> {
        if id.is_empty() {
            return Err(invalid_request());
        }
        let mut database = self.database.lock().map_err(|_| storage_failed())?;
        database.remove_scan_root(&id).map_err(map_storage_error)?;
        Ok(ScanRootRemoved { id })
    }
}

/// Resolve a renderer-supplied path to the canonical directory form stored for
/// scan roots. Canonicalization resolves aliases and reparse points, so two
/// alias paths map to one stored root and are rejected as duplicates.
/// Returns None for missing paths, non-directories, and oversized input.
fn canonical_scan_root(path: &str) -> Option<String> {
    if path.is_empty() || path.len() > 32_767 {
        return None;
    }
    let canonical = std::fs::canonicalize(path).ok()?;
    if !canonical.is_dir() {
        return None;
    }
    let text = canonical.to_string_lossy().into_owned();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        return Some(format!(r"\\{unc}"));
    }
    Some(text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned())
}

/// Duplicate and ancestor/descendant roots are rejected so reconciliation
/// stays unambiguous. Comparison is separator-aware and case-insensitive on
/// Windows, where the filesystem preserves case but ignores it. Trailing
/// separators are normalized so volume roots such as `C:\` cover their
/// children in both insertion orders.
fn roots_overlap(first: &str, second: &str) -> bool {
    fn normalize(path: &str) -> String {
        let trimmed = path.trim_end_matches(['/', '\\']);
        if trimmed.is_empty() {
            return std::path::MAIN_SEPARATOR.to_string();
        }
        #[cfg(windows)]
        return trimmed.to_lowercase();
        #[cfg(not(windows))]
        return trimmed.to_owned();
    }

    fn covers(parent: &str, child: &str) -> bool {
        child == parent
            || child
                .strip_prefix(parent)
                .is_some_and(|rest| rest.starts_with('/') || rest.starts_with('\\'))
    }
    let (first, second) = (normalize(first), normalize(second));
    covers(&first, &second) || covers(&second, &first)
}

struct NativeFoundation {
    commands: CommandRuntime,
    preferences: PreferencesService,
    scan_roots: ScanRootsService,
}

impl NativeFoundation {
    fn new(
        log_directory: Option<PathBuf>,
        data_directory: &std::path::Path,
    ) -> Result<Self, StorageError> {
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let ids: Arc<dyn IdGenerator> = Arc::new(SystemIdGenerator);
        let logs = default_log_sink(log_directory, clock.clone());
        let database = Arc::new(Mutex::new(Database::open(data_directory)?));

        Ok(Self {
            commands: CommandRuntime::new(clock, ids, logs),
            preferences: PreferencesService::new(database.clone()),
            scan_roots: ScanRootsService::new(database),
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
        StorageError::Conflict => {
            AppError::new(ErrorCode::Conflict, DiagnosticCode::ScanRootConflict)
        }
        StorageError::NotFound => {
            AppError::new(ErrorCode::NotFound, DiagnosticCode::UnknownScanRoot)
        }
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ListScanRootsRequest {
    schema_version: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct AddScanRootRequest {
    schema_version: u64,
    display_name: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RemoveScanRootRequest {
    schema_version: u64,
    id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct PickScanRootRequest {
    schema_version: u64,
}

fn handle_list_scan_roots(
    commands: &CommandRuntime,
    scan_roots: &ScanRootsService,
    request: Option<Value>,
) -> CommandEnvelope<Vec<ScanRoot>> {
    commands.execute("list_scan_roots", move || {
        let request: ListScanRootsRequest = decode_request(request)?;
        if request.schema_version != COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        scan_roots.list_scan_roots()
    })
}

fn handle_add_scan_root(
    commands: &CommandRuntime,
    scan_roots: &ScanRootsService,
    request: Option<Value>,
) -> CommandEnvelope<ScanRoot> {
    commands.execute("add_scan_root", move || {
        let request: AddScanRootRequest = decode_request(request)?;
        if request.schema_version != COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        scan_roots.add_scan_root(request.display_name, request.path)
    })
}

fn handle_remove_scan_root(
    commands: &CommandRuntime,
    scan_roots: &ScanRootsService,
    request: Option<Value>,
) -> CommandEnvelope<ScanRootRemoved> {
    commands.execute("remove_scan_root", move || {
        let request: RemoveScanRootRequest = decode_request(request)?;
        if request.schema_version != COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        scan_roots.remove_scan_root(request.id)
    })
}

fn handle_pick_scan_root(
    commands: &CommandRuntime,
    request: Option<Value>,
    select_folder: impl FnOnce() -> Option<String>,
) -> CommandEnvelope<ScanRootSelection> {
    commands.execute("pick_scan_root", move || {
        let request: PickScanRootRequest = decode_request(request)?;
        if request.schema_version != COMMAND_SCHEMA_VERSION {
            return Err(invalid_request());
        }
        // Cancellation is an ordinary no-change outcome: a closed dialog
        // yields a null selection, never an error and never a mutation.
        Ok(ScanRootSelection {
            selected_path: select_folder(),
        })
    })
}

#[tauri::command]
fn list_scan_roots(
    request: Option<Value>,
    state: tauri::State<'_, NativeFoundation>,
) -> CommandEnvelope<Vec<ScanRoot>> {
    handle_list_scan_roots(&state.commands, &state.scan_roots, request)
}

#[tauri::command]
fn add_scan_root(
    request: Option<Value>,
    state: tauri::State<'_, NativeFoundation>,
) -> CommandEnvelope<ScanRoot> {
    handle_add_scan_root(&state.commands, &state.scan_roots, request)
}

#[tauri::command]
fn remove_scan_root(
    request: Option<Value>,
    state: tauri::State<'_, NativeFoundation>,
) -> CommandEnvelope<ScanRootRemoved> {
    handle_remove_scan_root(&state.commands, &state.scan_roots, request)
}

#[tauri::command]
async fn pick_scan_root(
    request: Option<Value>,
    app: tauri::AppHandle,
) -> CommandEnvelope<ScanRootSelection> {
    // Reject malformed callers before opening any UI.
    let schema_valid = decode_request::<PickScanRootRequest>(request.clone())
        .is_ok_and(|parsed| parsed.schema_version == COMMAND_SCHEMA_VERSION);
    if !schema_valid {
        let state = app.state::<NativeFoundation>();
        return handle_pick_scan_root(&state.commands, request, || Option::<String>::None);
    }
    // The blocking folder dialog must never run on the main thread: async
    // commands are polled off-thread, the picker itself is isolated on a
    // dedicated blocking thread, and native state is borrowed from the owned
    // app handle only after the final await.
    let dialog_app = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .blocking_pick_folder()
            .and_then(|picked| picked.simplified().into_path().ok())
            .map(|path| path.to_string_lossy().into_owned())
    })
    .await;
    let state = app.state::<NativeFoundation>();
    match picked {
        Ok(selection) => handle_pick_scan_root(&state.commands, request, || selection),
        Err(_) => state.commands.execute("pick_scan_root", || {
            Err::<ScanRootSelection, AppError>(AppError::command_panicked())
        }),
    }
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
    let builder = builder.plugin(tauri_plugin_dialog::init());

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
            set_startup_view,
            list_scan_roots,
            add_scan_root,
            remove_scan_root,
            pick_scan_root
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

    fn test_preferences(directory: &TestDirectory) -> PreferencesService {
        PreferencesService::new(Arc::new(Mutex::new(
            Database::open(directory.path()).expect("test database should open"),
        )))
    }

    fn test_scan_roots(directory: &TestDirectory) -> ScanRootsService {
        ScanRootsService::new(Arc::new(Mutex::new(
            Database::open(directory.path()).expect("test database should open"),
        )))
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
            let preferences = test_preferences(&directory);
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

        let preferences = PreferencesService::new(Arc::new(Mutex::new(
            Database::open(directory.path()).expect("test database should reopen"),
        )));
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
        let preferences = test_preferences(&directory);
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

        let conflict = map_storage_error(StorageError::Conflict);
        assert_eq!(conflict.user().code, ErrorCode::Conflict);
        assert_eq!(
            conflict.diagnostic().diagnostic_code.as_str(),
            "scan_root_conflict"
        );

        let missing = map_storage_error(StorageError::NotFound);
        assert_eq!(missing.user().code, ErrorCode::NotFound);
        assert_eq!(
            missing.diagnostic().diagnostic_code.as_str(),
            "unknown_scan_root"
        );

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

    fn test_root_directory(directory: &TestDirectory, name: &str) -> PathBuf {
        let path = directory.path().join(name);
        fs::create_dir(&path).expect("test scan root directory should exist");
        path
    }

    fn add_root_request(display_name: &str, path: &std::path::Path) -> Option<Value> {
        Some(json!({
            "schemaVersion": COMMAND_SCHEMA_VERSION,
            "displayName": display_name,
            "path": path.to_string_lossy(),
        }))
    }

    #[test]
    fn scan_root_commands_round_trip_and_persist_across_restart() {
        let directory = TestDirectory::new();
        let root_path = test_root_directory(&directory, "music");
        let root_id;

        {
            let scan_roots = test_scan_roots(&directory);
            let (runtime, logs) = test_runtime();
            let response =
                handle_add_scan_root(&runtime, &scan_roots, add_root_request("Music", &root_path));
            let serialized = serde_json::to_value(response).expect("add response should serialize");
            assert_eq!(serialized["status"], "ok");
            assert_eq!(serialized["data"]["displayName"], "Music");
            assert_eq!(logs.events()[0].operation, "add_scan_root");
            root_id = serialized["data"]["id"]
                .as_str()
                .expect("added root should have an id")
                .to_owned();

            let (runtime, _) = test_runtime();
            let listed = handle_list_scan_roots(
                &runtime,
                &scan_roots,
                Some(json!({ "schemaVersion": COMMAND_SCHEMA_VERSION })),
            );
            let serialized = serde_json::to_value(listed).expect("list response should serialize");
            assert_eq!(serialized["data"].as_array().expect("roots").len(), 1);
        }

        let scan_roots = ScanRootsService::new(Arc::new(Mutex::new(
            Database::open(directory.path()).expect("test database should reopen"),
        )));
        let (runtime, _) = test_runtime();
        let response = handle_remove_scan_root(
            &runtime,
            &scan_roots,
            Some(json!({
                "schemaVersion": COMMAND_SCHEMA_VERSION,
                "id": root_id,
            })),
        );
        let serialized = serde_json::to_value(response).expect("remove response should serialize");
        assert_eq!(serialized["status"], "ok");
        assert_eq!(serialized["data"]["id"], root_id);

        let (runtime, _) = test_runtime();
        let listed = handle_list_scan_roots(
            &runtime,
            &scan_roots,
            Some(json!({ "schemaVersion": COMMAND_SCHEMA_VERSION })),
        );
        assert_eq!(
            serde_json::to_value(listed).expect("list response should serialize")["data"]
                .as_array()
                .expect("roots")
                .len(),
            0
        );
        // Removal drops configuration only; the source directory survives.
        assert!(root_path.is_dir());
    }

    #[test]
    fn scan_root_duplicates_overlaps_and_unknown_paths_fail_closed() {
        let directory = TestDirectory::new();
        let root_path = test_root_directory(&directory, "music");
        let child_path = root_path.join("sub");
        fs::create_dir(&child_path).expect("test child directory should exist");
        let scan_roots = test_scan_roots(&directory);
        let (runtime, _) = test_runtime();
        handle_add_scan_root(&runtime, &scan_roots, add_root_request("Music", &root_path));

        let duplicate = handle_add_scan_root(
            &test_runtime().0,
            &scan_roots,
            add_root_request("Music again", &root_path),
        );
        let serialized = serde_json::to_value(duplicate).expect("duplicate error should serialize");
        assert_eq!(serialized["status"], "error");
        assert_eq!(serialized["error"]["code"], "conflict");

        let child = handle_add_scan_root(
            &test_runtime().0,
            &scan_roots,
            add_root_request("Child", &child_path),
        );
        assert_eq!(
            serde_json::to_value(child).expect("overlap error should serialize")["error"]["code"],
            "conflict"
        );

        let missing = handle_add_scan_root(
            &test_runtime().0,
            &scan_roots,
            add_root_request("Missing", &directory.path().join("absent")),
        );
        let serialized = serde_json::to_value(missing).expect("missing error should serialize");
        assert_eq!(serialized["error"]["code"], "invalid_request");
        assert!(!serialized.to_string().contains("absent"));

        let file_path = directory.path().join("file.flp");
        fs::write(&file_path, b"placeholder").expect("test file should exist");
        let not_directory = handle_add_scan_root(
            &test_runtime().0,
            &scan_roots,
            add_root_request("File", &file_path),
        );
        assert_eq!(
            serde_json::to_value(not_directory).expect("file error should serialize")["error"]["code"],
            "invalid_request"
        );

        let unnamed = handle_add_scan_root(
            &test_runtime().0,
            &scan_roots,
            add_root_request("   ", &root_path),
        );
        assert_eq!(
            serde_json::to_value(unnamed).expect("name error should serialize")["error"]["code"],
            "invalid_request"
        );

        let unknown = handle_remove_scan_root(
            &test_runtime().0,
            &scan_roots,
            Some(json!({
                "schemaVersion": COMMAND_SCHEMA_VERSION,
                "id": "missing-root-id",
            })),
        );
        assert_eq!(
            serde_json::to_value(unknown).expect("unknown error should serialize")["error"]["code"],
            "not_found"
        );

        let malformed = [
            None,
            Some(Value::Null),
            Some(json!({ "schemaVersion": COMMAND_SCHEMA_VERSION })),
            Some(json!({
                "schemaVersion": COMMAND_SCHEMA_VERSION,
                "displayName": "Music",
                "path": root_path.to_string_lossy(),
                "extra": true,
            })),
        ];
        for request in malformed {
            let (runtime, logs) = test_runtime();
            let response = handle_add_scan_root(&runtime, &scan_roots, request);
            assert_eq!(
                serde_json::to_value(response).expect("error should serialize")["error"]["code"],
                "invalid_request"
            );
            assert_eq!(logs.events()[0].operation, "add_scan_root");
        }

        let (runtime, _) = test_runtime();
        let listed = handle_list_scan_roots(
            &runtime,
            &scan_roots,
            Some(json!({ "schemaVersion": COMMAND_SCHEMA_VERSION })),
        );
        assert_eq!(
            serde_json::to_value(listed).expect("list should serialize")["data"]
                .as_array()
                .expect("roots")
                .len(),
            1
        );
    }

    #[test]
    fn roots_overlap_compares_separator_aware_paths() {
        assert!(roots_overlap("/media/music", "/media/music"));
        assert!(roots_overlap("/media/music", "/media/music/flp"));
        assert!(roots_overlap("/media/music/flp", "/media/music"));
        assert!(!roots_overlap("/media/music", "/media/music-backup"));
        assert!(!roots_overlap("/media/music", "/media/other"));
        assert!(roots_overlap("/media/music/", "/media/music/flp"));
        assert!(roots_overlap("/media/music", "/media/music/"));
        assert!(roots_overlap(r"\\server\share", r"\\server\share\projects"));
        assert!(roots_overlap(r"\\server\share\projects", r"\\server\share"));
        assert!(!roots_overlap(r"\\server\share", r"\\server\other"));
        #[cfg(windows)]
        {
            assert!(roots_overlap("C:\\Music", "c:\\music\\flp"));
            assert!(roots_overlap("C:\\Music", "C:\\MUSIC"));
            assert!(roots_overlap("C:\\", "C:\\Music"));
            assert!(roots_overlap("C:\\Music", "C:\\"));
            assert!(roots_overlap("C:\\Music\\", "C:\\Music\\flp"));
            assert!(!roots_overlap("C:\\Music", "D:\\Music"));
        }
    }

    #[test]
    fn pick_cancellation_returns_null_without_mutation_or_dialog_error() {
        let directory = TestDirectory::new();
        let scan_roots = test_scan_roots(&directory);

        let (runtime, logs) = test_runtime();
        let cancelled = handle_pick_scan_root(
            &runtime,
            Some(json!({ "schemaVersion": COMMAND_SCHEMA_VERSION })),
            || Option::<String>::None,
        );
        let serialized = serde_json::to_value(cancelled).expect("cancel response should serialize");
        assert_eq!(serialized["status"], "ok");
        assert_eq!(serialized["data"]["selectedPath"], Value::Null);
        assert_eq!(logs.events()[0].operation, "pick_scan_root");
        assert!(scan_roots.list_scan_roots().unwrap().is_empty());

        let (runtime, _) = test_runtime();
        let picked = handle_pick_scan_root(
            &runtime,
            Some(json!({ "schemaVersion": COMMAND_SCHEMA_VERSION })),
            || Some("C:\\Music".to_owned()),
        );
        assert_eq!(
            serde_json::to_value(picked).expect("pick response should serialize")["data"]["selectedPath"],
            "C:\\Music"
        );

        let called = std::cell::Cell::new(false);
        let (runtime, _) = test_runtime();
        let rejected = handle_pick_scan_root(&runtime, Some(json!({ "schemaVersion": 0 })), || {
            called.set(true);
            Option::<String>::None
        });
        assert_eq!(
            serde_json::to_value(rejected).expect("error should serialize")["error"]["code"],
            "invalid_request"
        );
        assert!(!called.get());
    }
}
