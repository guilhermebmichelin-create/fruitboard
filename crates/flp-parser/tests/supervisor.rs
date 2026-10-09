use fruitboard_flp_parser::supervisor::{
    CancellationToken, ParseRequest, ParserRequest, ParserSupervisor, ProtocolReply,
    SupervisorLimits,
};
use fruitboard_flp_parser::{ExpectedFingerprint, modified_at_ms, sha256_hex};
use std::fs;
use std::path::PathBuf;

fn value(reply: ProtocolReply) -> serde_json::Value {
    match reply {
        ProtocolReply::Result(value) => value,
        ProtocolReply::Rejected { .. } => panic!("expected result"),
    }
}

#[test]
fn actual_parser_health_describe_and_approved_file_share_one_supervisor() {
    let mut supervisor = ParserSupervisor::new(
        PathBuf::from(env!("CARGO_BIN_EXE_fruitboard-flp-parser")),
        SupervisorLimits::default(),
    )
    .unwrap();
    let cancellation = CancellationToken::default();
    assert_eq!(supervisor.process_id(), None);
    let description = value(
        supervisor
            .request(ParserRequest::Describe, &cancellation)
            .unwrap(),
    );
    assert_eq!(description["adapter"], "rust-flp-parser");
    let child_id = supervisor.process_id().unwrap();
    let health = value(
        supervisor
            .request(ParserRequest::HealthCheck, &cancellation)
            .unwrap(),
    );
    assert_eq!(health["status"], "ok");
    assert_eq!(supervisor.process_id(), Some(child_id));
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp")
        .canonicalize()
        .unwrap();
    let before = fs::read(&path).unwrap();
    let metadata = fs::metadata(&path).unwrap();
    let request = || {
        ParserRequest::Parse(ParseRequest {
            path: path.clone(),
            expected: ExpectedFingerprint {
                size: metadata.len(),
                modified_at_ms: modified_at_ms(&metadata).unwrap(),
            },
            allowed_roots: vec![path.parent().unwrap().to_path_buf()],
        })
    };
    let parsed = value(supervisor.request(request(), &cancellation).unwrap());
    assert_eq!(parsed["outcome"], "complete");
    assert_eq!(parsed["baseTempoBpm"]["value"], 137.0);
    assert_eq!(
        parsed["inputFingerprint"]["hash"]["value"],
        sha256_hex(&before)
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    let denied = ParserRequest::Parse(ParseRequest {
        path,
        expected: ExpectedFingerprint {
            size: 1,
            modified_at_ms: 1,
        },
        allowed_roots: vec![],
    });
    assert!(
        matches!(supervisor.request(denied, &cancellation).unwrap(), ProtocolReply::Rejected { code } if code == "INVALID_PATH")
    );
    assert_eq!(
        value(
            supervisor
                .request(ParserRequest::HealthCheck, &cancellation)
                .unwrap()
        )["status"],
        "ok"
    );
    supervisor.shutdown().unwrap();
    assert_eq!(supervisor.process_id(), None);
    assert_eq!(
        value(
            supervisor
                .request(ParserRequest::HealthCheck, &cancellation)
                .unwrap()
        )["status"],
        "ok"
    );
    assert!(supervisor.process_id().is_some());
    supervisor.shutdown().unwrap();
    assert_eq!(supervisor.process_id(), None);
}

#[test]
fn actual_parser_corpus_replies_pass_project_metadata_validation_without_mutation() {
    use fruitboard_flp_parser::validation::{
        MetadataOutcome, MixerInsertName, NoteRecordCount, ParseContext, PatternNoteCounts,
        SavedMixerInserts, ValidatedProjectReply, validate_descriptor, validate_project_reply,
    };
    let mut supervisor = ParserSupervisor::new(
        PathBuf::from(env!("CARGO_BIN_EXE_fruitboard-flp-parser")),
        SupervisorLimits::default(),
    )
    .unwrap();
    let cancellation = CancellationToken::default();
    let descriptor = value(
        supervisor
            .request(ParserRequest::Describe, &cancellation)
            .unwrap(),
    );
    let capabilities = validate_descriptor(&descriptor).unwrap();
    let mixer_expectations: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/parser-corpus/mixer-insert-expectations.json"
    ))
    .unwrap();
    let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/parser-corpus")
        .canonicalize()
        .unwrap();
    let mut fixtures = fs::read_dir(&corpus)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "flp"))
        .collect::<Vec<_>>();
    fixtures.sort();
    assert_eq!(fixtures.len(), 19, "only the maintained approved corpus");
    for (index, path) in fixtures.into_iter().enumerate() {
        let before = fs::read(&path).unwrap();
        let attributes = fs::metadata(&path).unwrap();
        let expected = ExpectedFingerprint {
            size: attributes.len(),
            modified_at_ms: modified_at_ms(&attributes).unwrap(),
        };
        let context = ParseContext {
            root_id: "approved-corpus".into(),
            file_id: format!("fixture-{index}"),
            root_revision: 1,
            file_revision: 1,
            root_enabled: true,
            expected,
            content_sha256: Some(sha256_hex(&before)),
        };
        let reply = supervisor
            .request(
                ParserRequest::Parse(ParseRequest {
                    path: path.clone(),
                    expected,
                    allowed_roots: vec![corpus.clone()],
                }),
                &cancellation,
            )
            .unwrap();
        let validated = validate_project_reply(reply, &capabilities, &context, &context).unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        let expected_failure = match name {
            "FIX-RB-TRUNC.flp" => Some("TRUNCATED_DATA_CHUNK"),
            "FIX-RB-MALFORM.flp" => Some("EVENT_LENGTH_OUT_OF_BOUNDS"),
            "FIX-RB-LIMIT.flp" => Some("CHANNEL_COUNT_LIMIT"),
            _ => None,
        };
        if let Some(expected) = expected_failure {
            assert!(
                matches!(validated, ValidatedProjectReply::Failed(code) if code.as_str() == expected),
                "{name}"
            );
        } else {
            let ValidatedProjectReply::Metadata(metadata) = validated else {
                panic!("approved valid fixture must retain metadata: {name}");
            };
            assert_eq!(
                metadata.initial().outcome(),
                if name == "FIX-RB-UNKNOWN.flp" {
                    MetadataOutcome::Partial
                } else {
                    MetadataOutcome::Complete
                },
                "{name}"
            );
            assert_eq!(metadata.file_size_bytes(), expected.size);
            if let Some(case) = mixer_expectations["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|case| case["file"] == name)
            {
                assert_eq!(sha256_hex(&before), case["sha256"].as_str().unwrap());
                let SavedMixerInserts::Entries(items) = metadata.mixer_inserts() else {
                    panic!("complete qualified mixer required: {name}")
                };
                let observed = case["savedSection"]["ordinaryRecords"].as_array().unwrap();
                assert_eq!(items.len(), 16);
                for (item, registered) in items.iter().zip(observed) {
                    assert_eq!(
                        u64::from(item.id()),
                        registered["sectionRecordOrdinal"].as_u64().unwrap()
                    );
                    match item.name() {
                        MixerInsertName::Extracted(value) => {
                            assert_eq!(value, registered["name"]["value"].as_str().unwrap())
                        }
                        MixerInsertName::Unavailable => {
                            assert_eq!(registered["name"]["status"], "unavailable")
                        }
                    }
                }
            }
            if name == "FIX-FL2026-NOTES.flp" {
                let PatternNoteCounts::Entries(items) = metadata.pattern_note_counts() else {
                    panic!("typed note counts expected")
                };
                assert_eq!(
                    items
                        .iter()
                        .map(|item| match item.count() {
                            NoteRecordCount::Extracted(n) => (item.id(), *n),
                            _ => panic!("qualified count expected"),
                        })
                        .collect::<Vec<_>>(),
                    [(1, 3), (2, 2), (3, 4)]
                );
            } else if name == "FIX-FL2026-NAMED-EMPTY.flp" {
                let PatternNoteCounts::Entries(items) = metadata.pattern_note_counts() else {
                    panic!("typed empty pattern expected")
                };
                assert!(matches!(items[0].count(), NoteRecordCount::Unavailable));
            }
            assert_eq!(metadata.file_modified_at_ms(), expected.modified_at_ms);
            assert!(metadata.project_created_local().value().is_some());
            assert!(metadata.fl_studio_time_spent_ms().value().is_some());
        }
        assert_eq!(
            fs::read(path).unwrap(),
            before,
            "approved bytes must remain unchanged"
        );
    }
    supervisor.shutdown().unwrap();
}
