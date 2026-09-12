use super::*;
use enumeration::{
    CancellationToken, DirectoryCaseSensitivity, DirectoryCursor, DirectoryEntry, EntryKind,
    FileMetadata, FilesystemPort, FilesystemQualification, NeverCancelled, OpenedDirectory,
    Outcome as EnumOutcome, PortError, QualifiedIdentity, RootMetadata,
};
use fruitboard_filesystem_watcher::{
    Coalescer, CoalescerConfig, HintKind, RootId, WatchHint, WatchOutcome, WatcherPort,
};
use fruitboard_storage::{
    MAX_LIBRARY_PAGE_SIZE, ScanJobState, ScanKind, ScanRunState, ScanStageState,
};
use rusqlite::{Connection, params};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
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
    open_error: Option<PortError>,
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
        open_error: None,
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
        open_error: None,
    }
}

fn vanishing_dir_entry(name: &str, file_id: u128, children: Vec<Entry>) -> Entry {
    Entry {
        name: name.to_owned(),
        kind: EntryKind::Directory,
        size: 0,
        mtime: DEFAULT_MTIME_NS,
        identity: Some(file_id),
        children,
        deny_open: false,
        // Simulates a directory that disappears mid-traversal: the enumerator
        // records a Partial (non-authoritative) outcome, never a publication.
        open_error: Some(PortError::NotFound),
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
        open_error: None,
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
/// the middle of traversal; `open_error` on an entry scripts a Partial
/// (disappeared/changed) directory; the optional hook observes every metadata
/// read so tests can commit durable changes mid-traversal.
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

    fn with_root_error(tree: Entry, error: PortError) -> Self {
        Self {
            root: tree,
            root_error: Some(error),
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
        Ok(self.directory.children.get(self.index).map(|child| {
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
        if let Some(error) = child.open_error {
            return Err(error);
        }
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
    modified_at_ns: i64,
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
                modified_at_ns: location.modified_at_ns,
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

/// Committed rows plus the root success marker, serialized deterministically
/// for byte-identical before/after comparison. Rows are already in stable
/// `(locator_key, id)` order via `query_library`; the marker is the Library
/// snapshot that pagination binds to.
fn committed_snapshot_bytes(
    rows: &[CommittedRow],
    marker: &fruitboard_storage::ScanRootPublication,
) -> Vec<u8> {
    format!("{rows:?}|{marker:?}").into_bytes()
}

fn committed_state(
    harness: &Harness,
) -> (
    Vec<CommittedRow>,
    fruitboard_storage::ScanRootPublication,
    Vec<u8>,
) {
    let rows = harness.committed();
    let marker = harness
        .db
        .scan_root_publication(&harness.root_id)
        .expect("publication marker");
    let bytes = committed_snapshot_bytes(&rows, &marker);
    (rows, marker, bytes)
}

fn assert_committed_unchanged(
    before_rows: &[CommittedRow],
    before_marker: &fruitboard_storage::ScanRootPublication,
    before_bytes: &[u8],
    harness: &Harness,
) {
    let (after_rows, after_marker, after_bytes) = committed_state(harness);
    assert_eq!(
        after_rows, before_rows,
        "non-authoritative run must leave committed rows identical"
    );
    assert_eq!(
        after_marker, *before_marker,
        "non-authoritative run must leave the success marker identical"
    );
    assert_eq!(
        after_bytes, before_bytes,
        "rows+snapshot must be byte-identical before/after"
    );
}

struct Harness {
    directory: TestDir,
    db: Database,
    clock: FakeClock,
    worker: ScanWorker,
    session_id: String,
    root_id: String,
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
            .poll(
                &mut self.db,
                &self.session_id,
                port,
                &NeverCancelled,
                &self.clock,
            )
            .expect("worker poll")
            .expect("one execution")
    }

    fn drain(&mut self, tree: Entry) -> Option<ScanExecution> {
        let mut port = FakePort::new(tree);
        self.worker
            .poll(
                &mut self.db,
                &self.session_id,
                &mut port,
                &NeverCancelled,
                &self.clock,
            )
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
        let session = self
            .worker
            .start_session(&mut self.db, &self.clock)
            .expect("restart session");
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
    assert_eq!(
        ScanWorker::new(WorkerConfig {
            lease_duration_ms: 0,
            ..WorkerConfig::default()
        })
        .err(),
        Some(WorkerConfigError::LeaseDuration)
    );
    let config = WorkerConfig::default();
    assert_eq!(
        ScanWorker::new(WorkerConfig {
            lease_renewal_interval_ms: config.lease_duration_ms,
            ..config
        })
        .err(),
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
    assert_eq!(
        first
            .publication
            .as_ref()
            .expect("publication")
            .location_count,
        2
    );
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
    let modified = rows
        .iter()
        .find(|row| row.relative_path == "a.flp")
        .unwrap();
    assert!(modified.present);
    assert_eq!(modified.byte_size, 20);
    assert!(
        rows.iter()
            .any(|row| row.relative_path == "b.flp" && row.present)
    );
}

// P2-02: rename evidence, missing and restore converge per path.
#[test]
fn rename_missing_and_restore_converge() {
    let mut harness = Harness::new("rename-restore");
    harness.scan(tree(vec![
        file_entry("a.flp", 101),
        file_entry("c.flp", 303),
    ]));

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
    let renamed_old = after_rename
        .iter()
        .find(|row| row.relative_path == "c.flp")
        .unwrap();
    assert!(!renamed_old.present);
    let renamed_new = after_rename
        .iter()
        .find(|row| row.relative_path == "d.flp")
        .unwrap();
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
    assert!(
        rows.iter()
            .filter(|row| row.relative_path != "c.flp")
            .all(|row| row.present)
    );
    assert!(
        !rows
            .iter()
            .find(|row| row.relative_path == "c.flp")
            .unwrap()
            .present
    );
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
    assert_eq!(run.error_code.as_deref(), Some("access_denied"));
    assert_eq!(
        harness.staging_state(&second.run_id),
        ScanStageState::Discarded
    );
    let job = harness.db.scan_job(&second.job_id).expect("job");
    assert_eq!(job.state, ScanJobState::Failed);
    assert_eq!(job.attempt, 1);
    assert_eq!(job.last_error_code.as_deref(), Some("access_denied"));
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
    assert_eq!(
        failed.enumeration_outcome,
        Some(EnumOutcome::RootUnavailable)
    );
    assert!(failed.publication.is_none());
    assert_eq!(harness.committed(), before);

    let job = harness.db.scan_job(&failed.job_id).expect("job");
    assert_eq!(job.state, ScanJobState::Failed);
    assert_eq!(job.attempt, 1);
    assert_eq!(
        harness.run(&failed.run_id).error_code.as_deref(),
        Some("unavailable")
    );
    assert_eq!(job.last_error_code.as_deref(), Some("unavailable"));

    // Backoff is storage's 1s base plus at most 20% deterministic jitter.
    let eligible = ScanWorker::retry_eligible_at(&job);
    assert!((T0 + 1_000..=T0 + 1_200).contains(&eligible));

    harness.clock.advance(999);
    assert_eq!(
        harness
            .worker
            .service_retries(&mut harness.db, &harness.clock)
            .expect("retries"),
        0
    );
    harness.clock.set(eligible);
    assert_eq!(
        harness
            .worker
            .service_retries(&mut harness.db, &harness.clock)
            .expect("retries"),
        1
    );
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
    assert_eq!(
        harness.staging_state(&execution.run_id),
        ScanStageState::Discarded
    );
    let run = harness.run(&execution.run_id);
    assert_eq!(run.state, ScanRunState::Cancelled);
    let job = harness.db.scan_job(&execution.job_id).expect("job");
    assert_eq!(job.state, ScanJobState::Cancelled);
    assert_eq!(
        harness.root_jobs().len(),
        1,
        "cancellation is never requeued"
    );
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
    assert_eq!(
        harness.run(&execution.run_id).state,
        ScanRunState::Cancelled
    );
    assert_eq!(
        harness.staging_state(&execution.run_id),
        ScanStageState::Discarded
    );
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
    let execution =
        harness
            .worker
            .execute(&mut harness.db, scan, &mut port, &token, &harness.clock);
    assert_eq!(execution.status, ScanExecutionStatus::Cancelled);
    assert_eq!(execution.enumeration_outcome, Some(EnumOutcome::Cancelled));
    assert!(execution.publication.is_none());
    assert!(harness.committed().is_empty());
    assert_eq!(
        harness.staging_state(&execution.run_id),
        ScanStageState::Discarded
    );
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
    assert_eq!(
        harness.staging_state(&execution.run_id),
        ScanStageState::Discarded
    );
    let run = harness.run(&execution.run_id);
    assert_eq!(run.state, ScanRunState::Interrupted);
    assert_eq!(run.error_code.as_deref(), Some("follow_up_requested"));

    let jobs = harness.root_jobs();
    assert_eq!(jobs.len(), 2, "exactly one follow-up was scheduled");
    let queued = jobs
        .iter()
        .filter(|job| job.state == ScanJobState::Queued)
        .count();
    assert_eq!(queued, 1);

    // The deduplicated follow-up is the first successful publication of
    // this tree: every row is added by it.
    let follow_up = harness.drain(tree(many_files(600))).expect("execution");
    assert_eq!(follow_up.status, ScanExecutionStatus::Published);
    assert_eq!(
        follow_up
            .publication
            .as_ref()
            .expect("publication")
            .location_count,
        600
    );
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
    assert_eq!(
        harness.run(&execution.run_id).state,
        ScanRunState::Cancelled
    );
    assert_eq!(
        harness.staging_state(&execution.run_id),
        ScanStageState::Discarded
    );

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
    assert!(
        harness
            .worker
            .claim(&mut harness.db, &harness.session_id, &harness.clock)
            .expect("claim")
            .is_none()
    );
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
    assert_eq!(
        harness.staging_state(&execution.run_id),
        ScanStageState::Discarded
    );
    let job = harness.db.scan_job(&execution.job_id).expect("job");
    assert_eq!(job.state, ScanJobState::Failed);
    assert_eq!(job.attempt, 1);

    trigger
        .execute_batch("DROP TRIGGER fruitboard_test_first_success_abort;")
        .expect("disarm injected failure");
    let eligible = ScanWorker::retry_eligible_at(&job);
    harness.clock.set(eligible);
    assert_eq!(
        harness
            .worker
            .service_retries(&mut harness.db, &harness.clock)
            .expect("retries"),
        1
    );
    let recovered = harness.drain(tree).expect("execution");
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    assert_eq!(
        recovered
            .publication
            .as_ref()
            .expect("publication")
            .location_count,
        3
    );
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
    assert_eq!(harness.run(&execution.run_id).state, ScanRunState::Running);
    assert_eq!(harness.run(&execution.run_id).error_code, None);
    assert_eq!(
        harness.db.scan_job(&execution.job_id).expect("job").state,
        ScanJobState::Running
    );
    assert_eq!(
        harness
            .db
            .scan_job(&execution.job_id)
            .expect("job")
            .last_error_code,
        None
    );

    // The reaper converts the expired run to interrupted and requeues the
    // same chain; after the backoff a fresh attempt completes the work.
    assert!(harness.drain(tree.clone()).is_none(), "not yet due");
    assert_eq!(
        harness.run(&execution.run_id).error_code.as_deref(),
        Some("lease_expired")
    );
    assert_eq!(
        harness
            .db
            .scan_job(&execution.job_id)
            .expect("reaped job")
            .last_error_code
            .as_deref(),
        Some("lease_expired")
    );
    harness.clock.advance(1_001);
    let recovered = harness.drain(tree).expect("execution");
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    assert_eq!(harness.committed(), before);
    assert_eq!(
        harness.db.scan_job(&execution.job_id).expect("job").attempt,
        2
    );
    assert_eq!(
        harness.run(&execution.run_id).state,
        ScanRunState::Interrupted
    );
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
    assert_ne!(
        stale_lease_token,
        harness.run(&replacement.run_id).lease_token
    );
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
    assert_eq!(
        harness.staging_state(&crashed_run),
        ScanStageState::Discarded
    );
    let jobs = harness.root_jobs();
    assert_eq!(
        jobs.len(),
        1,
        "no duplicate recovery job next to queued work"
    );
    assert_eq!(jobs[0].state, ScanJobState::Queued);
    assert_eq!(jobs[0].attempt, 1);

    harness.clock.advance(1_001);
    let recovered = harness
        .drain(tree(vec![file_entry("a.flp", 101)]))
        .expect("execution");
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
    assert!(
        jobs.iter()
            .any(|job| job.kind == ScanKind::Recovery && job.state == ScanJobState::Queued)
    );

    // A second restart while that recovery job is queued must not duplicate.
    harness.restart();
    assert_eq!(harness.root_jobs().len(), 2);

    let recovered = harness
        .drain(tree(vec![file_entry("a.flp", 101)]))
        .expect("execution");
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    no_changes(recovered.changes.as_ref().expect("change summary"));

    // A user-cancelled chain is never revived by recovery.
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let job = harness
        .root_jobs()
        .into_iter()
        .find(|job| job.state == ScanJobState::Queued)
        .unwrap();
    let state = harness
        .db
        .cancel_scan_job(&job.id, harness.clock.now_ms())
        .expect("cancel");
    assert_eq!(state, ScanJobState::Cancelled);
    harness.restart();
    assert_eq!(harness.root_jobs().len(), 3);
    assert!(
        harness
            .worker
            .claim(&mut harness.db, &harness.session_id, &harness.clock)
            .expect("claim")
            .is_none()
    );
}

// P2-04: repeated triggers coalesce onto one active slot; a trigger during
// traversal invalidates that attempt and schedules exactly one follow-up.
#[test]
fn repeated_triggers_coalesce_and_follow_up_reconciles() {
    let mut harness = Harness::new("coalesce");
    let tree = tree(vec![file_entry("a.flp", 101), file_entry("b.flp", 102)]);
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("trigger");
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("trigger");
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("trigger");
    assert_eq!(harness.root_jobs().len(), 1, "queued triggers coalesce");

    let scan = harness.claim();
    assert!(
        harness
            .worker
            .claim(&mut harness.db, &harness.session_id, &harness.clock)
            .expect("claim")
            .is_none(),
        "one worker runs one scan"
    );
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("trigger");
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
    assert_eq!(
        first
            .publication
            .as_ref()
            .expect("publication")
            .location_count,
        2
    );
    let before = harness.committed();

    // Four observations exceed the configured limit of two.
    let second_tree = tree(many_files(4));
    let second = harness.scan(second_tree);
    assert_eq!(second.status, ScanExecutionStatus::Failed);
    assert_eq!(second.enumeration_outcome, Some(EnumOutcome::ResourceLimit));
    assert!(second.publication.is_none());
    assert_eq!(harness.committed(), before);
    assert_eq!(
        harness.staging_state(&second.run_id),
        ScanStageState::Discarded
    );
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
    assert_eq!(
        second
            .publication
            .as_ref()
            .expect("publication")
            .location_count,
        1
    );
    let rows = harness.committed();
    assert_eq!(rows.len(), 1);
    assert!(
        !rows[0].present,
        "the vanished path is missing, not deleted"
    );

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

// --- Watcher follow-up wiring (#37, P2-09 storage half) ---

const WATCH_ROOT: RootId = RootId(1);

/// Scripted watcher port: raw activity and coverage-loss signals replayed
/// into the real bounded coalescer under the caller's tick, exactly like the
/// platform watcher does with its own clock. The host loop below is the same
/// shape the desktop host will bind.
struct ScriptedWatcher {
    activity: VecDeque<(RootId, u64, u64)>,
    lost: VecDeque<(RootId, u64, u64)>,
    coalescer: Coalescer,
}

impl ScriptedWatcher {
    fn new(config: CoalescerConfig) -> Self {
        Self {
            activity: VecDeque::new(),
            lost: VecDeque::new(),
            coalescer: Coalescer::new(config).expect("valid coalescer configuration"),
        }
    }

    fn activity(mut self, root: RootId, generation: u64, at: u64) -> Self {
        self.activity.push_back((root, generation, at));
        self
    }

    fn lost(mut self, root: RootId, generation: u64, at: u64) -> Self {
        self.lost.push_back((root, generation, at));
        self
    }
}

impl WatcherPort for ScriptedWatcher {
    fn poll_hints(&mut self, now: u64) -> Vec<WatchHint> {
        while let Some(&(root, generation, at)) = self.activity.front() {
            if at > now {
                break;
            }
            self.coalescer.record_activity(root, generation, at);
            self.activity.pop_front();
        }
        while let Some(&(root, generation, at)) = self.lost.front() {
            if at > now {
                break;
            }
            self.coalescer.record_coverage_lost(root, generation);
            self.lost.pop_front();
        }
        self.coalescer.poll(now)
    }

    fn outcome(&mut self) -> Option<WatchOutcome> {
        None
    }
}

/// Host-side watcher→storage root table: opaque watcher ids to opaque durable
/// storage ids. The host owns this table together with watch lifecycle; the
/// adapter never sees a path.
#[derive(Clone, Debug, Default)]
struct FakeMapping {
    roots: BTreeMap<u64, String>,
}

impl FakeMapping {
    fn track(root: RootId, storage_root_id: &str) -> Self {
        let mut roots = BTreeMap::new();
        roots.insert(root.0, storage_root_id.to_owned());
        Self { roots }
    }

    fn forget(&mut self, root: RootId) {
        self.roots.remove(&root.0);
    }
}

impl RootIdMapping for FakeMapping {
    fn storage_root_id(&self, root: RootId) -> Option<String> {
        self.roots.get(&root.0).cloned()
    }
}

/// The host installs each watch with a seeded generation before processing
/// hints, so the shared helper seeds the initial generation exactly like the
/// host lifecycle.
fn adapter(harness: &Harness) -> WatcherFollowUpAdapter<FakeMapping> {
    let mut follow_ups =
        WatcherFollowUpAdapter::new(FakeMapping::track(WATCH_ROOT, &harness.root_id));
    follow_ups.watch_started(WATCH_ROOT, 1);
    follow_ups
}

/// The host poll loop: drain the watcher port into the adapter at every tick.
fn host_loop(
    db: &mut Database,
    port: &mut dyn WatcherPort,
    follow_ups: &mut WatcherFollowUpAdapter<FakeMapping>,
    clock: &dyn ScanClock,
    end: u64,
    step: u64,
) -> Vec<FollowUpOutcome> {
    let mut outcomes = Vec::new();
    let mut now = 0;
    while now <= end {
        let hints = port.poll_hints(now);
        if !hints.is_empty() {
            outcomes.push(
                follow_ups
                    .process_hints(db, &hints, clock)
                    .expect("adapter"),
            );
        }
        now += step;
    }
    outcomes
}

// P2-09: a burst inside one coalescing window collapses into exactly one
// follow-up request; the durable queue never grows beyond the one queued
// follow-up, and later windows schedule fresh follow-ups only when work is no
// longer pending.
#[test]
fn watcher_burst_yields_exactly_one_follow_up_per_window() {
    let mut harness = Harness::new("followup-burst");
    let tree = tree(vec![file_entry("a.flp", 101), file_entry("b.flp", 102)]);
    let mut port = ScriptedWatcher::new(CoalescerConfig {
        window: 100,
        max_tracked_roots: 8,
    })
    .activity(WATCH_ROOT, 1, 10)
    .activity(WATCH_ROOT, 1, 20)
    .activity(WATCH_ROOT, 1, 30)
    .activity(WATCH_ROOT, 1, 40)
    .activity(WATCH_ROOT, 1, 210)
    .activity(WATCH_ROOT, 1, 220)
    .activity(WATCH_ROOT, 1, 230)
    .activity(WATCH_ROOT, 1, 410);
    let mut follow_ups = adapter(&harness);

    // The first burst's window closes at 110: one hint, one follow-up.
    let first = host_loop(
        &mut harness.db,
        &mut port,
        &mut follow_ups,
        &harness.clock,
        120,
        10,
    );
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].requests.len(), 1);
    assert!(!first[0].requests[0].coalesced);
    assert_eq!(first[0].requests[0].cause, FollowUpCause::ActivityBurst);
    assert_eq!(harness.root_jobs().len(), 1, "a burst creates one job");
    let job = &harness.root_jobs()[0];
    assert_eq!(job.kind, ScanKind::Periodic);
    assert_eq!(job.state, ScanJobState::Queued);

    // The second burst's window closes at 310 while the follow-up is still
    // queued: the hint coalesces onto the existing slot, queue unchanged.
    let second = host_loop(
        &mut harness.db,
        &mut port,
        &mut follow_ups,
        &harness.clock,
        320,
        10,
    );
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].requests.len(), 1);
    assert!(second[0].requests[0].coalesced);
    assert_eq!(harness.root_jobs().len(), 1);

    let completed = harness.drain(tree.clone()).expect("execution");
    assert_eq!(completed.status, ScanExecutionStatus::Published);
    assert_eq!(
        completed
            .publication
            .as_ref()
            .expect("publication")
            .location_count,
        2
    );

    // The third burst's window closes at 510; the queue is empty again, so
    // the same watch schedules a fresh follow-up.
    let third = host_loop(
        &mut harness.db,
        &mut port,
        &mut follow_ups,
        &harness.clock,
        520,
        10,
    );
    assert_eq!(third.len(), 1);
    assert_eq!(third[0].requests.len(), 1);
    assert!(!third[0].requests[0].coalesced);
    assert_eq!(harness.root_jobs().len(), 2, "one completed, one queued");
    assert_eq!(follow_ups.stale_generation_dropped(), 0);
}

// P2-09: a coverage-loss (overflow) signal schedules the same single
// full-reconciliation follow-up with the overflow cause recorded, and is
// never treated as absence evidence: the follow-up scans the entire root.
#[test]
fn coverage_lost_schedules_one_full_reconciliation_and_records_overflow() {
    let mut harness = Harness::new("followup-overflow");
    let mut follow_ups = adapter(&harness);
    let hint = WatchHint {
        root: WATCH_ROOT,
        generation: 1,
        kind: HintKind::CoverageLost,
    };

    let outcome = follow_ups
        .process_hints(&mut harness.db, &[hint], &harness.clock)
        .expect("adapter");
    assert_eq!(outcome.requests.len(), 1);
    assert_eq!(outcome.requests[0].cause, FollowUpCause::Overflow);
    assert!(!outcome.requests[0].coalesced);
    assert_eq!(outcome.overflow, 1);
    assert_eq!(follow_ups.overflow_follow_ups(WATCH_ROOT), 1);
    assert_eq!(harness.root_jobs().len(), 1);
    let job = &harness.root_jobs()[0];
    assert_eq!(job.kind, ScanKind::Periodic);
    assert_eq!(job.state, ScanJobState::Queued);
    assert!(job.last_error_code.is_none());

    // A second overflow within the same watch coalesces onto the same job.
    let again = follow_ups
        .process_hints(&mut harness.db, &[hint], &harness.clock)
        .expect("adapter");
    assert_eq!(again.requests.len(), 1);
    assert!(again.requests[0].coalesced);
    assert_eq!(again.requests[0].cause, FollowUpCause::Overflow);
    assert_eq!(
        harness.root_jobs().len(),
        1,
        "overflow never grows the queue"
    );
    assert_eq!(follow_ups.overflow_follow_ups(WATCH_ROOT), 2);

    // The overflow-driven follow-up is a full-root reconciliation: the whole
    // tree is published, nothing is inferred as absent.
    let execution = harness
        .drain(tree(vec![
            file_entry("a.flp", 101),
            file_entry("b.flp", 102),
        ]))
        .expect("execution");
    assert_eq!(execution.status, ScanExecutionStatus::Published);
    assert_eq!(
        execution
            .publication
            .as_ref()
            .expect("publication")
            .location_count,
        2
    );
}

// P2-09: a burst and an overflow inside one coalescing window subsume to a
// single durable follow-up with the overflow cause. The coalescer delivers
// the coverage-loss hint immediately and drops the pending window, so the
// adapter sees exactly one hint and the queue never grows beyond one job.
#[test]
fn burst_and_overflow_in_one_window_subsume_to_a_single_follow_up() {
    let mut harness = Harness::new("followup-burst-overflow");
    let mut port = ScriptedWatcher::new(CoalescerConfig {
        window: 100,
        max_tracked_roots: 8,
    })
    .activity(WATCH_ROOT, 1, 10)
    .activity(WATCH_ROOT, 1, 20)
    .lost(WATCH_ROOT, 1, 30);
    let mut follow_ups = adapter(&harness);

    let outcomes = host_loop(
        &mut harness.db,
        &mut port,
        &mut follow_ups,
        &harness.clock,
        200,
        10,
    );
    assert_eq!(outcomes.len(), 1, "one coalesced poll delivers hints once");
    assert_eq!(outcomes[0].requests.len(), 1);
    assert_eq!(outcomes[0].requests[0].cause, FollowUpCause::Overflow);
    assert!(!outcomes[0].requests[0].coalesced);
    assert_eq!(outcomes[0].overflow, 1);
    assert_eq!(harness.root_jobs().len(), 1, "burst + loss is one job");

    let execution = harness
        .drain(tree(vec![file_entry("a.flp", 101)]))
        .expect("execution");
    assert_eq!(execution.status, ScanExecutionStatus::Published);
    assert_eq!(
        execution
            .publication
            .as_ref()
            .expect("publication")
            .location_count,
        1
    );
}

// P2-09: a hint older than the current watch generation is dropped; replays
// at the current generation are current and coalesce on storage; a strictly
// newer generation reopens the root.
#[test]
fn stale_generation_hints_are_dropped_and_newer_generations_reopen() {
    let mut harness = Harness::new("followup-stale-gen");
    let mut follow_ups = adapter(&harness);
    let hint = |generation: u64| WatchHint {
        root: WATCH_ROOT,
        generation,
        kind: HintKind::ReconciliationRequested,
    };

    let first = follow_ups
        .process_hints(&mut harness.db, &[hint(1)], &harness.clock)
        .expect("adapter");
    assert_eq!(first.requests.len(), 1);
    assert!(!first.requests[0].coalesced);

    let older = follow_ups
        .process_hints(&mut harness.db, &[hint(0)], &harness.clock)
        .expect("adapter");
    assert_eq!(older.stale_generation_dropped, 1);
    assert!(older.requests.is_empty());

    let replay = follow_ups
        .process_hints(&mut harness.db, &[hint(1)], &harness.clock)
        .expect("adapter");
    assert_eq!(replay.requests.len(), 1);
    assert!(
        replay.requests[0].coalesced,
        "a replay at the current generation is current"
    );

    let newer = follow_ups
        .process_hints(&mut harness.db, &[hint(2)], &harness.clock)
        .expect("adapter");
    assert_eq!(newer.requests.len(), 1);
    assert!(
        newer.requests[0].coalesced,
        "a fresh watch still coalesces onto pending work"
    );
    assert_eq!(newer.stale_generation_dropped, 0);

    let late_old = follow_ups
        .process_hints(&mut harness.db, &[hint(1)], &harness.clock)
        .expect("adapter");
    assert_eq!(
        late_old.stale_generation_dropped, 1,
        "a late old-generation hint is dropped"
    );
    assert_eq!(follow_ups.stale_generation_dropped(), 2);
    assert_eq!(harness.root_jobs().len(), 1, "the queue never grew");
}

// P2-09: when the host ends a watch, replayed hints are dropped idempotently
// (double-discard) until a strictly newer generation starts a fresh watch.
#[test]
fn watch_ended_drops_replayed_hints_until_a_fresh_generation() {
    let mut harness = Harness::new("followup-watch-ended");
    let mut follow_ups = adapter(&harness);
    let hint = |generation: u64| WatchHint {
        root: WATCH_ROOT,
        generation,
        kind: HintKind::ReconciliationRequested,
    };

    follow_ups
        .process_hints(&mut harness.db, &[hint(1)], &harness.clock)
        .expect("adapter");
    assert_eq!(harness.root_jobs().len(), 1);
    follow_ups.watch_ended(WATCH_ROOT);
    follow_ups.watch_ended(WATCH_ROOT);

    let replay = follow_ups
        .process_hints(&mut harness.db, &[hint(1)], &harness.clock)
        .expect("adapter");
    assert_eq!(replay.stale_generation_dropped, 1);
    assert!(replay.requests.is_empty());
    let replay = follow_ups
        .process_hints(&mut harness.db, &[hint(1)], &harness.clock)
        .expect("adapter");
    assert_eq!(
        replay.stale_generation_dropped, 1,
        "double-discard is idempotent"
    );
    assert_eq!(
        harness.root_jobs().len(),
        1,
        "replays after the end never schedule work"
    );

    let fresh = follow_ups
        .process_hints(&mut harness.db, &[hint(2)], &harness.clock)
        .expect("adapter");
    assert_eq!(fresh.requests.len(), 1);
    assert!(fresh.requests[0].coalesced);
    assert_eq!(fresh.stale_generation_dropped, 0);
    assert_eq!(harness.root_jobs().len(), 1);
}

// P2-09: disabled roots are suppressed by storage itself, not by the
// adapter's mapping: the request reaches the storage boundary and storage
// refuses it, and no overflow is recorded for a follow-up that was never
// requested.
#[test]
fn disabled_root_suppresses_follow_ups_through_storage() {
    let mut harness = Harness::new("followup-disabled");
    let mut follow_ups = adapter(&harness);
    harness
        .db
        .set_scan_root_enabled_at(&harness.root_id, false, harness.clock.now_ms())
        .expect("disable root");

    let hint = WatchHint {
        root: WATCH_ROOT,
        generation: 1,
        kind: HintKind::CoverageLost,
    };
    let outcome = follow_ups
        .process_hints(&mut harness.db, &[hint], &harness.clock)
        .expect("adapter");
    assert_eq!(outcome.suppressed, 1);
    assert!(outcome.requests.is_empty());
    assert_eq!(outcome.overflow, 0, "a suppressed request is no follow-up");
    assert_eq!(harness.root_jobs().len(), 0, "no job for a disabled root");
    assert_eq!(follow_ups.suppressed(), 1);
    assert_eq!(follow_ups.overflow_follow_ups(WATCH_ROOT), 0);

    harness
        .db
        .set_scan_root_enabled_at(&harness.root_id, true, harness.clock.now_ms())
        .expect("re-enable root");
    let reenabled = follow_ups
        .process_hints(&mut harness.db, &[hint], &harness.clock)
        .expect("adapter");
    assert_eq!(reenabled.requests.len(), 1);
    assert_eq!(reenabled.requests[0].cause, FollowUpCause::Overflow);
    assert_eq!(harness.root_jobs().len(), 1);
}

// P2-09: hints for a removed root are dropped without errors — both when the
// host mapping no longer knows the root and when storage reports it removed —
// and repeated drops are idempotent.
#[test]
fn removed_root_hints_are_dropped_without_errors() {
    let mut harness = Harness::new("followup-removed");
    harness
        .db
        .remove_scan_root_at(&harness.root_id, harness.clock.now_ms())
        .expect("remove root");
    let mut mapping = FakeMapping::track(WATCH_ROOT, &harness.root_id);
    let mut follow_ups = WatcherFollowUpAdapter::new(mapping.clone());
    follow_ups.watch_started(WATCH_ROOT, 1);
    let hint = |kind: HintKind| WatchHint {
        root: WATCH_ROOT,
        generation: 1,
        kind,
    };

    // The mapping still resolves, but storage no longer has the root.
    let stale = follow_ups
        .process_hints(
            &mut harness.db,
            &[hint(HintKind::ReconciliationRequested)],
            &harness.clock,
        )
        .expect("adapter");
    assert_eq!(stale.unknown_root_dropped, 1);
    assert!(stale.requests.is_empty());
    assert_eq!(follow_ups.unknown_root_dropped(), 1);
    assert_eq!(follow_ups.overflow_follow_ups(WATCH_ROOT), 0);

    // The host mapping forgets the root: the same drop through the mapping.
    mapping.forget(WATCH_ROOT);
    let mut follow_ups = WatcherFollowUpAdapter::new(mapping);
    follow_ups.watch_started(WATCH_ROOT, 1);
    let dropped = follow_ups
        .process_hints(
            &mut harness.db,
            &[
                hint(HintKind::ReconciliationRequested),
                hint(HintKind::CoverageLost),
            ],
            &harness.clock,
        )
        .expect("adapter");
    assert_eq!(dropped.unknown_root_dropped, 2);
    assert!(dropped.requests.is_empty());
    assert_eq!(follow_ups.unknown_root_dropped(), 2);
    assert!(harness.db.list_scan_roots().expect("roots").is_empty());
    assert_eq!(harness.root_jobs().len(), 0);
}

// P2-09: a hint while work is running coalesces onto the running attempt's
// follow-up flag; storage schedules exactly one follow-up after the attempt
// is interrupted, and the queue never grows beyond it.
#[test]
fn running_work_coalesces_hints_onto_one_follow_up_request() {
    let mut harness = Harness::new("followup-running");
    let mut follow_ups = adapter(&harness);
    let tree = tree(vec![file_entry("a.flp", 101), file_entry("b.flp", 102)]);
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();

    let hint = WatchHint {
        root: WATCH_ROOT,
        generation: 1,
        kind: HintKind::ReconciliationRequested,
    };
    let outcome = follow_ups
        .process_hints(&mut harness.db, &[hint], &harness.clock)
        .expect("adapter");
    assert_eq!(outcome.requests.len(), 1);
    assert!(
        outcome.requests[0].coalesced,
        "running work owns the active slot"
    );
    assert_eq!(outcome.requests[0].job_id, scan.leased.run.scan_job_id);
    assert_eq!(harness.root_jobs().len(), 1, "no new job while running");
    let running = harness
        .db
        .scan_job(&scan.leased.run.scan_job_id)
        .expect("job");
    assert_eq!(running.state, ScanJobState::Running);
    assert!(
        running.follow_up_requested,
        "the hint lands on the running job's follow-up flag"
    );

    let mut port = FakePort::new(tree.clone());
    let execution = harness.worker.execute(
        &mut harness.db,
        scan,
        &mut port,
        &NeverCancelled,
        &harness.clock,
    );
    assert_eq!(execution.status, ScanExecutionStatus::Interrupted);

    let follow_up = harness.drain(tree).expect("follow-up execution");
    assert_eq!(follow_up.status, ScanExecutionStatus::Published);
    assert_eq!(
        follow_up
            .publication
            .as_ref()
            .expect("publication")
            .location_count,
        2
    );
    assert_eq!(
        harness.root_jobs().len(),
        2,
        "one follow-up job, never more"
    );
}

// P2-09: a cancelled chain is never revived, but a fresh trigger after the
// cancellation starts a new chain.
#[test]
fn cancelled_chain_is_not_revived_but_a_new_trigger_starts_fresh_work() {
    let mut harness = Harness::new("followup-cancelled");
    let mut follow_ups = adapter(&harness);
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let queued = harness
        .root_jobs()
        .into_iter()
        .find(|job| job.state == ScanJobState::Queued)
        .unwrap();
    let state = harness
        .db
        .cancel_scan_job(&queued.id, harness.clock.now_ms())
        .expect("cancel");
    assert_eq!(state, ScanJobState::Cancelled);

    let hint = WatchHint {
        root: WATCH_ROOT,
        generation: 1,
        kind: HintKind::ReconciliationRequested,
    };
    let outcome = follow_ups
        .process_hints(&mut harness.db, &[hint], &harness.clock)
        .expect("adapter");
    assert_eq!(outcome.requests.len(), 1);
    assert!(
        !outcome.requests[0].coalesced,
        "a cancelled chain owns no active slot"
    );
    let jobs = harness.root_jobs();
    assert_eq!(jobs.len(), 2, "cancelled job untouched, one fresh job");
    assert!(jobs.iter().any(|job| job.state == ScanJobState::Cancelled));
    assert!(jobs.iter().any(|job| job.state == ScanJobState::Queued));

    let execution = harness.drain(tree(vec![file_entry("a.flp", 101)]));
    assert_eq!(
        execution.expect("execution").status,
        ScanExecutionStatus::Published
    );
}

// P2-09: replaying the same hint is idempotent at the durable boundary — the
// queue never grows — and after the follow-up completes, the same signal is a
// fresh window that schedules the next reconciliation. Hints are scheduling
// signals; the durable queue state decides dedup.
#[test]
fn idempotent_replay_never_grows_the_queue() {
    let mut harness = Harness::new("followup-replay");
    let mut follow_ups = adapter(&harness);
    let hint = WatchHint {
        root: WATCH_ROOT,
        generation: 1,
        kind: HintKind::ReconciliationRequested,
    };
    let tree = tree(vec![file_entry("a.flp", 101)]);

    let outcome = follow_ups
        .process_hints(&mut harness.db, &[hint, hint, hint], &harness.clock)
        .expect("adapter");
    assert_eq!(outcome.requests.len(), 3);
    assert!(!outcome.requests[0].coalesced);
    assert!(outcome.requests[1].coalesced);
    assert!(outcome.requests[2].coalesced);
    assert_eq!(
        harness.root_jobs().len(),
        1,
        "a burst never grows the queue"
    );

    let replay = follow_ups
        .process_hints(&mut harness.db, &[hint], &harness.clock)
        .expect("adapter");
    assert_eq!(replay.requests.len(), 1);
    assert!(replay.requests[0].coalesced);
    assert_eq!(harness.root_jobs().len(), 1);

    let completed = harness.drain(tree).expect("execution");
    assert_eq!(completed.status, ScanExecutionStatus::Published);

    let fresh = follow_ups
        .process_hints(&mut harness.db, &[hint], &harness.clock)
        .expect("adapter");
    assert_eq!(fresh.requests.len(), 1);
    assert!(
        !fresh.requests[0].coalesced,
        "after completion the same signal is a new window"
    );
    assert_eq!(
        harness.root_jobs().len(),
        2,
        "one completed, one newly queued"
    );
}

// P2-09: a watcher restart with a fresh generation (a new watcher per the #69
// contract) drops late hints from the dead generation and schedules the
// restart's own follow-up.
#[test]
fn watcher_restart_with_fresh_generation_drops_old_hints_and_reschedules() {
    let mut harness = Harness::new("followup-restart");
    let tree = tree(vec![file_entry("a.flp", 101)]);
    let mut first_watch = ScriptedWatcher::new(CoalescerConfig {
        window: 100,
        max_tracked_roots: 8,
    })
    .activity(WATCH_ROOT, 1, 10);
    let mut follow_ups = adapter(&harness);

    let first = host_loop(
        &mut harness.db,
        &mut first_watch,
        &mut follow_ups,
        &harness.clock,
        120,
        10,
    );
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].requests.len(), 1);
    assert_eq!(harness.root_jobs().len(), 1);
    let completed = harness.drain(tree.clone()).expect("execution");
    assert_eq!(completed.status, ScanExecutionStatus::Published);

    // The host restarts the watch with a strictly newer generation.
    let mut second_watch = ScriptedWatcher::new(CoalescerConfig {
        window: 100,
        max_tracked_roots: 8,
    })
    .activity(WATCH_ROOT, 2, 10);
    let second = host_loop(
        &mut harness.db,
        &mut second_watch,
        &mut follow_ups,
        &harness.clock,
        120,
        10,
    );
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].requests.len(), 1);
    assert!(
        !second[0].requests[0].coalesced,
        "the fresh generation schedules its own follow-up"
    );
    assert_eq!(harness.root_jobs().len(), 2);
    let restarted = harness.drain(tree).expect("execution");
    assert_eq!(restarted.status, ScanExecutionStatus::Published);

    // A late hint from the dead generation is dropped by the fence.
    let late = WatchHint {
        root: WATCH_ROOT,
        generation: 1,
        kind: HintKind::ReconciliationRequested,
    };
    let stale = follow_ups
        .process_hints(&mut harness.db, &[late], &harness.clock)
        .expect("adapter");
    assert_eq!(stale.stale_generation_dropped, 1);
    assert!(stale.requests.is_empty());
    assert_eq!(follow_ups.stale_generation_dropped(), 1);
}

// P2-09: the host must seed each watch before its hints are processed. A
// fresh adapter (for example after a host restart) drops hints until the
// seed arrives, so a replayed hint from a dead generation can never be
// adopted as the current generation at bootstrap.
#[test]
fn unseeded_adapter_drops_hints_until_the_host_seeds_the_watch() {
    let mut harness = Harness::new("followup-unseeded");
    let mut follow_ups =
        WatcherFollowUpAdapter::new(FakeMapping::track(WATCH_ROOT, &harness.root_id));
    let hint = WatchHint {
        root: WATCH_ROOT,
        generation: 2,
        kind: HintKind::ReconciliationRequested,
    };

    // No seed yet: the hint is dropped as stale, not adopted as current.
    let unseeded = follow_ups
        .process_hints(&mut harness.db, &[hint], &harness.clock)
        .expect("adapter");
    assert_eq!(unseeded.stale_generation_dropped, 1);
    assert!(unseeded.requests.is_empty());
    assert_eq!(follow_ups.stale_generation_dropped(), 1);
    assert_eq!(harness.root_jobs().len(), 0);

    // The host installs the watch at generation 2; the same hint is accepted
    // and schedules its follow-up.
    follow_ups.watch_started(WATCH_ROOT, 2);
    let seeded = follow_ups
        .process_hints(&mut harness.db, &[hint], &harness.clock)
        .expect("adapter");
    assert_eq!(seeded.requests.len(), 1);
    assert_eq!(harness.root_jobs().len(), 1);
}

// P2-09: a seeded adapter fences replayed hints from a dead generation at
// bootstrap: the seeded generation is the live watch's, older replayed hints
// are dropped in batch order, and the seeded generation's own hint is
// accepted.
#[test]
fn seeded_adapter_fences_replayed_hints_from_a_dead_generation() {
    let mut harness = Harness::new("followup-seeded-fence");
    let mut follow_ups =
        WatcherFollowUpAdapter::new(FakeMapping::track(WATCH_ROOT, &harness.root_id));
    follow_ups.watch_started(WATCH_ROOT, 3);
    let hint = |generation: u64| WatchHint {
        root: WATCH_ROOT,
        generation,
        kind: HintKind::ReconciliationRequested,
    };

    let outcome = follow_ups
        .process_hints(
            &mut harness.db,
            &[hint(2), hint(3), hint(1)],
            &harness.clock,
        )
        .expect("adapter");
    assert_eq!(outcome.stale_generation_dropped, 2);
    assert_eq!(outcome.requests.len(), 1);
    assert!(!outcome.requests[0].coalesced);
    assert_eq!(follow_ups.stale_generation_dropped(), 2);
    assert_eq!(harness.root_jobs().len(), 1);
}

/// The fake consumer proving the exact trait shape the desktop host binds:
/// a `WatcherPort` polled on a loop, hints translated by the adapter, and the
/// worker executing the durable follow-ups. This is the compiling contract
/// for the host wiring slice (still out of scope here).
struct FakeConsumer {
    port: Box<dyn WatcherPort>,
    follow_ups: WatcherFollowUpAdapter<FakeMapping>,
}

impl FakeConsumer {
    fn tick(&mut self, db: &mut Database, now: u64, clock: &dyn ScanClock) -> FollowUpOutcome {
        let hints = self.port.poll_hints(now);
        if hints.is_empty() {
            return FollowUpOutcome::default();
        }
        self.follow_ups
            .process_hints(db, &hints, clock)
            .expect("adapter")
    }
}

// P2-09: the full host-loop shape end to end — bursts and an overflow signal
// each become one durable follow-up, the worker reconciles every one of them,
// and the library converges on the observed tree with no queue growth.
#[test]
fn fake_consumer_binds_the_host_loop_shape_end_to_end() {
    let mut harness = Harness::new("followup-host-shape");
    let tree = tree(vec![file_entry("a.flp", 101), file_entry("b.flp", 102)]);
    let port = Box::new(
        ScriptedWatcher::new(CoalescerConfig {
            window: 100,
            max_tracked_roots: 8,
        })
        .activity(WATCH_ROOT, 1, 10)
        .activity(WATCH_ROOT, 1, 20)
        .lost(WATCH_ROOT, 1, 200)
        .activity(WATCH_ROOT, 1, 310),
    ) as Box<dyn WatcherPort>;
    let mut consumer = FakeConsumer {
        port,
        follow_ups: adapter(&harness),
    };

    let mut now = 0;
    while now <= 450 {
        consumer.tick(&mut harness.db, now, &harness.clock);
        harness
            .worker
            .poll(
                &mut harness.db,
                &harness.session_id,
                &mut FakePort::new(tree.clone()),
                &NeverCancelled,
                &harness.clock,
            )
            .expect("worker poll");
        now += 10;
    }

    let rows = harness.committed();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.present));
    let jobs = harness.root_jobs();
    assert_eq!(
        jobs.len(),
        3,
        "burst, overflow and burst follow-ups each coalesce to one job"
    );
    assert!(jobs.iter().all(|job| job.state == ScanJobState::Completed));
    assert_eq!(consumer.follow_ups.overflow_follow_ups(WATCH_ROOT), 1);
}

// P2-09/P2-10 privacy regression: no absolute path, no relative path, and no
// display name can cross the watcher→storage boundary. The adapter's public
// types are structurally incapable of carrying a path (only opaque ids and
// counters), and the durable request lands on storage with the opaque root id
// alone.
#[test]
fn follow_up_boundary_never_carries_path_data() {
    let mut harness = Harness::new("followup-privacy");
    let mut follow_ups = adapter(&harness);
    let hints = [
        WatchHint {
            root: WATCH_ROOT,
            generation: 1,
            kind: HintKind::ReconciliationRequested,
        },
        WatchHint {
            root: WATCH_ROOT,
            generation: 1,
            kind: HintKind::CoverageLost,
        },
    ];
    let outcome = follow_ups
        .process_hints(&mut harness.db, &hints, &harness.clock)
        .expect("adapter");
    assert_eq!(outcome.requests.len(), 2);

    // Every public type renders without any path-like text: the type shapes
    // carry no path field, and Debug is the only rendering surface.
    let rendered = [
        format!("{:?}", FollowUpCause::ActivityBurst),
        format!("{:?}", FollowUpCause::Overflow),
        format!("{:?}", outcome.requests[0]),
        format!("{:?}", outcome.requests[1]),
        format!("{:?}", outcome),
        format!("{:?}", hints[0]),
        format!("{:?}", follow_ups),
    ];
    for text in rendered {
        assert!(
            !text.contains('\\') && !text.contains('/'),
            "path separator leaked: {text}"
        );
        assert!(!text.contains(".flp"), "file name leaked: {text}");
        assert!(!text.contains("synthetic-root"), "root path leaked: {text}");
    }

    // The durable request reaches storage with the opaque root id only; the
    // job carries ids and a trigger kind, never a path.
    let job = &harness.root_jobs()[0];
    assert_eq!(job.scan_root_id, harness.root_id);
    assert_eq!(job.scan_root_id, outcome.requests[0].storage_root_id);
    assert_eq!(job.kind, ScanKind::Periodic);
    assert!(job.last_error_code.is_none());
}

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

        let mut harness = Harness::with_config(
            "ntfs-authoritative",
            WorkerConfig::default(),
            &fixture.root.to_string_lossy(),
        );
        let mut port = WindowsFilesystemPort::new();
        let first = harness.scan_with_port(&mut port);
        assert_eq!(first.status, ScanExecutionStatus::Published);
        assert_eq!(first.enumeration_outcome, Some(EnumOutcome::Complete));
        let publication = first.publication.as_ref().expect("publication");
        assert_eq!(publication.location_count, 2);

        let rows = harness.committed();
        assert_eq!(rows.len(), 2);
        let alpha = rows
            .iter()
            .find(|row| row.relative_path == "alpha.FLP")
            .expect("alpha row");
        assert!(alpha.present);
        assert_eq!(alpha.byte_size, marker_before.len() as u64);
        assert!(
            rows.iter()
                .any(|row| row.relative_path == "nested\\beta.flp")
        );
        assert!(
            !rows
                .iter()
                .any(|row| row.relative_path.contains("notes.txt"))
        );
        let marker_after = std::fs::read(fixture.root.join("alpha.FLP")).expect("marker");
        assert_eq!(
            marker_before, marker_after,
            "source bytes are never read or written"
        );

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

        let mut harness = Harness::with_config(
            "ntfs-denied",
            WorkerConfig::default(),
            &fixture.root.to_string_lossy(),
        );
        let mut port = WindowsFilesystemPort::new();
        let first = harness.scan_with_port(&mut port);
        assert_eq!(first.status, ScanExecutionStatus::Published);
        let before = harness.committed();
        assert_eq!(before.len(), 2);

        let _guard = fixture.deny("denied");
        let second = harness.scan_with_port(&mut port);
        if second.enumeration_outcome == Some(EnumOutcome::Complete) {
            // Elevated tokens (SeBackupPrivilege, common on CI runners)
            // bypass the fixture ACL, so the denial precondition does not
            // hold on this host. The portable fake
            // (`denied_subtree_discards_staging_and_preserves_committed_rows`)
            // remains the P2-03 gate; assert only the invariant that any
            // rescan leaves committed rows converged.
            eprintln!(
                "SKIP: fixture ACL denial ineffective under this token; \
                 committed rows unchanged ({})",
                harness.committed().len()
            );
            return;
        }
        assert_eq!(second.status, ScanExecutionStatus::Failed);
        assert_eq!(second.enumeration_outcome, Some(EnumOutcome::Denied));
        assert!(second.publication.is_none());
        assert_eq!(
            harness.committed(),
            before,
            "denied subtree is never marked missing"
        );

        drop(_guard);
        let third = harness.scan_with_port(&mut port);
        assert_eq!(third.status, ScanExecutionStatus::Published);
        assert!(harness.committed().iter().all(|row| row.present));
    }
}

// --- Wave 5 durability close-out (P2-03/P2-05/P2-06/P2-07) ---
//
// Every test below compares committed rows PLUS the root success marker as
// byte-identical before/after blobs. Row equality alone would miss a marker
// advance; marker equality alone would miss a partial apply. The pair is the
// durable Library snapshot.

fn seed_two_files(label: &str) -> (Harness, Vec<CommittedRow>) {
    let mut harness = Harness::new(label);
    let tree = tree(vec![
        file_entry("kept.flp", 101),
        file_entry("other.flp", 102),
    ]);
    let first = harness.scan(tree);
    assert_eq!(first.status, ScanExecutionStatus::Published);
    let before = harness.committed();
    assert_eq!(before.len(), 2);
    assert!(before.iter().all(|row| row.present));
    (harness, before)
}

// P2-03: offline (RootUnavailable) leaves rows+snapshot byte-identical.
#[test]
fn p2_03_offline_preserves_rows_and_snapshot_byte_identical() {
    let (mut harness, _) = seed_two_files("p2-03-offline");
    let (before_rows, before_marker, before_bytes) = committed_state(&harness);
    let failed = harness.scan_offline(tree(vec![file_entry("kept.flp", 101)]));
    assert_eq!(failed.status, ScanExecutionStatus::Failed);
    assert_eq!(
        failed.enumeration_outcome,
        Some(EnumOutcome::RootUnavailable)
    );
    assert!(!failed.authoritative);
    assert!(failed.publication.is_none());
    assert_committed_unchanged(&before_rows, &before_marker, &before_bytes, &harness);
    assert_eq!(
        harness.staging_state(&failed.run_id),
        ScanStageState::Discarded
    );
}

// P2-03: root-level denial leaves rows+snapshot byte-identical.
#[test]
fn p2_03_root_denied_preserves_rows_and_snapshot_byte_identical() {
    let (mut harness, _) = seed_two_files("p2-03-root-denied");
    let (before_rows, before_marker, before_bytes) = committed_state(&harness);
    let mut port = FakePort::with_root_error(
        tree(vec![file_entry("kept.flp", 101)]),
        PortError::AccessDenied,
    );
    let execution = harness.scan_with_port(&mut port);
    assert_eq!(execution.status, ScanExecutionStatus::Failed);
    assert_eq!(execution.enumeration_outcome, Some(EnumOutcome::Denied));
    assert!(!execution.authoritative);
    assert!(execution.publication.is_none());
    assert_committed_unchanged(&before_rows, &before_marker, &before_bytes, &harness);
    assert_eq!(
        harness.staging_state(&execution.run_id),
        ScanStageState::Discarded
    );
}

// P2-03: a disappeared directory (Partial) never publishes partial results.
#[test]
fn p2_03_partial_disappearance_preserves_rows_and_snapshot_byte_identical() {
    let (mut harness, _) = seed_two_files("p2-03-partial");
    let (before_rows, before_marker, before_bytes) = committed_state(&harness);
    let partial_tree = tree(vec![
        file_entry("kept.flp", 101),
        vanishing_dir_entry("gone", 201, vec![file_entry("inner.flp", 301)]),
    ]);
    let mut port = FakePort::new(partial_tree);
    let execution = harness.scan_with_port(&mut port);
    assert!(!execution.authoritative);
    assert!(execution.publication.is_none());
    assert_eq!(execution.enumeration_outcome, Some(EnumOutcome::Partial));
    assert_eq!(
        execution.partial_class,
        Some(PartialClass::DirectoryNotFound)
    );
    assert_eq!(execution.status, ScanExecutionStatus::Failed);
    assert_committed_unchanged(&before_rows, &before_marker, &before_bytes, &harness);
    assert_eq!(
        harness.staging_state(&execution.run_id),
        ScanStageState::Discarded
    );
    let recovered = harness.scan(tree(vec![
        file_entry("kept.flp", 101),
        file_entry("other.flp", 102),
        dir_entry("gone", 201, vec![file_entry("inner.flp", 301)]),
    ]));
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    assert_eq!(recovered.enumeration_outcome, Some(EnumOutcome::Complete));
    assert!(recovered.authoritative);
    assert_eq!(recovered.partial_class, None);
    assert!(recovered.publication.is_some());
    let (_, recovered_marker, _) = committed_state(&harness);
    assert_ne!(recovered_marker, before_marker);
}

#[test]
fn partial_class_is_bounded_and_does_not_guess_after_truncation() {
    fn report(
        failures: Vec<enumeration::CoverageFailure>,
        omitted: usize,
    ) -> enumeration::EnumerationReport {
        enumeration::EnumerationReport {
            outcome: EnumOutcome::Partial,
            authoritative: false,
            root_qualification: None,
            directories_visited: 0,
            entries_examined: 0,
            observations_discovered: 0,
            batches_delivered: 0,
            identity_unavailable: 0,
            exclusions: Vec::new(),
            omitted_exclusions: 0,
            coverage_failures: failures,
            omitted_coverage_failures: omitted,
        }
    }

    let one = report(
        vec![enumeration::CoverageFailure {
            display_path: None,
            kind: enumeration::CoverageFailureKind::MetadataRead,
        }],
        0,
    );
    assert_eq!(partial_class(&one), Some(PartialClass::MetadataRead));

    let multiple = report(
        vec![
            enumeration::CoverageFailure {
                display_path: None,
                kind: enumeration::CoverageFailureKind::MetadataRead,
            },
            enumeration::CoverageFailure {
                display_path: None,
                kind: enumeration::CoverageFailureKind::DirectoryChanged,
            },
        ],
        0,
    );
    assert_eq!(partial_class(&multiple), Some(PartialClass::Multiple));

    let truncated = report(
        vec![enumeration::CoverageFailure {
            display_path: None,
            kind: enumeration::CoverageFailureKind::MetadataRead,
        }],
        1,
    );
    assert_eq!(partial_class(&truncated), Some(PartialClass::Unknown));

    let empty = report(Vec::new(), 0);
    assert_eq!(partial_class(&empty), Some(PartialClass::Unknown));

    let mut complete = empty;
    complete.outcome = EnumOutcome::Complete;
    assert_eq!(partial_class(&complete), None);
    assert_eq!(PartialClass::Multiple.as_str(), "partial_multiple");
    assert_eq!(PartialClass::Unknown.as_str(), "partial_unknown");
}

// P2-03: unsupported filesystems are non-authoritative and preserve state.
#[test]
fn p2_03_unsupported_filesystem_preserves_rows_and_snapshot_byte_identical() {
    let (mut harness, _) = seed_two_files("p2-03-unsupported");
    let (before_rows, before_marker, before_bytes) = committed_state(&harness);
    let mut port = FakePort::with_root_error(
        tree(vec![file_entry("kept.flp", 101)]),
        PortError::Unsupported,
    );
    let execution = harness.scan_with_port(&mut port);
    assert!(!execution.authoritative);
    assert!(execution.publication.is_none());
    assert_eq!(
        execution.enumeration_outcome,
        Some(EnumOutcome::UnsupportedFilesystem)
    );
    assert_committed_unchanged(&before_rows, &before_marker, &before_bytes, &harness);
}

// P2-03: cooperative cancellation with prior committed rows preserves them.
#[test]
fn p2_03_cooperative_cancel_with_prior_rows_preserves_snapshot_byte_identical() {
    let (mut harness, _) = seed_two_files("p2-03-cancel-token");
    let (before_rows, before_marker, before_bytes) = committed_state(&harness);
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    let token = CancellationToken::new();
    token.cancel();
    let mut port = FakePort::new(tree(vec![file_entry("kept.flp", 101)]));
    let execution =
        harness
            .worker
            .execute(&mut harness.db, scan, &mut port, &token, &harness.clock);
    assert_eq!(execution.status, ScanExecutionStatus::Cancelled);
    assert_eq!(execution.enumeration_outcome, Some(EnumOutcome::Cancelled));
    assert!(!execution.authoritative);
    assert!(execution.publication.is_none());
    assert_committed_unchanged(&before_rows, &before_marker, &before_bytes, &harness);
    assert_eq!(
        harness.staging_state(&execution.run_id),
        ScanStageState::Discarded
    );
}

// P2-03: durable cancellation with prior rows preserves rows+snapshot.
#[test]
fn p2_03_durable_cancel_with_prior_rows_preserves_snapshot_byte_identical() {
    let (mut harness, _) = seed_two_files("p2-03-durable-cancel");
    let (before_rows, before_marker, before_bytes) = committed_state(&harness);
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    let run_id = scan.leased.run.id.clone();
    harness
        .db
        .request_scan_cancellation(&run_id, harness.clock.now_ms())
        .expect("durable cancellation");
    let mut port = FakePort::new(tree(vec![file_entry("kept.flp", 101)]));
    let execution = harness.worker.execute(
        &mut harness.db,
        scan,
        &mut port,
        &NeverCancelled,
        &harness.clock,
    );
    assert_eq!(execution.status, ScanExecutionStatus::Cancelled);
    assert!(execution.publication.is_none());
    assert_committed_unchanged(&before_rows, &before_marker, &before_bytes, &harness);
    assert_eq!(
        harness.staging_state(&execution.run_id),
        ScanStageState::Discarded
    );
}

// P2-03: enumeration resource limits (limited/ResourceLimit) preserve state.
#[test]
fn p2_03_resource_limit_preserves_rows_and_snapshot_byte_identical() {
    let mut config = WorkerConfig::default();
    config.enumeration_limits.max_observations = 2;
    let mut harness = Harness::with_config("p2-03-limited", config, r"C:\synthetic-root");
    let first = harness.scan(tree(many_files(2)));
    assert_eq!(first.status, ScanExecutionStatus::Published);
    let (before_rows, before_marker, before_bytes) = committed_state(&harness);
    assert_eq!(before_rows.len(), 2);

    let second = harness.scan(tree(many_files(4)));
    assert_eq!(second.status, ScanExecutionStatus::Failed);
    assert_eq!(second.enumeration_outcome, Some(EnumOutcome::ResourceLimit));
    assert!(!second.authoritative);
    assert!(second.publication.is_none());
    assert_eq!(
        harness
            .db
            .scan_run(&second.run_id)
            .expect("resource-limit run")
            .error_code
            .as_deref(),
        Some("resource_limit")
    );
    assert_eq!(
        harness
            .db
            .scan_job(&second.job_id)
            .expect("resource-limit job")
            .last_error_code
            .as_deref(),
        Some("resource_limit")
    );
    assert_committed_unchanged(&before_rows, &before_marker, &before_bytes, &harness);
    assert_eq!(
        harness.staging_state(&second.run_id),
        ScanStageState::Discarded
    );
}

// P2-03: a staging-level rejection (invalid observation) never marks the
// uncovered files missing and leaves the snapshot identical.
#[test]
fn p2_03_sink_failed_preserves_rows_and_snapshot_byte_identical() {
    let (mut harness, _) = seed_two_files("p2-03-sink-failed");
    let (before_rows, before_marker, before_bytes) = committed_state(&harness);
    // ':' is rejected by the staging boundary (`relative_path` shape guard),
    // so the enumerator stays non-authoritative and the worker discards.
    let bad_tree = tree(vec![
        file_entry("kept.flp", 101),
        file_entry("bad:name.flp", 102),
    ]);
    let mut port = FakePort::new(bad_tree);
    let execution = harness.scan_with_port(&mut port);
    assert!(!execution.authoritative);
    assert!(execution.publication.is_none());
    assert!(
        execution.enumeration_outcome == Some(EnumOutcome::SinkFailed)
            || execution.enumeration_outcome == Some(EnumOutcome::Invalid),
        "invalid input must stay non-authoritative, got {:?}",
        execution.enumeration_outcome
    );
    assert_committed_unchanged(&before_rows, &before_marker, &before_bytes, &harness);
}

// P2-06: crash before any staging batches leaves no partial Library rows.
#[test]
fn p2_06_crash_before_staging_leaves_no_partial_rows() {
    let mut harness = Harness::new("p2-06-crash-before");
    let (empty_rows, empty_marker, empty_bytes) = committed_state(&harness);
    assert!(empty_rows.is_empty());

    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let crashed = harness.claim();
    let crashed_run = crashed.leased.run.id.clone();
    drop(crashed); // crash after claim, before any batch is staged

    harness.restart();
    assert_eq!(harness.run(&crashed_run).state, ScanRunState::Interrupted);
    assert_eq!(
        harness.staging_state(&crashed_run),
        ScanStageState::Discarded
    );
    assert_committed_unchanged(&empty_rows, &empty_marker, &empty_bytes, &harness);

    // The requeued chain converges on the next attempt with no residue.
    harness.clock.advance(1_001);
    let recovered = harness
        .drain(tree(vec![file_entry("a.flp", 101)]))
        .expect("execution");
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    assert_eq!(harness.committed().len(), 1);
}

// P2-06: crash after staging but before apply rolls back with no partial rows.
#[test]
fn p2_06_crash_after_staging_before_apply_rolls_back() {
    let mut harness = Harness::new("p2-06-crash-staged");
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    let run_id = scan.leased.run.id.clone();
    let session_id = scan.leased.run.session_id.clone();
    let lease_token = scan.leased.run.lease_token.clone();
    // Simulate the worker having staged complete batches, then crashing
    // before the atomic publication transaction.
    harness
        .db
        .stage_scan_observations(
            &run_id,
            &session_id,
            &lease_token,
            harness.clock.now_ms(),
            &[
                fruitboard_storage::ScanObservation {
                    locator_key: "v1:i:staged.flp".to_owned(),
                    relative_path: "staged.flp".to_owned(),
                    byte_size: 10,
                    modified_at_ns: DEFAULT_MTIME_NS,
                    identity: None,
                },
                fruitboard_storage::ScanObservation {
                    locator_key: "v1:i:second.flp".to_owned(),
                    relative_path: "second.flp".to_owned(),
                    byte_size: 20,
                    modified_at_ns: DEFAULT_MTIME_NS,
                    identity: None,
                },
            ],
        )
        .expect("stage batches");
    drop(scan); // crash before publish_scan_run
    let (empty_rows, empty_marker, empty_bytes) = committed_state(&harness);
    assert!(
        empty_rows.is_empty(),
        "staging must stay invisible to Library reads"
    );

    harness.restart();
    assert_eq!(harness.staging_state(&run_id), ScanStageState::Discarded);
    assert_committed_unchanged(&empty_rows, &empty_marker, &empty_bytes, &harness);

    harness.clock.advance(1_001);
    let recovered = harness
        .drain(tree(vec![file_entry("a.flp", 101)]))
        .expect("execution");
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    // Only the recovery scan's observation is committed; the crashed staging
    // never leaked a partial row.
    let rows = harness.committed();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].relative_path, "a.flp");
}

// P2-06: a failure during the apply transaction rolls back visible rows and
// the ledger together; retry converges with no partial state.
#[test]
fn p2_06_crash_during_apply_rolls_back_rows_and_ledger() {
    let mut harness = Harness::new("p2-06-during-apply");
    let first = harness.scan(tree(vec![file_entry("old.flp", 101)]));
    assert_eq!(first.status, ScanExecutionStatus::Published);
    let (before_rows, before_marker, before_bytes) = committed_state(&harness);

    let trigger = Connection::open(harness.db_path()).expect("fixture connection");
    trigger
        .execute_batch(
            "CREATE TRIGGER fruitboard_test_p2_06_apply_abort
             BEFORE UPDATE OF last_successful_at_ms ON scan_root
             WHEN NEW.last_successful_at_ms IS NOT NULL
             BEGIN
                 SELECT RAISE(ABORT, 'injected_p2_06_apply_failure');
             END;",
        )
        .expect("arm injected apply failure");

    let failed = harness.scan(tree(vec![
        file_entry("old.flp", 101),
        file_entry("new.flp", 102),
    ]));
    assert!(failed.publication.is_none());
    assert!(failed.status != ScanExecutionStatus::Published);
    assert_committed_unchanged(&before_rows, &before_marker, &before_bytes, &harness);
    // The failed attempt holds no publishable staging for a later stale
    // publication.
    assert_eq!(
        harness.staging_state(&failed.run_id),
        ScanStageState::Discarded
    );

    trigger
        .execute_batch("DROP TRIGGER fruitboard_test_p2_06_apply_abort;")
        .expect("disarm injected failure");
    let job = harness.db.scan_job(&failed.job_id).expect("job");
    harness.clock.set(ScanWorker::retry_eligible_at(&job));
    assert_eq!(
        harness
            .worker
            .service_retries(&mut harness.db, &harness.clock)
            .expect("retries"),
        1
    );
    let recovered = harness
        .drain(tree(vec![
            file_entry("old.flp", 101),
            file_entry("new.flp", 102),
        ]))
        .expect("execution");
    assert_eq!(recovered.status, ScanExecutionStatus::Published);
    assert_eq!(harness.committed().len(), 2);
}

// P2-06: backup recovery preserves the committed dataset and discards open
// staging; the recovered database resumes without partial rows.
#[test]
fn p2_06_backup_recovery_preserves_committed_rows_and_discards_staging() {
    let mut harness = Harness::new("p2-06-backup");
    let first = harness.scan(tree(vec![
        file_entry("a.flp", 101),
        file_entry("b.flp", 102),
    ]));
    assert_eq!(first.status, ScanExecutionStatus::Published);
    let (before_rows, before_marker, before_bytes) = committed_state(&harness);

    // Leave one run with open staging, then back up: recovery must fence it.
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let inflight = harness.claim();
    let inflight_run = inflight.leased.run.id.clone();
    let backup = harness.db.create_backup().expect("backup");

    let recovery_dir = TestDir::new("p2-06-recovery");
    let recovered = Database::recover_to(&backup, &recovery_dir.0).expect("recover");
    let recovered_rows = committed_rows(&recovered, &harness.root_id);
    assert_eq!(
        recovered_rows, before_rows,
        "recovery preserves committed rows"
    );
    assert_eq!(
        recovered
            .scan_root_publication(&harness.root_id)
            .expect("marker"),
        before_marker
    );
    assert_eq!(
        committed_snapshot_bytes(&recovered_rows, &before_marker),
        before_bytes
    );
    assert_eq!(
        recovered
            .scan_staging(&inflight_run)
            .expect("staging")
            .state,
        ScanStageState::Discarded
    );
    assert_eq!(
        recovered.scan_run(&inflight_run).expect("run").state,
        ScanRunState::Interrupted
    );
    drop(inflight);
}

// P2-07: two file_location rows may share one (volume, file) identity under
// distinct paths; deleting one leaves the other present with identity intact.
#[test]
fn p2_07_hardlink_delete_one_preserves_other_with_identity_intact() {
    let mut harness = Harness::new("p2-07-hardlink");
    // Same qualified identity (500) at two distinct paths: hardlink aliases.
    // Locations stay per-path; the physical record is shared.
    let first = harness.scan(tree(vec![
        file_entry("a.flp", 500),
        file_entry("b.flp", 500),
    ]));
    assert_eq!(first.status, ScanExecutionStatus::Published);
    assert_eq!(
        first
            .publication
            .as_ref()
            .expect("publication")
            .location_count,
        2
    );
    let before = harness.committed();
    assert_eq!(before.len(), 2);
    let a = before
        .iter()
        .find(|row| row.relative_path == "a.flp")
        .unwrap();
    let b = before
        .iter()
        .find(|row| row.relative_path == "b.flp")
        .unwrap();
    assert!(a.present && b.present);
    assert_eq!(
        a.project_file_id, b.project_file_id,
        "aliases share one physical record"
    );
    assert_eq!(a.identity, b.identity);
    assert!(a.identity.is_some());

    // Delete one alias: only that path goes missing; the survivor stays
    // present with the same identity and physical record.
    let second = harness.scan(tree(vec![file_entry("b.flp", 500)]));
    assert_eq!(second.status, ScanExecutionStatus::Published);
    let after_delete = harness.committed();
    assert_eq!(
        after_delete.len(),
        2,
        "missing rows are retained, not deleted"
    );
    let missing = after_delete
        .iter()
        .find(|row| row.relative_path == "a.flp")
        .unwrap();
    let survivor = after_delete
        .iter()
        .find(|row| row.relative_path == "b.flp")
        .unwrap();
    assert!(
        !missing.present,
        "the deleted alias is missing, not removed"
    );
    assert!(survivor.present);
    assert_eq!(survivor.project_file_id, b.project_file_id);
    assert_eq!(survivor.identity, b.identity);

    // Restoring the deleted alias converges without duplicating the physical
    // record.
    let third = harness.scan(tree(vec![
        file_entry("a.flp", 500),
        file_entry("b.flp", 500),
    ]));
    assert_eq!(third.status, ScanExecutionStatus::Published);
    let restored = harness.committed();
    assert!(restored.iter().all(|row| row.present));
    assert!(
        restored
            .iter()
            .all(|row| row.project_file_id == b.project_file_id)
    );
}

// P2-07: rename evidence is conservative and replacements mint a fresh
// physical record; locations are never collapsed or grouped (no Phase-4
// grouping: every path stays its own file_location row).
#[test]
fn p2_07_rename_replacement_is_conservative_without_grouping() {
    let mut harness = Harness::new("p2-07-rename");
    let first = harness.scan(tree(vec![
        file_entry("old.flp", 600),
        file_entry("survivor.flp", 600),
        file_entry("replace.flp", 600),
    ]));
    assert_eq!(first.status, ScanExecutionStatus::Published);
    let before = harness.committed();
    assert_eq!(before.len(), 3);
    let shared = before[0].project_file_id.clone();
    assert!(before.iter().all(|row| row.project_file_id == shared));

    // Same scan: old.flp renamed to moved.flp (same identity 600),
    // survivor.flp unchanged (same identity), replace.flp overwritten with a
    // different identity (601) at the same path.
    let second = harness.scan(tree(vec![
        file_entry("moved.flp", 600),
        file_entry("survivor.flp", 600),
        file_entry("replace.flp", 601),
    ]));
    assert_eq!(second.status, ScanExecutionStatus::Published);
    let after = harness.committed();
    assert_eq!(after.len(), 4, "one missing history row plus three present");
    let old = after
        .iter()
        .find(|row| row.relative_path == "old.flp")
        .unwrap();
    let moved = after
        .iter()
        .find(|row| row.relative_path == "moved.flp")
        .unwrap();
    let survivor = after
        .iter()
        .find(|row| row.relative_path == "survivor.flp")
        .unwrap();
    let replaced = after
        .iter()
        .find(|row| row.relative_path == "replace.flp")
        .unwrap();
    assert!(!old.present, "the rename source is missing history");
    assert_eq!(old.project_file_id, shared);
    assert!(moved.present && survivor.present);
    assert_eq!(
        moved.project_file_id, shared,
        "rename target keeps the physical record"
    );
    assert_eq!(survivor.project_file_id, shared);
    assert!(replaced.present);
    assert_ne!(
        replaced.project_file_id, shared,
        "same-path replacement is conservative: fresh physical record"
    );
    // No grouping: every observed path is its own row.
    let present_paths: Vec<_> = after
        .iter()
        .filter(|row| row.present)
        .map(|row| row.relative_path.as_str())
        .collect();
    assert_eq!(present_paths.len(), 3);
    assert!(present_paths.contains(&"moved.flp"));
    assert!(present_paths.contains(&"survivor.flp"));
    assert!(present_paths.contains(&"replace.flp"));
}

// P2-05: disabling mid-queue/running invalidates leases+staging in one
// transaction; the stale run can never publish afterwards.
#[test]
fn p2_05_disable_invalidates_lease_and_staging_in_same_txn_without_stale_publish() {
    let (mut harness, _) = seed_two_files("p2-05-disable");
    let (before_rows, before_marker, before_bytes) = committed_state(&harness);

    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    let stale_run = scan.leased.run.id.clone();
    let stale_token = scan.leased.run.lease_token.clone();
    let stale_session = scan.leased.run.session_id.clone();

    // Disable is one configuration transaction: staging discarded, queued and
    // running work cancelled, generation+revision bumped for lease fencing.
    harness
        .db
        .set_scan_root_enabled_at(&harness.root_id, false, harness.clock.now_ms())
        .expect("disable root");
    assert_eq!(harness.staging_state(&stale_run), ScanStageState::Discarded);
    assert_eq!(harness.run(&stale_run).state, ScanRunState::Cancelled);
    // The stale lease is fenced everywhere in the same state.
    assert!(
        harness
            .db
            .renew_scan_lease(
                &stale_run,
                &stale_session,
                &stale_token,
                harness.clock.now_ms(),
                30_000,
            )
            .is_err()
    );
    assert!(
        harness
            .db
            .publish_scan_run(
                &stale_run,
                &stale_session,
                &stale_token,
                harness.clock.now_ms(),
            )
            .is_err()
    );

    let mut port = FakePort::new(tree(vec![file_entry("kept.flp", 101)]));
    let execution = harness.worker.execute(
        &mut harness.db,
        scan,
        &mut port,
        &NeverCancelled,
        &harness.clock,
    );
    assert_eq!(execution.status, ScanExecutionStatus::Cancelled);
    assert!(execution.publication.is_none());
    assert_committed_unchanged(&before_rows, &before_marker, &before_bytes, &harness);

    // Re-enabling runs a fresh generation: no stale publication leaks through
    // and the next authoritative scan converges.
    harness
        .db
        .set_scan_root_enabled_at(&harness.root_id, true, harness.clock.now_ms())
        .expect("re-enable root");
    assert!(
        harness
            .db
            .publish_scan_run(
                &stale_run,
                &stale_session,
                &stale_token,
                harness.clock.now_ms(),
            )
            .is_err()
    );
    let fresh = harness.scan(tree(vec![
        file_entry("kept.flp", 101),
        file_entry("other.flp", 102),
    ]));
    assert_eq!(fresh.status, ScanExecutionStatus::Published);
    assert_eq!(harness.committed(), before_rows);
}

// P2-05: removing mid-queue/running detaches history; re-adding the same path
// gets a fresh root ID with no stale publication and unchanged source markers.
#[test]
fn p2_05_remove_then_readd_gets_fresh_id_without_stale_publication() {
    let mut harness = Harness::new("p2-05-remove");
    let first = harness.scan(tree(vec![file_entry("a.flp", 101)]));
    assert_eq!(first.status, ScanExecutionStatus::Published);
    let before = harness.committed();
    assert_eq!(before.len(), 1);
    let marker_sizes: Vec<u64> = before.iter().map(|row| row.byte_size).collect();
    let old_root = harness.root_id.clone();
    let old_path = harness
        .db
        .list_scan_roots()
        .expect("roots")
        .into_iter()
        .find(|root| root.id == old_root)
        .expect("root")
        .canonical_path;

    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let scan = harness.claim();
    let stale_run = scan.leased.run.id.clone();
    harness
        .db
        .remove_scan_root_at(&harness.root_id, harness.clock.now_ms())
        .expect("remove root");
    assert!(harness.db.list_scan_roots().expect("roots").is_empty());
    assert_eq!(harness.run(&stale_run).state, ScanRunState::Cancelled);
    assert!(harness.db.scan_staging(&stale_run).is_err());

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

    // Re-adding the same canonical path mints a fresh root ID; history does
    // not leak into the new root and source markers are unchanged.
    let replacement = harness
        .db
        .add_scan_root("Synthetic", &old_path)
        .expect("re-add");
    assert_ne!(replacement.id, old_root);
    harness.root_id = replacement.id.clone();
    assert!(
        harness.committed().is_empty(),
        "a re-added root starts with no stale Library rows"
    );
    let fresh = harness.scan(tree(vec![file_entry("a.flp", 101)]));
    assert_eq!(fresh.status, ScanExecutionStatus::Published);
    let after = harness.committed();
    assert_eq!(after.len(), 1);
    assert_eq!(
        after.iter().map(|row| row.byte_size).collect::<Vec<_>>(),
        marker_sizes,
        "source markers (byte sizes) are unchanged by remove/re-add"
    );
    assert!(after[0].present);
}

// P2-05: queued work is invalidated by disable and never leased afterwards;
// re-enable schedules fresh work with a new generation.
#[test]
fn p2_05_queued_work_invalidated_by_disable_never_leased_after_reenable() {
    let mut harness = Harness::new("p2-05-queued");
    harness
        .worker
        .request_manual_scan(&mut harness.db, &harness.root_id, &harness.clock)
        .expect("enqueue");
    let queued_id = harness.root_jobs()[0].id.clone();
    harness
        .db
        .set_scan_root_enabled_at(&harness.root_id, false, harness.clock.now_ms())
        .expect("disable root");
    assert_eq!(
        harness.db.scan_job(&queued_id).expect("job").state,
        ScanJobState::Cancelled
    );
    assert!(
        harness
            .worker
            .claim(&mut harness.db, &harness.session_id, &harness.clock)
            .expect("claim")
            .is_none(),
        "disabled roots lease nothing"
    );
    harness
        .db
        .set_scan_root_enabled_at(&harness.root_id, true, harness.clock.now_ms())
        .expect("re-enable root");
    let fresh = harness.scan(tree(vec![file_entry("a.flp", 101)]));
    assert_eq!(fresh.status, ScanExecutionStatus::Published);
    assert_ne!(fresh.job_id, queued_id);
    assert_eq!(harness.committed().len(), 1);
}
