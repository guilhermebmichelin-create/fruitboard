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
    let description = value(
        supervisor
            .request(ParserRequest::Describe, &cancellation)
            .unwrap(),
    );
    assert_eq!(description["adapter"], "rust-flp-parser");
    let health = value(
        supervisor
            .request(ParserRequest::HealthCheck, &cancellation)
            .unwrap(),
    );
    assert_eq!(health["status"], "ok");
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
}

#[test]
fn actual_parser_corpus_replies_pass_initial_metadata_validation_without_mutation() {
    use fruitboard_flp_parser::validation::{
        MetadataOutcome, ParseContext, ValidatedReply, validate_descriptor, validate_reply,
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
    assert_eq!(fixtures.len(), 12, "only the maintained approved corpus");
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
        let validated = validate_reply(reply, &capabilities, &context, &context).unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        let expected_failure = match name {
            "FIX-RB-TRUNC.flp" => Some("TRUNCATED_DATA_CHUNK"),
            "FIX-RB-MALFORM.flp" => Some("EVENT_LENGTH_OUT_OF_BOUNDS"),
            "FIX-RB-LIMIT.flp" => Some("CHANNEL_COUNT_LIMIT"),
            _ => None,
        };
        if let Some(expected) = expected_failure {
            assert!(
                matches!(validated, ValidatedReply::Failed(code) if code.as_str() == expected),
                "{name}"
            );
        } else {
            let ValidatedReply::Metadata(metadata) = validated else {
                panic!("approved valid fixture must retain metadata: {name}");
            };
            assert_eq!(
                metadata.outcome(),
                if name == "FIX-RB-UNKNOWN.flp" {
                    MetadataOutcome::Partial
                } else {
                    MetadataOutcome::Complete
                },
                "{name}"
            );
        }
        assert_eq!(
            fs::read(path).unwrap(),
            before,
            "approved bytes must remain unchanged"
        );
    }
    supervisor.shutdown().unwrap();
}
