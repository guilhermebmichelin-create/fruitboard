//! Versioned JSON-lines transport for the selected Rust FLP parser.
//! This process receives requests only from a trusted native supervisor.

use fruitboard_flp_parser::{
    ExpectedFingerprint, MAX_FILE_BYTES, MAX_PATTERNS, MAX_PLAYLIST_CLIPS, parse_authorized_file,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};
use std::path::Path;

use fruitboard_flp_parser::{ADAPTER_ID, ADAPTER_VERSION, PROTOCOL_VERSION, SCHEMA_VERSION};
const MAX_REQUEST_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    protocol_version: u64,
    schema_version: u64,
    id: String,
    method: String,
    params: Option<ParseParams>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParseParams {
    path: String,
    expected: ExpectedFingerprint,
    #[serde(default)]
    features: Vec<String>,
    /// Enabled scan roots supplied by the trusted supervisor. `parse` is
    /// refused unless the file path lies inside one of them; an empty list
    /// denies every path.
    #[serde(default)]
    allowed_roots: Vec<String>,
}

enum LineRead {
    End,
    Line(Vec<u8>),
    TooLong,
}

fn read_line_bounded(reader: &mut impl BufRead) -> io::Result<LineRead> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(LineRead::End)
            } else {
                Ok(LineRead::Line(line))
            };
        }
        let used = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        if line.len().saturating_add(used) > MAX_REQUEST_BYTES {
            return Ok(LineRead::TooLong);
        }
        let has_newline = available[used - 1] == b'\n';
        line.extend_from_slice(&available[..used]);
        reader.consume(used);
        if has_newline {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return Ok(LineRead::Line(line));
        }
    }
}

fn valid_request_id(id: &str) -> bool {
    if id.len() != 36 {
        return false;
    }
    id.bytes().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        }
    })
}

fn error(id: Value, code: &'static str) -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "schemaVersion": SCHEMA_VERSION,
        "id": id,
        "error": {"code": code}
    })
}

fn respond(line: &[u8]) -> Value {
    let Ok(request) = serde_json::from_slice::<Request>(line) else {
        return error(Value::Null, "INVALID_REQUEST");
    };
    if !valid_request_id(&request.id) {
        return error(Value::Null, "INVALID_REQUEST_ID");
    }
    let id = json!(request.id);
    if request.protocol_version != PROTOCOL_VERSION {
        return error(id, "UNSUPPORTED_PROTOCOL_VERSION");
    }
    if request.schema_version != SCHEMA_VERSION {
        return error(id, "UNSUPPORTED_SCHEMA_VERSION");
    }
    let result = match request.method.as_str() {
        "describe" if request.params.is_none() => json!({
            "adapter":ADAPTER_ID, "adapterVersion":ADAPTER_VERSION,
            "fields":["savedVersion","baseTempoBpm","channelCount","patternCount","patternNames","patternNoteCounts","playlistPatternClips","playlistPatternEndTick","playlistPatternNominalSeconds","playlistPatternSpanBars","channelNames","channelGeneratorNames","sampleReferences","projectCreatedLocal","flStudioTimeSpentMs","pluginReferences","filesystemCreatedAtMs"],
            "maxFileBytes":MAX_FILE_BYTES, "maxEvents":100_000,
            "maxChannels":256, "maxPatterns":MAX_PATTERNS, "maxPlaylistClips":MAX_PLAYLIST_CLIPS,
            "maxNoteRecordsPerPattern":fruitboard_flp_parser::MAX_NOTE_RECORDS_PER_PATTERN,
            "maxNoteRecordsTotal":fruitboard_flp_parser::MAX_NOTE_RECORDS_TOTAL,
            "maxEventBytes":fruitboard_flp_parser::MAX_EVENT_BYTES
        }),
        "healthCheck" if request.params.is_none() => json!({"status":"ok"}),
        "parse" => {
            let Some(params) = request.params else {
                return error(id, "INVALID_REQUEST");
            };
            if !params.features.is_empty() && params.features != ["basic-metadata"] {
                return error(id, "UNSUPPORTED_FEATURE");
            }
            let path = Path::new(&params.path);
            let started = std::time::Instant::now();
            let mut result =
                match parse_authorized_file(path, &params.allowed_roots, params.expected) {
                    Ok(result) => result,
                    Err(code) => return error(id, code),
                };
            result["parseElapsedMicros"] = json!(started.elapsed().as_micros());
            result
        }
        _ => return error(id, "UNKNOWN_METHOD"),
    };
    json!({
        "protocolVersion":PROTOCOL_VERSION,
        "schemaVersion":SCHEMA_VERSION,
        "id":id,
        "result":result
    })
}

fn write_response(writer: &mut impl Write, response: Value) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(&response)?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        bytes = serde_json::to_vec(&error(response["id"].clone(), "RESPONSE_LIMIT"))?;
    }
    bytes.push(b'\n');
    writer.write_all(&bytes)?;
    writer.flush()
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    loop {
        match read_line_bounded(&mut reader)? {
            LineRead::End => break,
            LineRead::Line(line) => write_response(&mut writer, respond(&line))?,
            LineRead::TooLong => {
                write_response(&mut writer, error(Value::Null, "REQUEST_LIMIT"))?;
                break;
            }
        }
    }
    Ok(())
}
