use std::path::{Path, PathBuf};
use std::time::Instant;

pub(crate) struct ShutdownDiagnostics {
    progress_output: PathBuf,
}

impl ShutdownDiagnostics {
    pub(crate) fn new(output: &Path) -> Self {
        Self {
            progress_output: super::packaging_smoke::shutdown_progress_path(output),
        }
    }

    fn record(&self, started: &Instant, marker: Marker) {
        super::packaging_smoke::record_stage_at(&self.progress_output, started, marker.as_str());
    }
}

#[derive(Clone, Copy)]
enum Marker {
    ShutdownStart,
    ShutdownEnd,
    SampleChecksStart,
    SampleChecksEnd,
    #[cfg(feature = "analysis-jobs")]
    AnalysisHostStart,
    #[cfg(feature = "analysis-jobs")]
    AnalysisHostEnd,
    ScanConsoleStart,
    ScanConsoleEnd,
}

impl Marker {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ShutdownStart => "shutdown_start",
            Self::ShutdownEnd => "shutdown_end",
            Self::SampleChecksStart => "sample_checks_start",
            Self::SampleChecksEnd => "sample_checks_end",
            #[cfg(feature = "analysis-jobs")]
            Self::AnalysisHostStart => "analysis_host_start",
            #[cfg(feature = "analysis-jobs")]
            Self::AnalysisHostEnd => "analysis_host_end",
            Self::ScanConsoleStart => "scan_console_start",
            Self::ScanConsoleEnd => "scan_console_end",
        }
    }
}

fn run_phase(
    diagnostics: &ShutdownDiagnostics,
    started: &Instant,
    start_marker: Marker,
    end_marker: Marker,
    operation: impl FnOnce(),
) {
    diagnostics.record(started, start_marker);
    operation();
    diagnostics.record(started, end_marker);
}

/// Runs the existing ordered shutdown operations, adding bounded phase
/// markers only when an explicit packaging-smoke request supplied a path.
pub(crate) fn run_shutdown(
    diagnostics: Option<&ShutdownDiagnostics>,
    sample_checks: impl FnOnce(),
    #[cfg(feature = "analysis-jobs")] analysis_host: impl FnOnce(),
    scan_console: impl FnOnce(),
) {
    let Some(diagnostics) = diagnostics else {
        sample_checks();
        #[cfg(feature = "analysis-jobs")]
        analysis_host();
        scan_console();
        return;
    };

    let started = Instant::now();
    diagnostics.record(&started, Marker::ShutdownStart);
    run_phase(
        diagnostics,
        &started,
        Marker::SampleChecksStart,
        Marker::SampleChecksEnd,
        sample_checks,
    );
    #[cfg(feature = "analysis-jobs")]
    run_phase(
        diagnostics,
        &started,
        Marker::AnalysisHostStart,
        Marker::AnalysisHostEnd,
        analysis_host,
    );
    run_phase(
        diagnostics,
        &started,
        Marker::ScanConsoleStart,
        Marker::ScanConsoleEnd,
        scan_console,
    );
    diagnostics.record(&started, Marker::ShutdownEnd);
}

#[cfg(test)]
mod tests {
    use super::{ShutdownDiagnostics, run_shutdown};
    use serde_json::Value;
    use std::cell::RefCell;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "fruitboard-shutdown-diagnostics-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("test directory should be unique");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn run_recorded_shutdown(diagnostics: Option<&ShutdownDiagnostics>) -> Vec<&'static str> {
        let operations = RefCell::new(Vec::new());
        run_shutdown(
            diagnostics,
            || operations.borrow_mut().push("sample_checks"),
            #[cfg(feature = "analysis-jobs")]
            || operations.borrow_mut().push("analysis_host"),
            || operations.borrow_mut().push("scan_console"),
        );
        operations.into_inner()
    }

    #[test]
    fn records_allowlisted_phases_around_the_ordered_shutdown_operations() {
        let directory = TestDirectory::new();
        let output = directory.path().join("selected-smoke.json");
        let diagnostics = ShutdownDiagnostics::new(&output);
        let progress_path = output.with_extension("shutdown-stages.jsonl");

        run_shutdown(
            Some(&diagnostics),
            || {
                assert_stages(&progress_path, &["shutdown_start", "sample_checks_start"]);
            },
            #[cfg(feature = "analysis-jobs")]
            || {
                assert_stages(
                    &progress_path,
                    &[
                        "shutdown_start",
                        "sample_checks_start",
                        "sample_checks_end",
                        "analysis_host_start",
                    ],
                );
            },
            || {
                #[cfg(feature = "analysis-jobs")]
                let expected = [
                    "shutdown_start",
                    "sample_checks_start",
                    "sample_checks_end",
                    "analysis_host_start",
                    "analysis_host_end",
                    "scan_console_start",
                ];
                #[cfg(not(feature = "analysis-jobs"))]
                let expected = [
                    "shutdown_start",
                    "sample_checks_start",
                    "sample_checks_end",
                    "scan_console_start",
                ];
                assert_stages(&progress_path, &expected);
            },
        );

        let contents = fs::read_to_string(&progress_path).expect("shutdown trace should exist");
        let records = contents
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("record should be JSON"))
            .collect::<Vec<_>>();
        assert_eq!(
            records
                .iter()
                .map(|record| record["stage"].as_str().expect("fixed stage"))
                .collect::<Vec<_>>(),
            expected_markers()
        );
        let mut previous_elapsed_ms = 0;
        for record in &records {
            let object = record.as_object().expect("record should be an object");
            assert_eq!(object.len(), 2);
            assert!(object.contains_key("stage"));
            let elapsed_ms = object["elapsedMs"]
                .as_u64()
                .expect("elapsed time should be an unsigned integer");
            assert!(elapsed_ms >= previous_elapsed_ms);
            previous_elapsed_ms = elapsed_ms;
        }
        assert!(!contents.contains(&output.to_string_lossy().to_string()));
    }

    #[test]
    fn a_phase_that_does_not_return_keeps_its_flushed_start_marker() {
        let directory = TestDirectory::new();
        let output = directory.path().join("selected-smoke.json");
        let diagnostics = ShutdownDiagnostics::new(&output);
        let progress_path = output.with_extension("shutdown-stages.jsonl");

        let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_shutdown(
                Some(&diagnostics),
                || panic!("controlled interrupted phase"),
                #[cfg(feature = "analysis-jobs")]
                || panic!("later phase must not run"),
                || panic!("later phase must not run"),
            );
        }));

        assert!(interrupted.is_err());
        assert_stages(&progress_path, &["shutdown_start", "sample_checks_start"]);
    }

    #[test]
    fn missing_smoke_request_runs_the_same_order_without_creating_diagnostics() {
        let directory = TestDirectory::new();
        let output = directory.path().join("selected-smoke.json");
        let operations = run_recorded_shutdown(None);
        assert_eq!(operations, expected_operations());
        assert!(!output.with_extension("shutdown-stages.jsonl").exists());
    }

    #[test]
    fn unwritable_diagnostics_do_not_interrupt_shutdown_operations() {
        let directory = TestDirectory::new();
        let obstruction = directory.path().join("not-a-directory");
        fs::write(&obstruction, "block directory creation").expect("obstruction file");
        let output = obstruction.join("selected-smoke.json");
        let diagnostics = ShutdownDiagnostics::new(&output);

        let operations = run_recorded_shutdown(Some(&diagnostics));
        assert_eq!(operations, expected_operations());
        assert!(!output.with_extension("shutdown-stages.jsonl").exists());
    }

    fn expected_operations() -> Vec<&'static str> {
        let mut operations = vec!["sample_checks"];
        #[cfg(feature = "analysis-jobs")]
        operations.push("analysis_host");
        operations.push("scan_console");
        operations
    }

    fn expected_markers() -> Vec<&'static str> {
        let mut markers = vec!["shutdown_start", "sample_checks_start", "sample_checks_end"];
        #[cfg(feature = "analysis-jobs")]
        markers.extend(["analysis_host_start", "analysis_host_end"]);
        markers.extend(["scan_console_start", "scan_console_end", "shutdown_end"]);
        markers
    }

    fn read_stages(path: &Path) -> Vec<String> {
        fs::read_to_string(path)
            .expect("phase marker should be flushed before its operation")
            .lines()
            .map(|line| {
                serde_json::from_str::<Value>(line).expect("record should be JSON")["stage"]
                    .as_str()
                    .expect("stage should be a string")
                    .to_owned()
            })
            .collect()
    }

    fn assert_stages(path: &Path, expected: &[&str]) {
        let actual = read_stages(path);
        assert_eq!(
            actual.iter().map(String::as_str).collect::<Vec<_>>(),
            expected
        );
    }
}
