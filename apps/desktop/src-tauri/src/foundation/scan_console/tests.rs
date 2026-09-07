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
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

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
