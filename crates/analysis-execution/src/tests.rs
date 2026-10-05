use super::*;
#[cfg(windows)]
use fruitboard_flp_parser::supervisor::SupervisorLimits;
use fruitboard_flp_parser::{ADAPTER_ID, ADAPTER_VERSION, parse_bytes, sha256_hex};
use fruitboard_storage::{EncodedIdentity, ScanKind, ScanObservation, ScanRoot};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("fruitboard-analysis-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Clock(AtomicI64);
impl AnalysisClock for Clock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }
}
fn bytes() -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp"),
    )
    .unwrap()
}
fn observation() -> SourceObservation {
    SourceObservation {
        byte_size: bytes().len() as u64,
        modified_at_ns: 42_000_123,
        identity: Some((7, 3)),
        sha256: sha256_hex(&bytes()),
    }
}

fn seed(
    directory: &Path,
    root_path: &str,
    observed: &SourceObservation,
) -> (Arc<Mutex<Database>>, ScanRoot, String, String) {
    let mut db = Database::open(directory).unwrap();
    let root = db.add_scan_root("Synthetic", root_path).unwrap();
    let scan_session = db.start_scan_session(1).unwrap();
    db.enqueue_scan(&root.id, ScanKind::Manual, 2).unwrap();
    let scan = db
        .lease_next_scan(&scan_session.id, 3, 1000)
        .unwrap()
        .unwrap();
    let observed = ScanObservation {
        locator_key: "v1:i:sample.flp".into(),
        relative_path: "sample.flp".into(),
        byte_size: observed.byte_size,
        modified_at_ns: i128::from(observed.modified_at_ns),
        identity: observed.identity.map(|(volume, file)| EncodedIdentity {
            volume_serial: volume.to_string(),
            file_id: file.to_string(),
        }),
    };
    db.stage_scan_observations(
        &scan.run.id,
        &scan_session.id,
        &scan.run.lease_token,
        4,
        &[observed],
    )
    .unwrap();
    db.publish_scan_run(&scan.run.id, &scan_session.id, &scan.run.lease_token, 5)
        .unwrap();
    let location = db
        .query_library(&fruitboard_storage::LibraryQuery {
            scan_root_id: root.id.clone(),
            page_size: 10,
            cursor: None,
            snapshot: None,
        })
        .unwrap()
        .locations
        .remove(0);
    let session = db.start_analysis_session(6).unwrap();
    db.discover_analysis_jobs(None, 128, 7).unwrap();
    (Arc::new(Mutex::new(db)), root, location.id, session)
}
struct Authority {
    observations: Vec<SourceObservation>,
    denied: bool,
    open_count: Arc<AtomicUsize>,
}
struct Guard {
    observations: Vec<SourceObservation>,
    reads: usize,
}
impl SourceAuthority for Authority {
    type Guard = Guard;
    fn open(&self, _: &AnalysisSource, _: &CancellationToken) -> Result<Guard, AnalysisFailure> {
        self.open_count.fetch_add(1, Ordering::Relaxed);
        if self.denied {
            Err(AnalysisFailure::SourceUnavailable)
        } else {
            Ok(Guard {
                observations: self.observations.clone(),
                reads: 0,
            })
        }
    }
}
impl OpenSource for Guard {
    fn observe(&mut self, _: &CancellationToken) -> Result<SourceObservation, AnalysisFailure> {
        let value = self.observations[self.reads.min(self.observations.len() - 1)].clone();
        self.reads += 1;
        Ok(value)
    }
}
type ParseHook = Box<dyn FnMut(&CancellationToken)>;
struct Parser {
    calls: Arc<AtomicUsize>,
    on_parse: Option<ParseHook>,
    bad_hash: bool,
    transport: bool,
}
impl ParserPort for Parser {
    fn request(
        &mut self,
        request: ParserRequest,
        token: &CancellationToken,
    ) -> Result<ProtocolReply, SupervisorError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(ProtocolReply::Result(match request {
            ParserRequest::HealthCheck => json!({"status":"ok"}),
            ParserRequest::Describe => {
                json!({"adapter":ADAPTER_ID,"adapterVersion":ADAPTER_VERSION,
                "fields":["savedVersion","baseTempoBpm","channelCount","channelNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","filesystemCreatedAtMs","pluginReferences","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars"],
                "maxFileBytes":4194304,"maxEvents":100000,"maxChannels":256,"maxEventBytes":2097152,"maxPatterns":1024,"maxPlaylistClips":1024})
            }
            ParserRequest::Parse(request) => {
                if let Some(hook) = &mut self.on_parse {
                    hook(token);
                }
                if self.transport {
                    return Err(SupervisorError::TimedOut);
                }
                let mut result = parse_bytes(&bytes());
                result["filesystemCreatedAtMs"] =
                    json!({"status":"unavailable","reason":"FILESYSTEM_CREATION_TIME_UNAVAILABLE"});
                result["inputFingerprint"] = json!({"size":request.expected.size,"modifiedAtMs":request.expected.modified_at_ms,"hash":{"algorithm":"sha256","value":if self.bad_hash {"0".repeat(64)} else {sha256_hex(&bytes())}}});
                result
            }
        }))
    }
    fn shutdown(&mut self) -> Result<(), SupervisorError> {
        Ok(())
    }
}
fn worker(observations: Vec<SourceObservation>) -> AnalysisWorker<Parser, Authority> {
    AnalysisWorker::new(
        Parser {
            calls: Arc::new(AtomicUsize::new(0)),
            on_parse: None,
            bad_hash: false,
            transport: false,
        },
        Authority {
            observations,
            denied: false,
            open_count: Arc::new(AtomicUsize::new(0)),
        },
    )
}
fn lease(db: &Mutex<Database>, session: &str) -> AnalysisLease {
    db.lock()
        .unwrap()
        .claim_analysis_job(session, 8)
        .unwrap()
        .unwrap()
}

#[test]
fn worker_runs_only_authorized_fresh_input_and_publishes_a_validated_snapshot() {
    let directory = Directory::new();
    let (db, _, location, session) = seed(&directory.0, "C:\\Synthetic", &observation());
    let lease = lease(&db, &session);
    let mut worker = worker(vec![observation()]);
    assert_eq!(
        worker
            .execute(
                &db,
                &lease,
                &Clock(AtomicI64::new(9)),
                &CancellationToken::default()
            )
            .unwrap(),
        ExecutionOutcome::Complete
    );
    assert_eq!(worker.parser.calls.load(Ordering::Relaxed), 3);
    let db = db.lock().unwrap();
    let status = db.analysis_status(&location).unwrap().unwrap();
    assert_eq!(status.state, AnalysisState::Complete);
    let snapshot = db
        .metadata_snapshot(status.snapshot_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert!(snapshot.payload_json().unwrap().contains("26.1.0.5530"));
}

#[test]
fn worker_reanalyzes_current_good_facts_after_an_explicit_bounded_request() {
    let directory = Directory::new();
    let (db, root, location, session) = seed(&directory.0, "C:\\Synthetic", &observation());
    let first_lease = lease(&db, &session);
    let mut worker = worker(vec![observation()]);
    worker
        .execute(
            &db,
            &first_lease,
            &Clock(AtomicI64::new(9)),
            &CancellationToken::default(),
        )
        .unwrap();
    let old_snapshot = {
        let mut database = db.lock().unwrap();
        let old = database
            .analysis_status(&location)
            .unwrap()
            .unwrap()
            .snapshot_id
            .unwrap();
        let input = database
            .capture_metadata_input(&root.id, &location)
            .unwrap();
        let fruitboard_storage::AnalysisRequest::Ready { key } =
            database.analysis_request(&input).unwrap()
        else {
            panic!("explicit request should be eligible");
        };
        database.request_analysis_job(&input, &key, 10).unwrap();
        database.discover_analysis_jobs(None, 128, 11).unwrap();
        assert_eq!(
            database
                .current_metadata_snapshot(input.project_file_id())
                .unwrap()
                .unwrap()
                .header()
                .id,
            old
        );
        old
    };
    let second = db
        .lock()
        .unwrap()
        .claim_analysis_job(&session, 12)
        .unwrap()
        .unwrap();
    assert_eq!(
        worker
            .execute(
                &db,
                &second,
                &Clock(AtomicI64::new(13)),
                &CancellationToken::default()
            )
            .unwrap(),
        ExecutionOutcome::Complete
    );
    let database = db.lock().unwrap();
    let status = database.analysis_status(&location).unwrap().unwrap();
    assert_eq!(status.attempt, 2);
    assert_ne!(status.snapshot_id.as_deref(), Some(old_snapshot.as_str()));
    assert!(
        database
            .metadata_snapshot(&old_snapshot)
            .unwrap()
            .unwrap()
            .payload_json()
            .is_some()
    );
}

#[test]
fn worker_denied_open_mismatched_metadata_and_changed_bytes_never_publish() {
    for scenario in 0..4 {
        let directory = Directory::new();
        let (db, _, location, session) = seed(&directory.0, "C:\\Synthetic", &observation());
        let lease = lease(&db, &session);
        let mut worker = worker(vec![observation()]);
        match scenario {
            0 => worker.authority.denied = true,
            1 => worker.authority.observations[0].modified_at_ns += 1,
            2 => {
                let mut after = observation();
                after.sha256 = "1".repeat(64);
                worker.authority.observations.push(after);
            }
            _ => worker.parser.bad_hash = true,
        }
        let result = worker
            .execute(
                &db,
                &lease,
                &Clock(AtomicI64::new(9)),
                &CancellationToken::default(),
            )
            .unwrap();
        assert!(matches!(
            result,
            ExecutionOutcome::Stale | ExecutionOutcome::Failed
        ));
        if scenario <= 1 {
            assert_eq!(worker.parser.calls.load(Ordering::Relaxed), 0);
        }
        assert!(
            db.lock()
                .unwrap()
                .analysis_status(&location)
                .unwrap()
                .unwrap()
                .snapshot_id
                .is_none()
        );
    }
}

#[test]
fn worker_rechecks_durable_authority_after_parse_without_holding_the_database_mutex() {
    for cancel in [false, true] {
        let directory = Directory::new();
        let (db, root, location, session) = seed(&directory.0, "C:\\Synthetic", &observation());
        let lease = lease(&db, &session);
        let mut worker = worker(vec![observation()]);
        let shared = db.clone();
        let id = lease.job_id().to_owned();
        worker.parser.on_parse = Some(Box::new(move |_| {
            // Would deadlock if parser work held the DB mutex.
            let mut db = shared.lock().unwrap();
            if cancel {
                db.cancel_analysis_job(&id, 9).unwrap();
            } else {
                db.set_scan_root_enabled(&root.id, false).unwrap();
            }
        }));
        let result = worker
            .execute(
                &db,
                &lease,
                &Clock(AtomicI64::new(10)),
                &CancellationToken::default(),
            )
            .unwrap();
        assert_eq!(
            result,
            if cancel {
                ExecutionOutcome::Cancelled
            } else {
                ExecutionOutcome::Stale
            }
        );
        assert!(
            db.lock()
                .unwrap()
                .analysis_status(&location)
                .unwrap()
                .unwrap()
                .snapshot_id
                .is_none()
        );
    }
}

#[test]
fn worker_transport_failure_requeues_once_and_shutdown_cancellation_stops_before_open() {
    let directory = Directory::new();
    let (db, _, location, session) = seed(&directory.0, "C:\\Synthetic", &observation());
    let lease = lease(&db, &session);
    let mut worker = worker(vec![observation()]);
    worker.parser.transport = true;
    assert_eq!(
        worker
            .execute(
                &db,
                &lease,
                &Clock(AtomicI64::new(10)),
                &CancellationToken::default()
            )
            .unwrap(),
        ExecutionOutcome::RetryQueued
    );
    let lease = db
        .lock()
        .unwrap()
        .claim_analysis_job(&session, 1011)
        .unwrap()
        .unwrap();
    let cancellation = CancellationToken::default();
    cancellation.cancel();
    let calls = worker.parser.calls.load(Ordering::Relaxed);
    assert_eq!(
        worker
            .execute(&db, &lease, &Clock(AtomicI64::new(1012)), &cancellation)
            .unwrap(),
        ExecutionOutcome::RetryQueued
    );
    assert_eq!(worker.parser.calls.load(Ordering::Relaxed), calls);
    assert_eq!(
        db.lock()
            .unwrap()
            .analysis_status(&location)
            .unwrap()
            .unwrap()
            .attempt,
        2
    );
}

#[cfg(windows)]
#[test]
#[ignore = "requires freshly built FRUITBOARD_ANALYSIS_TEST_PARSER; run by Windows feature CI and local validation"]
fn native_analysis_uses_real_parser_and_holds_source_through_publication() {
    run_native_analysis(bytes());
}

#[cfg(windows)]
#[test]
#[ignore = "requires freshly built FRUITBOARD_ANALYSIS_TEST_PARSER; run with the real-parser integration checks"]
fn native_analysis_publishes_approved_saved_patterns() {
    let saved = run_native_analysis(
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fixtures/parser-corpus/FIX-FL2026-PATTERNS.flp"),
        )
        .unwrap(),
    );
    assert_eq!(saved["patterns"]["count"], 3);
    assert_eq!(
        saved["patterns"]["items"][2]["name"]["value"],
        "Fixture Pattern C"
    );
}

#[cfg(windows)]
#[test]
#[ignore = "requires freshly built FRUITBOARD_ANALYSIS_TEST_PARSER; run by Windows feature CI and local validation"]
fn native_analysis_accepts_ordinary_project_sizes_through_publication() {
    for size in [4_601_596, 25_000_000] {
        let mut payload = bytes();
        let state_length = size - payload.len() - 5;
        payload.push(213);
        let mut length = state_length;
        loop {
            let byte = (length & 127) as u8;
            length >>= 7;
            payload.push(byte | if length == 0 { 0 } else { 128 });
            if length == 0 {
                break;
            }
        }
        payload.resize(size, 0xa5);
        payload[18..22].copy_from_slice(&((size - 22) as u32).to_le_bytes());
        run_native_analysis(payload);
    }
}

#[cfg(windows)]
fn run_native_analysis(payload: Vec<u8>) -> serde_json::Value {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandleEx;
    let directory = Directory::new();
    let root = directory.0.join("Projects 音");
    std::fs::create_dir(&root).unwrap();
    let path = root.join("sample.flp");
    std::fs::write(&path, &payload).unwrap();
    let file = std::fs::File::open(&path).unwrap();
    let metadata = file.metadata().unwrap();
    #[repr(C)]
    struct FileId {
        volume: u64,
        id: [u8; 16],
    }
    let mut id = FileId {
        volume: 0,
        id: [0; 16],
    };
    assert_ne!(
        unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                18,
                (&mut id as *mut FileId).cast(),
                std::mem::size_of::<FileId>() as u32,
            )
        },
        0
    );
    drop(file);
    let observed = SourceObservation {
        byte_size: metadata.len(),
        modified_at_ns: i64::try_from(
            metadata
                .modified()
                .unwrap()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
        .unwrap(),
        identity: Some((id.volume, u128::from_le_bytes(id.id))),
        sha256: sha256_hex(&payload),
    };
    let (db, scan_root, location, session) = seed(&directory.0, root.to_str().unwrap(), &observed);
    let lease = lease(&db, &session);
    let token = CancellationToken::default();
    let mut guard = native::WindowsAuthority
        .open(lease.source(), &token)
        .unwrap();
    assert_eq!(guard.observe(&token).unwrap().sha256, observed.sha256);
    assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
    assert!(std::fs::rename(&root, directory.0.join("Moved")).is_err());
    drop(guard);
    let parser = PathBuf::from(
        std::env::var_os("FRUITBOARD_ANALYSIS_TEST_PARSER")
            .expect("explicit freshly built parser required"),
    );
    let mut worker = AnalysisWorker::new(
        ParserSupervisor::new(parser, SupervisorLimits::default()).unwrap(),
        native::WindowsAuthority,
    );
    assert_eq!(
        worker
            .execute(&db, &lease, &Clock(AtomicI64::new(9)), &token)
            .unwrap(),
        ExecutionOutcome::Complete
    );
    worker.shutdown().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), payload);
    assert_eq!(
        db.lock()
            .unwrap()
            .analysis_status(&location)
            .unwrap()
            .unwrap()
            .state,
        AnalysisState::Complete
    );
    let saved = {
        let database = db.lock().unwrap();
        let input = database
            .capture_metadata_input(&scan_root.id, &location)
            .unwrap();
        let snapshot = database
            .current_metadata_snapshot(input.project_file_id())
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.header().input_byte_size, observed.byte_size);
        assert_eq!(
            snapshot.header().input_content_sha256.as_deref(),
            Some(observed.sha256.as_str())
        );
        let saved: serde_json::Value =
            serde_json::from_str(snapshot.payload_json().unwrap()).unwrap();
        assert_eq!(saved["savedVersion"], "26.1.0.5530");
        assert_eq!(
            saved["channelCount"],
            fruitboard_flp_parser::parse_bytes(&payload)["channelCount"]["value"]
        );
        saved
    };
    std::fs::rename(&root, directory.0.join("Moved")).unwrap();
    saved
}
