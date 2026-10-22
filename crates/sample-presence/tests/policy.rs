use fruitboard_sample_presence::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};

#[derive(Default)]
struct Clock(AtomicU64);
impl MonotonicClock for Clock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

struct Authority {
    root: Vec<String>,
    _source: HandleLease,
}
struct Directory {
    relative: Vec<String>,
    _handle: HandleLease,
}

#[derive(Clone, Copy)]
enum Node {
    File,
    Directory,
    Absent,
    Error(PortError),
}

#[derive(Default)]
struct Port {
    nodes: BTreeMap<String, Node>,
    calls: Vec<String>,
    qualification_error: Option<PortError>,
    final_error: Option<PortError>,
    insensitive: bool,
    case_unknown: bool,
    burn_operations: usize,
    inflate_handles: usize,
    control: Option<RequestControl>,
    invalidate_on_child: bool,
    clock: Option<Arc<Clock>>,
    advance_on_child: bool,
    blocked: Option<(Sender<()>, Receiver<()>)>,
    final_operations: usize,
    final_handles: usize,
    final_burn: usize,
}

impl MetadataPort for Port {
    type Authority = Authority;
    type Directory = Directory;

    fn qualify(
        &mut self,
        input: &CapturedInput,
        operations: &mut Operations,
    ) -> Result<Qualified<Authority, Directory>, PortError> {
        let mut extras = Vec::new();
        for _ in 0..self.inflate_handles {
            extras.push(operations.acquire_handle()?);
        }
        let source = operations.acquire_handle()?;
        let root = operations.acquire_handle()?;
        operations.perform(|| {
            self.calls.push("qualify".into());
            if let Some(error) = self.qualification_error {
                return Err(error);
            }
            Ok(Qualified {
                authority: Authority {
                    root: input.root().components().to_vec(),
                    _source: source,
                },
                root_directory: Directory {
                    relative: Vec::new(),
                    _handle: root,
                },
            })
        })
    }

    fn root_component_matches(
        &mut self,
        authority: &Authority,
        index: usize,
        saved: &str,
        _operations: &mut Operations,
    ) -> Result<bool, PortError> {
        if self.case_unknown {
            return Err(PortError::Unchecked(UncheckedReason::UnsupportedCaseMode));
        }
        // Fake's declared case mode, deliberately not used by production policy.
        Ok(if self.insensitive {
            authority.root[index].eq_ignore_ascii_case(saved)
        } else {
            authority.root[index] == saved
        })
    }

    fn child(
        &mut self,
        _authority: &Authority,
        parent: &Directory,
        name: &str,
        operations: &mut Operations,
    ) -> Result<Child<Directory>, PortError> {
        let token = operations.acquire_handle()?;
        let mut relative = parent.relative.clone();
        relative.push(name.to_owned());
        let key = relative.join("\\");
        let node = operations.perform(|| {
            self.calls.push(format!("child:{key}"));
            if let Some((started, resume)) = self.blocked.take() {
                started.send(()).unwrap();
                resume.recv().unwrap();
            }
            if self.invalidate_on_child {
                self.control.as_ref().unwrap().invalidate();
            }
            if self.advance_on_child {
                self.clock
                    .as_ref()
                    .unwrap()
                    .0
                    .store(DEADLINE_MS, Ordering::SeqCst);
            }
            Ok(self.nodes.get(&key).copied().unwrap_or(Node::Absent))
        })?;
        for _ in 0..self.burn_operations {
            operations.perform(|| Ok(()))?;
        }
        match node {
            Node::File => Ok(Child::RegularFile),
            Node::Directory => Ok(Child::Directory(Directory {
                relative,
                _handle: token,
            })),
            Node::Absent => Ok(Child::Absent),
            Node::Error(error) => Err(error),
        }
    }

    fn revalidate(
        &mut self,
        _input: &CapturedInput,
        _authority: &Authority,
        operations: &mut Operations,
    ) -> Result<(), PortError> {
        self.final_handles = operations.live_handles();
        for _ in 0..self.final_burn {
            operations.perform(|| Ok(()))?;
        }
        self.final_operations = operations.used() + 1;
        operations.perform(|| {
            self.calls.push("revalidate".into());
            self.final_error.map_or(Ok(()), Err)
        })
    }
}

fn context() -> Context {
    Context::new(
        "request-1",
        "root-1",
        "location-1",
        "snapshot-1",
        "session-1",
    )
    .unwrap()
}
fn fence() -> AuthorityFence {
    AuthorityFence {
        root_revision: 3,
        location_revision: 7,
        root_identity: Some(ObjectIdentity { volume: 1, file: 2 }),
        source: SourceFingerprint {
            identity: ObjectIdentity { volume: 1, file: 4 },
            byte_size: 100,
            modified_at_ns: 123,
        },
    }
}
fn input(values: &[Option<&str>]) -> CapturedInput {
    input_for(r"C:\Projects", r"C:\Projects\song.flp", values)
}
fn input_for(root: &str, source: &str, values: &[Option<&str>]) -> CapturedInput {
    CapturedInput::new(
        context(),
        root,
        source,
        fence(),
        Some(values.iter().map(|v| v.map(str::to_owned)).collect()),
    )
    .unwrap()
}
fn run(input: &CapturedInput, port: &mut Port) -> Result<Report, RequestFailure> {
    let gate = WorkerGate::default();
    let mut lease = gate.try_start(Arc::new(Clock::default())).unwrap();
    check(input, &mut lease, port, 1_791_216_000_000)
}
fn outcomes(report: &Report) -> Vec<Outcome> {
    report.channels().iter().map(|v| v.outcome).collect()
}
fn unchecked(reason: UncheckedReason) -> Outcome {
    Outcome::NotChecked { reason }
}
fn candidate_calls(port: &Port) -> usize {
    port.calls
        .iter()
        .filter(|v| v.starts_with("child:"))
        .count()
}
fn child_calls(port: &Port) -> Vec<String> {
    port.calls
        .iter()
        .filter(|value| value.starts_with("child:"))
        .cloned()
        .collect()
}

#[test]
fn exact_file_absent_leaf_and_absent_intermediate_are_distinct_from_unchecked() {
    let mut port = Port::default();
    port.nodes.insert("samples".into(), Node::Directory);
    port.nodes.insert(r"samples\kick.wav".into(), Node::File);
    let report = run(
        &input(&[
            Some(r"C:\Projects\samples\kick.wav"),
            Some(r"C:\Projects\samples\missing.wav"),
            Some(r"C:\Projects\absent\kick.wav"),
            None,
        ]),
        &mut port,
    )
    .unwrap();
    assert_eq!(
        outcomes(&report),
        vec![
            Outcome::Present,
            Outcome::NotFound,
            Outcome::NotFound,
            Outcome::NoSavedReference
        ]
    );
    assert!(!port.calls.contains(&r"child:absent\kick.wav".to_owned()));
    assert_eq!(
        report
            .channels()
            .iter()
            .map(|c| c.position)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(port.calls.last().unwrap(), "revalidate");
}

#[test]
fn forbidden_syntax_never_reaches_a_candidate_operation() {
    let rejected = [
        r"\\server\share\x.wav",
        r"\\?\C:\Projects\x.wav",
        r"\\.\C:\Projects\x.wav",
        "https://example.invalid/x",
        r"C:x.wav",
        r"C:\Projects\x.wav:stream",
        r"C:\Projects\..\x.wav",
        r"C:\Projects\.\x.wav",
        r"C:\Projects\*.wav",
        r"C:\Projects\CON.wav",
        r"C:\Projects\COM¹.wav",
        r"C:\Projects\NUL .wav",
        r"C:\Projects\trailing.\x.wav",
        r"C:\Projects\trailing \x.wav",
        r"C:\Projects\\x.wav",
        r"C:\Projects\x.wav\",
        "C:/Projects/x.wav",
        r"C:\Projects\PROJEC~1\x.wav",
        "C:\\Projects\\x\n.wav",
        "C:\\Projects\\x\u{202e}.wav",
    ];
    for value in rejected {
        let mut port = Port::default();
        let report = run(&input(&[Some(value)]), &mut port).unwrap();
        assert_eq!(
            outcomes(&report),
            vec![unchecked(UncheckedReason::UnsupportedPathSyntax)]
        );
        assert_eq!(candidate_calls(&port), 0);
    }
}

#[test]
fn relative_child_sibling_and_parent_candidates_use_the_project_parent() {
    let values = input_for(
        r"C:\Projects",
        r"C:\Projects\Albums\Song.flp",
        &[
            Some(r"Samples\Kick.wav"),
            Some(r"..\Sibling\Kick.wav"),
            Some(r"..\Kick.wav"),
            Some(r"..\..\Albums\..\Kick.wav"),
        ],
    );
    let mut port = Port::default();
    for directory in ["Albums", r"Albums\Samples", "Samples", "Sibling"] {
        port.nodes.insert(directory.into(), Node::Directory);
    }
    for file in [
        r"Albums\Samples\Kick.wav",
        r"Sibling\Kick.wav",
        "Kick.wav",
        // This root-level sentinel proves Samples\Kick.wav was based at Albums.
        r"Samples\Kick.wav",
    ] {
        port.nodes.insert(file.into(), Node::File);
    }

    let report = run(&values, &mut port).unwrap();
    assert_eq!(
        outcomes(&report),
        vec![
            Outcome::Present,
            Outcome::Present,
            Outcome::Present,
            unchecked(UncheckedReason::OutsideRoot),
        ]
    );
    assert_eq!(
        child_calls(&port),
        vec![
            "child:Albums",
            r"child:Albums\Samples",
            r"child:Albums\Samples\Kick.wav",
            "child:Sibling",
            r"child:Sibling\Kick.wav",
            "child:Kick.wav",
        ]
    );
    assert_eq!(port.calls.last().unwrap(), "revalidate");
}

#[test]
fn valid_cancelled_components_are_not_queried_but_invalid_raw_components_still_fail() {
    let values = input(&[
        Some(r"Missing\..\Kick.wav"),
        Some(r".\Kick.wav"),
        Some(r"bad*name\..\Kick.wav"),
        Some(r"trailing.\..\Kick.wav"),
        Some(r"trailing \..\Kick.wav"),
        Some(r"CON\..\Kick.wav"),
        Some(r"COM¹\..\Kick.wav"),
        Some(r"PROJEC~1\..\Kick.wav"),
        Some(r"a//..\Kick.wav"),
        Some(r"a\\..\Kick.wav"),
        Some(r"Kick.wav\"),
        Some("bad\u{202e}name\\..\\Kick.wav"),
        Some(r"%FLStudioData%\..\Kick.wav"),
        Some(r"Missing\$CACHE\..\Kick.wav"),
        Some(r"$HOME\..\Kick.wav"),
    ]);
    let mut port = Port::default();
    port.nodes.insert("Kick.wav".into(), Node::File);
    let report = run(&values, &mut port).unwrap();
    assert_eq!(
        outcomes(&report),
        vec![
            Outcome::Present,
            Outcome::Present,
            unchecked(UncheckedReason::UnsupportedPathSyntax),
            unchecked(UncheckedReason::UnsupportedPathSyntax),
            unchecked(UncheckedReason::UnsupportedPathSyntax),
            unchecked(UncheckedReason::UnsupportedPathSyntax),
            unchecked(UncheckedReason::UnsupportedPathSyntax),
            unchecked(UncheckedReason::UnsupportedPathSyntax),
            unchecked(UncheckedReason::UnsupportedPathSyntax),
            unchecked(UncheckedReason::UnsupportedPathSyntax),
            unchecked(UncheckedReason::UnsupportedPathSyntax),
            unchecked(UncheckedReason::UnsupportedPathSyntax),
            unchecked(UncheckedReason::UnresolvedPlaceholder),
            unchecked(UncheckedReason::UnresolvedPlaceholder),
            unchecked(UncheckedReason::UnresolvedPlaceholder),
        ]
    );
    assert_eq!(child_calls(&port), vec!["child:Kick.wav", "child:Kick.wav"]);
}

#[test]
fn only_the_existing_absolute_parser_relative_classification_dispatches_to_candidates() {
    let rejected = [
        r"\\server\share\Kick.wav",
        r"\\?\C:\Projects\Kick.wav",
        r"\\.\C:\Projects\Kick.wav",
        r"\rooted\Kick.wav",
        "/rooted/Kick.wav",
        "https://example.invalid/Kick.wav",
        r"C:Samples\Kick.wav",
        r"C:\Projects\Kick.wav:stream",
        r"bad:name\..\Kick.wav",
    ];
    for value in rejected {
        let mut port = Port::default();
        let report = run(&input(&[Some(value)]), &mut port).unwrap();
        assert_eq!(candidate_calls(&port), 0, "{value:?}");
        assert_eq!(
            outcomes(&report),
            vec![unchecked(UncheckedReason::UnsupportedPathSyntax)],
            "{value:?}"
        );
    }
    let mut placeholder = Port::default();
    assert_eq!(
        outcomes(
            &run(
                &input(&[Some(r"C:\Projects\%USERPROFILE%\Kick.wav")]),
                &mut placeholder,
            )
            .unwrap()
        ),
        vec![unchecked(UncheckedReason::UnresolvedPlaceholder)]
    );
    assert_eq!(candidate_calls(&placeholder), 0);
    assert!(matches!(
        AbsolutePath::parse(r"Samples\Kick.wav"),
        Err(UncheckedReason::RelativeReference)
    ));
}

#[test]
fn an_escape_stays_rejected_after_reentry_and_root_equal_candidates_are_directories() {
    let values = input_for(
        r"C:\Projects",
        r"C:\Projects\Albums\Song.flp",
        &[Some(r"..\..\Albums\..\Kick.wav"), Some(r"..")],
    );
    let mut port = Port::default();
    let report = run(&values, &mut port).unwrap();
    assert_eq!(
        outcomes(&report),
        vec![
            unchecked(UncheckedReason::OutsideRoot),
            unchecked(UncheckedReason::NotRegularFile),
        ]
    );
    assert_eq!(candidate_calls(&port), 0);
    assert_eq!(port.calls.last().unwrap(), "revalidate");
}

#[test]
fn relative_component_spelling_is_preserved_for_the_qualified_port() {
    let composed = format!("Samples\\{}.wav", '\u{e9}');
    let decomposed = "Samples\\e\u{301}.wav";
    let values = CapturedInput::new(
        context(),
        r"C:\Projects",
        r"C:\Projects\Albums\Song.flp",
        fence(),
        Some(vec![Some(composed.clone()), Some(decomposed.to_owned())]),
    )
    .unwrap();
    let mut port = Port::default();
    port.nodes.insert("Albums".into(), Node::Directory);
    port.nodes.insert(r"Albums\Samples".into(), Node::Directory);
    port.nodes.insert(format!("Albums\\{composed}"), Node::File);
    let report = run(&values, &mut port).unwrap();
    assert_eq!(outcomes(&report), vec![Outcome::Present, Outcome::NotFound]);
    assert_eq!(
        child_calls(&port),
        vec![
            "child:Albums",
            r"child:Albums\Samples",
            format!("child:Albums\\{composed}"),
            "child:Albums",
            r"child:Albums\Samples",
            format!("child:Albums\\{decomposed}"),
        ]
    );
}

#[test]
fn sibling_other_root_other_drive_and_shorter_path_grant_no_candidate_access() {
    let mut port = Port::default();
    let report = run(
        &input(&[
            Some(r"C:\Projects2\kick.wav"),
            Some(r"C:\Samples\kick.wav"),
            Some(r"D:\Projects\kick.wav"),
            Some(r"C:\"),
        ]),
        &mut port,
    )
    .unwrap();
    assert_eq!(
        outcomes(&report),
        vec![unchecked(UncheckedReason::OutsideRoot); 4]
    );
    assert_eq!(candidate_calls(&port), 0);
}

#[test]
fn case_and_unicode_comparison_belong_to_the_qualified_port() {
    let values = input(&[Some(r"c:\projects\kick.wav")]);
    let mut sensitive = Port::default();
    assert_eq!(
        outcomes(&run(&values, &mut sensitive).unwrap()),
        vec![unchecked(UncheckedReason::OutsideRoot)]
    );
    assert_eq!(candidate_calls(&sensitive), 0);
    let mut insensitive = Port {
        insensitive: true,
        ..Port::default()
    };
    insensitive.nodes.insert("kick.wav".into(), Node::File);
    assert_eq!(
        outcomes(&run(&values, &mut insensitive).unwrap()),
        vec![Outcome::Present]
    );
    let mut unknown = Port {
        case_unknown: true,
        ..Port::default()
    };
    assert_eq!(
        outcomes(&run(&values, &mut unknown).unwrap()),
        vec![unchecked(UncheckedReason::UnsupportedCaseMode)]
    );
    assert_eq!(candidate_calls(&unknown), 0);
    let composed = AbsolutePath::parse("C:\\Projects\\é.wav").unwrap();
    let decomposed = AbsolutePath::parse("C:\\Projects\\e\u{301}.wav").unwrap();
    assert_ne!(composed.components(), decomposed.components());
}

#[test]
fn denied_offline_reparse_unknown_and_directory_observations_never_become_missing() {
    for reason in [
        UncheckedReason::AccessDenied,
        UncheckedReason::ReparseOrOffline,
        UncheckedReason::UnqualifiedFilesystem,
        UncheckedReason::UnsupportedCaseMode,
    ] {
        let mut port = Port::default();
        port.nodes
            .insert("samples".into(), Node::Error(PortError::Unchecked(reason)));
        let report = run(&input(&[Some(r"C:\Projects\samples\kick.wav")]), &mut port).unwrap();
        assert_eq!(outcomes(&report), vec![unchecked(reason)]);
        assert_eq!(candidate_calls(&port), 1);
    }
    let mut port = Port::default();
    port.nodes.insert("directory".into(), Node::Directory);
    port.nodes.insert("file".into(), Node::File);
    let report = run(
        &input(&[
            Some(r"C:\Projects\directory"),
            Some(r"C:\Projects\file\x.wav"),
            Some(r"C:\Projects"),
        ]),
        &mut port,
    )
    .unwrap();
    assert_eq!(
        outcomes(&report),
        vec![unchecked(UncheckedReason::NotRegularFile); 3]
    );
}

#[test]
fn unqualified_root_or_source_prevents_all_sample_probes() {
    for reason in [
        UncheckedReason::UnqualifiedFilesystem,
        UncheckedReason::ReparseOrOffline,
        UncheckedReason::AccessDenied,
    ] {
        let mut port = Port {
            qualification_error: Some(PortError::Unchecked(reason)),
            ..Port::default()
        };
        assert!(
            matches!(run(&input(&[Some(r"kick.wav")]), &mut port), Err(RequestFailure::Unavailable(actual)) if actual == reason)
        );
        assert_eq!(candidate_calls(&port), 0);
    }
}

#[test]
fn replacement_or_snapshot_session_revision_change_discards_every_provisional_result() {
    // Fake final authority fence models the host/Windows port's stale signal;
    // actual namespace replacement and durable DB tests belong to later slices.
    let mut port = Port {
        final_error: Some(PortError::Request(RequestFailure::Stale)),
        ..Port::default()
    };
    port.nodes.insert("kick.wav".into(), Node::File);
    assert!(matches!(
        run(&input(&[Some(r"C:\Projects\kick.wav")]), &mut port),
        Err(RequestFailure::Stale)
    ));
    assert_eq!(candidate_calls(&port), 1);
    port.final_error = None;
    port.nodes.insert(
        "kick.wav".into(),
        Node::Error(PortError::Request(RequestFailure::Stale)),
    );
    assert!(matches!(
        run(
            &input(&[
                Some(r"C:\Projects\kick.wav"),
                Some(r"C:\Projects\another.wav")
            ]),
            &mut port
        ),
        Err(RequestFailure::Stale)
    ));
    assert_eq!(candidate_calls(&port), 2);
}

#[test]
fn relative_results_remain_provisional_until_the_same_revalidation_fence() {
    let mut stale_at_final_fence = Port {
        final_error: Some(PortError::Request(RequestFailure::Stale)),
        ..Port::default()
    };
    stale_at_final_fence
        .nodes
        .insert("Kick.wav".into(), Node::File);
    assert!(matches!(
        run(&input(&[Some(r"Kick.wav")]), &mut stale_at_final_fence),
        Err(RequestFailure::Stale)
    ));
    assert_eq!(child_calls(&stale_at_final_fence), vec!["child:Kick.wav"]);
    assert_eq!(stale_at_final_fence.calls.last().unwrap(), "revalidate");

    let gate = WorkerGate::default();
    let mut lease = gate.try_start(Arc::new(Clock::default())).unwrap();
    let mut cancelled = Port {
        invalidate_on_child: true,
        control: Some(lease.control()),
        ..Port::default()
    };
    assert!(matches!(
        check(
            &input(&[Some(r"Kick.wav")]),
            &mut lease,
            &mut cancelled,
            1_791_216_000_000,
        ),
        Err(RequestFailure::Stale)
    ));
    assert_eq!(child_calls(&cancelled), vec!["child:Kick.wav"]);
    assert!(!cancelled.calls.contains(&"revalidate".into()));
}

#[test]
fn exact_literal_duplicates_reuse_only_within_one_request_and_keep_all_positions() {
    let values = input(&[
        Some(r"C:\Projects\kick.wav"),
        None,
        Some(r"C:\Projects\kick.wav"),
        Some(r"C:\Projects\KICK.wav"),
    ]);
    let mut port = Port::default();
    port.nodes.insert("kick.wav".into(), Node::File);
    let report = run(&values, &mut port).unwrap();
    assert_eq!(
        outcomes(&report),
        vec![
            Outcome::Present,
            Outcome::NoSavedReference,
            Outcome::Present,
            Outcome::NotFound
        ]
    );
    assert_eq!(candidate_calls(&port), 2);
    run(&values, &mut port).unwrap();
    assert_eq!(candidate_calls(&port), 4);
}

#[test]
fn relative_memoization_uses_the_literal_reference_not_its_candidate() {
    let values = input(&[
        Some(r"Missing\..\Kick.wav"),
        Some(r"Missing\..\Kick.wav"),
        Some(r"Kick.wav"),
    ]);
    let mut port = Port::default();
    port.nodes.insert("Kick.wav".into(), Node::File);
    let report = run(&values, &mut port).unwrap();
    assert_eq!(outcomes(&report), vec![Outcome::Present; 3]);
    assert_eq!(child_calls(&port), vec!["child:Kick.wav", "child:Kick.wav"]);
}

#[test]
fn maximum_empty_and_all_unavailable_projection_preserve_complete_shape() {
    for values in [
        vec![],
        vec![None; MAX_CHANNELS],
        vec![Some(r"C:\Projects\kick.wav"); MAX_CHANNELS],
    ] {
        let mut port = Port::default();
        let report = run(&input(&values), &mut port).unwrap();
        assert_eq!(report.channels().len(), values.len());
        assert!(report.to_json().unwrap().len() <= MAX_REPORT_BYTES);
        assert_eq!(
            candidate_calls(&port),
            usize::from(values.first().is_some_and(Option::is_some))
        );
    }
}

#[test]
fn malformed_and_older_capture_is_rejected_before_execution() {
    let capture = |values| {
        CapturedInput::new(
            context(),
            r"C:\Projects",
            r"C:\Projects\song.flp",
            fence(),
            values,
        )
    };
    assert!(matches!(
        capture(None),
        Err(RequestFailure::ReferencesUnavailable)
    ));
    assert!(matches!(
        capture(Some(vec![None; 257])),
        Err(RequestFailure::InvalidInput)
    ));
    for value in [
        String::new(),
        "nul\0value".into(),
        "x".repeat(4096),
        "🎵".repeat(2048),
    ] {
        assert!(matches!(
            capture(Some(vec![Some(value)])),
            Err(RequestFailure::InvalidInput)
        ));
    }
    assert!(matches!(
        capture(Some(vec![Some("x".repeat(4095)); 65])),
        Err(RequestFailure::InvalidInput)
    ));
    assert!(Context::new("C:\\private", "r", "l", "s", "h").is_err());
    assert!(Context::new(&"x".repeat(129), "r", "l", "s", "h").is_err());
}

#[test]
fn relative_raw_and_full_stack_depths_are_bounded_before_candidate_io() {
    let valid = format!("C:\\Projects\\{}\\x.wav", vec!["a"; 62].join("\\"));
    assert_eq!(AbsolutePath::parse(&valid).unwrap().components().len(), 64);
    let invalid = format!("C:\\Projects\\{}\\x.wav", vec!["a"; 63].join("\\"));
    assert!(matches!(
        AbsolutePath::parse(&invalid),
        Err(UncheckedReason::LimitReached)
    ));
    let mut port = Port::default();
    assert_eq!(
        outcomes(&run(&input(&[Some(&invalid)]), &mut port).unwrap()),
        vec![unchecked(UncheckedReason::LimitReached)]
    );
    assert_eq!(candidate_calls(&port), 0);

    // Root (2) + qualified source parent (1) + 61 raw children = 64.
    let full_64 = format!("{}\\Kick.wav", vec!["a"; 60].join("\\"));
    let full_65 = format!("{}\\Kick.wav", vec!["a"; 61].join("\\"));
    let nested_root = |reference: &str| {
        input_for(
            r"C:\One\Two",
            r"C:\One\Two\Albums\Song.flp",
            &[Some(reference)],
        )
    };
    let mut at_full_64 = Port::default();
    assert_eq!(
        outcomes(&run(&nested_root(&full_64), &mut at_full_64).unwrap()),
        vec![Outcome::NotFound]
    );
    assert_eq!(candidate_calls(&at_full_64), 1);
    let mut over_full_64 = Port::default();
    assert_eq!(
        outcomes(&run(&nested_root(&full_65), &mut over_full_64).unwrap()),
        vec![unchecked(UncheckedReason::LimitReached)]
    );
    assert_eq!(candidate_calls(&over_full_64), 0);

    // Sixty-four exact dot tokens are within the raw bound; 65 are not.
    let dots_64 = vec!["."; 64].join("\\");
    let dots_65 = vec!["."; 65].join("\\");
    let mut at_raw_64 = Port::default();
    assert_eq!(
        outcomes(&run(&input(&[Some(&dots_64)]), &mut at_raw_64).unwrap()),
        vec![unchecked(UncheckedReason::NotRegularFile)]
    );
    assert_eq!(candidate_calls(&at_raw_64), 0);
    let mut over_raw_64 = Port::default();
    assert_eq!(
        outcomes(&run(&input(&[Some(&dots_65)]), &mut over_raw_64).unwrap()),
        vec![unchecked(UncheckedReason::LimitReached)]
    );
    assert_eq!(candidate_calls(&over_raw_64), 0);

    // A temporary 65-component stack is rejected even if a later `..` would
    // have reduced the final candidate back within range.
    let source = format!("C:\\Projects\\{}\\Song.flp", vec!["d"; 62].join("\\"));
    let intermediate = input_for(r"C:\Projects", &source, &[Some(r"a\b\..\Kick.wav")]);
    let mut over_intermediate = Port::default();
    assert_eq!(
        outcomes(&run(&intermediate, &mut over_intermediate).unwrap()),
        vec![unchecked(UncheckedReason::LimitReached)]
    );
    assert_eq!(candidate_calls(&over_intermediate), 0);
}

#[test]
fn operation_exhaustion_keeps_all_slots_and_reserves_the_final_fence() {
    let mut port = Port {
        burn_operations: MAX_OPERATIONS,
        ..Port::default()
    };
    let report = run(
        &input(&[
            Some(r"C:\Projects\kick.wav"),
            Some(r"C:\Projects\other.wav"),
            None,
            Some(r"C:\Projects\kick.wav"),
        ]),
        &mut port,
    )
    .unwrap();
    assert_eq!(
        outcomes(&report),
        vec![
            unchecked(UncheckedReason::LimitReached),
            unchecked(UncheckedReason::LimitReached),
            Outcome::NoSavedReference,
            unchecked(UncheckedReason::LimitReached)
        ]
    );
    assert_eq!(candidate_calls(&port), 1);
    assert_eq!(port.calls.last().unwrap(), "revalidate");
    assert_eq!(port.final_operations, MAX_OPERATIONS - 256 + 1);
    assert_eq!(port.final_handles, 2);
}

#[test]
fn the_total_operation_cap_includes_final_revalidation_and_incomplete_fences_fail_closed() {
    let mut exact = Port {
        burn_operations: MAX_OPERATIONS,
        final_burn: 255,
        ..Port::default()
    };
    assert!(run(&input(&[Some(r"C:\Projects\kick.wav")]), &mut exact).is_ok());
    assert_eq!(exact.final_operations, MAX_OPERATIONS);
    let mut over = Port {
        burn_operations: MAX_OPERATIONS,
        final_burn: 256,
        ..Port::default()
    };
    assert!(matches!(
        run(&input(&[Some(r"C:\Projects\kick.wav")]), &mut over),
        Err(RequestFailure::Unavailable(UncheckedReason::LimitReached))
    ));
    assert!(!over.calls.contains(&"revalidate".into()));
}

#[test]
fn candidate_chain_handles_retire_before_the_next_reference_and_final_fence() {
    let mut port = Port::default();
    port.nodes.insert("a".into(), Node::Directory);
    port.nodes.insert(r"a\b".into(), Node::Directory);
    port.nodes.insert(r"a\b\x.wav".into(), Node::File);
    let report = run(
        &input(&[
            Some(r"C:\Projects\a\b\x.wav"),
            Some(r"C:\Projects\a\b\missing.wav"),
        ]),
        &mut port,
    )
    .unwrap();
    assert_eq!(outcomes(&report), vec![Outcome::Present, Outcome::NotFound]);
    assert_eq!(port.final_handles, 2);
}

#[test]
fn maximum_unchecked_report_is_bounded_with_one_slot_per_channel() {
    let mut port = Port {
        case_unknown: true,
        ..Port::default()
    };
    let report = run(
        &input(&vec![Some(r"C:\Projects\x.wav"); MAX_CHANNELS]),
        &mut port,
    )
    .unwrap();
    assert_eq!(
        outcomes(&report),
        vec![unchecked(UncheckedReason::UnsupportedCaseMode); MAX_CHANNELS]
    );
    assert!(report.to_json().unwrap().len() < MAX_REPORT_BYTES);
    assert_eq!(candidate_calls(&port), 0);
}

#[test]
fn handle_bound_counts_qualification_and_releases_tokens_on_failure() {
    let mut at_bound = Port {
        inflate_handles: MAX_HANDLES - 2,
        ..Port::default()
    };
    assert!(run(&input(&[]), &mut at_bound).is_ok());
    let mut over_bound = Port {
        inflate_handles: MAX_HANDLES - 1,
        ..Port::default()
    };
    assert!(matches!(
        run(&input(&[]), &mut over_bound),
        Err(RequestFailure::Unavailable(UncheckedReason::LimitReached))
    ));
    assert!(over_bound.calls.is_empty());
}

#[test]
fn admission_cancel_and_invalidation_prevent_io_without_releasing_the_gate() {
    for stale in [false, true] {
        let clock = Arc::new(Clock::default());
        let gate = WorkerGate::default();
        let mut lease = gate.try_start(clock.clone()).unwrap();
        let control = lease.control();
        if stale {
            control.invalidate();
        } else {
            control.cancel();
        }
        let mut port = Port::default();
        assert!(matches!(
            check(
                &input(&[Some(r"C:\Projects\kick.wav")]),
                &mut lease,
                &mut port,
                0
            ),
            Err(RequestFailure::Stale | RequestFailure::Cancelled)
        ));
        assert!(port.calls.is_empty());
        assert!(matches!(
            gate.try_start(clock.clone()),
            Err(AdmissionError::Busy)
        ));
        drop(lease);
        assert!(gate.try_start(clock).is_ok());
    }
}

#[test]
fn cancellation_after_an_operation_fences_its_result_and_every_subsequent_operation() {
    let gate = WorkerGate::default();
    let mut lease = gate.try_start(Arc::new(Clock::default())).unwrap();
    let mut port = Port {
        invalidate_on_child: true,
        control: Some(lease.control()),
        ..Port::default()
    };
    assert!(matches!(
        check(
            &input(&[
                Some(r"C:\Projects\kick.wav"),
                Some(r"C:\Projects\other.wav")
            ]),
            &mut lease,
            &mut port,
            0
        ),
        Err(RequestFailure::Stale)
    ));
    assert_eq!(candidate_calls(&port), 1);
    assert!(!port.calls.contains(&"revalidate".into()));
}

#[test]
fn deadline_before_and_after_operations_never_publishes_an_unfenced_report() {
    let clock = Arc::new(Clock::default());
    let gate = WorkerGate::default();
    let mut lease = gate.try_start(clock.clone()).unwrap();
    clock.0.store(DEADLINE_MS, Ordering::SeqCst);
    let mut port = Port::default();
    assert!(matches!(
        check(&input(&[]), &mut lease, &mut port, 0),
        Err(RequestFailure::Deadline)
    ));
    assert!(port.calls.is_empty());
    drop(lease);
    clock.0.store(0, Ordering::SeqCst);
    let mut lease = gate.try_start(clock.clone()).unwrap();
    let mut port = Port {
        advance_on_child: true,
        clock: Some(clock),
        ..Port::default()
    };
    assert!(matches!(
        check(
            &input(&[Some(r"C:\Projects\kick.wav")]),
            &mut lease,
            &mut port,
            0
        ),
        Err(RequestFailure::Deadline)
    ));
    assert_eq!(candidate_calls(&port), 1);
}

#[test]
fn a_blocked_call_remains_the_only_worker_after_timeout_until_it_retires() {
    let clock = Arc::new(Clock::default());
    let gate = WorkerGate::default();
    let mut lease = gate.try_start(clock.clone()).unwrap();
    let control = lease.control();
    let (started_tx, started_rx) = channel();
    let (resume_tx, resume_rx) = channel();
    let worker = std::thread::spawn(move || {
        let mut port = Port {
            blocked: Some((started_tx, resume_rx)),
            ..Port::default()
        };
        let result = check(
            &input(&[Some(r"C:\Projects\kick.wav")]),
            &mut lease,
            &mut port,
            0,
        );
        assert!(matches!(result, Err(RequestFailure::Deadline)));
        assert_eq!(candidate_calls(&port), 1);
        // Lease still exists here; the root/source guards survived the syscall.
    });
    started_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    clock.0.store(DEADLINE_MS, Ordering::SeqCst);
    assert_eq!(control.poll_deadline(), Err(RequestFailure::Deadline));
    assert!(matches!(
        gate.try_start(clock.clone()),
        Err(AdmissionError::Busy)
    ));
    resume_tx.send(()).unwrap();
    worker.join().unwrap();
    assert!(gate.try_start(clock).is_ok());
}

#[test]
fn one_lease_cannot_retry_a_failed_request_and_reset_its_budgets() {
    let gate = WorkerGate::default();
    let mut lease = gate.try_start(Arc::new(Clock::default())).unwrap();
    let mut port = Port {
        qualification_error: Some(PortError::Unchecked(UncheckedReason::AccessDenied)),
        ..Port::default()
    };
    assert!(check(&input(&[]), &mut lease, &mut port, 0).is_err());
    port.qualification_error = None;
    assert!(matches!(
        check(&input(&[]), &mut lease, &mut port, 0),
        Err(RequestFailure::WorkerRetired)
    ));
    assert_eq!(port.calls.len(), 1);
}

#[test]
fn shutdown_closes_admission_and_invalidates_pending_publication() {
    let gate = WorkerGate::default();
    let clock = Arc::new(Clock::default());
    let mut lease = gate.try_start(clock.clone()).unwrap();
    gate.shutdown();
    assert!(matches!(
        gate.try_start(clock.clone()),
        Err(AdmissionError::Closed)
    ));
    assert!(matches!(
        check(&input(&[]), &mut lease, &mut Port::default(), 0),
        Err(RequestFailure::Stale)
    ));
    drop(lease);
    assert!(matches!(gate.try_start(clock), Err(AdmissionError::Closed)));
}

#[test]
fn report_contains_only_fixed_outcomes_and_correlation_without_saved_paths() {
    let sentinel = r"C:\Projects\PRIVATE_SENTINEL.wav";
    let report = run(&input(&[Some(sentinel), None]), &mut Port::default()).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&report.to_json().unwrap()).unwrap();
    assert_eq!(value["version"], 1);
    assert_eq!(value["context"]["snapshot_id"], "snapshot-1");
    assert_eq!(
        value["channels"][0],
        serde_json::json!({"position":1,"status":"not_found"})
    );
    assert_eq!(
        value["channels"][1],
        serde_json::json!({"position":2,"status":"no_saved_reference"})
    );
    let encoded = value.to_string();
    assert!(!encoded.contains("PRIVATE_SENTINEL"));
    assert!(!encoded.contains("path"));
    assert!(!encoded.contains("modified_at"));
    assert!(!encoded.contains("volume"));
    let gate = WorkerGate::default();
    let mut lease = gate.try_start(Arc::new(Clock::default())).unwrap();
    let mut port = Port::default();
    assert!(matches!(
        check(&input(&[]), &mut lease, &mut port, u64::MAX),
        Err(RequestFailure::InvalidInput)
    ));
    assert!(port.calls.is_empty());
}
