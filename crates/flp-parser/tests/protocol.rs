use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

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
        "protocolVersion":1,"schemaVersion":2,"id":ID,"method":"describe"
    });
    let health = json!({
        "protocolVersion":1,"schemaVersion":2,"id":ID,"method":"healthCheck"
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
    assert!(
        responses[0]["result"]["fields"]
            .as_array()
            .unwrap()
            .contains(&json!("patternNoteCounts"))
    );
    assert_eq!(responses[0]["result"]["maxNoteRecordsPerPattern"], 65536);
    assert_eq!(responses[0]["result"]["maxNoteRecordsTotal"], 262144);
    for field in ["mixerInsertCount", "mixerInsertNames"] {
        assert!(
            responses[0]["result"]["fields"]
                .as_array()
                .unwrap()
                .contains(&json!(field))
        );
    }
    assert_eq!(responses[0]["result"]["maxMixerInserts"], 512);
    assert_eq!(responses[0]["result"]["maxMixerInsertCandidates"], 512);
    assert_eq!(responses[0]["result"]["maxFileBytes"], 67_108_864);
    assert_eq!(responses[0]["result"]["maxEventBytes"], 67_108_864);
    assert!(
        fruitboard_flp_parser::validation::validate_descriptor(&responses[0]["result"]).is_ok()
    );
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
        "protocolVersion":2,"schemaVersion":2,"id":ID,"method":"describe"
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
        .join("fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp")
        .canonicalize()
        .unwrap();
    let metadata = fs::metadata(&path).unwrap();
    let before = fs::read(&path).unwrap();
    let allowed_root = path.parent().unwrap().to_path_buf();
    let request = json!({
        "protocolVersion":1,"schemaVersion":2,"id":ID,"method":"parse",
        "params":{
            "path":path,
            "expected":{
                "size":metadata.len(),
                "modifiedAtMs":fruitboard_flp_parser::modified_at_ms(&metadata).unwrap()
            },
            "features":["basic-metadata"],
            "allowedRoots":[allowed_root]
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
    // The reported content digest tracks the exact read buffer: it is the
    // SHA-256 of the bytes as they were before and after parsing.
    assert_eq!(
        responses[0]["result"]["inputFingerprint"]["hash"]["algorithm"],
        "sha256"
    );
    assert_eq!(
        responses[0]["result"]["inputFingerprint"]["hash"]["value"],
        fruitboard_flp_parser::sha256_hex(&before)
    );
    assert!(
        !responses[0]
            .to_string()
            .contains(&path.to_string_lossy().to_string())
    );
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn parse_refuses_paths_outside_the_allowed_roots() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp")
        .canonicalize()
        .unwrap();
    let metadata = fs::metadata(&path).unwrap();
    let expected = json!({
        "size":metadata.len(),
        "modifiedAtMs":fruitboard_flp_parser::modified_at_ms(&metadata).unwrap()
    });

    // A sibling directory whose name merely shares a prefix must not match
    // (component-wise containment), and an empty allowlist denies everything.
    let sibling_root = path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("parser-corpus-evil");
    let outside_root = path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("other-corpus");
    for allowed in [
        json!([sibling_root]),
        json!([outside_root]),
        json!([]),
        Value::Null,
    ] as [Value; 4]
    {
        let mut params = json!({
            "path":path,
            "expected":expected.clone(),
            "features":["basic-metadata"]
        });
        if allowed != Value::Null {
            params["allowedRoots"] = allowed;
        }
        let request = json!({
            "protocolVersion":1,"schemaVersion":2,"id":ID,"method":"parse",
            "params":params
        });
        let (responses, stderr) = exchange(&[serde_json::to_vec(&request).unwrap()]);
        assert!(stderr.is_empty());
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0]["error"]["code"], "INVALID_PATH");
    }

    // The file itself is untouched by refusals.
    assert!(path.exists());

    // Prefix-sibling attack: `parser-corpus-evil` shares a string prefix
    // with the allowed root `parser-corpus` but is not inside it.
    let allowed_root = path.parent().unwrap().to_path_buf();
    let sibling_path = path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("parser-corpus-evil")
        .join("sneaky.flp");
    let request = json!({
        "protocolVersion":1,"schemaVersion":2,"id":ID,"method":"parse",
        "params":{
            "path":sibling_path,
            "expected":expected.clone(),
            "features":["basic-metadata"],
            "allowedRoots":[allowed_root]
        }
    });
    let (responses, stderr) = exchange(&[serde_json::to_vec(&request).unwrap()]);
    assert!(stderr.is_empty());
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0]["error"]["code"], "INVALID_PATH");
}

struct PathFixture(PathBuf);
static NEXT_PATH_FIXTURE: AtomicU64 = AtomicU64::new(0);

impl PathFixture {
    fn new() -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Self::at_timestamp(unique)
    }

    fn at_timestamp(unique: u128) -> Self {
        let sequence = NEXT_PATH_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "fruitboard-parser-path-{}-{unique}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        let path = path.canonicalize().unwrap();
        #[cfg(windows)]
        let path = PathBuf::from(
            path.to_str()
                .unwrap()
                .strip_prefix(r"\\?\")
                .unwrap_or(path.to_str().unwrap()),
        );
        fs::create_dir_all(path.join("allowed/nested")).unwrap();
        fs::create_dir(path.join("outside")).unwrap();
        let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/parser-corpus/FIX-FL2026-SAMPLE.flp");
        fs::copy(&corpus, path.join("allowed/nested/inside.flp")).unwrap();
        fs::copy(corpus, path.join("outside/outside.flp")).unwrap();
        Self(path)
    }
}

impl Drop for PathFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn concurrent_path_fixtures_are_isolated_when_the_clock_collides() {
    let fixtures = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| PathFixture::at_timestamp(0)))
            .collect();
        threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect::<Vec<_>>()
    });
    let paths: std::collections::HashSet<_> = fixtures.iter().map(|fixture| &fixture.0).collect();
    assert_eq!(paths.len(), 8);
    for fixture in &fixtures {
        assert!(fixture.0.join("allowed/nested/inside.flp").is_file());
        assert!(fixture.0.join("outside/outside.flp").is_file());
    }
}

fn path_request(
    path: &std::path::Path,
    root: &std::path::Path,
    fingerprint_file: &std::path::Path,
) -> Vec<u8> {
    let metadata = fs::metadata(fingerprint_file).unwrap();
    serde_json::to_vec(&json!({
        "protocolVersion":1, "schemaVersion":2, "id":ID, "method":"parse",
        "params":{
            "path":path, "allowedRoots":[root], "features":["basic-metadata"],
            "expected":{"size":metadata.len(), "modifiedAtMs":fruitboard_flp_parser::modified_at_ms(&metadata).unwrap()}
        }
    })).unwrap()
}

#[test]
fn nested_file_is_accepted_but_parent_traversal_is_rejected() {
    let fixture = PathFixture::new();
    let root = fixture.0.join("allowed");
    let inside = root.join("nested/inside.flp");
    let outside = fixture.0.join("outside/outside.flp");
    let escaped = root.join("../outside/outside.flp");
    assert!(
        escaped.starts_with(&root),
        "regression for the old predicate"
    );
    let before = fs::read(&outside).unwrap();
    let (responses, stderr) = exchange(&[
        path_request(&inside, &root, &inside),
        path_request(&escaped, &root, &outside),
        path_request(&outside, &root.join(".."), &outside),
    ]);
    assert!(stderr.is_empty());
    assert_eq!(responses[0]["result"]["outcome"], "complete");
    for response in &responses[1..] {
        assert_eq!(response["error"]["code"], "INVALID_PATH");
        assert!(
            !response
                .to_string()
                .contains(&fixture.0.to_string_lossy().to_string())
        );
    }
    assert_eq!(fs::read(&outside).unwrap(), before);
}

#[test]
fn linked_directory_and_linked_root_are_rejected() {
    let fixture = PathFixture::new();
    let root = fixture.0.join("allowed");
    let outside = fixture.0.join("outside");
    let alias = root.join("alias");
    #[cfg(windows)]
    {
        let output = Command::new("cmd")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&alias)
            .arg(&outside)
            .output()
            .unwrap();
        assert!(output.status.success(), "fixture junction creation failed");
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, &alias).unwrap();
    let linked = alias.join("outside.flp");
    let actual = outside.join("outside.flp");
    let (responses, stderr) = exchange(&[
        path_request(&linked, &root, &actual),
        path_request(&linked, &alias, &actual),
    ]);
    assert!(stderr.is_empty());
    for response in responses {
        assert_eq!(response["error"]["code"], "INVALID_PATH");
    }
}

#[cfg(unix)]
#[test]
fn linked_leaf_is_rejected_even_when_target_is_inside_root() {
    let fixture = PathFixture::new();
    let root = fixture.0.join("allowed");
    let inside = root.join("nested/inside.flp");
    let alias = root.join("alias.flp");
    std::os::unix::fs::symlink(&inside, &alias).unwrap();
    let (responses, stderr) = exchange(&[path_request(&alias, &root, &inside)]);
    assert!(stderr.is_empty());
    assert_eq!(responses[0]["error"]["code"], "INVALID_PATH");
}

#[cfg(windows)]
#[test]
fn alternate_stream_and_win32_parent_alias_are_rejected() {
    let fixture = PathFixture::new();
    let root = fixture.0.join("allowed");
    let inside = root.join("nested/inside.flp");
    let (responses, stderr) = exchange(&[
        path_request(&root.join("nested/inside.flp:secret"), &root, &inside),
        path_request(&root.join(".. /outside/outside.flp"), &root, &inside),
    ]);
    assert!(stderr.is_empty());
    for response in responses {
        assert_eq!(response["error"]["code"], "INVALID_PATH");
    }
}
