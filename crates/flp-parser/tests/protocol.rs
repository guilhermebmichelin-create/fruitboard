use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const ID: &str = "01234567-89ab-cdef-0123-456789abcdef";

fn exchange(lines: &[Vec<u8>]) -> (Vec<Value>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fruitboard-flp-parser"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        for line in lines {
            stdin.write_all(line).unwrap();
            stdin.write_all(b"\n").unwrap();
        }
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    (responses, String::from_utf8(output.stderr).unwrap())
}

#[test]
fn versioned_protocol_handles_multiple_requests_without_stderr() {
    let describe = json!({
        "protocolVersion":1,"schemaVersion":1,"id":ID,"method":"describe"
    });
    let health = json!({
        "protocolVersion":1,"schemaVersion":1,"id":ID,"method":"healthCheck"
    });
    let (responses, stderr) = exchange(&[
        serde_json::to_vec(&describe).unwrap(),
        serde_json::to_vec(&health).unwrap(),
    ]);
    assert!(stderr.is_empty());
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["result"]["adapter"], "rust-flp-parser");
    assert!(
        responses[0]["result"]["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("channelCount"))
    );
    assert!(
        responses[0]["result"]["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("patternCount"))
    );
    assert_eq!(responses[0]["result"]["maxPatterns"], 1024);
    assert_eq!(responses[0]["result"]["maxPlaylistClips"], 1024);
    for field in ["playlistPatternEndTick", "playlistPatternNominalSeconds"] {
        assert!(
            responses[0]["result"]["fields"]
                .as_array()
                .unwrap()
                .contains(&json!(field))
        );
    }
    assert!(
        responses[0]["result"]["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("playlistPatternClips"))
    );
    assert!(
        responses[0]["result"]["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("patternNames"))
    );
    assert!(
        responses[0]["result"]["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("channelGeneratorNames"))
    );
    assert_eq!(responses[1]["result"]["status"], "ok");
    assert_eq!(responses[0]["id"], ID);
}

#[test]
fn protocol_rejects_invalid_versions_and_overlong_requests() {
    let wrong_version = json!({
        "protocolVersion":2,"schemaVersion":1,"id":ID,"method":"describe"
    });
    let (responses, stderr) = exchange(&[
        serde_json::to_vec(&wrong_version).unwrap(),
        vec![b'x'; 64 * 1024 + 1],
    ]);
    assert!(stderr.is_empty());
    assert_eq!(responses.len(), 2);
    assert_eq!(
        responses[0]["error"]["code"],
        "UNSUPPORTED_PROTOCOL_VERSION"
    );
    assert_eq!(responses[1]["error"]["code"], "REQUEST_LIMIT");
    assert_eq!(responses[1]["id"], Value::Null);
}

#[test]
fn parse_request_returns_f12_fields_without_echoing_input_path() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp");
    let metadata = fs::metadata(&path).unwrap();
    let before = fs::read(&path).unwrap();
    let request = json!({
        "protocolVersion":1,"schemaVersion":1,"id":ID,"method":"parse",
        "params":{
            "path":path,
            "expected":{
                "size":metadata.len(),
                "modifiedAtMs":fruitboard_flp_parser::modified_at_ms(&metadata).unwrap()
            },
            "features":["basic-metadata"]
        }
    });
    let (responses, stderr) = exchange(&[serde_json::to_vec(&request).unwrap()]);
    assert!(stderr.is_empty());
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["result"]["outcome"], "complete");
    assert_eq!(responses[0]["result"]["baseTempoBpm"]["value"], 137.0);
    assert_eq!(responses[0]["result"]["channelCount"]["value"], 1);
    assert_eq!(
        responses[0]["result"]["channelNames"]["value"][0],
        "Fixture Sample A"
    );
    assert_eq!(
        responses[0]["result"]["sampleReferences"]["status"],
        "extracted"
    );
    assert!(
        !responses[0]
            .to_string()
            .contains(&path.to_string_lossy().to_string())
    );
    assert_eq!(fs::read(&path).unwrap(), before);
}
