use super::*;
use crate::{MonotonicClock, Outcome, WorkerGate, check};
use std::fs::{self, OpenOptions};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

struct Clock;
impl MonotonicClock for Clock {
    fn now_ms(&self) -> u64 {
        0
    }
}
#[derive(Default)]
struct Fence {
    revoked: Arc<AtomicBool>,
}
impl CurrentAuthorization for Fence {
    fn current(
        &mut self,
        _context: &Context,
        _fence: &AuthorityFence,
    ) -> Result<(), RequestFailure> {
        if self.revoked.load(Ordering::SeqCst) {
            Err(RequestFailure::Stale)
        } else {
            Ok(())
        }
    }
}

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    base: PathBuf,
    root: PathBuf,
    source: PathBuf,
    links: Arc<Mutex<Vec<PathBuf>>>,
}
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "fruitboard-sample-metadata-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        assert!(!base.exists());
        fs::create_dir(&base).unwrap();
        // Test setup owns this new directory. Resolve Windows' abbreviated
        // TEMP spelling, then remove the verbatim drive prefix returned by
        // canonicalize. Production authority never canonicalizes saved paths.
        let canonical = fs::canonicalize(&base).unwrap();
        let full = canonical.to_str().unwrap();
        let base = PathBuf::from(full.strip_prefix(r"\\?\").unwrap_or(full));
        AbsolutePath::parse(base.to_str().unwrap()).expect("ordinary owned fixture root");
        let root = base.join("projects");
        fs::create_dir(&root).unwrap();
        let source = root.join("project.flp");
        fs::write(
            &source,
            b"private constructed source; not an FL Studio corpus fixture",
        )
        .unwrap();
        Self {
            base,
            root,
            source,
            links: Arc::new(Mutex::new(Vec::new())),
        }
    }
    fn sample(&self, name: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, b"metadata-only target sentinel").unwrap();
        path
    }
    fn nest_source(&mut self, name: &str) -> PathBuf {
        let parent = self.root.join(name);
        fs::create_dir_all(&parent).unwrap();
        let source = parent.join("project.flp");
        fs::rename(&self.source, &source).unwrap();
        self.source = source;
        parent
    }
    fn input(&self, references: &[Option<&Path>]) -> CapturedInput {
        let values = references
            .iter()
            .map(|value| value.map(|path| path.to_str().unwrap()))
            .collect::<Vec<_>>();
        self.input_values(&values)
    }
    fn input_values(&self, references: &[Option<&str>]) -> CapturedInput {
        let gate = WorkerGate::default();
        let lease = gate.try_start(Arc::new(Clock)).unwrap();
        let operations = Operations::new(lease.control());
        let open = |path: &Path, directory: bool, operations: &Operations| {
            let file = OpenOptions::new()
                .access_mode(0x80 | 0x100000)
                .share_mode(7)
                .custom_flags(0x200000 | if directory { 0x2000000 } else { 0 })
                .open(path)
                .unwrap();
            ffi::independent_test_handle(file, operations.acquire_handle().unwrap())
        };
        let root = open(&self.root, true, &operations);
        let source = open(&self.source, false, &operations);
        let root_id = ffi::identity(&root).unwrap();
        let source_id = ffi::identity(&source).unwrap();
        // Independent std observations, rather than the adapter's conversion,
        // register the expected size/high-resolution modification timestamp.
        let recorded = fs::metadata(&self.source).unwrap();
        let modified_ns = i64::try_from(
            recorded
                .modified()
                .unwrap()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
        .unwrap();
        CapturedInput::new(
            Context::new("req", "root", "location", "snapshot", "session").unwrap(),
            self.root.to_str().unwrap(),
            self.source.to_str().unwrap(),
            AuthorityFence {
                root_revision: 1,
                location_revision: 1,
                root_identity: Some(root_id),
                source: SourceFingerprint {
                    identity: source_id,
                    byte_size: recorded.len(),
                    modified_at_ns: modified_ns,
                },
            },
            Some(
                references
                    .iter()
                    .map(|value| value.map(str::to_owned))
                    .collect(),
            ),
        )
        .unwrap()
    }
    fn junction(&self, link: &Path, target: &Path) {
        assert!(link.starts_with(&self.base) && target.starts_with(&self.base));
        self.links.lock().unwrap().push(link.to_owned());
        ffi::junction(link, target);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Remove registered junctions as links, never traversing them. Inventory
        // only this uniquely owned tree and refuse unexpected reparses first.
        for link in self.links.lock().unwrap().iter() {
            if link.exists() {
                fs::remove_dir(link).unwrap();
            }
        }
        fn inventory(root: &Path, entries: &mut Vec<(PathBuf, bool)>) {
            let metadata = fs::symlink_metadata(root).unwrap();
            assert_eq!(
                metadata.file_attributes() & 0x400,
                0,
                "unexpected fixture reparse; cleanup refused"
            );
            if metadata.is_dir() {
                for entry in fs::read_dir(root).unwrap() {
                    inventory(&entry.unwrap().path(), entries);
                }
            }
            entries.push((root.to_owned(), metadata.is_dir()));
        }
        assert!(
            self.base.is_absolute()
                && self
                    .base
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("fruitboard-sample-metadata-")
        );
        let mut entries = Vec::new();
        inventory(&self.base, &mut entries);
        for (path, directory) in entries {
            assert!(path.starts_with(&self.base));
            if directory {
                fs::remove_dir(path).unwrap();
            } else {
                fs::remove_file(path).unwrap();
            }
        }
    }
}

fn run(
    input: &CapturedInput,
    port: &mut WindowsPort<Fence>,
) -> Result<crate::Report, RequestFailure> {
    let gate = WorkerGate::default();
    let mut lease = gate.try_start(Arc::new(Clock)).unwrap();
    check(input, &mut lease, port, 1_791_216_000_000)
}
fn outcomes(report: &crate::Report) -> Vec<Outcome> {
    report.channels().iter().map(|v| v.outcome).collect()
}
fn unchecked(reason: UncheckedReason) -> Outcome {
    Outcome::NotChecked { reason }
}

#[test]
fn root_identity_captured_on_qualification_is_pinned_through_final_revalidation() {
    let fixture = Fixture::new();
    let sample = fixture.sample("capture.wav");
    let original = fixture.input(&[Some(&sample)]);
    let mut fence = original.fence().clone();
    fence.root_identity = None;
    let input = CapturedInput::new(
        original.context().clone(),
        fixture.root.to_str().unwrap(),
        fixture.source.to_str().unwrap(),
        fence,
        Some(vec![Some(sample.to_str().unwrap().to_owned())]),
    )
    .unwrap();
    let root = fixture.root.clone();
    let moved = fixture.base.join("replaced-root");
    let attempted = Arc::new(AtomicBool::new(false));
    let observed = attempted.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeQuery && name == "capture.wav" {
            assert!(
                fs::rename(&root, &moved).is_err(),
                "qualified root namespace was not pinned"
            );
            observed.store(true, Ordering::SeqCst);
        }
    }));
    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        [Outcome::Present]
    );
    assert!(attempted.load(Ordering::SeqCst));
}

#[test]
fn ordinary_present_absent_intermediate_directory_and_duplicate_slots_preserve_bytes() {
    let fixture = Fixture::new();
    let sample = fixture.sample("Kick.wav");
    let missing = fixture.root.join("missing.wav");
    let absent_parent = fixture.root.join("absent").join("x.wav");
    let source_before = fs::read(&fixture.source).unwrap();
    let sample_before = fs::read(&sample).unwrap();
    let input = fixture.input(&[
        Some(&sample),
        Some(&missing),
        Some(&absent_parent),
        None,
        Some(&sample),
        Some(&fixture.root),
    ]);
    let report = run(&input, &mut WindowsPort::new(Fence::default())).unwrap();
    assert_eq!(
        outcomes(&report),
        vec![
            Outcome::Present,
            Outcome::NotFound,
            Outcome::NotFound,
            Outcome::NoSavedReference,
            Outcome::Present,
            unchecked(UncheckedReason::NotRegularFile)
        ]
    );
    assert_eq!(fs::read(&fixture.source).unwrap(), source_before);
    assert_eq!(fs::read(&sample).unwrap(), sample_before);
}

#[test]
fn source_handles_cannot_read_contents_and_allow_ordinary_writers() {
    let fixture = Fixture::new();
    let input = fixture.input(&[]);
    let gate = WorkerGate::default();
    let lease = gate.try_start(Arc::new(Clock)).unwrap();
    let mut operations = Operations::new(lease.control());
    let mut port = WindowsPort::new(Fence::default());
    let qualified = port.qualify(&input, &mut operations).unwrap();
    assert!(ffi::content_read_is_denied(&qualified.authority.source));
    fixture.sample("rights.wav");
    let (sample, _) = port
        .open_child(
            &qualified.root_directory.node,
            "rights.wav",
            Some(false),
            &mut operations,
        )
        .unwrap()
        .unwrap();
    assert!(ffi::content_read_is_denied(&sample));
    let writer = OpenOptions::new()
        .write(true)
        .open(&fixture.source)
        .unwrap();
    drop(writer);
    assert!(
        port.revalidate(&input, &qualified.authority, &mut operations)
            .is_ok()
    );
}

#[test]
fn root_replaced_by_a_junction_during_qualification_cannot_grant_target_access() {
    let fixture = Fixture::new();
    let sample = fixture.root.join("x.wav");
    let input = fixture.input(&[Some(&sample)]);
    let root = fixture.root.clone();
    let old = fixture.base.join("old-projects");
    let outside = fixture.base.join("outside");
    fs::create_dir(&outside).unwrap();
    let links = fixture.links.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeOpen && name == "projects" {
            fs::rename(&root, &old).unwrap();
            links.lock().unwrap().push(root.clone());
            ffi::junction(&root, &outside);
        }
    }));
    assert!(matches!(
        run(&input, &mut port),
        Err(RequestFailure::Unavailable(
            UncheckedReason::ReparseOrOffline
        ))
    ));
}

#[test]
fn native_cancellation_between_query_and_open_prevents_the_next_syscall() {
    let fixture = Fixture::new();
    let sample = fixture.sample("cancel.wav");
    let input = fixture.input(&[Some(&sample)]);
    let gate = WorkerGate::default();
    let mut lease = gate.try_start(Arc::new(Clock)).unwrap();
    let control = lease.control();
    let opens = Arc::new(Mutex::new(0));
    let counter = opens.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if name == "cancel.wav" {
            if stage == TestStage::BeforeOpen {
                control.cancel();
            }
            if stage == TestStage::AfterOpen {
                *counter.lock().unwrap() += 1;
            }
        }
    }));
    assert!(matches!(
        check(&input, &mut lease, &mut port, 0),
        Err(RequestFailure::Cancelled)
    ));
    assert_eq!(*opens.lock().unwrap(), 0);
    assert!(matches!(
        gate.try_start(Arc::new(Clock)),
        Err(crate::AdmissionError::Busy)
    ));
}

#[test]
fn maximum_source_depth_and_separate_candidate_chain_fit_reserved_fence_and_handles() {
    let mut fixture = Fixture::new();
    let root_depth = AbsolutePath::parse(fixture.root.to_str().unwrap())
        .unwrap()
        .components()
        .len();
    assert!(root_depth < 62);
    let mut source_parent = fixture.root.clone();
    let mut sample_parent = fixture.root.clone();
    for _ in root_depth..63 {
        source_parent = source_parent.join("s");
        sample_parent = sample_parent.join("t");
        fs::create_dir(&source_parent).unwrap();
        fs::create_dir(&sample_parent).unwrap();
    }
    let source = source_parent.join("project.flp");
    fs::rename(&fixture.source, &source).unwrap();
    fixture.source = source;
    let sample = sample_parent.join("x.wav");
    fs::write(&sample, b"deep target").unwrap();
    let input = fixture.input(&[Some(&sample)]);
    assert_eq!(input.source().components().len(), 64);
    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        vec![Outcome::Present]
    );
}

#[test]
fn held_ancestor_names_cannot_be_replaced_while_descendant_writes_remain_allowed() {
    let fixture = Fixture::new();
    let input = fixture.input(&[]);
    let gate = WorkerGate::default();
    let lease = gate.try_start(Arc::new(Clock)).unwrap();
    let mut operations = Operations::new(lease.control());
    let qualified = WindowsPort::new(Fence::default())
        .qualify(&input, &mut operations)
        .unwrap();
    assert!(fs::rename(&fixture.root, fixture.base.join("replacement")).is_err());
    fs::write(fixture.root.join("ordinary-new-file"), b"ordinary save").unwrap();
    assert!(ffi::content_read_is_denied(&qualified.authority.source));
}

#[test]
fn same_name_source_replacement_is_stale_even_while_the_old_handle_survives() {
    let fixture = Fixture::new();
    let sample = fixture.sample("replace-trigger.wav");
    let input = fixture.input(&[Some(&sample)]);
    let source = fixture.source.clone();
    let old = fixture.root.join("old-source.flp");
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeQuery && name == "replace-trigger.wav" {
            fs::rename(&source, &old).unwrap();
            fs::write(&source, b"replacement has a different object identity").unwrap();
        }
    }));
    assert!(matches!(run(&input, &mut port), Err(RequestFailure::Stale)));
}

#[test]
fn normal_in_place_project_save_succeeds_and_invalidates_the_old_fingerprint() {
    let fixture = Fixture::new();
    let sample = fixture.sample("save-trigger.wav");
    let input = fixture.input(&[Some(&sample)]);
    let source = fixture.source.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeQuery && name == "save-trigger.wav" {
            fs::write(&source, b"ordinary shorter project save").unwrap();
        }
    }));
    assert!(matches!(run(&input, &mut port), Err(RequestFailure::Stale)));
}

#[test]
fn existing_junction_is_rejected_before_open_without_probing_its_target() {
    let fixture = Fixture::new();
    let outside = fixture.base.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("x.wav"), b"outside sentinel").unwrap();
    let link = fixture.root.join("jump");
    fixture.junction(&link, &outside);
    let candidate = link.join("x.wav");
    let input = fixture.input(&[Some(&candidate)]);
    let opens = Arc::new(Mutex::new(0));
    let counter = opens.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeOpen && (name == "jump" || name == "x.wav") {
            *counter.lock().unwrap() += 1;
        }
    }));
    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        vec![unchecked(UncheckedReason::ReparseOrOffline)]
    );
    assert_eq!(*opens.lock().unwrap(), 0);
    assert_eq!(
        fs::read(outside.join("x.wav")).unwrap(),
        b"outside sentinel"
    );
}

#[test]
fn junction_inserted_between_attributes_and_open_is_never_followed() {
    let fixture = Fixture::new();
    let directory = fixture.root.join("race");
    fs::create_dir(&directory).unwrap();
    let outside = fixture.base.join("outside");
    fs::create_dir(&outside).unwrap();
    let candidate = directory.join("x.wav");
    let input = fixture.input(&[Some(&candidate)]);
    let old = fixture.root.join("old-race");
    let links = fixture.links.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeOpen && name == "race" {
            fs::rename(&directory, &old).unwrap();
            links.lock().unwrap().push(directory.clone());
            ffi::junction(&directory, &outside);
        }
    }));
    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        vec![unchecked(UncheckedReason::ReparseOrOffline)]
    );
}

#[test]
fn a_data_share_lock_does_not_block_metadata_only_observation() {
    let fixture = Fixture::new();
    let sample = fixture.sample("locked.wav");
    let input = fixture.input(&[Some(&sample)]);
    let _lock = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&sample)
        .unwrap();
    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        vec![Outcome::Present]
    );
}

#[test]
fn metadata_permission_denial_is_unchecked_instead_of_false_absence() {
    let fixture = Fixture::new();
    let sample = fixture.sample("denied.wav");
    let input = fixture.input(&[Some(&sample)]);
    let _restore = ffi::deny_read_attributes(&sample);
    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        vec![unchecked(UncheckedReason::AccessDenied)]
    );
}

#[test]
fn injected_offline_recall_and_cloud_attributes_stop_before_candidate_open() {
    for bit in [0x400, 0x1000, 0x40000, 0x400000] {
        let fixture = Fixture::new();
        let sample = fixture.sample("excluded.wav");
        let input = fixture.input(&[Some(&sample)]);
        let opens = Arc::new(Mutex::new(0));
        let counter = opens.clone();
        let mut port = WindowsPort::new(Fence::default());
        port.attributes_filter = Some(Box::new(move |name, basic| {
            if name == "excluded.wav" {
                basic.attributes |= bit;
            }
        }));
        port.hook = Some(Box::new(move |stage, name| {
            if stage == TestStage::BeforeOpen && name == "excluded.wav" {
                *counter.lock().unwrap() += 1;
            }
        }));
        assert_eq!(
            outcomes(&run(&input, &mut port).unwrap()),
            vec![unchecked(UncheckedReason::ReparseOrOffline)]
        );
        assert_eq!(*opens.lock().unwrap(), 0);
    }
}

#[test]
fn native_leaf_case_lookup_uses_the_directory_mode_and_root_variants_remain_unproven() {
    let fixture = Fixture::new();
    fixture.sample("Kick.wav");
    let lower = fixture.root.join("kick.wav");
    let alternate_root = fixture.base.join("PROJECTS").join("Kick.wav");
    let input = fixture.input(&[Some(&lower), Some(&alternate_root)]);
    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        vec![
            Outcome::Present,
            unchecked(UncheckedReason::UnsupportedCaseMode)
        ]
    );
}

#[test]
fn outside_sibling_and_network_references_never_produce_candidate_queries() {
    let fixture = Fixture::new();
    let sibling = fixture.base.join("projects2").join("outside.wav");
    let outside = fixture.base.join("outside.wav");
    let network = Path::new(r"\\never-contact.invalid\share\outside.wav");
    let input = fixture.input(&[Some(&sibling), Some(&outside), Some(network)]);
    let queries = Arc::new(Mutex::new(0));
    let counter = queries.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeQuery && name == "outside.wav" {
            *counter.lock().unwrap() += 1;
        }
    }));
    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        vec![
            unchecked(UncheckedReason::OutsideRoot),
            unchecked(UncheckedReason::OutsideRoot),
            unchecked(UncheckedReason::UnsupportedPathSyntax)
        ]
    );
    assert_eq!(*queries.lock().unwrap(), 0);
}

#[test]
fn source_identity_mismatch_fails_before_any_sample_query() {
    let fixture = Fixture::new();
    let sample = fixture.sample("x.wav");
    let input = fixture.input(&[Some(&sample)]);
    let mut changed = input.fence().clone();
    changed.source.identity.file += 1;
    let input = CapturedInput::new(
        input.context().clone(),
        fixture.root.to_str().unwrap(),
        fixture.source.to_str().unwrap(),
        changed,
        Some(vec![Some(sample.to_str().unwrap().into())]),
    )
    .unwrap();
    assert!(matches!(
        run(&input, &mut WindowsPort::new(Fence::default())),
        Err(RequestFailure::Stale)
    ));
}

#[test]
fn durable_revocation_during_a_probe_discards_results_and_stops_the_batch() {
    let fixture = Fixture::new();
    let sample = fixture.sample("revoke.wav");
    let later = fixture.sample("later.wav");
    let input = fixture.input(&[Some(&sample), Some(&later)]);
    let fence = Fence::default();
    let revoked = fence.revoked.clone();
    let later_queries = Arc::new(Mutex::new(0));
    let counter = later_queries.clone();
    let mut port = WindowsPort::new(fence);
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::AfterOpen && name == "revoke.wav" {
            revoked.store(true, Ordering::SeqCst);
        }
        if stage == TestStage::BeforeQuery && name == "later.wav" {
            *counter.lock().unwrap() += 1;
        }
    }));
    assert!(matches!(run(&input, &mut port), Err(RequestFailure::Stale)));
    assert_eq!(*later_queries.lock().unwrap(), 0);
}

#[test]
fn case_sensitive_directories_keep_distinct_literal_leaf_names() {
    let fixture = Fixture::new();
    let folder = fixture.root.join("sensitive");
    fs::create_dir(&folder).unwrap();
    let handle = OpenOptions::new()
        .access_mode(0x100)
        .custom_flags(0x2000000)
        .open(&folder)
        .unwrap();
    assert!(
        ffi::set_case_sensitive(&handle),
        "owned NTFS case-mode fixture setup failed"
    );
    drop(handle);
    fs::write(folder.join("Kick.wav"), b"upper").unwrap();
    let upper = folder.join("Kick.wav");
    let lower = folder.join("kick.wav");
    let input = fixture.input(&[Some(&upper), Some(&lower)]);
    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        vec![Outcome::Present, Outcome::NotFound]
    );
}

#[test]
fn wrong_size_or_high_resolution_timestamp_rejects_source_qualification() {
    let fixture = Fixture::new();
    let input = fixture.input(&[]);
    for change_size in [true, false] {
        let mut fence = input.fence().clone();
        if change_size {
            fence.source.byte_size += 1;
        } else {
            fence.source.modified_at_ns += 100;
        }
        let changed = CapturedInput::new(
            input.context().clone(),
            fixture.root.to_str().unwrap(),
            fixture.source.to_str().unwrap(),
            fence,
            Some(vec![]),
        )
        .unwrap();
        assert!(matches!(
            run(&changed, &mut WindowsPort::new(Fence::default())),
            Err(RequestFailure::Stale)
        ));
    }
}

#[test]
fn deliberately_preserved_identity_size_and_timestamp_is_not_a_content_freshness_claim() {
    let fixture = Fixture::new();
    let sample = fixture.sample("preserved.wav");
    let input = fixture.input(&[Some(&sample)]);
    let source = fixture.source.clone();
    let modified = fs::metadata(&source).unwrap().modified().unwrap();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeQuery && name == "preserved.wav" {
            let mut bytes = fs::read(&source).unwrap();
            bytes[0] ^= 1;
            fs::write(&source, bytes).unwrap();
            let file = OpenOptions::new().access_mode(0x100).open(&source).unwrap();
            file.set_times(fs::FileTimes::new().set_modified(modified))
                .unwrap();
        }
    }));
    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        vec![Outcome::Present]
    );
}

#[test]
fn relative_candidates_start_at_the_qualified_source_parent_and_allow_an_in_root_sibling() {
    let mut fixture = Fixture::new();
    let project_parent = fixture.nest_source("Albums");
    let local_samples = project_parent.join("Samples");
    fs::create_dir(&local_samples).unwrap();
    let local_sample = local_samples.join("Kick.wav");
    fs::write(&local_sample, b"source-parent sentinel").unwrap();
    let root_samples = fixture.root.join("Samples");
    fs::create_dir(&root_samples).unwrap();
    let root_sentinel = root_samples.join("Kick.wav");
    fs::create_dir(&root_sentinel).unwrap();
    let root_sentinel_bytes = root_sentinel.join("contents.bin");
    fs::write(&root_sentinel_bytes, b"selected-root directory sentinel").unwrap();
    let sibling = fixture.root.join("Kick.wav");
    fs::write(&sibling, b"parent-relative sibling sentinel").unwrap();
    let source_before = fs::read(&fixture.source).unwrap();
    let local_before = fs::read(&local_sample).unwrap();
    let root_before = fs::read(&root_sentinel_bytes).unwrap();
    let sibling_before = fs::read(&sibling).unwrap();
    let input = fixture.input_values(&[Some("Samples\\Kick.wav"), Some("..\\Kick.wav")]);
    let queries = Arc::new(Mutex::new(Vec::new()));
    let recorded = queries.clone();
    let mut source_qualified = false;
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::AfterOpen && name == "project.flp" && !source_qualified {
            source_qualified = true;
        } else if stage == TestStage::BeforeQuery && source_qualified {
            recorded.lock().unwrap().push(name.to_owned());
        }
    }));

    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        [Outcome::Present, Outcome::Present]
    );
    assert_eq!(
        *queries.lock().unwrap(),
        [
            "Albums",
            "Samples",
            "Kick.wav",
            "Kick.wav",
            "projects",
            "project.flp"
        ]
    );
    assert_eq!(fs::read(&fixture.source).unwrap(), source_before);
    assert_eq!(fs::read(&local_sample).unwrap(), local_before);
    assert_eq!(fs::read(&root_sentinel_bytes).unwrap(), root_before);
    assert_eq!(fs::read(&sibling).unwrap(), sibling_before);
}

#[test]
fn cancelled_missing_relative_components_are_not_queried_but_survivors_are() {
    let mut fixture = Fixture::new();
    let project_parent = fixture.nest_source("Albums");
    let survivor = project_parent.join("Kick.wav");
    fs::write(&survivor, b"surviving candidate sentinel").unwrap();
    let input = fixture.input_values(&[Some("Missing\\..\\Kick.wav")]);
    let queries = Arc::new(Mutex::new(Vec::new()));
    let recorded = queries.clone();
    let mut source_qualified = false;
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::AfterOpen && name == "project.flp" && !source_qualified {
            source_qualified = true;
        } else if stage == TestStage::BeforeQuery && source_qualified {
            recorded.lock().unwrap().push(name.to_owned());
        }
    }));

    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        [Outcome::Present]
    );
    assert_eq!(
        *queries.lock().unwrap(),
        ["Albums", "Kick.wav", "projects", "project.flp"]
    );
}

#[test]
fn relative_root_equality_escape_and_cancelled_invalid_components_do_no_candidate_queries() {
    let mut fixture = Fixture::new();
    fixture.nest_source("Albums");
    let input = fixture.input_values(&[
        Some(".."),
        Some("..\\..\\escape-sentinel.wav"),
        Some("bad*name\\..\\Kick.wav"),
        Some("Missing\\$CACHE\\..\\Kick.wav"),
    ]);
    let queries = Arc::new(Mutex::new(Vec::new()));
    let recorded = queries.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeQuery {
            recorded.lock().unwrap().push(name.to_owned());
        }
    }));

    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        [
            unchecked(UncheckedReason::NotRegularFile),
            unchecked(UncheckedReason::OutsideRoot),
            unchecked(UncheckedReason::UnsupportedPathSyntax),
            unchecked(UncheckedReason::UnresolvedPlaceholder),
        ]
    );
    // Qualification and the final source/root fence still run. Neither the
    // root-equal nor escaped/invalid candidate opens any child component.
    assert_eq!(
        *queries.lock().unwrap(),
        [
            "projects",
            "Albums",
            "project.flp",
            "projects",
            "project.flp"
        ]
    );
}

#[test]
fn relative_case_and_unicode_lookup_preserves_ntfs_directory_semantics() {
    let mut fixture = Fixture::new();
    let project_parent = fixture.nest_source("Albums");
    let insensitive = project_parent.join("Insensitive");
    fs::create_dir(&insensitive).unwrap();
    fs::write(insensitive.join("Kick.wav"), b"insensitive case sentinel").unwrap();
    let sensitive = project_parent.join("Sensitive");
    fs::create_dir(&sensitive).unwrap();
    let handle = OpenOptions::new()
        .access_mode(0x100)
        .custom_flags(0x2000000)
        .open(&sensitive)
        .unwrap();
    assert!(
        ffi::set_case_sensitive(&handle),
        "owned NTFS case-mode fixture setup failed"
    );
    drop(handle);
    fs::write(sensitive.join("Kick.wav"), b"sensitive case sentinel").unwrap();
    let unicode = project_parent.join("Unicode");
    fs::create_dir(&unicode).unwrap();
    fs::write(unicode.join("éŸ音.wav"), b"unicode spelling sentinel").unwrap();
    fs::write(unicode.join("café.wav"), b"composed Unicode sentinel").unwrap();
    let input = fixture.input_values(&[
        Some("Insensitive\\kick.wav"),
        Some("Sensitive\\Kick.wav"),
        Some("Sensitive\\kick.wav"),
        Some("Unicode\\éŸ音.wav"),
        Some("Unicode\\cafe\u{301}.wav"),
    ]);

    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        [
            Outcome::Present,
            Outcome::Present,
            Outcome::NotFound,
            Outcome::Present,
            Outcome::NotFound,
        ]
    );
}

#[test]
fn relative_authoritative_absence_is_distinct_from_metadata_denial() {
    let mut fixture = Fixture::new();
    let project_parent = fixture.nest_source("Albums");
    let samples = project_parent.join("Samples");
    fs::create_dir(&samples).unwrap();
    let denied = samples.join("denied.wav");
    fs::write(&denied, b"denied candidate sentinel").unwrap();
    let input = fixture.input_values(&[
        Some("Samples\\authoritatively-absent.wav"),
        Some("Samples\\denied.wav"),
    ]);
    let _restore = ffi::deny_read_attributes(&denied);

    assert_eq!(
        outcomes(&run(&input, &mut WindowsPort::new(Fence::default())).unwrap()),
        [Outcome::NotFound, unchecked(UncheckedReason::AccessDenied)]
    );
}

#[test]
fn injected_relative_offline_and_recall_attributes_stop_before_candidate_open() {
    for bit in [0x400, 0x1000, 0x40000, 0x400000] {
        let mut fixture = Fixture::new();
        let project_parent = fixture.nest_source("Albums");
        let samples = project_parent.join("Samples");
        fs::create_dir(&samples).unwrap();
        fs::write(samples.join("excluded.wav"), b"excluded candidate sentinel").unwrap();
        let input = fixture.input_values(&[Some("Samples\\excluded.wav")]);
        let opens = Arc::new(Mutex::new(0));
        let counter = opens.clone();
        let mut port = WindowsPort::new(Fence::default());
        port.attributes_filter = Some(Box::new(move |name, basic| {
            if name == "excluded.wav" {
                basic.attributes |= bit;
            }
        }));
        port.hook = Some(Box::new(move |stage, name| {
            if stage == TestStage::BeforeOpen && name == "excluded.wav" {
                *counter.lock().unwrap() += 1;
            }
        }));

        assert_eq!(
            outcomes(&run(&input, &mut port).unwrap()),
            [unchecked(UncheckedReason::ReparseOrOffline)]
        );
        assert_eq!(*opens.lock().unwrap(), 0);
    }
}

#[test]
fn relative_reparse_replacement_race_never_opens_the_outside_target() {
    let mut fixture = Fixture::new();
    let project_parent = fixture.nest_source("Albums");
    let directory = project_parent.join("race");
    fs::create_dir(&directory).unwrap();
    let outside = fixture.base.join("outside");
    fs::create_dir(&outside).unwrap();
    let outside_target = outside.join("x.wav");
    fs::write(&outside_target, b"outside sentinel").unwrap();
    let source_before = fs::read(&fixture.source).unwrap();
    let target_before = fs::read(&outside_target).unwrap();
    let input = fixture.input_values(&[Some("race\\x.wav")]);
    let old = project_parent.join("old-race");
    let links = fixture.links.clone();
    let mut port = WindowsPort::new(Fence::default());
    port.hook = Some(Box::new(move |stage, name| {
        if stage == TestStage::BeforeOpen && name == "race" {
            fs::rename(&directory, &old).unwrap();
            links.lock().unwrap().push(directory.clone());
            ffi::junction(&directory, &outside);
        }
    }));

    assert_eq!(
        outcomes(&run(&input, &mut port).unwrap()),
        [unchecked(UncheckedReason::ReparseOrOffline)]
    );
    assert_eq!(fs::read(&fixture.source).unwrap(), source_before);
    assert_eq!(fs::read(&outside_target).unwrap(), target_before);
}

#[test]
fn relative_candidate_probe_is_discarded_after_source_save_replacement_or_revocation() {
    for change in 0..3 {
        let mut fixture = Fixture::new();
        let project_parent = fixture.nest_source("Albums");
        let samples = project_parent.join("Samples");
        fs::create_dir(&samples).unwrap();
        let target = samples.join("trigger.wav");
        fs::write(&target, b"relative target sentinel").unwrap();
        let target_before = fs::read(&target).unwrap();
        let source_before = fs::read(&fixture.source).unwrap();
        let source = fixture.source.clone();
        let old_source = project_parent.join("old-project.flp");
        let input = fixture.input_values(&[Some("Samples\\trigger.wav")]);
        let fence = Fence::default();
        let revoked = fence.revoked.clone();
        let probed = Arc::new(AtomicBool::new(false));
        let observed = probed.clone();
        let mut port = WindowsPort::new(fence);
        port.hook = Some(Box::new(move |stage, name| {
            if stage == TestStage::AfterOpen && name == "trigger.wav" {
                observed.store(true, Ordering::SeqCst);
                match change {
                    0 => fs::write(&source, b"ordinary relative-project save").unwrap(),
                    1 => {
                        fs::rename(&source, &old_source).unwrap();
                        fs::write(&source, b"replacement relative-project object").unwrap();
                    }
                    _ => revoked.store(true, Ordering::SeqCst),
                }
            }
        }));

        assert!(matches!(run(&input, &mut port), Err(RequestFailure::Stale)));
        assert!(probed.load(Ordering::SeqCst));
        assert_eq!(fs::read(&target).unwrap(), target_before);
        if change == 2 {
            assert_eq!(fs::read(&fixture.source).unwrap(), source_before);
        }
    }
}
