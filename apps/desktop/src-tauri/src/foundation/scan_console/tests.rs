//! Feature-gated integration tests for the hidden scan console. They drive
//! the real command handlers and the real host over a tempdir database with a
//! scripted fake filesystem port and an injected fake clock — no wall-clock
//! sleeps anywhere. The worker loop is split into `claim_due`/`execute_pending`
//! so tests can interleave commands while a job is durably `running`.

use super::*;
use crate::foundation::scan_console_host::ScanConsoleHost;
use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
use fruitboard_filesystem_enumeration::{
    DirectoryCaseSensitivity, DirectoryCursor, DirectoryEntry, EntryKind, FileMetadata,
    FilesystemQualification, IdentityQualification, OpenedDirectory, PortError, QualifiedIdentity,
    RootMetadata,
};
use fruitboard_scan_execution::{ScanClock, ScanWorker, WorkerConfig};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc as StdArc, mpsc};

const VOLUME_SERIAL: u64 = 7;
const ROOT_FILE_ID: u128 = 999;
const DEFAULT_MTIME_NS: i128 = 1_700_000_000_000_000_000;
const T0_MS: i64 = 1_700_000_000_000;

#[derive(Clone, Default)]
struct FakeScanClock(Arc<AtomicI64>);

impl FakeScanClock {
    fn new() -> Self {
        Self(Arc::new(AtomicI64::new(T0_MS)))
    }

    fn advance(&self, ms: i64) {
        self.0.fetch_add(ms, Ordering::SeqCst);
    }
}

impl ScanClock for FakeScanClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Clone)]
struct Entry {
    name: String,
    kind: EntryKind,
    size: u64,
    mtime: i128,
    identity: Option<u128>,
    children: Vec<Entry>,
}

impl Entry {
    fn metadata(&self) -> FileMetadata {
        FileMetadata {
            kind: self.kind,
            byte_size: self.size,
            modified_unix_ns: self.mtime,
            identity: self.identity.map(|file_id| QualifiedIdentity {
                volume_serial: VOLUME_SERIAL,
                file_id,
                qualification: IdentityQualification::LocalNtfs,
            }),
            reparse_point: false,
            recall_or_offline: false,
        }
    }
}

fn file_entry(name: &str, file_id: u128) -> Entry {
    Entry {
        name: name.to_owned(),
        kind: EntryKind::File,
        size: 1_024,
        mtime: DEFAULT_MTIME_NS,
        identity: Some(file_id),
        children: Vec::new(),
    }
}

fn tree(children: Vec<Entry>) -> Entry {
    Entry {
        name: String::new(),
        kind: EntryKind::Directory,
        size: 0,
        mtime: DEFAULT_MTIME_NS,
        identity: Some(ROOT_FILE_ID),
        children,
    }
}

/// Deterministic `FilesystemPort` over the scripted tree. `offline` scripts a
/// lost/unknown root (PortError::NotFound), which the worker resolves as a
/// failed run with a persisted retry chain.
struct FakePort {
    root: Entry,
    offline: bool,
}

impl FakePort {
    fn new(root: Entry) -> Self {
        Self {
            root,
            offline: false,
        }
    }

    fn offline(root: Entry) -> Self {
        Self {
            root,
            offline: true,
        }
    }
}

impl fruitboard_filesystem_enumeration::FilesystemPort for FakePort {
    fn inspect_root(&mut self, _root: &std::path::Path) -> Result<RootMetadata, PortError> {
        if self.offline {
            return Err(PortError::NotFound);
        }
        Ok(RootMetadata {
            metadata: self.root.metadata(),
            qualification: FilesystemQualification::LocalNtfs,
        })
    }

    fn open_root(&mut self, _root: &std::path::Path) -> Result<OpenedDirectory, PortError> {
        if self.offline {
            return Err(PortError::NotFound);
        }
        Ok(OpenedDirectory {
            metadata: self.root.metadata(),
            case_sensitivity: DirectoryCaseSensitivity::Insensitive,
            cursor: Box::new(FakeCursor {
                directory: self.root.clone(),
                prefix: String::new(),
                index: 0,
            }),
        })
    }
}

struct FakeCursor {
    directory: Entry,
    prefix: String,
    index: usize,
}

impl FakeCursor {
    fn display(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{}\\{}", self.prefix, name)
        }
    }
}

impl DirectoryCursor for FakeCursor {
    fn next_entry(&mut self) -> Result<Option<DirectoryEntry>, PortError> {
        Ok(self.directory.children.get(self.index).map(|child| {
            self.index += 1;
            DirectoryEntry::new(child.name.clone())
        }))
    }

    fn read_metadata(&mut self, entry: &DirectoryEntry) -> Result<FileMetadata, PortError> {
        let name = entry.name.to_string_lossy().into_owned();
        let child = self
            .directory
            .children
            .iter()
            .find(|child| child.name == name)
            .expect("scripted child exists");
        Ok(child.metadata())
    }

    fn open_directory(&mut self, entry: &DirectoryEntry) -> Result<OpenedDirectory, PortError> {
        let name = entry.name.to_string_lossy().into_owned();
        let child = self
            .directory
            .children
            .iter()
            .find(|child| child.name == name)
            .expect("scripted child exists");
        Ok(OpenedDirectory {
            metadata: child.metadata(),
            case_sensitivity: DirectoryCaseSensitivity::Insensitive,
            cursor: Box::new(FakeCursor {
                directory: child.clone(),
                prefix: self.display(&name),
                index: 0,
            }),
        })
    }
}

#[derive(Default)]
struct RecordingEventSink(Mutex<Vec<ScanStatusChangedEvent>>);

impl RecordingEventSink {
    fn events(&self) -> Vec<ScanStatusChangedEvent> {
        self.0.lock().unwrap().clone()
    }
}

impl ScanEventSink for RecordingEventSink {
    fn emit(&self, event: &ScanStatusChangedEvent) {
        self.0.lock().unwrap().push(event.clone());
    }
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let sequence = NEXT.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "fruitboard-scan-console-{label}-{}-{sequence}",
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

struct Harness {
    #[allow(dead_code)] // keeps the tempdir alive for the whole test
    directory: TestDirectory,
    database: Arc<Mutex<Database>>,
    service: ScanConsoleService,
    clock: FakeScanClock,
    sink: Arc<RecordingEventSink>,
    root_id: String,
}

impl Harness {
    fn new(label: &str) -> Self {
        let directory = TestDirectory::new(label);
        let clock = FakeScanClock::new();
        let mut database = Database::open(directory.path()).expect("open durable database");
        let root = database
            .add_scan_root("Synthetic", r"C:\synthetic-root")
            .expect("add synthetic root");
        let worker = ScanWorker::new(WorkerConfig::default()).expect("worker configuration");
        let session_id = worker
            .start_session(&mut database, &clock)
            .expect("start session")
            .id;
        let sink = Arc::new(RecordingEventSink::default());
        let host = Arc::new(ScanConsoleHost::new(
            worker,
            session_id,
            Arc::new(clock.clone()),
            sink.clone(),
        ));
        let mut service = ScanConsoleService::new_enabled(Arc::new(Mutex::new(database)));
        service.clock = Arc::new(clock.clone());
        *service.host.lock().unwrap() = Some(host);
        Self {
            directory,
            database: service.database.clone(),
            service,
            clock,
            sink,
            root_id: root.id,
        }
    }

    fn add_root(&self, display_name: &str, path: &str) -> String {
        self.database
            .lock()
            .unwrap()
            .add_scan_root(display_name, path)
            .expect("add root")
            .id
    }

    fn set_root_enabled(&self, root_id: &str, enabled: bool) {
        self.database
            .lock()
            .unwrap()
            .set_scan_root_enabled(root_id, enabled)
            .expect("set enabled");
    }

    fn host(&self) -> Arc<ScanConsoleHost> {
        self.service
            .host
            .lock()
            .unwrap()
            .clone()
            .expect("host installed")
    }

    fn claim_due(&self) {
        self.host().claim_due(&self.database);
    }

    fn execute_pending(&self, tree: Entry) {
        let mut port = FakePort::new(tree);
        self.host().execute_pending(&self.database, &mut port);
    }

    fn tick(&self, tree: Entry) {
        self.tick_port(FakePort::new(tree));
    }

    fn tick_port(&self, mut port: FakePort) {
        self.host().tick(&self.database, &mut port);
    }

    /// Simulate a process restart: begin a fresh session over the same
    /// database and install a fresh host (the old host is abandoned exactly
    /// like a crashed process abandons its worker).
    fn restart(&mut self) {
        let mut database = self.database.lock().unwrap();
        let worker = ScanWorker::new(WorkerConfig::default()).expect("worker configuration");
        let session_id = worker
            .start_session(&mut database, &self.clock)
            .expect("restart session")
            .id;
        drop(database);
        let host = Arc::new(ScanConsoleHost::new(
            worker,
            session_id,
            Arc::new(self.clock.clone()),
            self.sink.clone(),
        ));
        *self.service.host.lock().unwrap() = Some(host);
    }
}

fn scan_now_request(root_id: &str) -> Option<Value> {
    Some(json!({ "schemaVersion": 1, "rootId": root_id }))
}

fn cancel_request(job_id: &str) -> Option<Value> {
    Some(json!({ "schemaVersion": 1, "jobId": job_id }))
}

fn retry_request(job_id: &str) -> Option<Value> {
    Some(json!({ "schemaVersion": 1, "jobId": job_id }))
}

fn statuses_request() -> Option<Value> {
    Some(json!({ "schemaVersion": 1 }))
}

fn page_request(
    root_id: &str,
    limit: u64,
    cursor: Option<&str>,
    snapshot_id: Option<&str>,
) -> Option<Value> {
    Some(json!({
        "schemaVersion": 1,
        "rootId": root_id,
        "limit": limit,
        "cursor": cursor,
        "snapshotId": snapshot_id,
    }))
}

fn ok_data<T: Serialize>(envelope: crate::foundation::CommandEnvelope<T>) -> serde_json::Value {
    let value = serde_json::to_value(envelope).expect("envelope serializes");
    assert_eq!(value["status"], "ok", "{value}");
    value["data"].clone()
}

fn error_code<T: Serialize>(envelope: crate::foundation::CommandEnvelope<T>) -> String {
    let value = serde_json::to_value(envelope).expect("envelope serializes");
    assert_eq!(value["status"], "error", "{value}");
    value["error"]["code"]
        .as_str()
        .expect("error code")
        .to_owned()
}

#[test]
fn scan_now_runs_to_completion_and_statuses_report_contract_fields() {
    let harness = Harness::new("queued-running-completed");
    let (runtime, _) = test_runtime();

    let data = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    assert_eq!(data["outcome"], "queued");
    assert_eq!(data["rootId"], harness.root_id);
    let job_id = data["jobId"].as_str().expect("job id").to_owned();
    assert!(data["runId"].is_null());

    harness.tick(tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
    ]));

    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data.as_array().expect("statuses").len(), 1);
    let status = &data[0];
    assert_eq!(status["root"]["id"], harness.root_id);
    assert_eq!(status["root"]["displayName"], "Synthetic");
    assert_eq!(status["root"]["canonicalPath"], r"C:\synthetic-root");
    assert_eq!(status["root"]["enabled"], true);
    assert_eq!(status["root"]["availability"], "available");
    assert!(status["root"]["lastErrorCode"].is_null());
    assert_eq!(status["state"], "completed");
    assert_eq!(status["jobId"], job_id);
    assert!(!status["runId"].as_str().expect("run id").is_empty());
    assert_eq!(status["cancellationRequested"], false);
    assert_eq!(status["retryAvailable"], false);
    assert_eq!(status["counters"]["filesObserved"], 2);
    assert_eq!(status["counters"]["directoriesVisited"], 0);
    assert!(status["counters"]["totalFiles"].is_null());
    assert_eq!(status["lastSuccessfulScanAt"], "2023-11-14T22:13:20Z");
    assert_eq!(status["lastOutcomeAt"], "2023-11-14T22:13:20Z");
    assert!(status["errorCode"].is_null());

    let status_keys: std::collections::BTreeSet<&str> = status
        .as_object()
        .expect("status object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        status_keys,
        [
            "root",
            "state",
            "jobId",
            "runId",
            "cancellationRequested",
            "retryAvailable",
            "counters",
            "lastSuccessfulScanAt",
            "lastOutcomeAt",
            "errorCode",
        ]
        .into_iter()
        .collect()
    );

    let page = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 10, None, None),
    ));
    assert_eq!(page["rootId"], harness.root_id);
    assert!(
        page["snapshotId"]
            .as_str()
            .expect("snapshot id")
            .starts_with("{\"v\":1")
    );
    assert!(
        page["nextCursor"].is_null(),
        "no more rows: cursor must be null"
    );
    let records = page["records"].as_array().expect("records");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["locationId"].as_str().expect("id").len(), 36);
    assert_eq!(records[0]["rootId"], harness.root_id);
    assert_eq!(records[0]["rootDisplayName"], "Synthetic");
    assert_eq!(records[0]["rootCanonicalPath"], r"C:\synthetic-root");
    assert_eq!(records[0]["fileName"], "a.flp");
    assert_eq!(records[0]["relativePath"], "a.flp");
    assert_eq!(records[0]["byteSize"], "1024");
    assert_eq!(records[0]["modifiedAt"], "2023-11-14T22:13:20Z");
    assert_eq!(records[0]["presence"], "present");
    assert_eq!(records[1]["fileName"], "b.flp");

    // Events: the command emitted queued; the poll loop emitted running then
    // completed, all carrying root ids, states and counters only.
    let events = harness.sink.events();
    let states: Vec<ScanExecutionState> = events.iter().map(|event| event.roots[0].state).collect();
    assert_eq!(
        states,
        [
            ScanExecutionState::Queued,
            ScanExecutionState::Running,
            ScanExecutionState::Completed
        ]
    );
    assert!(
        events
            .iter()
            .all(|event| event.roots[0].root_id == harness.root_id)
    );
    assert_eq!(events[2].roots[0].counters.files_observed, 2);
}

#[test]
fn cancelling_a_queued_job_is_immediate_and_creates_no_run() {
    let harness = Harness::new("cancel-queued");
    let (runtime, _) = test_runtime();

    let data = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let job_id = data["jobId"].as_str().expect("job id").to_owned();

    let data = ok_data(handle_cancel_scan(
        &runtime,
        &harness.service,
        cancel_request(&job_id),
    ));
    assert_eq!(data["rootId"], harness.root_id);
    assert_eq!(data["jobId"], job_id);
    assert_eq!(data["outcome"], "cancelled");
    assert!(data["runId"].is_null(), "a queued job has no run");

    // No work may be leased for a cancelled job.
    harness.tick(tree(vec![file_entry("a.flp", 101)]));
    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "cancelled");
    assert!(data[0]["runId"].is_null());
    assert_eq!(data[0]["errorCode"], "cancelled");
    assert!(data[0]["lastSuccessfulScanAt"].is_null());

    let data = ok_data(handle_cancel_scan(
        &runtime,
        &harness.service,
        cancel_request(&job_id),
    ));
    assert_eq!(data["outcome"], "already_cancelled");
}

#[test]
fn cancelling_a_running_job_requests_cancellation_and_never_publishes() {
    let harness = Harness::new("cancel-running");
    let (runtime, _) = test_runtime();

    let data = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let job_id = data["jobId"].as_str().expect("job id").to_owned();

    harness.claim_due();
    let run_id = {
        let database = harness.database.lock().unwrap();
        let runs = database.list_scan_runs().expect("runs");
        runs.iter()
            .find(|run| run.scan_job_id == job_id)
            .expect("leased run")
            .id
            .clone()
    };

    let data = ok_data(handle_cancel_scan(
        &runtime,
        &harness.service,
        cancel_request(&job_id),
    ));
    assert_eq!(data["outcome"], "cancellation_requested");
    assert_eq!(data["runId"], run_id);

    harness.execute_pending(tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
    ]));

    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "cancelled");
    assert_eq!(data[0]["runId"], run_id);
    assert_eq!(data[0]["cancellationRequested"], true);
    assert_eq!(data[0]["errorCode"], "cancelled");

    // Nothing was published: the library stays empty.
    let page = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 10, None, None),
    ));
    assert_eq!(page["records"].as_array().expect("records").len(), 0);

    let data = ok_data(handle_cancel_scan(
        &runtime,
        &harness.service,
        cancel_request(&job_id),
    ));
    assert_eq!(data["outcome"], "already_cancelled");
}

#[test]
fn failed_runs_retry_manually_and_through_the_persisted_backoff() {
    let harness = Harness::new("retry-failure");
    let (runtime, _) = test_runtime();

    let data = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let job_id = data["jobId"].as_str().expect("job id").to_owned();

    // An offline root fails the run; the retry chain persists with its budget.
    harness.tick_port(FakePort::offline(tree(vec![file_entry("a.flp", 101)])));
    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "failed");
    assert_eq!(data[0]["errorCode"], "unavailable");
    assert_eq!(data[0]["retryAvailable"], true);

    // Manual retry before the persisted backoff eligibility.
    let data = ok_data(handle_retry_scan(
        &runtime,
        &harness.service,
        retry_request(&job_id),
    ));
    assert_eq!(data["outcome"], "queued");
    assert_eq!(data["jobId"], job_id);
    assert!(data["runId"].is_null());

    harness.tick(tree(vec![file_entry("a.flp", 101)]));
    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "completed");
    assert_eq!(data[0]["counters"]["filesObserved"], 1);
    assert!(data[0]["lastSuccessfulScanAt"].is_string());

    // Automatic retry: fail again, advance past the 1 s backoff (+jitter), and
    // one tick both services the retry and executes it.
    let data = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let job_id = data["jobId"].as_str().expect("job id").to_owned();
    harness.tick_port(FakePort::offline(tree(vec![file_entry("a.flp", 101)])));
    harness.clock.advance(1_300);
    harness.tick(tree(vec![file_entry("a.flp", 101)]));
    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "completed");
    assert_eq!(data[0]["jobId"], job_id);
}

#[test]
fn restart_marks_interrupted_work_and_resumes_it_through_recovery() {
    let mut harness = Harness::new("restart-recovery");
    let (runtime, _) = test_runtime();

    let data = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let job_id = data["jobId"].as_str().expect("job id").to_owned();
    harness.claim_due();
    let run_id = {
        let database = harness.database.lock().unwrap();
        database
            .list_scan_runs()
            .expect("runs")
            .iter()
            .find(|run| run.scan_job_id == job_id)
            .expect("leased run")
            .id
            .clone()
    };

    // "Crash" and restart: a fresh session fences the prior running lease as
    // interrupted and resumes the same job with its persisted attempt budget.
    harness.restart();

    {
        let database = harness.database.lock().unwrap();
        let run = database.scan_run(&run_id).expect("run");
        assert_eq!(run.state, fruitboard_storage::ScanRunState::Interrupted);
        assert_eq!(run.error_code.as_deref(), Some("restart"));
        let job = database.scan_job(&job_id).expect("job");
        assert_eq!(job.state, ScanJobState::Queued);
        assert_eq!(job.attempt, 1, "the retry-chain budget survives restart");
    }

    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "queued");
    assert_eq!(data[0]["jobId"], job_id);

    harness.clock.advance(1_300);
    harness.tick(tree(vec![file_entry("a.flp", 101)]));
    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "completed");
    assert!(
        data[0]["runId"].as_str().expect("run id") != run_id,
        "the recovery allocated a fresh run for the resumed attempt"
    );
    {
        // The interrupted run keeps its durable identity and outcome.
        let database = harness.database.lock().unwrap();
        let run = database.scan_run(&run_id).expect("run");
        assert_eq!(run.state, fruitboard_storage::ScanRunState::Interrupted);
        assert_eq!(
            run.outcome,
            Some(fruitboard_storage::ScanRunOutcome::Interrupted)
        );
    }
}

#[test]
fn library_pages_are_snapshot_fenced_against_publication() {
    let harness = Harness::new("page-fencing");
    let (runtime, _) = test_runtime();

    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.tick(tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
    ]));

    let page = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 1, None, None),
    ));
    assert_eq!(page["records"].as_array().expect("records").len(), 1);
    assert_eq!(page["records"][0]["fileName"], "a.flp");
    let snapshot_one = page["snapshotId"].as_str().expect("snapshot id").to_owned();
    let cursor = page["nextCursor"].as_str().expect("next cursor").to_owned();

    // A second publication replaces the committed snapshot.
    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.tick(tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
        file_entry("c.flp", 103),
    ]));

    assert_eq!(
        error_code(handle_get_library_page(
            &runtime,
            &harness.service,
            page_request(&harness.root_id, 1, Some(&cursor), None),
        )),
        "stale_cursor"
    );
    assert_eq!(
        error_code(handle_get_library_page(
            &runtime,
            &harness.service,
            page_request(&harness.root_id, 1, None, Some(&snapshot_one)),
        )),
        "stale_cursor"
    );

    // A fresh pagination reads the new snapshot completely and stably.
    let page = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 2, None, None),
    ));
    let snapshot_two = page["snapshotId"].as_str().expect("snapshot id").to_owned();
    assert_ne!(snapshot_two, snapshot_one);
    assert_eq!(page["records"].as_array().expect("records").len(), 2);
    let cursor = page["nextCursor"].as_str().expect("next cursor").to_owned();
    let page = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 2, Some(&cursor), None),
    ));
    assert_eq!(page["records"].as_array().expect("records").len(), 1);
    assert_eq!(page["records"][0]["fileName"], "c.flp");
    assert!(page["nextCursor"].is_null());
}

#[test]
fn malformed_and_wrong_root_cursors_return_typed_invalid_cursor() {
    let harness = Harness::new("invalid-cursors");
    let (runtime, _) = test_runtime();
    let other_root = harness.add_root("Other", r"C:\synthetic-other");

    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.tick(tree(vec![file_entry("a.flp", 101)]));
    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&other_root),
    ));
    harness.tick(tree(vec![
        file_entry("x.flp", 201),
        file_entry("y.flp", 202),
    ]));
    let other_page = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&other_root, 1, None, None),
    ));
    let other_cursor = other_page["nextCursor"]
        .as_str()
        .expect("cursor")
        .to_owned();

    for (label, request) in [
        (
            "garbage",
            page_request(&harness.root_id, 10, Some("not-a-token"), None),
        ),
        (
            "truncated json",
            page_request(&harness.root_id, 10, Some("{\"v\":1,\"rootId\":"), None),
        ),
        (
            "wrong version",
            page_request(
                &harness.root_id,
                10,
                Some(
                    "{\"v\":9,\"rootId\":\"x\",\"snapshot\":null,\"locatorKey\":\"k\",\"locationId\":\"l\"}",
                ),
                None,
            ),
        ),
        (
            "wrong root",
            page_request(&harness.root_id, 10, Some(&other_cursor), None),
        ),
        (
            "bad snapshot token",
            page_request(
                &harness.root_id,
                10,
                None,
                Some(
                    "{\"v\":1,\"lastSuccessfulRunId\":\"r\",\"lastSuccessfulGeneration\":\"01\",\"lastSuccessfulAtMs\":\"0\"}",
                ),
            ),
        ),
    ] {
        assert_eq!(
            error_code(handle_get_library_page(&runtime, &harness.service, request)),
            "invalid_cursor",
            "{label}"
        );
    }

    // Unknown root pages surface not_found.
    assert_eq!(
        error_code(handle_get_library_page(
            &runtime,
            &harness.service,
            page_request("missing-root", 10, None, None),
        )),
        "not_found"
    );
}

#[test]
fn scan_now_coalesces_and_rejects_unknown_and_disabled_roots() {
    let harness = Harness::new("coalescing");
    let (runtime, _) = test_runtime();

    let first = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let job_id = first["jobId"].as_str().expect("job id").to_owned();
    assert_eq!(first["outcome"], "queued");

    let second = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    assert_eq!(second["outcome"], "already_queued");
    assert_eq!(second["jobId"], job_id);

    harness.claim_due();
    let third = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    assert_eq!(third["outcome"], "already_running");
    assert_eq!(third["jobId"], job_id);
    assert!(!third["runId"].as_str().expect("run id").is_empty());

    // The trigger during traversal invalidates the attempt and schedules one
    // deduplicated follow-up, which the statuses surface as queued.
    harness.execute_pending(tree(vec![file_entry("a.flp", 101)]));
    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "queued");
    assert!(!data[0]["jobId"].as_str().expect("job id").is_empty());

    assert_eq!(
        error_code(handle_scan_now(
            &runtime,
            &harness.service,
            scan_now_request("missing-root")
        )),
        "not_found"
    );

    let disabled_root = harness.add_root("Disabled", r"C:\synthetic-disabled");
    harness.set_root_enabled(&disabled_root, false);
    assert_eq!(
        error_code(handle_scan_now(
            &runtime,
            &harness.service,
            scan_now_request(&disabled_root)
        )),
        "conflict"
    );
}

#[test]
fn disabled_failed_roots_do_not_offer_or_start_recovery() {
    let harness = Harness::new("disabled-failed-recovery");
    let (runtime, _) = test_runtime();
    let started = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let job_id = started["jobId"].as_str().expect("job id").to_owned();
    harness.tick_port(FakePort::offline(tree(vec![file_entry("a.flp", 101)])));
    harness.set_root_enabled(&harness.root_id, false);

    let statuses = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(statuses[0]["state"], "failed");
    assert_eq!(statuses[0]["retryAvailable"], false);
    assert_eq!(statuses[0]["root"]["enabled"], false);

    assert_eq!(
        error_code(handle_retry_scan(
            &runtime,
            &harness.service,
            retry_request(&job_id),
        )),
        "conflict"
    );
    assert_eq!(
        error_code(handle_scan_now(
            &runtime,
            &harness.service,
            scan_now_request(&harness.root_id),
        )),
        "conflict"
    );
    let database = harness.database.lock().unwrap();
    let job = database.scan_job(&job_id).expect("failed job");
    assert_eq!(job.state, ScanJobState::Failed);
    assert_eq!(job.attempt, 1);
}

#[test]
fn terminal_jobs_reject_retry_and_report_already_outcomes() {
    let harness = Harness::new("terminal-jobs");
    let (runtime, _) = test_runtime();

    let data = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let job_id = data["jobId"].as_str().expect("job id").to_owned();
    harness.tick(tree(vec![file_entry("a.flp", 101)]));

    assert_eq!(
        error_code(handle_retry_scan(
            &runtime,
            &harness.service,
            retry_request(&job_id)
        )),
        "conflict",
        "completed chains are never revived"
    );
    let data = ok_data(handle_cancel_scan(
        &runtime,
        &harness.service,
        cancel_request(&job_id),
    ));
    assert_eq!(data["outcome"], "already_completed");
    assert_eq!(
        error_code(handle_cancel_scan(
            &runtime,
            &harness.service,
            cancel_request("missing-job")
        )),
        "not_found"
    );
    assert_eq!(
        error_code(handle_retry_scan(
            &runtime,
            &harness.service,
            retry_request("missing-job")
        )),
        "not_found"
    );
}

#[test]
fn scan_now_starts_a_fresh_chain_after_cancelled_work() {
    let harness = Harness::new("scan-now-after-cancel");
    let (runtime, _) = test_runtime();

    let first = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let cancelled_job_id = first["jobId"].as_str().expect("job id").to_owned();
    ok_data(handle_cancel_scan(
        &runtime,
        &harness.service,
        cancel_request(&cancelled_job_id),
    ));

    harness.clock.advance(1);
    let second = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let fresh_job_id = second["jobId"].as_str().expect("fresh job id");
    assert_ne!(fresh_job_id, cancelled_job_id);
    assert_eq!(second["outcome"], "queued");
    assert!(second["runId"].is_null());

    let database = harness.database.lock().unwrap();
    let cancelled = database.scan_job(&cancelled_job_id).expect("cancelled job");
    let fresh = database.scan_job(fresh_job_id).expect("fresh job");
    assert_eq!(cancelled.state, ScanJobState::Cancelled);
    assert_eq!(fresh.state, ScanJobState::Queued);
    assert_eq!(fresh.attempt, 0, "fresh work starts a new chain");
    assert_ne!(cancelled.retry_chain_id, fresh.retry_chain_id);
}

#[test]
fn page_limit_is_clamped_to_the_contract_bound() {
    let harness = Harness::new("limit-clamp");
    let (runtime, _) = test_runtime();
    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.tick(tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
        file_entry("c.flp", 103),
    ]));

    let page = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 0, None, None),
    ));
    assert_eq!(page["records"].as_array().expect("records").len(), 1);
    assert!(page["nextCursor"].is_string());

    let page = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 10_000, None, None),
    ));
    assert_eq!(page["records"].as_array().expect("records").len(), 3);
    assert!(page["nextCursor"].is_null());
}

#[test]
fn idle_roots_report_honest_empty_status() {
    let harness = Harness::new("idle-status");
    let (runtime, _) = test_runtime();

    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data.as_array().expect("statuses").len(), 1);
    assert_eq!(data[0]["state"], "idle");
    assert!(data[0]["jobId"].is_null());
    assert!(data[0]["runId"].is_null());
    assert_eq!(data[0]["cancellationRequested"], false);
    assert_eq!(data[0]["counters"]["filesObserved"], 0);
    assert!(data[0]["lastSuccessfulScanAt"].is_null());
    assert!(data[0]["lastOutcomeAt"].is_null());
    assert!(data[0]["errorCode"].is_null());

    let data = ok_data(handle_get_scan_console_state(
        &runtime,
        &harness.service,
        Some(json!({ "schemaVersion": 1 })),
    ));
    assert_eq!(data["enabled"], true);
}

#[test]
fn rfc3339_formatter_preserves_nanoseconds_and_trims_zeroes() {
    assert_eq!(unix_ns_to_rfc3339(0), "1970-01-01T00:00:00Z");
    assert_eq!(
        unix_ns_to_rfc3339(1_700_000_000_000_000_000),
        "2023-11-14T22:13:20Z"
    );
    assert_eq!(
        unix_ns_to_rfc3339(1_700_000_000_000_000_000 + 123_456_789),
        "2023-11-14T22:13:20.123456789Z"
    );
    assert_eq!(
        unix_ns_to_rfc3339(1_700_000_000_000_000_000 + 500_000_000),
        "2023-11-14T22:13:20.5Z"
    );
    assert_eq!(unix_ns_to_rfc3339(-1), "1969-12-31T23:59:59.999999999Z");
    assert_eq!(
        unix_ns_to_rfc3339(1_709_164_800_000_000_000),
        "2024-02-29T00:00:00Z"
    );
}

#[test]
fn opaque_tokens_round_trip_and_reject_malformed_input() {
    let snapshot = LibrarySnapshot {
        last_successful_run_id: Some("run-1".to_owned()),
        last_successful_generation: Some(3),
        last_successful_at_ms: Some(1_700_000_000_000),
    };
    let encoded = encode_snapshot_token(&snapshot);
    assert_eq!(decode_snapshot_token(&encoded).expect("decode"), snapshot);
    assert!(decode_snapshot_token("garbage").is_err());
    assert!(decode_snapshot_token("{\"v\":2,\"lastSuccessfulRunId\":null,\"lastSuccessfulGeneration\":null,\"lastSuccessfulAtMs\":null}").is_err());
    assert!(decode_snapshot_token("{\"v\":1,\"lastSuccessfulRunId\":\"r\",\"lastSuccessfulGeneration\":null,\"lastSuccessfulAtMs\":null}").is_err());

    let cursor = LibraryCursor {
        scan_root_id: "root-1".to_owned(),
        snapshot: snapshot.clone(),
        locator_key: "v1:i:a.flp".to_owned(),
        location_id: "location-1".to_owned(),
    };
    let encoded = encode_cursor_token(&cursor);
    assert_eq!(
        decode_cursor_token(&encoded, "root-1").expect("decode"),
        cursor
    );
    assert!(
        decode_cursor_token(&encoded, "root-2").is_err(),
        "wrong root rejected"
    );
    assert!(decode_cursor_token("{\"v\":1,\"rootId\":\"root-1\",\"snapshot\":null,\"locatorKey\":\"\",\"locationId\":\"l\"}", "root-1").is_err());

    assert_eq!(parse_canonical_i64("0"), Ok(0));
    assert_eq!(parse_canonical_i64("42"), Ok(42));
    assert!(parse_canonical_i64("01").is_err());
    assert!(parse_canonical_i64("-1").is_err());
    assert!(parse_canonical_i64("").is_err());
    assert!(parse_canonical_i64("9_223_372_036_854_775_808").is_err());
}

#[test]
fn events_never_leak_paths_or_tokens() {
    let harness = Harness::new("event-privacy");
    let (runtime, _) = test_runtime();
    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.tick(tree(vec![file_entry("private-project.flp", 101)]));

    for event in harness.sink.events() {
        let serialized = serde_json::to_string(&event).expect("event serializes");
        assert!(!serialized.contains("synthetic-root"));
        assert!(!serialized.contains("private-project"));
        assert!(
            event
                .roots
                .iter()
                .all(|root| root.state != ScanExecutionState::Completed
                    || root.counters.files_observed > 0)
        );
    }
}

fn many_files(count: usize) -> Vec<Entry> {
    (1..=count)
        .map(|index| file_entry(&format!("p{index:03}.flp"), 10_000 + index as u128))
        .collect()
}

/// Deterministic gate that parks the worker inside filesystem I/O (no
/// database lock held) so the main thread can prove statuses/pages stay
/// responsive mid-scan without any wall-clock sleeps.
struct BlockingGate {
    entered: Mutex<Option<mpsc::Sender<()>>>,
    release: Mutex<mpsc::Receiver<()>>,
    fired: AtomicBool,
}

struct BlockingPort {
    root: Entry,
    gate: StdArc<BlockingGate>,
}

impl BlockingPort {
    fn new(root: Entry, gate: StdArc<BlockingGate>) -> Self {
        Self { root, gate }
    }
}

struct BlockingCursor {
    directory: Entry,
    prefix: String,
    index: usize,
    gate: StdArc<BlockingGate>,
}

impl BlockingCursor {
    fn display(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{}\\{}", self.prefix, name)
        }
    }

    fn park_once(&self, name: &str) {
        // Park exactly once on the first observed file so the worker is
        // deterministically inside traversal (between staged batches) while
        // holding no database lock.
        if name == "p001.flp" && !self.gate.fired.swap(true, Ordering::SeqCst) {
            if let Some(sender) = self.gate.entered.lock().unwrap().take() {
                let _ = sender.send(());
            }
            let _ = self.gate.release.lock().unwrap().recv();
        }
    }
}

impl fruitboard_filesystem_enumeration::FilesystemPort for BlockingPort {
    fn inspect_root(&mut self, _root: &std::path::Path) -> Result<RootMetadata, PortError> {
        Ok(RootMetadata {
            metadata: self.root.metadata(),
            qualification: FilesystemQualification::LocalNtfs,
        })
    }

    fn open_root(&mut self, _root: &std::path::Path) -> Result<OpenedDirectory, PortError> {
        Ok(OpenedDirectory {
            metadata: self.root.metadata(),
            case_sensitivity: DirectoryCaseSensitivity::Insensitive,
            cursor: Box::new(BlockingCursor {
                directory: self.root.clone(),
                prefix: String::new(),
                index: 0,
                gate: self.gate.clone(),
            }),
        })
    }
}

impl DirectoryCursor for BlockingCursor {
    fn next_entry(&mut self) -> Result<Option<DirectoryEntry>, PortError> {
        Ok(self.directory.children.get(self.index).map(|child| {
            self.index += 1;
            DirectoryEntry::new(child.name.clone())
        }))
    }

    fn read_metadata(&mut self, entry: &DirectoryEntry) -> Result<FileMetadata, PortError> {
        let name = entry.name.to_string_lossy().into_owned();
        self.park_once(&name);
        let child = self
            .directory
            .children
            .iter()
            .find(|child| child.name == name)
            .expect("scripted child exists");
        Ok(child.metadata())
    }

    fn open_directory(&mut self, entry: &DirectoryEntry) -> Result<OpenedDirectory, PortError> {
        let name = entry.name.to_string_lossy().into_owned();
        let child = self
            .directory
            .children
            .iter()
            .find(|child| child.name == name)
            .expect("scripted child exists");
        Ok(OpenedDirectory {
            metadata: child.metadata(),
            case_sensitivity: DirectoryCaseSensitivity::Insensitive,
            cursor: Box::new(BlockingCursor {
                directory: child.clone(),
                prefix: self.display(&name),
                index: 0,
                gate: self.gate.clone(),
            }),
        })
    }
}

fn blocking_gate() -> (StdArc<BlockingGate>, mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (entered_tx, entered_rx) = mpsc::channel::<()>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let gate = StdArc::new(BlockingGate {
        entered: Mutex::new(Some(entered_tx)),
        release: Mutex::new(release_rx),
        fired: AtomicBool::new(false),
    });
    (gate, entered_rx, release_tx)
}

#[test]
fn statuses_and_pages_stay_responsive_mid_scan() {
    let harness = Harness::new("responsive-mid-scan");
    let (runtime, _) = test_runtime();

    // Seed one committed snapshot so pages have stable rows to read back.
    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.tick(tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
    ]));

    // Queue a multi-batch follow-up (600 files over the 512-record batch
    // bound) and claim it so the job is durably running.
    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.claim_due();

    // While durably running (before traversal starts) both reads succeed and
    // still observe the prior committed snapshot.
    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "running");
    let page = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 10, None, None),
    ));
    assert_eq!(page["records"].as_array().expect("records").len(), 2);

    // Park the worker inside filesystem I/O on a background thread. The
    // worker holds no database lock while parked, so the main thread's reads
    // must succeed instead of serializing behind the traversal.
    let (gate, entered_rx, release_tx) = blocking_gate();
    let host = harness.host();
    let database = harness.database.clone();
    let handle = std::thread::spawn(move || {
        let mut port = BlockingPort::new(tree(many_files(600)), gate);
        host.execute_pending(&database, &mut port)
    });
    entered_rx.recv().expect("worker parks mid-scan");

    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "running");
    let page = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 10, None, None),
    ));
    assert_eq!(
        page["records"].as_array().expect("records").len(),
        2,
        "mid-scan pages still read the prior committed snapshot"
    );

    release_tx.send(()).expect("release worker");
    assert!(handle.join().expect("join worker"), "pending executed");

    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "completed");
    // The second publication retains the two prior rows as missing plus the
    // 600 new present rows, so the durable location count is 602.
    assert_eq!(data[0]["counters"]["filesObserved"], 602);
    let page = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 10, None, None),
    ));
    assert_eq!(page["records"].as_array().expect("records").len(), 10);
}

#[test]
fn cancel_mid_batches_leaves_prior_snapshot_untouched() {
    let harness = Harness::new("cancel-mid-batches");
    let (runtime, _) = test_runtime();

    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.tick(tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
    ]));
    let committed_before = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 10, None, None),
    ));

    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.claim_due();
    let job_id = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ))[0]["jobId"]
        .as_str()
        .expect("job id")
        .to_owned();
    let run_id = {
        let database = harness.database.lock().unwrap();
        database
            .list_scan_runs()
            .expect("runs")
            .iter()
            .find(|run| run.scan_job_id == job_id)
            .expect("leased run")
            .id
            .clone()
    };

    let (gate, entered_rx, release_tx) = blocking_gate();
    let host = harness.host();
    let database = harness.database.clone();
    let handle = std::thread::spawn(move || {
        let mut port = BlockingPort::new(tree(many_files(600)), gate);
        host.execute_pending(&database, &mut port)
    });
    entered_rx.recv().expect("worker parks mid-scan");

    // Mirror-first cancel: the in-memory flip never blocks, then the durable
    // commit is the authority. Statuses stay responsive while parked.
    let data = ok_data(handle_cancel_scan(
        &runtime,
        &harness.service,
        cancel_request(&job_id),
    ));
    assert_eq!(data["outcome"], "cancellation_requested");
    assert_eq!(data["runId"], run_id);
    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "running");
    assert_eq!(data[0]["cancellationRequested"], true);

    release_tx.send(()).expect("release worker");
    assert!(handle.join().expect("join worker"));

    let data = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(data[0]["state"], "cancelled");
    assert_eq!(data[0]["runId"], run_id);
    assert_eq!(data[0]["cancellationRequested"], true);
    assert_eq!(data[0]["errorCode"], "cancelled");

    // Nothing published: the prior committed snapshot is untouched.
    let committed_after = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 10, None, None),
    ));
    assert_eq!(committed_after, committed_before);
}

#[test]
fn restart_drops_in_memory_counters_but_durable_statuses_survive() {
    let mut harness = Harness::new("restart-counters");
    let (runtime, _) = test_runtime();

    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.tick(tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
    ]));
    let before = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(before[0]["state"], "completed");
    assert_eq!(before[0]["counters"]["filesObserved"], 2);
    assert_eq!(before[0]["counters"]["directoriesVisited"], 0);
    assert!(before[0]["counters"]["totalFiles"].is_null());
    let page_before = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 10, None, None),
    ));
    assert_eq!(page_before["records"].as_array().expect("records").len(), 2);

    harness.restart();

    // Durable statuses survive the restart; the honest in-memory counters
    // reset to zero (documented limitation of this slice). The restart
    // schedules one deduplicated recovery job, so the latest job is queued —
    // the prior publication timestamp and library rows are what survive.
    let after = ok_data(handle_list_scan_statuses(
        &runtime,
        &harness.service,
        statuses_request(),
    ));
    assert_eq!(after[0]["state"], "queued");
    assert_ne!(after[0]["jobId"], before[0]["jobId"]);
    assert!(after[0]["runId"].is_null());
    assert_eq!(
        after[0]["lastSuccessfulScanAt"],
        before[0]["lastSuccessfulScanAt"]
    );
    assert_eq!(after[0]["counters"]["filesObserved"], 0);
    assert_eq!(after[0]["counters"]["directoriesVisited"], 0);
    assert!(after[0]["counters"]["totalFiles"].is_null());

    let page_after = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 10, None, None),
    ));
    assert_eq!(page_after, page_before);
}

#[test]
fn retry_exhausted_reports_conflict_not_requeue() {
    let harness = Harness::new("retry-exhausted");
    let (runtime, _) = test_runtime();

    let data = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let job_id = data["jobId"].as_str().expect("job id").to_owned();

    // Exhaust the persisted retry budget (1 initial + 3 automatic retries):
    // four offline failures bring the attempt to its durable maximum.
    for _ in 0..4 {
        harness.tick_port(FakePort::offline(tree(vec![file_entry("a.flp", 101)])));
        let data = ok_data(handle_list_scan_statuses(
            &runtime,
            &harness.service,
            statuses_request(),
        ));
        assert_eq!(data[0]["state"], "failed");
        if data[0]["jobId"].as_str().expect("job id") != job_id {
            panic!("exhaustion must stay on the same retry chain");
        }
        // All but the last failure are still requeueable.
        let failed_attempt = {
            let database = harness.database.lock().unwrap();
            database.scan_job(&job_id).expect("job").attempt
        };
        if failed_attempt < 4 {
            let data = ok_data(handle_retry_scan(
                &runtime,
                &harness.service,
                retry_request(&job_id),
            ));
            assert_eq!(data["outcome"], "queued");
        }
    }
    {
        let database = harness.database.lock().unwrap();
        let job = database.scan_job(&job_id).expect("job");
        assert_eq!(job.state, fruitboard_storage::ScanJobState::Failed);
        assert_eq!(job.attempt, job.max_attempts);
    }
    // The closed ScanStartOutcome union has no "already_failed": the native
    // boundary reports the safe conflict; an explicit Scan now below creates
    // a new job instead of reviving this exhausted chain.
    assert_eq!(
        error_code(handle_retry_scan(
            &runtime,
            &harness.service,
            retry_request(&job_id)
        )),
        "conflict"
    );

    // Exhausted work is not revived by Retry. An explicit Scan now creates a
    // separate job and retry chain while preserving the exhausted record.
    harness.clock.advance(1);
    let fresh = ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    let fresh_job_id = fresh["jobId"].as_str().expect("fresh job id");
    assert_ne!(fresh_job_id, job_id);
    assert_eq!(fresh["outcome"], "queued");
    let database = harness.database.lock().unwrap();
    let exhausted = database.scan_job(&job_id).expect("exhausted job");
    let fresh = database.scan_job(fresh_job_id).expect("fresh job");
    assert_eq!(exhausted.state, ScanJobState::Failed);
    assert_eq!(exhausted.attempt, exhausted.max_attempts);
    assert_eq!(fresh.state, ScanJobState::Queued);
    assert_eq!(fresh.attempt, 0);
    assert_ne!(exhausted.retry_chain_id, fresh.retry_chain_id);
}

#[test]
fn storage_busy_maps_to_unavailable_without_leaking_diagnostics() {
    use fruitboard_storage::StorageError;
    let scan = map_scan_storage_error(StorageError::Busy);
    assert_eq!(
        scan.user().code,
        crate::foundation::errors::ErrorCode::Unavailable
    );
    assert_eq!(
        scan.diagnostic().diagnostic_code,
        crate::foundation::errors::DiagnosticCode::StorageBusy
    );
    let enqueue = map_enqueue_storage_error(StorageError::Busy);
    assert_eq!(
        enqueue.user().code,
        crate::foundation::errors::ErrorCode::Unavailable
    );
    let page = map_page_storage_error(StorageError::Busy);
    assert_eq!(
        page.user().code,
        crate::foundation::errors::ErrorCode::Unavailable
    );
    // Fixed safe copy only: no SQL, paths, or tokens in the user message.
    for error in [scan, enqueue, page] {
        let serialized = serde_json::to_value(error.user()).expect("user error serializes");
        let text = serialized.to_string();
        assert!(!text.contains("SELECT"));
        assert!(!text.contains("scan_"));
        assert_eq!(
            serialized["message"],
            "The requested service is temporarily unavailable."
        );
    }
}

#[test]
fn cursor_snapshot_mismatch_is_invalid_not_stale() {
    let harness = Harness::new("cursor-mismatch");
    let (runtime, _) = test_runtime();

    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.tick(tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
    ]));
    let first = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 1, None, None),
    ));
    let stale_cursor = first["nextCursor"].as_str().expect("cursor").to_owned();
    let stale_snapshot = first["snapshotId"].as_str().expect("snapshot").to_owned();

    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.tick(tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
        file_entry("c.flp", 103),
    ]));
    let fresh = ok_data(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 1, None, None),
    ));
    let fresh_snapshot = fresh["snapshotId"].as_str().expect("snapshot").to_owned();
    assert_ne!(fresh_snapshot, stale_snapshot);

    // Old cursor or old snapshot alone against the replaced publication is
    // stale; mixing generations across the two token slots is malformed.
    assert_eq!(
        error_code(handle_get_library_page(
            &runtime,
            &harness.service,
            page_request(&harness.root_id, 1, Some(&stale_cursor), None),
        )),
        "stale_cursor"
    );
    assert_eq!(
        error_code(handle_get_library_page(
            &runtime,
            &harness.service,
            page_request(
                &harness.root_id,
                1,
                Some(&stale_cursor),
                Some(&fresh_snapshot),
            ),
        )),
        "invalid_cursor"
    );
}

#[test]
fn error_and_event_payloads_carry_no_paths_tokens_or_sql() {
    let harness = Harness::new("privacy-gaps");
    let (runtime, _) = test_runtime();

    ok_data(handle_scan_now(
        &runtime,
        &harness.service,
        scan_now_request(&harness.root_id),
    ));
    harness.tick(tree(vec![file_entry("secret-project.flp", 101)]));

    // Events carry root ids, states and counters only.
    for event in harness.sink.events() {
        let serialized = serde_json::to_string(&event).expect("event serializes");
        let lowered = serialized.to_lowercase();
        for needle in [
            "secret-project",
            "synthetic-root",
            "c:\\",
            "select",
            "lease",
            "token",
            "bearer",
            "cursor",
            "snapshot",
        ] {
            assert!(
                !lowered.contains(needle),
                "event leaks {needle}: {serialized}"
            );
        }
    }

    // Typed error envelopes carry fixed safe copy only.
    let page_err = serde_json::to_value(handle_get_library_page(
        &runtime,
        &harness.service,
        page_request(&harness.root_id, 10, Some("not-a-token"), None),
    ))
    .expect("envelope serializes");
    let cancel_err = serde_json::to_value(handle_cancel_scan(
        &runtime,
        &harness.service,
        cancel_request("missing-job"),
    ))
    .expect("envelope serializes");
    for serialized in [page_err, cancel_err] {
        let text = serialized.to_string().to_lowercase();
        for needle in ["select", "lease", "token", "c:\\", "secret-project"] {
            assert!(!text.contains(needle), "error leaks {needle}: {text}");
        }
    }
}
