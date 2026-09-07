//! Feature-off coverage: with `scan-console` disabled every console command
//! returns the typed unavailable envelope and the state command reports
//! `{enabled: false}`. These tests run in default builds.

use super::*;
use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let sequence = NEXT.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "fruitboard-scan-console-disabled-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("test app-data directory should be created");
        Self(path)
    }

    fn path(&self) -> &PathBuf {
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
    let log_sink: Arc<dyn crate::foundation::LogSink> = logs.clone();
    (
        CommandRuntime::new(
            Arc::new(FakeClock::new(1_234)),
            Arc::new(FakeIdGenerator::new(1)),
            log_sink,
        ),
        logs,
    )
}

fn disabled_service(directory: &TestDirectory) -> ScanConsoleService {
    ScanConsoleService::new(Arc::new(Mutex::new(
        Database::open(directory.path()).expect("test database should open"),
    )))
}

type CommandCase = (
    &'static str,
    &'static dyn Fn(&CommandRuntime, &ScanConsoleService) -> serde_json::Value,
);

#[test]
fn console_commands_return_the_typed_unavailable_envelope_when_disabled() {
    let directory = TestDirectory::new();
    let service = disabled_service(&directory);

    let cases: Vec<CommandCase> = vec![
        ("scan_now", &|runtime, service| {
            serde_json::to_value(handle_scan_now(
                runtime,
                service,
                Some(json!({ "schemaVersion": 1, "rootId": "root-1" })),
            ))
            .expect("serialize")
        }),
        ("cancel_scan", &|runtime, service| {
            serde_json::to_value(handle_cancel_scan(
                runtime,
                service,
                Some(json!({ "schemaVersion": 1, "jobId": "job-1" })),
            ))
            .expect("serialize")
        }),
        ("retry_scan", &|runtime, service| {
            serde_json::to_value(handle_retry_scan(
                runtime,
                service,
                Some(json!({ "schemaVersion": 1, "jobId": "job-1" })),
            ))
            .expect("serialize")
        }),
        ("list_scan_statuses", &|runtime, service| {
            serde_json::to_value(handle_list_scan_statuses(
                runtime,
                service,
                Some(json!({ "schemaVersion": 1 })),
            ))
            .expect("serialize")
        }),
        ("get_library_page", &|runtime, service| {
            serde_json::to_value(handle_get_library_page(
                runtime,
                service,
                Some(json!({
                    "schemaVersion": 1,
                    "rootId": "root-1",
                    "limit": 10,
                    "cursor": null,
                    "snapshotId": null,
                })),
            ))
            .expect("serialize")
        }),
    ];

    for (operation, call) in cases {
        let (runtime, logs) = test_runtime();
        let serialized = call(&runtime, &service);
        assert_eq!(serialized["status"], "error", "{operation}");
        assert_eq!(serialized["error"]["code"], "unavailable", "{operation}");
        assert_eq!(
            serialized["error"]["message"],
            "The requested service is temporarily unavailable."
        );
        assert_eq!(serialized["error"]["retryable"], true, "{operation}");
        assert_eq!(logs.events()[0].operation, operation);
        assert_eq!(
            logs.events()[0]
                .diagnostic
                .as_ref()
                .map(|diagnostic| diagnostic.as_str()),
            Some("scan_console_disabled")
        );
    }
}

#[test]
fn console_state_reports_disabled_without_erroring() {
    let directory = TestDirectory::new();
    let service = disabled_service(&directory);
    let (runtime, logs) = test_runtime();
    let response =
        handle_get_scan_console_state(&runtime, &service, Some(json!({ "schemaVersion": 1 })));
    let serialized = serde_json::to_value(response).expect("serialize");
    assert_eq!(serialized["status"], "ok");
    assert_eq!(serialized["data"]["enabled"], false);
    assert_eq!(logs.events()[0].operation, "get_scan_console_state");
}

#[test]
fn malformed_console_requests_still_fail_closed_without_mutation() {
    let directory = TestDirectory::new();
    let service = disabled_service(&directory);
    let malformed = [
        None,
        Some(Value::Null),
        Some(json!({ "schemaVersion": 1 })),
        Some(json!({ "schemaVersion": 0, "rootId": "root-1" })),
        Some(json!({
            "schemaVersion": 1,
            "rootId": "root-1",
            "extra": "Bearer secret",
        })),
    ];
    for request in malformed {
        let (runtime, _) = test_runtime();
        let response = handle_scan_now(&runtime, &service, request);
        let serialized = serde_json::to_value(response).expect("serialize");
        assert_eq!(serialized["status"], "error");
        assert_eq!(serialized["error"]["code"], "invalid_request");
    }
}
