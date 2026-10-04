//! Opt-in Windows release worker probe. Inputs must be explicitly prepared
//! synthetic fixtures; existing databases are refused and evidence is retained.
#[cfg(windows)]
#[path = "../../../scripts/native-resource-probe.rs"]
mod probe;

#[cfg(windows)]
mod windows {
    use super::probe;
    use fruitboard_analysis_execution::{
        AnalysisClock, AnalysisWorker, ExecutionOutcome, ParserPort, native::WindowsAuthority,
    };
    use fruitboard_flp_parser::supervisor::{
        CancellationToken, ParserRequest, ParserSupervisor, ProtocolReply, SupervisorError,
        SupervisorLimits,
    };
    use fruitboard_storage::{Database, EncodedIdentity, LibraryQuery, ScanKind, ScanObservation};
    use serde_json::json;
    use std::os::windows::io::AsRawHandle;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::{Instant, UNIX_EPOCH};
    use windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandleEx;

    struct Clock;
    impl AnalysisClock for Clock {
        fn now_ms(&self) -> i64 {
            // This is a worker/source/publication probe, not the desktop lease
            // renewal monitor. Keep ledger time inside the fixed valid lease.
            1_000
        }
    }
    struct Parser {
        supervisor: ParserSupervisor,
        recorded: Arc<Mutex<serde_json::Value>>,
    }
    impl ParserPort for Parser {
        fn request(
            &mut self,
            request: ParserRequest,
            cancellation: &CancellationToken,
        ) -> Result<ProtocolReply, SupervisorError> {
            let parse = matches!(&request, ParserRequest::Parse(_));
            let started = Instant::now();
            let reply = self.supervisor.request(request, cancellation)?;
            if parse {
                let elapsed = started.elapsed().as_secs_f64() * 1_000.0;
                let ProtocolReply::Result(ref value) = reply else {
                    return Err(SupervisorError::InvalidProtocol);
                };
                let parser_micros = value["parseElapsedMicros"]
                    .as_u64()
                    .ok_or(SupervisorError::InvalidProtocol)?;
                let pid = self
                    .supervisor
                    .process_id()
                    .ok_or(SupervisorError::ProcessStopped)?;
                *self
                    .recorded
                    .lock()
                    .map_err(|_| SupervisorError::ProcessStopped)? = json!({
                    "supervisedParseMs":elapsed,
                    "parserReportedMs":parser_micros as f64/1_000.0,
                    "parserMemory":probe::memory(Some(pid)).map_err(|_|SupervisorError::ProcessStopped)?
                });
            }
            Ok(reply)
        }
        fn shutdown(&mut self) -> Result<(), SupervisorError> {
            self.supervisor.shutdown()
        }
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let arguments: Vec<_> = std::env::args_os().skip(1).collect();
        if arguments.len() != 2 {
            return Err(
                "expected prepared fixture directory and independently retained parser".into(),
            );
        }
        let directory = PathBuf::from(&arguments[0]);
        let parser = PathBuf::from(&arguments[1]);
        if !directory.is_absolute() || !parser.is_absolute() {
            return Err("explicit absolute probe inputs required".into());
        }
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("synthetic-manifest.json"))?)?;
        if manifest["schema"] != "fruitboard/native-resource-fixture/1"
            || manifest["copies"] != 12
            || manifest["savedBuild"] != "26.1.0.5530"
        {
            return Err("prepared synthetic fixture manifest required".into());
        }
        let case = manifest["case"].as_str().ok_or("invalid fixture case")?;
        let bytes = manifest["bytes"].as_u64().ok_or("invalid fixture size")?;
        let expected_size = match case {
            "sample" => 46_703,
            "reported" => 4_601_596,
            "25mb" => 25_000_000,
            "64mib" => 67_108_864,
            _ => return Err("unsupported probe geometry".into()),
        };
        if bytes != expected_size {
            return Err("unsupported probe geometry".into());
        }
        let hash = manifest["sha256"].as_str().ok_or("missing fixture hash")?;
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid fixture digest".into());
        }
        let db_path = directory.join("probe-db");
        if db_path.exists() {
            return Err("refuse any existing probe database/profile".into());
        }
        std::fs::create_dir(&db_path)?;
        let source = directory.join("source");
        let mut db = Database::open(&db_path)?;
        let root = db.add_scan_root(
            "Synthetic resources",
            source.to_str().ok_or("invalid root")?,
        )?;
        let session = db.start_scan_session(1)?;
        db.enqueue_scan(&root.id, ScanKind::Manual, 2)?;
        let scan = db
            .lease_next_scan(&session.id, 3, 1_000)?
            .ok_or("missing scan")?;
        let mut observations = Vec::new();
        for index in 0..12 {
            let relative = format!("fixture-{index:02}.flp");
            let file = std::fs::File::open(source.join(&relative))?;
            let metadata = file.metadata()?;
            if metadata.len() != bytes {
                return Err("fixture size differs from manifest".into());
            }
            #[repr(C)]
            struct FileId {
                volume: u64,
                id: [u8; 16],
            }
            let mut id = FileId {
                volume: 0,
                id: [0; 16],
            };
            if unsafe {
                GetFileInformationByHandleEx(
                    file.as_raw_handle(),
                    18,
                    (&mut id as *mut FileId).cast(),
                    std::mem::size_of::<FileId>() as u32,
                )
            } == 0
            {
                return Err("fixture identity unavailable".into());
            }
            observations.push(ScanObservation {
                locator_key: format!("v1:i:{relative}"),
                relative_path: relative,
                byte_size: bytes,
                modified_at_ns: i128::try_from(
                    metadata.modified()?.duration_since(UNIX_EPOCH)?.as_nanos(),
                )?,
                identity: Some(EncodedIdentity {
                    volume_serial: id.volume.to_string(),
                    file_id: u128::from_le_bytes(id.id).to_string(),
                }),
            });
        }
        db.stage_scan_observations(
            &scan.run.id,
            &session.id,
            &scan.run.lease_token,
            4,
            &observations,
        )?;
        db.publish_scan_run(&scan.run.id, &session.id, &scan.run.lease_token, 5)?;
        let locations = db
            .query_library(&LibraryQuery {
                scan_root_id: root.id.clone(),
                page_size: 12,
                cursor: None,
                snapshot: None,
            })?
            .locations;
        let analysis_session = db.start_analysis_session(6)?;
        db.discover_analysis_jobs(None, 128, 7)?;
        let db = Arc::new(Mutex::new(db));
        let recorded = Arc::new(Mutex::new(serde_json::Value::Null));
        let parser = Parser {
            supervisor: ParserSupervisor::new(parser, SupervisorLimits::default())
                .map_err(|_| "parser setup failed")?,
            recorded: recorded.clone(),
        };
        let mut worker = AnalysisWorker::new(parser, WindowsAuthority);
        let baseline = probe::memory(None)?;
        let mut runs = Vec::new();
        let mut elapsed_samples = Vec::new();
        for index in 0..12 {
            let lease = db
                .lock()
                .map_err(|_| "probe lock failed")?
                .claim_analysis_job(&analysis_session, 1_000)?
                .ok_or("missing analysis lease")?;
            let start = Instant::now();
            let outcome = worker.execute(&db, &lease, &Clock, &CancellationToken::default())?;
            let elapsed = start.elapsed().as_secs_f64() * 1_000.0;
            if outcome != ExecutionOutcome::Complete {
                return Err("native worker did not complete".into());
            }
            let mut row = recorded.lock().map_err(|_| "probe lock failed")?.clone();
            row["phase"] = json!(match index {
                0 => "first_run",
                1 => "warm_up",
                _ => "measured",
            });
            row["workerMs"] = json!(elapsed);
            row["parentMemory"] = probe::memory(None)?;
            runs.push(row);
            if index >= 2 {
                elapsed_samples.push(elapsed);
            }
        }
        worker.shutdown().map_err(|_| "parser shutdown failed")?;
        let db = db.lock().map_err(|_| "probe lock failed")?;
        for location in locations {
            let input = db.capture_metadata_input(&root.id, &location.id)?;
            let snapshot = db
                .current_metadata_snapshot(input.project_file_id())?
                .ok_or("missing saved result")?;
            if snapshot.header().input_content_sha256.as_deref() != Some(hash) {
                return Err("published fixture digest mismatch".into());
            }
            let saved: serde_json::Value =
                serde_json::from_str(snapshot.payload_json().ok_or("missing payload")?)?;
            if saved["savedVersion"] != "26.1.0.5530" || saved["channelCount"] != 1 {
                return Err("published facts differ".into());
            }
        }
        println!(
            "{}",
            json!({"schema":"fruitboard/native-resource-result/1","probe":"native-analysis","case":case,"bytes":bytes,"copies":12,"baselineMemory":baseline,"runs":runs,"warmWorker":probe::warm_summary(&elapsed_samples),"supervisedRequestDeadlineMs":10_000,"sourceHashesMatch":true})
        );
        Ok(())
    }
}

fn main() {
    #[cfg(windows)]
    if windows::run().is_err() {
        eprintln!("native_resource_probe_failed");
        std::process::exit(1);
    }
    #[cfg(not(windows))]
    {
        eprintln!("native_resource_probe_requires_windows");
        std::process::exit(1);
    }
}
