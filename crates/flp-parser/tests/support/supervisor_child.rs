// Standalone fault fixture compiled once by the supervisor unit tests. It has
// no parser or file-input authority and uses only the Rust standard library.
use std::io::{self, BufRead, Write};
use std::time::Duration;

fn main() {
    let mode = std::env::args().nth(1).unwrap();
    let marker = std::env::args_os().nth(2);
    if mode == "blocked-input" {
        if let Some(marker) = &marker {
            std::fs::write(marker, b"started").unwrap();
        }
        std::thread::sleep(Duration::from_secs(120));
        return;
    }
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let id_start = line.find("\"id\":\"").unwrap() + 6;
        let id = &line[id_start..id_start + 36];
        if let Some(marker) = &marker {
            std::fs::write(marker, b"request received").unwrap();
        }
        match mode.as_str() {
            "crash" => std::process::exit(7),
            "stall" => std::thread::sleep(Duration::from_secs(120)),
            "oversized" => {
                stdout.write_all(&vec![b'x'; 256 * 1024 + 2]).unwrap();
                stdout.flush().unwrap();
            }
            "truncated" => {
                stdout.write_all(b"{\"protocolVersion\":1").unwrap();
                return;
            }
            "malformed" => writeln!(stdout, "not json").unwrap(),
            "wrong-id" => writeln!(stdout, "{{\"protocolVersion\":1,\"schemaVersion\":2,\"id\":\"00000000-0000-0000-0000-ffffffffffff\",\"result\":{{}}}}").unwrap(),
            "wrong-version" => writeln!(stdout, "{{\"protocolVersion\":2,\"schemaVersion\":2,\"id\":\"{id}\",\"result\":{{}}}}").unwrap(),
            "wrong-schema" => writeln!(stdout, "{{\"protocolVersion\":1,\"schemaVersion\":3,\"id\":\"{id}\",\"result\":{{}}}}").unwrap(),
            "ambiguous" => writeln!(stdout, "{{\"protocolVersion\":1,\"schemaVersion\":2,\"id\":\"{id}\",\"result\":{{}},\"error\":{{\"code\":\"INPUT_CHANGED\"}}}}").unwrap(),
            "missing-result" => writeln!(stdout, "{{\"protocolVersion\":1,\"schemaVersion\":2,\"id\":\"{id}\"}}").unwrap(),
            "null-result" => writeln!(stdout, "{{\"protocolVersion\":1,\"schemaVersion\":2,\"id\":\"{id}\",\"result\":null}}").unwrap(),
            "private-error" => writeln!(stdout, "{{\"protocolVersion\":1,\"schemaVersion\":2,\"id\":\"{id}\",\"error\":{{\"code\":\"C:\\\\private\\\\project.flp\"}}}}").unwrap(),
            "rejection" => writeln!(stdout, "{{\"protocolVersion\":1,\"schemaVersion\":2,\"id\":\"{id}\",\"error\":{{\"code\":\"INPUT_CHANGED\"}}}}").unwrap(),
            _ => {
                if mode == "stderr-flood" {
                    io::stderr().write_all(&vec![b'x'; 1024 * 1024]).unwrap();
                }
                writeln!(stdout, "{{\"protocolVersion\":1,\"schemaVersion\":2,\"id\":\"{id}\",\"result\":{{\"pid\":{},\"status\":\"ok\"}}}}", std::process::id()).unwrap();
            }
        }
        stdout.flush().unwrap();
    }
}
