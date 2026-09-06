use super::*;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Default)]
struct RecordingSink {
    batches: Vec<ObservationBatch>,
    error: Option<SinkError>,
    discarded: bool,
}

impl BatchSink for RecordingSink {
    fn accept(&mut self, batch: ObservationBatch) -> Result<(), SinkError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        self.batches.push(batch);
        Ok(())
    }

    fn discard(&mut self) {
        self.batches.clear();
        self.discarded = true;
    }
}

#[derive(Default)]
struct RecordingRunStorage {
    staged: BTreeMap<String, Vec<ObservationBatch>>,
    invalidated: Vec<String>,
    error: Option<SinkError>,
}

impl RunScopedStorage for RecordingRunStorage {
    fn stage_batch(&mut self, run_id: &str, batch: ObservationBatch) -> Result<(), SinkError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        self.staged
            .entry(run_id.to_owned())
            .or_default()
            .push(batch);
        Ok(())
    }

    fn invalidate_run(&mut self, run_id: &str) {
        self.staged.remove(run_id);
        self.invalidated.push(run_id.to_owned());
    }
}

#[derive(Default)]
struct RecordingProgress {
    updates: Vec<ProgressUpdate>,
}

impl ProgressSink for RecordingProgress {
    fn report(&mut self, update: ProgressUpdate) {
        self.updates.push(update);
    }
}

struct FakeCursor {
    state: Rc<RefCell<FakeState>>,
    key: String,
    links: Vec<FakeLink>,
    entries: VecDeque<Result<DirectoryEntry, PortError>>,
}

#[derive(Clone)]
struct FakeLink {
    key: String,
    generation: u64,
}

impl DirectoryCursor for FakeCursor {
    fn next_entry(&mut self) -> Result<Option<DirectoryEntry>, PortError> {
        {
            let mut state = self.state.borrow_mut();
            if state.replace_before_next_entry.remove(&self.key) {
                replace_with_junction(&mut state, &self.key);
            }
        }
        self.ensure_current()?;
        self.entries.pop_front().transpose()
    }

    fn read_metadata(&mut self, entry: &DirectoryEntry) -> Result<FileMetadata, PortError> {
        self.ensure_current()?;
        let key = child_key(&self.key, &entry.name);
        let state = self.state.borrow_mut();
        if let Some((cancel_key, token)) = &state.cancel_on_metadata
            && cancel_key == &key
        {
            token.cancel();
        }
        state
            .metadata
            .get(&key)
            .cloned()
            .unwrap_or(Err(PortError::NotFound))
    }

    fn open_directory(&mut self, entry: &DirectoryEntry) -> Result<OpenedDirectory, PortError> {
        self.ensure_current()?;
        let key = child_key(&self.key, &entry.name);
        let mut state = self.state.borrow_mut();
        if state.replace_before_open.remove(&key) {
            replace_with_junction(&mut state, &key);
            return Err(PortError::ReparsePoint);
        }
        let metadata = state
            .metadata
            .get(&key)
            .cloned()
            .unwrap_or(Err(PortError::NotFound))?;
        if metadata.reparse_point || state.reparse.contains(&key) {
            return Err(PortError::ReparsePoint);
        }
        if metadata.kind != EntryKind::Directory {
            return Err(PortError::Other);
        }
        let entries = state
            .directories
            .get(&key)
            .cloned()
            .ok_or(PortError::NotFound)?;
        state.opened.push(key.clone());
        let generation = state.generations.get(&key).copied().unwrap_or(0);
        let mut links = self.links.clone();
        links.push(FakeLink {
            key: key.clone(),
            generation,
        });
        let case_sensitivity = state
            .case_sensitivity
            .get(&key)
            .copied()
            .unwrap_or_default();
        let cursor = FakeCursor {
            state: Rc::clone(&self.state),
            key,
            links,
            entries: entries.into_iter().collect(),
        };
        Ok(OpenedDirectory {
            metadata,
            case_sensitivity,
            cursor: Box::new(cursor),
        })
    }
}

impl FakeCursor {
    fn ensure_current(&self) -> Result<(), PortError> {
        let state = self.state.borrow();
        if self.links.iter().any(|link| {
            state.reparse.contains(&link.key)
                || state.generations.get(&link.key).copied().unwrap_or(0) != link.generation
        }) {
            Err(PortError::Changed)
        } else {
            Ok(())
        }
    }
}

struct FakePort {
    state: Rc<RefCell<FakeState>>,
}

struct FakeState {
    root_results: VecDeque<Result<RootMetadata, PortError>>,
    directories: BTreeMap<String, Vec<Result<DirectoryEntry, PortError>>>,
    metadata: BTreeMap<String, Result<FileMetadata, PortError>>,
    case_sensitivity: BTreeMap<String, DirectoryCaseSensitivity>,
    generations: BTreeMap<String, u64>,
    reparse: BTreeSet<String>,
    replace_before_open: BTreeSet<String>,
    replace_before_next_entry: BTreeSet<String>,
    replace_root_before_open: bool,
    opened: Vec<String>,
    outside_accesses: usize,
    cancel_on_open: Option<CancellationToken>,
    cancel_on_metadata: Option<(String, CancellationToken)>,
}

impl FakePort {
    fn new(_root: &Path) -> Self {
        let root_metadata = RootMetadata {
            metadata: directory_metadata(900),
            qualification: FilesystemQualification::LocalNtfs,
        };
        Self {
            state: Rc::new(RefCell::new(FakeState {
                root_results: VecDeque::from([Ok(root_metadata)]),
                directories: BTreeMap::new(),
                metadata: BTreeMap::new(),
                case_sensitivity: BTreeMap::new(),
                generations: BTreeMap::new(),
                reparse: BTreeSet::new(),
                replace_before_open: BTreeSet::new(),
                replace_before_next_entry: BTreeSet::new(),
                replace_root_before_open: false,
                opened: Vec::new(),
                outside_accesses: 0,
                cancel_on_open: None,
                cancel_on_metadata: None,
            })),
        }
    }

    fn root_metadata(&self) -> RootMetadata {
        RootMetadata {
            metadata: directory_metadata(900),
            qualification: FilesystemQualification::LocalNtfs,
        }
    }

    fn add_directory(&mut self, relative: &str, entries: Vec<Result<DirectoryEntry, PortError>>) {
        self.state
            .borrow_mut()
            .directories
            .insert(relative.to_owned(), entries);
    }

    fn add_file(&mut self, relative: &str, metadata: Result<FileMetadata, PortError>) {
        self.state
            .borrow_mut()
            .metadata
            .insert(relative.to_owned(), metadata);
    }

    fn add_root_result(&mut self, result: Result<RootMetadata, PortError>) {
        self.state.borrow_mut().root_results.push_back(result);
    }

    fn set_root_results(&mut self, results: Vec<Result<RootMetadata, PortError>>) {
        self.state.borrow_mut().root_results = results.into_iter().collect();
    }

    fn deny_directory(&mut self, relative: &str) {
        self.state
            .borrow_mut()
            .directories
            .insert(relative.to_owned(), vec![Err(PortError::AccessDenied)]);
    }

    fn cancel_on_open(&mut self, token: CancellationToken) {
        self.state.borrow_mut().cancel_on_open = Some(token);
    }

    fn replace_before_open(&mut self, relative: &str) {
        self.state
            .borrow_mut()
            .replace_before_open
            .insert(relative.to_owned());
    }

    fn replace_before_next_entry(&mut self, relative: &str) {
        self.state
            .borrow_mut()
            .replace_before_next_entry
            .insert(relative.to_owned());
    }

    fn replace_root_before_open(&mut self) {
        self.state.borrow_mut().replace_root_before_open = true;
    }

    fn set_case_sensitivity(&mut self, relative: &str, sensitivity: DirectoryCaseSensitivity) {
        self.state
            .borrow_mut()
            .case_sensitivity
            .insert(relative.to_owned(), sensitivity);
    }

    fn outside_accesses(&self) -> usize {
        self.state.borrow().outside_accesses
    }

    fn opened(&self) -> Vec<String> {
        self.state.borrow().opened.clone()
    }
}

impl FilesystemPort for FakePort {
    fn inspect_root(&mut self, _root: &Path) -> Result<RootMetadata, PortError> {
        self.state
            .borrow_mut()
            .root_results
            .pop_front()
            .unwrap_or_else(|| Ok(self.root_metadata()))
    }

    fn open_root(&mut self, _root: &Path) -> Result<OpenedDirectory, PortError> {
        let mut state = self.state.borrow_mut();
        if state.replace_root_before_open {
            state.replace_root_before_open = false;
            replace_with_junction(&mut state, ".");
            return Err(PortError::ReparsePoint);
        }
        if let Some(token) = &state.cancel_on_open {
            token.cancel();
        }
        if state.reparse.contains(".") {
            return Err(PortError::ReparsePoint);
        }
        let entries = state
            .directories
            .get(".")
            .cloned()
            .ok_or(PortError::NotFound)?;
        let key = ".".to_owned();
        let generation = state.generations.get(&key).copied().unwrap_or(0);
        let case_sensitivity = state
            .case_sensitivity
            .get(&key)
            .copied()
            .unwrap_or_default();
        let metadata = self.root_metadata().metadata;
        let cursor = FakeCursor {
            state: Rc::clone(&self.state),
            key: key.clone(),
            links: vec![FakeLink { key, generation }],
            entries: entries.into_iter().collect(),
        };
        Ok(OpenedDirectory {
            metadata,
            case_sensitivity,
            cursor: Box::new(cursor),
        })
    }
}

fn child_key(parent: &str, name: &std::ffi::OsStr) -> String {
    let name = name.to_string_lossy();
    if parent == "." {
        name.into_owned()
    } else {
        format!("{parent}/{name}")
    }
}

fn replace_with_junction(state: &mut FakeState, key: &str) {
    let generation = state.generations.entry(key.to_owned()).or_default();
    *generation = generation.saturating_add(1);
    state.reparse.insert(key.to_owned());
}

fn identity(value: u128) -> QualifiedIdentity {
    QualifiedIdentity {
        volume_serial: 7,
        file_id: value,
        qualification: IdentityQualification::LocalNtfs,
    }
}

fn directory_metadata(file_id: u128) -> FileMetadata {
    FileMetadata {
        kind: EntryKind::Directory,
        byte_size: 0,
        modified_unix_ns: 100,
        identity: Some(identity(file_id)),
        reparse_point: false,
        recall_or_offline: false,
    }
}

fn file_metadata(file_id: u128, byte_size: u64) -> FileMetadata {
    FileMetadata {
        kind: EntryKind::File,
        byte_size,
        modified_unix_ns: 100,
        identity: Some(identity(file_id)),
        reparse_point: false,
        recall_or_offline: false,
    }
}

fn reparse_directory_metadata() -> FileMetadata {
    FileMetadata {
        kind: EntryKind::Directory,
        byte_size: 0,
        modified_unix_ns: 100,
        identity: None,
        reparse_point: true,
        recall_or_offline: false,
    }
}

fn limits() -> EnumerationLimits {
    EnumerationLimits {
        progress_interval: std::time::Duration::ZERO,
        ..EnumerationLimits::default()
    }
}

fn root_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(r"C:\synthetic-root")
    } else {
        PathBuf::from("/synthetic-root")
    }
}

fn file_entry(name: &str) -> Result<DirectoryEntry, PortError> {
    Ok(DirectoryEntry::new(name))
}

fn simple_port() -> (FakePort, PathBuf) {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(
        ".",
        vec![file_entry("Mixed.FLP"), file_entry("ignored.txt")],
    );
    port.add_file("Mixed.FLP", Ok(file_metadata(1, 42)));
    port.add_file("ignored.txt", Ok(file_metadata(2, 9)));
    (port, root)
}

#[test]
fn successful_empty_root_is_explicitly_authoritative() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", Vec::new());
    let mut sink = RecordingSink::default();
    let mut progress = RecordingProgress::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut progress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    assert!(report.authoritative);
    assert_eq!(report.observations_discovered, 0);
    assert_eq!(report.batches_delivered, 0);
    assert!(sink.batches.is_empty());
    assert!(
        progress
            .updates
            .last()
            .is_some_and(|update| update.final_update && update.outcome == Some(Outcome::Complete))
    );
}

#[test]
fn extension_filter_is_case_insensitive_and_metadata_stays_separate_from_content() {
    let (mut port, root) = simple_port();
    let mut sink = RecordingSink::default();
    let mut progress = NoProgress;
    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut progress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    let observations = sink
        .batches
        .iter()
        .flat_map(|batch| batch.records.iter())
        .collect::<Vec<_>>();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].display_path.as_str(), "Mixed.FLP");
    assert_eq!(observations[0].byte_size, 42);
}

#[test]
fn batches_are_bounded_and_keep_sequence_order() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    let mut entries = Vec::new();
    for index in 0..5 {
        let name = format!("file-{index}.flp");
        entries.push(file_entry(&name));
        port.add_file(&name, Ok(file_metadata(index + 1, index as u64)));
    }
    port.add_directory(".", entries);
    let mut configured = limits();
    configured.max_batch_records = 2;
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &configured,
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    assert_eq!(report.observations_discovered, 5);
    assert_eq!(report.batches_delivered, 3);
    assert_eq!(
        sink.batches
            .iter()
            .map(|batch| batch.records.len())
            .collect::<Vec<_>>(),
        vec![2, 2, 1]
    );
    assert_eq!(
        sink.batches
            .iter()
            .map(|batch| batch.sequence)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
}

#[test]
fn resource_limit_is_non_authoritative_and_never_implies_missing() {
    let (mut port, root) = simple_port();
    let mut configured = limits();
    configured.max_entries = 1;
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &configured,
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::ResourceLimit);
    assert!(!report.authoritative);
    assert!(report.batches_delivered <= 1);
    assert!(sink.discarded);
}

#[test]
fn pending_directory_work_is_bounded() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("one"), file_entry("two")]);
    port.add_file(
        "one",
        Ok(FileMetadata {
            kind: EntryKind::Directory,
            ..directory_metadata(1)
        }),
    );
    port.add_file(
        "two",
        Ok(FileMetadata {
            kind: EntryKind::Directory,
            ..directory_metadata(2)
        }),
    );
    port.add_directory("one", Vec::new());
    port.add_directory("two", Vec::new());
    let mut configured = limits();
    configured.max_pending_directories = 1;
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &configured,
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::ResourceLimit);
    assert!(!report.authoritative);
}

#[test]
fn denied_directory_is_a_coverage_failure_not_a_policy_exclusion() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("denied")]);
    port.add_file(
        "denied",
        Ok(FileMetadata {
            kind: EntryKind::Directory,
            ..directory_metadata(2)
        }),
    );
    port.deny_directory("denied");
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Denied);
    assert!(!report.authoritative);
    assert!(report.exclusions.is_empty());
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::DirectoryDenied)
    );
}

#[test]
fn disappearing_file_is_partial_and_does_not_authorize_absence() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("vanished.flp")]);
    port.add_file("vanished.flp", Err(PortError::NotFound));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Partial);
    assert!(!report.authoritative);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::EntryDisappeared)
    );
}

#[test]
fn cursor_failure_is_isolated_but_completion_is_not_authoritative() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("one.flp"), Err(PortError::Other)]);
    port.add_file("one.flp", Ok(file_metadata(1, 1)));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Partial);
    assert!(!report.authoritative);
    assert_eq!(report.observations_discovered, 1);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::DirectoryRead)
    );
}

#[test]
fn cancellation_is_cooperative_and_drops_the_pending_batch() {
    let (mut port, root) = simple_port();
    let token = CancellationToken::new();
    port.cancel_on_open(token.clone());
    let mut sink = RecordingSink::default();
    let mut progress = RecordingProgress::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &token,
        &mut sink,
        &mut progress,
    );

    assert_eq!(report.outcome, Outcome::Cancelled);
    assert!(!report.authoritative);
    assert!(sink.batches.is_empty());
    assert!(sink.discarded);
    assert!(progress
        .updates
        .last()
        .is_some_and(|update| update.final_update && update.outcome == Some(Outcome::Cancelled)));
}

#[test]
fn reparse_points_are_excluded_without_being_traversed() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("alias")]);
    port.add_file("alias", Ok(reparse_directory_metadata()));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    assert!(report.authoritative);
    assert!(
        report
            .exclusions
            .iter()
            .any(|exclusion| exclusion.reason == ExclusionReason::ReparsePoint)
    );
    assert!(report.coverage_failures.is_empty());
}

#[test]
fn hardlink_like_aliases_keep_two_observations_with_one_identity() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("one.flp"), file_entry("two.FLP")]);
    port.add_file("one.flp", Ok(file_metadata(99, 12)));
    port.add_file("two.FLP", Ok(file_metadata(99, 12)));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    let observations = sink
        .batches
        .iter()
        .flat_map(|batch| batch.records.iter())
        .collect::<Vec<_>>();
    assert_eq!(observations.len(), 2);
    assert_eq!(observations[0].identity, observations[1].identity);
    assert_ne!(observations[0].locator, observations[1].locator);
}

#[test]
fn case_only_duplicate_locators_invalidate_the_run() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("Same.flp"), file_entry("same.FLP")]);
    port.add_file("Same.flp", Ok(file_metadata(1, 1)));
    port.add_file("same.FLP", Ok(file_metadata(2, 1)));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Invalid);
    assert!(!report.authoritative);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::DuplicateLocator)
    );
}

#[test]
fn root_loss_after_traversal_is_not_complete() {
    let (mut port, root) = simple_port();
    port.add_root_result(Err(PortError::NotFound));
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::RootUnavailable);
    assert!(!report.authoritative);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::RootNotFound)
    );
}

#[test]
fn unsupported_filesystem_is_explicitly_non_authoritative() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.set_root_results(vec![Ok(RootMetadata {
        metadata: directory_metadata(900),
        qualification: FilesystemQualification::Unqualified,
    })]);
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::UnsupportedFilesystem);
    assert!(!report.authoritative);
}

#[test]
fn sink_failure_is_non_authoritative() {
    let (mut port, root) = simple_port();
    let mut sink = RecordingSink {
        batches: Vec::new(),
        error: Some(SinkError::Unavailable),
        discarded: false,
    };

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::SinkFailed);
    assert!(!report.authoritative);
    assert!(sink.discarded);
}

#[test]
fn run_scoped_storage_invalidates_provisional_batches_on_failure() {
    let (mut port, root) = simple_port();
    let mut configured = limits();
    configured.max_entries = 1;
    configured.max_batch_records = 1;
    let mut storage = RecordingRunStorage::default();

    let report = enumerate_into_run(
        &mut port,
        &root,
        &configured,
        &NeverCancelled,
        &mut storage,
        "run-resource-limit",
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::ResourceLimit);
    assert!(!report.authoritative);
    assert!(!storage.staged.contains_key("run-resource-limit"));
    assert_eq!(storage.invalidated, vec!["run-resource-limit"]);
}

#[test]
fn run_scoped_storage_retains_a_successful_empty_root_for_publication() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", Vec::new());
    let mut storage = RecordingRunStorage::default();

    let report = enumerate_into_run(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut storage,
        "run-empty",
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    assert!(report.authoritative);
    assert!(storage.invalidated.is_empty());
    assert!(!storage.staged.contains_key("run-empty"));
}

#[test]
fn case_sensitive_directory_keeps_case_distinct_serialized_locators() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("case-tree")]);
    port.add_file("case-tree", Ok(directory_metadata(2)));
    port.add_directory("case-tree", vec![file_entry("A.flp"), file_entry("a.flp")]);
    port.add_file("case-tree/A.flp", Ok(file_metadata(3, 1)));
    port.add_file("case-tree/a.flp", Ok(file_metadata(4, 2)));
    port.set_case_sensitivity("case-tree", DirectoryCaseSensitivity::Sensitive);
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Complete);
    assert!(report.authoritative);
    let observations = sink
        .batches
        .iter()
        .flat_map(|batch| batch.records.iter())
        .collect::<Vec<_>>();
    let separator = if cfg!(windows) { "\\" } else { "/" };
    assert_eq!(
        observations
            .iter()
            .map(|observation| observation.locator.as_str().to_owned())
            .collect::<Vec<_>>(),
        vec![
            format!("case-tree{separator}A.flp"),
            format!("case-tree{separator}a.flp")
        ]
    );
}

#[test]
fn windows_timestamp_and_identity_conversions_keep_boundary_precision() {
    assert_eq!(
        windows_filetime_100ns_to_unix_ns(WINDOWS_EPOCH_OFFSET_100NS as i64),
        0
    );
    assert_eq!(
        windows_filetime_100ns_to_unix_ns(WINDOWS_EPOCH_OFFSET_100NS as i64 - 1),
        -100
    );
    assert_eq!(
        windows_filetime_100ns_to_unix_ns(WINDOWS_EPOCH_OFFSET_100NS as i64 + 1),
        100
    );
    assert_eq!(
        windows_filetime_100ns_to_unix_ns(i64::MIN),
        (i64::MIN as i128 - WINDOWS_EPOCH_OFFSET_100NS) * 100
    );
    assert_eq!(
        windows_filetime_100ns_to_unix_ns(i64::MAX),
        (i64::MAX as i128 - WINDOWS_EPOCH_OFFSET_100NS) * 100
    );

    assert_eq!(windows_file_id_to_u128([0; 16]), 0);
    assert_eq!(
        windows_file_id_to_u128([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        1
    );
    assert_eq!(
        windows_file_id_to_u128([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
        1u128 << 120
    );
    assert_eq!(windows_file_id_to_u128([u8::MAX; 16]), u128::MAX);
}

#[test]
fn root_replacement_between_inspection_and_open_is_refused_before_traversal() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("outside.flp")]);
    port.add_file("outside.flp", Ok(file_metadata(4, 1)));
    port.replace_root_before_open();
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::RootUnavailable);
    assert!(!report.authoritative);
    assert_eq!(report.observations_discovered, 0);
    assert!(sink.batches.is_empty());
    assert!(port.opened().is_empty());
    assert_eq!(port.outside_accesses(), 0);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::RootChanged)
    );
}

#[test]
fn child_replacement_between_metadata_and_open_is_refused_before_cursor_escape() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("ancestor")]);
    port.add_file("ancestor", Ok(directory_metadata(2)));
    port.add_directory("ancestor", vec![file_entry("outside.flp")]);
    port.add_file("ancestor/outside.flp", Ok(file_metadata(4, 1)));
    port.replace_before_open("ancestor");
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Partial);
    assert!(!report.authoritative);
    assert_eq!(report.observations_discovered, 0);
    assert!(port.opened().is_empty());
    assert_eq!(port.outside_accesses(), 0);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::DirectoryChanged)
    );
}

#[test]
fn ancestor_replacement_invalidates_bound_cursor_before_next_entry() {
    let root = root_path();
    let mut port = FakePort::new(&root);
    port.add_directory(".", vec![file_entry("ancestor")]);
    port.add_file("ancestor", Ok(directory_metadata(2)));
    port.add_directory("ancestor", vec![file_entry("outside.flp")]);
    port.add_file("ancestor/outside.flp", Ok(file_metadata(4, 1)));
    port.replace_before_next_entry("ancestor");
    let mut sink = RecordingSink::default();

    let report = enumerate(
        &mut port,
        &root,
        &limits(),
        &NeverCancelled,
        &mut sink,
        &mut NoProgress,
    );

    assert_eq!(report.outcome, Outcome::Partial);
    assert!(!report.authoritative);
    assert_eq!(report.observations_discovered, 0);
    assert_eq!(port.opened(), vec!["ancestor"]);
    assert_eq!(port.outside_accesses(), 0);
    assert!(
        report
            .coverage_failures
            .iter()
            .any(|failure| failure.kind == CoverageFailureKind::DirectoryChanged)
    );
}

#[test]
fn path_normalization_rejects_absolute_parent_and_ads_components() {
    let absolute = if cfg!(windows) {
        Path::new(r"C:\absolute\file.flp")
    } else {
        Path::new("/absolute/file.flp")
    };
    assert_eq!(normalize_relative_path(absolute), Err(PathError::Absolute));
    assert_eq!(
        normalize_relative_path(Path::new("folder/../file.flp")),
        Err(PathError::ParentTraversal)
    );
    assert_eq!(
        normalize_relative_path(Path::new("folder:ads.flp")),
        Err(PathError::AlternateDataStream)
    );
}

#[cfg(windows)]
mod windows_fixtures {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        root: PathBuf,
        marker_path: PathBuf,
        marker: Vec<u8>,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let tick = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before epoch")
                .as_nanos();
            let ordinal = NEXT.fetch_add(1, AtomicOrdering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "fruitboard-enumeration-{label}-{}-{tick}-{ordinal}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("create disposable fixture root");
            let marker_path = root.join("source-marker.FLP");
            let marker = vec![0, 1, 2, 0xF1, 0x00, 0xFF, 0x7E, 0x42];
            fs::write(&marker_path, &marker).expect("write disposable marker");
            Self {
                root,
                marker_path,
                marker,
            }
        }

        fn long_io_path(path: &Path) -> PathBuf {
            let text = path.to_string_lossy();
            if text.starts_with(r"\\?\") {
                return path.to_owned();
            }
            if let Some(unc) = text.strip_prefix(r"\\") {
                return PathBuf::from(format!(r"\\?\UNC\{unc}"));
            }
            PathBuf::from(format!(r"\\?\{text}"))
        }

        fn create_long_unicode_file(&self) -> PathBuf {
            let mut directory = self.root.join("ユニコード");
            for index in 0..18 {
                directory.push(format!("segment-{index:02}-long-name"));
            }
            let file = directory.join("long-project-name-with-spaces.FlP");
            fs::create_dir_all(Self::long_io_path(&directory)).expect("create long fixture path");
            fs::write(Self::long_io_path(&file), &self.marker).expect("write long FLP marker");
            file
        }

        fn create_junction(&self) -> PathBuf {
            let target = self.root.join("real-target");
            let junction = self.root.join("reparse-alias");
            fs::create_dir_all(&target).expect("create junction target");
            fs::write(target.join("outside-looking.FLP"), &self.marker)
                .expect("write junction target marker");
            let result = Command::new("cmd.exe")
                .args([
                    "/C",
                    "mklink",
                    "/J",
                    &junction.to_string_lossy(),
                    &target.to_string_lossy(),
                ])
                .output()
                .expect("launch mklink for disposable junction");
            assert!(
                result.status.success(),
                "junction fixture unavailable: {}",
                String::from_utf8_lossy(&result.stderr)
            );
            junction
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn ntfs_fixture_discovers_unicode_long_mixed_case_and_preserves_markers() {
        let fixture = Fixture::new("metadata");
        let long_file = fixture.create_long_unicode_file();
        fs::write(fixture.root.join("mixed-case.fLp"), &fixture.marker)
            .expect("write mixed-case marker");
        fs::write(fixture.root.join("not-an-flp.txt"), &fixture.marker)
            .expect("write filter marker");
        let before = fs::read(&fixture.marker_path).expect("read marker before scan");
        let mut port = WindowsFilesystemPort::new();
        let mut sink = RecordingSink::default();
        let report = enumerate(
            &mut port,
            &fixture.root,
            &EnumerationLimits {
                progress_interval: std::time::Duration::ZERO,
                ..EnumerationLimits::default()
            },
            &NeverCancelled,
            &mut sink,
            &mut NoProgress,
        );
        let after = fs::read(&fixture.marker_path).expect("read marker after scan");

        assert_eq!(report.outcome, Outcome::Complete);
        assert!(report.authoritative);
        assert_eq!(before, after);
        let observations = sink
            .batches
            .iter()
            .flat_map(|batch| batch.records.iter())
            .collect::<Vec<_>>();
        assert!(
            observations
                .iter()
                .any(|observation| observation.display_path.as_str() == "mixed-case.fLp")
        );
        assert!(observations.iter().any(|observation| {
            observation
                .display_path
                .as_str()
                .ends_with("long-project-name-with-spaces.FlP")
        }));
        assert!(!observations.iter().any(|observation| {
            observation
                .display_path
                .as_str()
                .ends_with("not-an-flp.txt")
        }));
        assert!(observations.iter().any(|observation| {
            observation.display_path.as_str()
                == long_file
                    .strip_prefix(&fixture.root)
                    .expect("long file under fixture")
                    .to_string_lossy()
                    .replace('/', "\\")
        }));
    }

    #[test]
    fn ntfs_fixture_keeps_hardlink_aliases_per_path_and_excludes_junctions() {
        let fixture = Fixture::new("aliases");
        let alias_source = fixture.root.join("alias-source.FLP");
        let alias = fixture.root.join("alias-second.flp");
        fs::hard_link(&fixture.marker_path, &alias_source).expect("create hardlink source");
        fs::hard_link(&alias_source, &alias).expect("create hardlink alias");
        let junction = fixture.create_junction();
        let mut port = WindowsFilesystemPort::new();
        let mut sink = RecordingSink::default();
        let report = enumerate(
            &mut port,
            &fixture.root,
            &EnumerationLimits {
                progress_interval: std::time::Duration::ZERO,
                ..EnumerationLimits::default()
            },
            &NeverCancelled,
            &mut sink,
            &mut NoProgress,
        );

        assert_eq!(report.outcome, Outcome::Complete);
        assert!(report.authoritative);
        let observations = sink
            .batches
            .iter()
            .flat_map(|batch| batch.records.iter())
            .collect::<Vec<_>>();
        let aliases = observations
            .iter()
            .filter(|observation| {
                observation.display_path.as_str() == "alias-source.FLP"
                    || observation.display_path.as_str() == "alias-second.flp"
            })
            .collect::<Vec<_>>();
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases[0].identity, aliases[1].identity);
        let junction_relative = junction
            .strip_prefix(&fixture.root)
            .expect("junction under fixture")
            .to_string_lossy()
            .replace('/', "\\");
        assert!(report.exclusions.iter().any(|exclusion| {
            exclusion.display_path.as_str() == junction_relative
                && exclusion.reason == ExclusionReason::ReparsePoint
        }));
        assert!(!observations.iter().any(|observation| {
            observation
                .display_path
                .as_str()
                .starts_with(&format!("{junction_relative}\\"))
        }));
    }

    #[test]
    #[ignore = "DriveFS/streamed placeholders are unavailable in this environment; #47 remains open"]
    fn drivefs_placeholder_fixture_requires_a_qualified_host() {}

    #[test]
    #[ignore = "ACL-denied fixture requires a disposable Windows security-policy setup"]
    fn acl_denied_fixture_requires_an_explicit_windows_acl_environment() {}
}
