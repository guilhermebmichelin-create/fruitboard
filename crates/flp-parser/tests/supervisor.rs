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
