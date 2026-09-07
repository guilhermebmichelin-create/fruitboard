use super::*;
use enumeration::{
    CancellationToken, DirectoryCaseSensitivity, DirectoryCursor, DirectoryEntry, EntryKind,
    FileMetadata, FilesystemPort, FilesystemQualification, NeverCancelled, OpenedDirectory,
    Outcome as EnumOutcome, PortError, QualifiedIdentity, RootMetadata,
};
use fruitboard_storage::{ScanJobState, ScanRunState, ScanStageState, MAX_LIBRARY_PAGE_SIZE};
use rusqlite::{params, Connection};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

const T0: i64 = 1_700_000_000_000;
const VOLUME_SERIAL: u64 = 7;
const ROOT_FILE_ID: u128 = 999;
const DEFAULT_MTIME_NS: i128 = 1_700_000_000_000_000_000;

#[derive(Clone, Default)]
struct FakeClock(Rc<Cell<i64>>);

impl FakeClock {
    fn new() -> Self {
        Self(Rc::new(Cell::new(T0)))
    }
    fn set(&self, ms: i64) {
        self.0.set(ms);
    }
    fn advance(&self, ms: i64) {
        self.0.set(self.0.get().saturating_add(ms));
    }
}

impl ScanClock for FakeClock {
    fn now_ms(&self) -> i64 {
        self.0.get()
    }
}

/// Scripted filesystem metadata tree. Directories carry the qualified
/// identity the engine requires for its handle-bound validation chain; files
/// carry optional identity like the real NTFS port.
#[derive(Clone)]
struct Entry {
    name: String,
    kind: EntryKind,
    size: u64,
    mtime: i128,
    identity: Option<u128>,
    children: Vec<Entry>,
    deny_open: bool,
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
                qualification: enumeration::IdentityQualification::LocalNtfs,
            }),
            reparse_point: false,
            recall_or_offline: false,
        }
    }

    fn child(&self, name: &str) -> Entry {
        self.children
            .iter()
            .find(|child| child.name == name)
            .expect("scripted child exists")
            .clone()
    }
}

fn file_entry(name: &str, file_id: u128) -> Entry {
    file_entry_sized(name, file_id, 1_024)
}

fn file_entry_sized(name: &str, file_id: u128, size: u64) -> Entry {
    Entry {
        name: name.to_owned(),
        kind: EntryKind::File,
        size,
        mtime: DEFAULT_MTIME_NS,
        identity: Some(file_id),
        children: Vec::new(),
        deny_open: false,
    }
}

fn dir_entry(name: &str, file_id: u128, children: Vec<Entry>) -> Entry {
    Entry {
        name: name.to_owned(),
        kind: EntryKind::Directory,
        size: 0,
        mtime: DEFAULT_MTIME_NS,
        identity: Some(file_id),
        children,
        deny_open: false,
    }
}

fn denied_dir_entry(name: &str, file_id: u128, children: Vec<Entry>) -> Entry {
    Entry {
        deny_open: true,
        ..dir_entry(name, file_id, children)
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
        deny_open: false,
    }
}

fn many_files(count: usize) -> Vec<Entry> {
    (1..=count)
        .map(|index| file_entry(&format!("p{index:03}.flp"), 10_000 + index as u128))
        .collect()
}

type Hook = Rc<RefCell<dyn FnMut(&str)>>;

/// Deterministic `FilesystemPort` over the scripted tree. `root_error`
/// scripts offline/unsupported roots; `deny_open` scripts access denial in
/// the middle of traversal; the optional hook observes every metadata read so
/// tests can commit durable changes mid-traversal.
struct FakePort {
    root: Entry,
    root_error: Option<PortError>,
    hook: Option<Hook>,
}

impl FakePort {
    fn new(tree: Entry) -> Self {
        Self {
            root: tree,
            root_error: None,
            hook: None,
        }
    }

    fn offline(tree: Entry) -> Self {
        Self {
            root: tree,
            root_error: Some(PortError::NotFound),
            hook: None,
        }
    }

    fn with_hook(tree: Entry, hook: Hook) -> Self {
        Self {
            root: tree,
            root_error: None,
            hook: Some(hook),
        }
    }
}

impl FilesystemPort for FakePort {
    fn inspect_root(&mut self, _root: &Path) -> Result<RootMetadata, PortError> {
        if let Some(error) = self.root_error {
            return Err(error);
        }
        Ok(RootMetadata {
            metadata: self.root.metadata(),
            qualification: FilesystemQualification::LocalNtfs,
        })
    }

    fn open_root(&mut self, _root: &Path) -> Result<OpenedDirectory, PortError> {
        if let Some(error) = self.root_error {
            return Err(error);
        }
        Ok(OpenedDirectory {
            metadata: self.root.metadata(),
            case_sensitivity: DirectoryCaseSensitivity::Insensitive,
            cursor: Box::new(FakeCursor {
                directory: self.root.clone(),
                prefix: String::new(),
                index: 0,
                hook: self.hook.clone(),
            }),
        })
    }
}

struct FakeCursor {
    directory: Entry,
    prefix: String,
    index: usize,
    hook: Option<Hook>,
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
        Ok(self
            .directory
            .children
            .get(self.index)
            .map(|child| {
                self.index += 1;
                DirectoryEntry::new(child.name.clone())
            }))
    }

    fn read_metadata(&mut self, entry: &DirectoryEntry) -> Result<FileMetadata, PortError> {
        let name = entry.name.to_string_lossy().into_owned();
        let child = self.directory.child(&name);
        if let Some(hook) = self.hook.as_ref() {
            (hook.borrow_mut())(&self.display(&name));
        }
        Ok(child.metadata())
    }

    fn open_directory(&mut self, entry: &DirectoryEntry) -> Result<OpenedDirectory, PortError> {
        let name = entry.name.to_string_lossy().into_owned();
        let child = self.directory.child(&name);
        if child.deny_open {
            return Err(PortError::AccessDenied);
        }
        Ok(OpenedDirectory {
            metadata: child.metadata(),
            case_sensitivity: DirectoryCaseSensitivity::Insensitive,
            cursor: Box::new(FakeCursor {
                directory: child,
                prefix: self.display(&name),
                index: 0,
                hook: self.hook.clone(),
            }),
        })
    }
}

struct TestDir(PathBuf);

impl TestDir {
    fn new(label: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let tick = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let ordinal = NEXT.fetch_add(1, AtomicOrdering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "fruitboard-scan-execution-{label}-{}-{tick}-{ordinal}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("create disposable test directory");
        Self(directory)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CommittedRow {
    locator_key: String,
    relative_path: String,
    byte_size: u64,
    present: bool,
    project_file_id: String,
    identity: Option<(String, String)>,
}

fn committed_rows(db: &Database, root_id: &str) -> Vec<CommittedRow> {
    let mut rows = Vec::new();
    let mut cursor = None;
    let mut snapshot = None;
    loop {
        let page = db
            .query_library(&LibraryQuery {
                scan_root_id: root_id.to_owned(),
                page_size: MAX_LIBRARY_PAGE_SIZE,
                cursor: cursor.clone(),
                snapshot: snapshot.clone(),
            })
            .expect("library page");
        for location in &page.locations {
            rows.push(CommittedRow {
                locator_key: location.locator_key.clone(),
                relative_path: location.relative_path.clone(),
                byte_size: location.byte_size,
                present: location.presence == FilePresence::Present,
                project_file_id: location.project_file_id.clone(),
                identity: location
                    .identity
                    .as_ref()
                    .map(|identity| (identity.volume_serial.clone(), identity.file_id.clone())),
            });
        }
        if !page.has_more {
            break;
        }
        cursor = page.next_cursor;
        snapshot = Some(page.snapshot);
    }
    rows
}

struct Harness {
    directory: TestDir,
    db: Database,
    clock: FakeClock,
    worker: ScanWorker,
    session_id: String,
    root_id: String,
    root_path: String,
}

impl Harness {
    fn new(label: &str) -> Self {
        Self::with_config(label, WorkerConfig::default(), r"C:\synthetic-root")
    }

    fn with_config(label: &str, config: WorkerConfig, root_path: &str) -> Self {
        let directory = TestDir::new(label);
        let mut db = Database::open(&directory.0).expect("open durable database");
        let worker = ScanWorker::new(config).expect("valid worker configuration");
        let clock = FakeClock::new();
        let session_id = worker
            .start_session(&mut db, &clock)
            .expect("start session")
            .id;
        let root = db
            .add_scan_root("Synthetic", root_path)
            .expect("add synthetic root");
        Self {
            directory,
            db,
            clock,
            worker,
            session_id,
            root_id: root.id,
            root_path: root.canonical_path,
        }
    }

    fn db_path(&self) -> PathBuf {
        self.directory.0.join("storage").join("fruitboard.db")
    }

    fn scan(&mut self, tree: Entry) -> ScanExecution {
        let mut port = FakePort::new(tree);
        self.scan_with_port(&mut port)
    }

    fn scan_offline(&mut self, tree: Entry) -> ScanExecution {
        let mut port = FakePort::offline(tree);
        self.scan_with_port(&mut port)
    }

    fn scan_with_port<P: FilesystemPort>(&mut self, port: &mut P) -> ScanExecution {
        self.worker
            .request_manual_scan(&mut self.db, &self.root_id, &self.clock)
            .expect("enqueue manual scan");
        self.worker
            .poll(&mut self.db, &self.session_id, port, &NeverCancelled, &self.clock)
            .expect("worker poll")
            .expect("one execution")
    }

    fn drain(&mut self, tree: Entry) -> Option<ScanExecution> {
        let mut port = FakePort::new(tree);
        self.worker
            .poll(&mut self.db, &self.session_id, &mut port, &NeverCancelled, &self.clock)
            .expect("worker poll")
    }

    fn claim(&mut self) -> ActiveScan {
        self.worker
            .claim(&mut self.db, &self.session_id, &self.clock)
            .expect("claim")
            .expect("leased scan")
    }

    fn committed(&self) -> Vec<CommittedRow> {
        committed_rows(&self.db, &self.root_id)
    }

    fn root_jobs(&self) -> Vec<ScanJob> {
        self.db
            .list_scan_jobs()
            .expect("list jobs")
            .into_iter()
            .filter(|job| job.scan_root_id == self.root_id)
            .collect()
    }

    fn staging_state(&self, run_id: &str) -> ScanStageState {
        self.db.scan_staging(run_id).expect("staging").state
    }

    fn run(&self, run_id: &str) -> fruitboard_storage::ScanRun {
        self.db.scan_run(run_id).expect("run")
    }

    /// Begin a fresh process session; prior sessions are ended by storage, so
    /// the harness continues polling with the new session identity.
    fn restart(&mut self) -> fruitboard_storage::ScanSession {
        let session = self.worker.start_session(&mut self.db, &self.clock).expect("restart session");
        self.session_id = session.id.clone();
        session
    }
}

fn no_changes(summary: &ChangeSummary) {
    assert!(summary.computed);
    assert_eq!(
        (
            summary.added,
            summary.modified,
            summary.replaced,
            summary.identity_uncertain,
            summary.missing,
            summary.restored,
            summary.renames
        ),
        (0, 0, 0, 0, 0, 0, 0)
    );
}

#[test]
fn worker_config_rejects_invalid_lease_settings() {
    let mut config = WorkerConfig::default();
    config.lease_duration_ms = 0;
    assert_eq!(
        ScanWorker::new(config).err(),
        Some(WorkerConfigError::LeaseDuration)
    );
    let mut config = WorkerConfig::default();
    config.lease_renewal_interval_ms = config.lease_duration_ms;
    assert_eq!(
        ScanWorker::new(config).err(),
        Some(WorkerConfigError::RenewalInterval)
    );
}

// P2-02: an unchanged tree produces no spurious file changes and the second
// publication keeps every committed row, including physical continuity.
#[test]
fn unchanged_tree_republishes_without_changes() {
    let mut harness = Harness::new("idempotent");
    let tree = tree(vec![file_entry("a.flp", 101), file_entry("b.flp", 102)]);
    let first = harness.scan(tree.clone());
    assert_eq!(first.status, ScanExecutionStatus::Published);
    assert_eq!(first.publication.as_ref().expect("publication").location_count, 2);
    let before = harness.committed();

    let second = harness.scan(tree);
    assert_eq!(second.status, ScanExecutionStatus::Published);
    no_changes(second.changes.as_ref().expect("change summary"));
    assert_eq!(harness.committed(), before);
}

// P2-02: add/modify converge and are classified by the reconciliation core.
#[test]
fn add_and_modify_converge() {
    let mut harness = Harness::new("add-modify");
    let first = harness.scan(tree(vec![file_entry_sized("a.flp", 101, 10)]));
    assert_eq!(first.status, ScanExecutionStatus::Published);

    let second = harness.scan(tree(vec![
        file_entry_sized("a.flp", 101, 20),
        file_entry("b.flp", 102),
    ]));
    assert_eq!(second.status, ScanExecutionStatus::Published);
    let changes = second.changes.expect("change summary");
    assert!(changes.computed);
    assert_eq!(changes.added, 1);
    assert_eq!(changes.modified, 1);
    assert_eq!(changes.missing, 0);

    let rows = harness.committed();
    assert_eq!(rows.len(), 2);
    let modified = rows.iter().find(|row| row.relative_path == "a.flp").unwrap();
    assert!(modified.present);
    assert_eq!(modified.byte_size, 20);
    assert!(rows.iter().any(|row| row.relative_path == "b.flp" && row.present));
}

// P2-02: rename evidence, missing and restore converge per path.
#[test]
fn rename_missing_and_restore_converge() {
    let mut harness = Harness::new("rename-restore");
    harness.scan(tree(vec![file_entry("a.flp", 101), file_entry("c.flp", 303)]));

    // c is renamed to d (same qualified identity); b is added; a disappears.
    let second = harness.scan(tree(vec![
        file_entry("d.flp", 303),
        file_entry("b.flp", 202),
    ]));
    assert_eq!(second.status, ScanExecutionStatus::Published);
    let changes = second.changes.expect("change summary");
    // d is both the rename target of c and a newly observed path, so the
    // decision core reports Added(d)/Missing(c) plus the rename evidence.
    assert_eq!(changes.renames, 1);
    assert_eq!(changes.added, 2);
    assert_eq!(changes.missing, 2);

    let after_rename = harness.committed();
    assert_eq!(after_rename.len(), 4);
    let renamed_old = after_rename.iter().find(|row| row.relative_path == "c.flp").unwrap();
    assert!(!renamed_old.present);
    let renamed_new = after_rename.iter().find(|row| row.relative_path == "d.flp").unwrap();
    assert!(renamed_new.present);
    assert_eq!(renamed_old.project_file_id, renamed_new.project_file_id);

    // a comes back with its original identity: restored, not replaced. c
    // stays missing with its retained history.
    let third = harness.scan(tree(vec![
        file_entry("a.flp", 101),
        file_entry("d.flp", 303),
        file_entry("b.flp", 202),
    ]));
    assert_eq!(third.status, ScanExecutionStatus::Published);
    let changes = third.changes.expect("change summary");
    assert_eq!(changes.restored, 1);
    assert_eq!(changes.added, 0);
    assert_eq!(changes.missing, 0);
    let rows = harness.committed();
    assert!(rows.iter().filter(|row| row.relative_path != "c.flp").all(|row| row.present));
    assert!(!rows.iter().find(|row| row.relative_path == "c.flp").unwrap().present);
}

// P2-03: a denial in the middle of traversal is non-authoritative, discards
// staging and never marks the uncovered file missing.
#[test]
fn denied_subtree_discards_staging_and_preserves_committed_rows() {
    let mut harness = Harness::new("denied");
    let tree = tree(vec![
        file_entry("kept.flp", 101),
        dir_entry("locked", 201, vec![file_entry("hidden.flp", 301)]),
    ]);
    let first = harness.scan(tree.clone());
    assert_eq!(first.status, ScanExecutionStatus::Published);
    let before = harness.committed();
    assert_eq!(before.len(), 2);

    let mut denied_tree = tree.clone();
    denied_tree.children[1].deny_open = true;
    let second = harness.scan(denied_tree);
    assert_eq!(second.status, ScanExecutionStatus::Failed);
    assert_eq!(second.enumeration_outcome, Some(EnumOutcome::Denied));
    assert!(!second.authoritative);
    assert!(second.publication.is_none());
    assert_eq!(harness.committed(), before);

    let run = harness.run(&second.run_id);
    assert_eq!(run.state, ScanRunState::Failed);
    assert_eq!(harness.staging_state(&second.run_id), ScanStageState::Discarded);
    let job = harness.db.scan_job(&second.job_id).expect("job");
    assert_eq!(job.state, ScanJobState::Failed);
    assert_eq!(job.attempt, 1);
}

// P2-03: an offline root fails safely, waits out the persisted backoff with
// bounded jitter, and converges on a later automatic retry.
#[test]
fn offline_root_fails_then_persisted_retry_converges() {
    let mut harness = Harness::new("offline");
    let tree = tree(vec![file_entry("a.flp", 101)]);
    let first = harness.scan(tree.clone());
    assert_eq!(first.status, ScanExecutionStatus::Published);
    let before = harness.committed();

    let failed = harness.scan_offline(tree.clone());
    assert_eq!(failed.status, ScanExecutionStatus::Failed);
    assert_eq!(failed.enumeration_outcome, Some(EnumOutcome::RootUnavailable));
    assert!(failed.publication.is_none());
    assert_eq!(harness.committed(), before);

    let job = harness.db.scan_job(&failed.job_id).expect("job");
    assert_eq!(job.state, ScanJobState::Failed);
    assert_eq!(job.attempt, 1);

    // Backoff is storage's 1s base plus at most 20% deterministic jitter.
    let eligible = ScanWorker::retry_eligible_at(&job);
    assert!(eligible >= T0 + 1_000 && eligible <= T0 + 1_200);

    harness.clock.advance(999);
    assert_eq!(harness.worker.service_retries(&mut harness.db, &harness.clock).expect("retries"), 0);
    harness.clock.set(eligible);
    assert_eq!(harness.worker.service_retries(&mut harness.db, &harness.clock).expect("retries"), 1);
    let requeued = harness.db.scan_job(&failed.job_id).expect("job");
    assert_eq!(requeued.state, ScanJobState::Queued);
    assert_eq!(requeued.attempt, 1, "chain budget is preserved, not reset");

    let recovered = harness.drain(tree).expect("execution");
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    assert_eq!(harness.committed(), before);
    assert_eq!(harness.db.scan_job(&failed.job_id).expect("job").attempt, 2);
}

// P2-03/P2-04: a durable cancellation committed by another party between two
// batches is observed at the next fence; staging is discarded and nothing is
// published. The hook uses a raw fixture connection purely as an external
// trigger source; the worker itself only ever talks to typed storage.
#[test]
fn durable_cancellation_between_batches_stops_without_publishing() {
    let mut harness = Harness::new("cancel-between-batches");
    let trigger = Connection::open(harness.db_path()).expect("fixture trigger connection");
    let conn_cell: Rc<RefCell<Option<Connection>>> = Rc::new(RefCell::new(Some(trigger)));
    let run_cell: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let fired: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let hook: Hook = Rc::new(RefCell::new({
        let conn_cell = conn_cell.clone();
        let run_cell = run_cell.clone();
        let fired = fired.clone();
        move |path: &str| {
            if path == "p520.flp" && !fired.replace(true) {
                let run_id = run_cell.borrow().clone().expect("run id captured");
                let conn = conn_cell.borrow();
                let conn = conn.as_ref().expect("trigger connection");
                conn.execute(
                    "UPDATE scan_run SET cancellation_requested = 1 WHERE id = ?1",
                    params![run_id],
                )
                .expect("cancel run");
                conn.execute(
                    "UPDATE scan_job SET cancellation_requested = 1
                     WHERE id = (SELECT scan_job_id FROM scan_run WHERE id = ?1)",
                    params![run_id],
                )
                .expect("cancel job");
            }
        }
    }));

    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    *run_cell.borrow_mut() = Some(scan.leased.run.id.clone());
    let mut port = FakePort::with_hook(tree(many_files(600)), hook);
    let execution = harness.worker.execute(
        &mut harness.db,
        scan,
        &mut port,
        &NeverCancelled,
        &harness.clock,
    );

    assert_eq!(execution.status, ScanExecutionStatus::Cancelled);
    assert!(!execution.authoritative);
    assert!(execution.publication.is_none());
    assert!(harness.committed().is_empty());
    assert_eq!(harness.staging_state(&execution.run_id), ScanStageState::Discarded);
    let run = harness.run(&execution.run_id);
    assert_eq!(run.state, ScanRunState::Cancelled);
    let job = harness.db.scan_job(&execution.job_id).expect("job");
    assert_eq!(job.state, ScanJobState::Cancelled);
    assert_eq!(harness.root_jobs().len(), 1, "cancellation is never requeued");
}

// P2-04: a cancellation committed before the final apply prevents apply; the
// worker observes it at its first fence through the durable flag only.
#[test]
fn durable_cancellation_committed_before_apply_prevents_publish() {
    let mut harness = Harness::new("cancel-before-apply");
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    let state = harness
        .db
        .request_scan_cancellation(&scan.leased.run.id, harness.clock.now_ms())
        .expect("durable cancellation");
    assert_eq!(state, ScanRunState::Running);
    let mut port = FakePort::new(tree(vec![file_entry("a.flp", 101)]));
    let execution = harness.worker.execute(
        &mut harness.db,
        scan,
        &mut port,
        &NeverCancelled,
        &harness.clock,
    );
    assert_eq!(execution.status, ScanExecutionStatus::Cancelled);
    assert!(execution.publication.is_none());
    assert!(harness.committed().is_empty());
    assert_eq!(harness.run(&execution.run_id).state, ScanRunState::Cancelled);
    assert_eq!(harness.staging_state(&execution.run_id), ScanStageState::Discarded);
}

// P2-03: the local cooperative cancellation token ends the run cancelled
// with no publication and no retried work.
#[test]
fn local_cancellation_token_ends_the_run_cancelled() {
    let mut harness = Harness::new("local-cancel");
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    let token = CancellationToken::new();
    token.cancel();
    let mut port = FakePort::new(tree(vec![file_entry("a.flp", 101)]));
    let execution = harness.worker.execute(&mut harness.db, scan, &mut port, &token, &harness.clock);
    assert_eq!(execution.status, ScanExecutionStatus::Cancelled);
    assert_eq!(execution.enumeration_outcome, Some(EnumOutcome::Cancelled));
    assert!(execution.publication.is_none());
    assert!(harness.committed().is_empty());
    assert_eq!(harness.staging_state(&execution.run_id), ScanStageState::Discarded);
    assert_eq!(harness.root_jobs().len(), 1);
}

// P2-04: a trigger received during staging invalidates the attempt, discards
// its staging and schedules exactly one deduplicated follow-up.
#[test]
fn follow_up_during_staging_invalidates_and_schedules_one_follow_up() {
    let mut harness = Harness::new("follow-up");
    let trigger = Connection::open(harness.db_path()).expect("fixture trigger connection");
    let conn_cell: Rc<RefCell<Option<Connection>>> = Rc::new(RefCell::new(Some(trigger)));
    let job_cell: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let fired: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let hook: Hook = Rc::new(RefCell::new({
        let conn_cell = conn_cell.clone();
        let job_cell = job_cell.clone();
        let fired = fired.clone();
        move |path: &str| {
            if path == "p520.flp" && !fired.replace(true) {
                let job_id = job_cell.borrow().clone().expect("job id captured");
                let conn = conn_cell.borrow();
                let conn = conn.as_ref().expect("trigger connection");
                conn.execute(
                    "UPDATE scan_job SET follow_up_requested = 1 WHERE id = ?1",
                    params![job_id],
                )
                .expect("request follow-up");
            }
        }
    }));

    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    *job_cell.borrow_mut() = Some(scan.leased.run.scan_job_id.clone());
    let mut port = FakePort::with_hook(tree(many_files(600)), hook);
    let execution = harness.worker.execute(
        &mut harness.db,
        scan,
        &mut port,
        &NeverCancelled,
        &harness.clock,
    );

    assert_eq!(execution.status, ScanExecutionStatus::Interrupted);
    assert!(execution.publication.is_none());
    assert!(harness.committed().is_empty());
    assert_eq!(harness.staging_state(&execution.run_id), ScanStageState::Discarded);
    let run = harness.run(&execution.run_id);
    assert_eq!(run.state, ScanRunState::Interrupted);
    assert_eq!(run.error_code.as_deref(), Some("follow_up_requested"));

    let jobs = harness.root_jobs();
    assert_eq!(jobs.len(), 2, "exactly one follow-up was scheduled");
    let queued = jobs.iter().filter(|job| job.state == ScanJobState::Queued).count();
    assert_eq!(queued, 1);

    // The deduplicated follow-up is the first successful publication of
    // this tree: every row is added by it.
    let follow_up = harness.drain(tree(many_files(600))).expect("execution");
    assert_eq!(follow_up.status, ScanExecutionStatus::Published);
    assert_eq!(follow_up.publication.as_ref().expect("publication").location_count, 600);
    let changes = follow_up.changes.expect("change summary");
    assert_eq!(changes.added, 600);
}

// P2-05: disabling the root while its scan is running invalidates the run in
// one configuration transaction; the worker observes the invalidation at its
// next fence and never publishes. Re-enabling runs a fresh generation.
#[test]
fn disable_while_running_stops_and_reenable_publishes_fresh_run() {
    let mut harness = Harness::new("disable-running");
    let tree = tree(vec![file_entry("a.flp", 101)]);
    let first = harness.scan(tree.clone());
    assert_eq!(first.status, ScanExecutionStatus::Published);
    let before = harness.committed();

    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    harness
        .db
        .set_scan_root_enabled_at(&harness.root_id, false, harness.clock.now_ms())
        .expect("disable root");
    let mut port = FakePort::new(tree.clone());
    let execution = harness.worker.execute(
        &mut harness.db,
        scan,
        &mut port,
        &NeverCancelled,
        &harness.clock,
    );
    assert_eq!(execution.status, ScanExecutionStatus::Cancelled);
    assert!(execution.publication.is_none());
    assert_eq!(harness.committed(), before);
    assert_eq!(harness.run(&execution.run_id).state, ScanRunState::Cancelled);
    assert_eq!(harness.staging_state(&execution.run_id), ScanStageState::Discarded);

    harness
        .db
        .set_scan_root_enabled_at(&harness.root_id, true, harness.clock.now_ms())
        .expect("re-enable root");
    let fresh = harness.scan(tree);
    assert_eq!(fresh.status, ScanExecutionStatus::Published);
    assert_eq!(harness.committed(), before);
}

// P2-05: removing the root while queued/running prevents publication; the
// run is cancelled and the root row disappears.
#[test]
fn removal_while_running_prevents_publication() {
    let mut harness = Harness::new("remove-running");
    let tree = tree(vec![file_entry("a.flp", 101)]);
    let first = harness.scan(tree.clone());
    assert_eq!(first.status, ScanExecutionStatus::Published);

    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    harness
        .db
        .remove_scan_root_at(&harness.root_id, harness.clock.now_ms())
        .expect("remove root");
    let mut port = FakePort::new(tree);
    let execution = harness.worker.execute(
        &mut harness.db,
        scan,
        &mut port,
        &NeverCancelled,
        &harness.clock,
    );
    assert_eq!(execution.status, ScanExecutionStatus::Cancelled);
    assert!(execution.publication.is_none());
    assert!(harness.db.list_scan_roots().expect("roots").is_empty());
    let run = harness.db.scan_run(&execution.run_id).expect("run");
    assert_eq!(run.state, ScanRunState::Cancelled);
    // The removal transaction detaches history and cascades the stage row
    // away with the root; nothing publishable remains for this run.
    assert!(harness.db.scan_staging(&execution.run_id).is_err());
}

// P2-05: disable/remove invalidates queued work too; nothing stale is leased.
#[test]
fn queued_work_is_invalidated_by_disable_and_never_leased() {
    let mut harness = Harness::new("disable-queued");
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    harness
        .db
        .set_scan_root_enabled_at(&harness.root_id, false, harness.clock.now_ms())
        .expect("disable root");
    assert!(harness.worker.claim(&mut harness.db, &harness.session_id, &harness.clock).expect("claim").is_none());
}

// P2-04: injected SQL failure inside the publication transaction rolls back
// the whole apply, discards staging, and the persisted retry chain converges.
#[test]
fn publication_rollback_on_injected_sql_failure_converges_on_retry() {
    let mut harness = Harness::new("rollback");
    let trigger = Connection::open(harness.db_path()).expect("fixture connection");
    trigger
        .execute_batch(
            "CREATE TRIGGER fruitboard_test_first_success_abort
             BEFORE UPDATE OF last_successful_at_ms ON scan_root
             WHEN NEW.last_successful_at_ms IS NOT NULL
               AND OLD.last_successful_at_ms IS NULL
             BEGIN
                 SELECT RAISE(ABORT, 'injected_first_success_failure');
             END;",
        )
        .expect("arm injected SQL failure");

    let tree = tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
        file_entry("c.flp", 103),
    ]);
    let execution = harness.scan(tree.clone());
    assert_eq!(execution.status, ScanExecutionStatus::Failed);
    assert!(execution.publication.is_none());
    assert!(harness.committed().is_empty());
    assert_eq!(harness.staging_state(&execution.run_id), ScanStageState::Discarded);
    let job = harness.db.scan_job(&execution.job_id).expect("job");
    assert_eq!(job.state, ScanJobState::Failed);
    assert_eq!(job.attempt, 1);

    trigger
        .execute_batch("DROP TRIGGER fruitboard_test_first_success_abort;")
        .expect("disarm injected failure");
    let eligible = ScanWorker::retry_eligible_at(&job);
    harness.clock.set(eligible);
    assert_eq!(harness.worker.service_retries(&mut harness.db, &harness.clock).expect("retries"), 1);
    let recovered = harness.drain(tree).expect("execution");
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    assert_eq!(recovered.publication.as_ref().expect("publication").location_count, 3);
}

// P2-04: a worker whose lease expired cannot commit, even with a healthy
// port. The reaper requeues the same chain with its budget intact and a
// later attempt converges.
#[test]
fn stale_worker_cannot_commit_after_lease_expiry() {
    let mut harness = Harness::new("lease-expiry");
    let tree = tree(vec![file_entry("a.flp", 101)]);
    let first = harness.scan(tree.clone());
    assert_eq!(first.status, ScanExecutionStatus::Published);
    let before = harness.committed();

    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    harness.clock.advance(31_000);
    let mut port = FakePort::new(tree.clone());
    let execution = harness.worker.execute(
        &mut harness.db,
        scan,
        &mut port,
        &NeverCancelled,
        &harness.clock,
    );
    assert_eq!(execution.status, ScanExecutionStatus::Fenced);
    assert_eq!(execution.error_code.as_deref(), Some("storage_conflict"));
    assert!(execution.publication.is_none());
    assert_eq!(harness.committed(), before);

    // The reaper converts the expired run to interrupted and requeues the
    // same chain; after the backoff a fresh attempt completes the work.
    assert!(harness.drain(tree.clone()).is_none(), "not yet due");
    harness.clock.advance(1_001);
    let recovered = harness.drain(tree).expect("execution");
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    assert_eq!(harness.committed(), before);
    assert_eq!(harness.db.scan_job(&execution.job_id).expect("job").attempt, 2);
    assert_eq!(harness.run(&execution.run_id).state, ScanRunState::Interrupted);
    assert_eq!(harness.run(&first.run_id).state, ScanRunState::Completed);
}

// P2-04: after the lease is replaced by a reaped-and-released attempt, the
// stale worker cannot commit and the replacement's publication stands.
#[test]
fn stale_worker_cannot_commit_after_lease_replacement() {
    let mut harness = Harness::new("lease-replacement");
    let tree = tree(vec![file_entry("a.flp", 101)]);
    harness.scan(tree.clone());

    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let stale = harness.claim();
    let stale_lease_token = stale.leased.run.lease_token.clone();
    harness.clock.advance(31_000);

    // The reaper requeues the expired attempt with its backoff; the
    // replacement is not immediately due.
    assert!(harness.drain(tree.clone()).is_none());
    harness.clock.advance(1_001);
    let replacement = harness.drain(tree.clone()).expect("replacement execution");
    assert_eq!(replacement.status, ScanExecutionStatus::Published);
    let committed_after_replacement = harness.committed();

    let mut port = FakePort::new(tree);
    let execution = harness.worker.execute(
        &mut harness.db,
        stale,
        &mut port,
        &NeverCancelled,
        &harness.clock,
    );
    assert!(!execution.authoritative);
    assert!(execution.publication.is_none());
    assert_eq!(harness.committed(), committed_after_replacement);
    assert_ne!(stale_lease_token, harness.run(&replacement.run_id).lease_token);
}

// P2-04/P2-06: startup recovery fences prior-session work (interrupted run,
// discarded staging, requeued chain) without enqueueing a duplicate recovery
// scan while queued work exists.
#[test]
fn restart_requeues_interrupted_work_without_resetting_the_chain() {
    let mut harness = Harness::new("restart-recovery");
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let crashed = harness.claim();
    let crashed_run = crashed.leased.run.id.clone();
    drop(crashed); // simulate a crash mid-run: staging stays open, lease held

    let restarted = harness.restart();
    assert!(!restarted.id.is_empty());

    let run = harness.run(&crashed_run);
    assert_eq!(run.state, ScanRunState::Interrupted);
    assert_eq!(harness.staging_state(&crashed_run), ScanStageState::Discarded);
    let jobs = harness.root_jobs();
    assert_eq!(jobs.len(), 1, "no duplicate recovery job next to queued work");
    assert_eq!(jobs[0].state, ScanJobState::Queued);
    assert_eq!(jobs[0].attempt, 1);

    harness.clock.advance(1_001);
    let recovered = harness.drain(tree(vec![file_entry("a.flp", 101)])).expect("execution");
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    assert_eq!(harness.db.scan_job(&jobs[0].id).expect("job").attempt, 2);
}

// P2-04: enabled roots without eligible work receive exactly one deduplicated
// recovery scan per restart; cancelled chains are never implicitly revived.
#[test]
fn recovery_scan_is_deduplicated_and_cancelled_work_is_not_revived() {
    let mut harness = Harness::new("recovery-scan");
    let first = harness.scan(tree(vec![file_entry("a.flp", 101)]));
    assert_eq!(first.status, ScanExecutionStatus::Published);

    harness.restart();
    let jobs = harness.root_jobs();
    assert_eq!(jobs.len(), 2, "one recovery job after restart");
    assert!(jobs.iter().any(|job| job.kind == ScanKind::Recovery && job.state == ScanJobState::Queued));

    // A second restart while that recovery job is queued must not duplicate.
    harness.restart();
    assert_eq!(harness.root_jobs().len(), 2);

    let recovered = harness.drain(tree(vec![file_entry("a.flp", 101)])).expect("execution");
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    no_changes(recovered.changes.as_ref().expect("change summary"));

    // A user-cancelled chain is never revived by recovery.
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let job = harness.root_jobs().into_iter().find(|job| job.state == ScanJobState::Queued).unwrap();
    let state = harness.db.cancel_scan_job(&job.id, harness.clock.now_ms()).expect("cancel");
    assert_eq!(state, ScanJobState::Cancelled);
    harness.restart();
    assert_eq!(harness.root_jobs().len(), 3);
    assert!(harness.worker.claim(&mut harness.db, &harness.session_id, &harness.clock).expect("claim").is_none());
}

// P2-04: repeated triggers coalesce onto one active slot; a trigger during
// traversal invalidates that attempt and schedules exactly one follow-up.
#[test]
fn repeated_triggers_coalesce_and_follow_up_reconciles() {
    let mut harness = Harness::new("coalesce");
    let tree = tree(vec![file_entry("a.flp", 101), file_entry("b.flp", 102)]);
    harness.worker.request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock).expect("trigger");
    harness.worker.request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock).expect("trigger");
    harness.worker.request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock).expect("trigger");
    assert_eq!(harness.root_jobs().len(), 1, "queued triggers coalesce");

    let scan = harness.claim();
    assert!(harness.worker.claim(&mut harness.db, &harness.session_id, &harness.clock).expect("claim").is_none(),
        "one worker runs one scan");
    harness.worker.request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock).expect("trigger");
    let mut port = FakePort::new(tree.clone());
    let execution = harness.worker.execute(
        &mut harness.db,
        scan,
        &mut port,
        &NeverCancelled,
        &harness.clock,
    );
    assert_eq!(execution.status, ScanExecutionStatus::Interrupted);
    assert!(execution.publication.is_none());
    assert_eq!(harness.root_jobs().len(), 2, "one queued follow-up");

    // The follow-up is the first successful publication of this tree.
    let follow_up = harness.drain(tree).expect("execution");
    assert_eq!(follow_up.status, ScanExecutionStatus::Published);
    let changes = follow_up.changes.expect("change summary");
    assert_eq!(changes.added, 2);
}

// P2-03: quota-limited traversal (fewer observations than the staging budget)
// is non-authoritative and preserves committed rows.
#[test]
fn resource_limited_traversal_discards_and_preserves_committed_rows() {
    let mut config = WorkerConfig::default();
    config.enumeration_limits.max_observations = 2;
    let mut harness = Harness::with_config("resource-limit", config, r"C:\synthetic-root");
    let first = harness.scan(tree(many_files(2)));
    assert_eq!(first.status, ScanExecutionStatus::Published);
    assert_eq!(first.publication.as_ref().expect("publication").location_count, 2);
    let before = harness.committed();

    // Four observations exceed the configured limit of two.
    let second_tree = tree(many_files(4));
    let second = harness.scan(second_tree);
    assert_eq!(second.status, ScanExecutionStatus::Failed);
    assert_eq!(second.enumeration_outcome, Some(EnumOutcome::ResourceLimit));
    assert!(second.publication.is_none());
    assert_eq!(harness.committed(), before);
    assert_eq!(harness.staging_state(&second.run_id), ScanStageState::Discarded);
}

// P2-02/P2-03: an empty authoritative enumeration publishes an empty result
// and marks previously present rows missing; observing them again restores
// each location individually.
#[test]
fn empty_root_marks_previous_rows_missing_and_restores_them() {
    let mut harness = Harness::new("empty-root");
    let first = harness.scan(tree(vec![file_entry("a.flp", 101)]));
    assert_eq!(first.status, ScanExecutionStatus::Published);

    let second = harness.scan(tree(vec![]));
    assert_eq!(second.status, ScanExecutionStatus::Published);
    let changes = second.changes.expect("change summary");
    assert_eq!(changes.missing, 1);
    assert_eq!(second.publication.as_ref().expect("publication").location_count, 1);
    let rows = harness.committed();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].present, "the vanished path is missing, not deleted");

    let third = harness.scan(tree(vec![file_entry("a.flp", 101)]));
    assert_eq!(third.status, ScanExecutionStatus::Published);
    let changes = third.changes.expect("change summary");
    assert_eq!(changes.restored, 1);
    assert!(harness.committed()[0].present);
}

// P2-04: completion commits first and wins; a late cancellation reports the
// committed outcome without claiming rollback.
#[test]
fn late_cancellation_after_commit_reports_completed_without_rollback() {
    let mut harness = Harness::new("commit-then-cancel");
    let execution = harness.scan(tree(vec![file_entry("a.flp", 101)]));
    assert_eq!(execution.status, ScanExecutionStatus::Published);
    let before = harness.committed();

    let state = harness
        .db
        .request_scan_cancellation(&execution.run_id, harness.clock.now_ms())
        .expect("late cancellation");
    assert_eq!(state, ScanRunState::Completed);
    assert_eq!(harness.committed(), before);
}

// The bounded scale/perf budgets (10k records, 512-record batches, 256 MiB
// staging, <=128 MiB working memory) are measured separately on the
// synthetic-tree benchmark fixture from #41. Correctness at the batch
// boundary is covered above; see docs/PHASE_2_EXECUTION_PLAN.md budgets.
#[test]
#[ignore = "scale/perf budgets require the #41 benchmark harness and a reference machine; not a correctness gate"]
fn large_tree_budget_measurement_requires_the_benchmark_harness() {}

#[cfg(windows)]
mod ntfs {
    use super::*;
    use fruitboard_filesystem_enumeration::WindowsFilesystemPort;
    use std::process::Command;

    struct NtfsFixture {
        root: PathBuf,
        marker: Vec<u8>,
    }

    impl NtfsFixture {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let tick = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();
            let ordinal = NEXT.fetch_add(1, AtomicOrdering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "fruitboard-scan-execution-ntfs-{label}-{}-{tick}-{ordinal}",
                std::process::id()
            ));
            std::fs::create_dir_all(&root).expect("create disposable NTFS fixture root");
            let marker = vec![0, 1, 2, 0xF1, 0x00, 0xFF, 0x7E, 0x42];
            Self { root, marker }
        }

        fn write(&self, relative: &str) {
            let path = self.root.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create fixture directory");
            }
            std::fs::write(&path, &self.marker).expect("write disposable marker file");
        }

        fn deny(&self, relative: &str) -> AclDenyGuard {
            let path = self.root.join(relative);
            let output = Command::new("icacls")
                .arg(&path)
                .arg("/inheritance:r")
                .output()
                .expect("icacls must exist on Windows");
            assert!(
                output.status.success(),
                "failed to strip inherited ACLs for the disposable fixture: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            AclDenyGuard(path)
        }
    }

    impl Drop for NtfsFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    /// Restores inherited ACEs on drop so fixture cleanup never fails behind
    /// the test's own deny entry.
    struct AclDenyGuard(PathBuf);

    impl Drop for AclDenyGuard {
        fn drop(&mut self) {
            let _ = Command::new("icacls").arg(&self.0).arg("/reset").output();
        }
    }

    // Real-NTFS authoritative success: the worker publishes the discovered
    // rows, preserves source bytes, and an unchanged rescan makes no changes.
    #[test]
    fn ntfs_worker_publishes_authoritative_rows_and_preserves_sources() {
        let fixture = NtfsFixture::new("authoritative");
        fixture.write("alpha.FLP");
        fixture.write("nested/beta.flp");
        fixture.write("notes.txt");
        let marker_before = std::fs::read(fixture.root.join("alpha.FLP")).expect("marker");

        let mut harness =
            Harness::with_config("ntfs-authoritative", WorkerConfig::default(), &fixture.root.to_string_lossy());
        let mut port = WindowsFilesystemPort::new();
        let first = harness.scan_with_port(&mut port);
        assert_eq!(first.status, ScanExecutionStatus::Published);
        assert_eq!(first.enumeration_outcome, Some(EnumOutcome::Complete));
        let publication = first.publication.as_ref().expect("publication");
        assert_eq!(publication.location_count, 2);

        let rows = harness.committed();
        assert_eq!(rows.len(), 2);
        let alpha = rows.iter().find(|row| row.relative_path == "alpha.FLP").expect("alpha row");
        assert!(alpha.present);
        assert_eq!(alpha.byte_size, marker_before.len() as u64);
        assert!(rows.iter().any(|row| row.relative_path == "nested\\beta.flp"));
        assert!(!rows.iter().any(|row| row.relative_path.contains("notes.txt")));
        let marker_after = std::fs::read(fixture.root.join("alpha.FLP")).expect("marker");
        assert_eq!(marker_before, marker_after, "source bytes are never read or written");

        let second = harness.scan_with_port(&mut port);
        assert_eq!(second.status, ScanExecutionStatus::Published);
        no_changes(second.changes.as_ref().expect("change summary"));
        assert_eq!(harness.committed(), rows);
    }

    // Real-NTFS denied subtree: the denied directory makes the run
    // non-authoritative, staging is discarded, and the uncovered file is
    // never marked missing (P2-03). Restoring access converges again.
    #[test]
    fn ntfs_worker_denied_subtree_preserves_committed_rows() {
        let fixture = NtfsFixture::new("denied");
        fixture.write("kept.flp");
        fixture.write("denied/hidden.flp");

        let mut harness =
            Harness::with_config("ntfs-denied", WorkerConfig::default(), &fixture.root.to_string_lossy());
        let mut port = WindowsFilesystemPort::new();
        let first = harness.scan_with_port(&mut port);
        assert_eq!(first.status, ScanExecutionStatus::Published);
        let before = harness.committed();
        assert_eq!(before.len(), 2);

        let _guard = fixture.deny("denied");
        let second = harness.scan_with_port(&mut port);
        assert_eq!(second.status, ScanExecutionStatus::Failed);
        assert_eq!(second.enumeration_outcome, Some(EnumOutcome::Denied));
        assert!(second.publication.is_none());
        assert_eq!(harness.committed(), before, "denied subtree is never marked missing");

        drop(_guard);
        let third = harness.scan_with_port(&mut port);
        assert_eq!(third.status, ScanExecutionStatus::Published);
        assert!(harness.committed().iter().all(|row| row.present));
    }
}
