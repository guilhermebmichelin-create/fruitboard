//! Fixed approved controls only. Private research output, never app metadata.
use fruitboard_flp_parser::research::{Event172Width, Profile, inspect};
use fruitboard_flp_parser::{authorize_source, modified_at_ms, sha256_hex};
use serde_json::{Value, json};
use std::io::{Read, Seek};
use std::path::Path;

fn run() -> Result<Value, &'static str> {
    if std::env::args_os().len() != 1 {
        return Err("RESEARCH_NO_ARGUMENTS");
    }
    let specification: Value =
        serde_json::from_str(include_str!("../../../research/cross-build-controls.json"))
            .map_err(|_| "RESEARCH_SPECIFICATION")?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/parser-corpus")
        .canonicalize()
        .map_err(|_| "RESEARCH_ROOT")?;
    let mut controls = Vec::new();
    for control in specification["controls"]
        .as_array()
        .ok_or("RESEARCH_SPECIFICATION")?
    {
        let name = control["file"].as_str().ok_or("RESEARCH_SPECIFICATION")?;
        if Path::new(name).components().count() != 1 {
            return Err("RESEARCH_SPECIFICATION");
        }
        let expected_size = control["bytes"].as_u64().ok_or("RESEARCH_SPECIFICATION")?;
        let expected_hash = control["sha256"].as_str().ok_or("RESEARCH_SPECIFICATION")?;
        if expected_size > 128 * 1024 {
            return Err("RESEARCH_CONTROL_LIMIT");
        }
        let source = authorize_source(&root.join(name), &root).map_err(|_| "RESEARCH_AUTHORITY")?;
        let before = source.file().metadata().map_err(|_| "RESEARCH_METADATA")?;
        let mut bytes = Vec::new();
        source
            .file()
            .take(expected_size + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "RESEARCH_READ")?;
        if !before.is_file()
            || before.len() != expected_size
            || bytes.len() as u64 != expected_size
            || sha256_hex(&bytes) != expected_hash
        {
            return Err("RESEARCH_CONTROL_CHANGED");
        }
        let mut observations = Vec::new();
        for width in [
            Event172Width::One,
            Event172Width::Three,
            Event172Width::Four,
        ] {
            let profile = Profile::new(
                control["fields"]["savedVersion"]["value"]
                    .as_str()
                    .ok_or("RESEARCH_SPECIFICATION")?,
                width,
            )?;
            observations.push(inspect(&bytes, &profile));
        }
        // Independently reread the held source; hashing the parse buffer again
        // would not establish that the on-disk input remained unchanged.
        let mut held = source.file();
        held.rewind().map_err(|_| "RESEARCH_READ")?;
        let mut after_bytes = Vec::new();
        held.take(expected_size + 1)
            .read_to_end(&mut after_bytes)
            .map_err(|_| "RESEARCH_READ")?;
        let after = held.metadata().map_err(|_| "RESEARCH_METADATA")?;
        if after.len() != expected_size
            || after_bytes != bytes
            || modified_at_ms(&before) != modified_at_ms(&after)
            || modified_at_ms(&after).is_none()
        {
            return Err("RESEARCH_CONTROL_CHANGED");
        }
        controls
            .push(json!({"id":control["id"],"sha256":expected_hash,"observations":observations}));
    }
    Ok(
        json!({"researchOnly":true,"qualified":false,"sourceHashesUnchanged":true,"controls":controls}),
    )
}

fn main() {
    match run().and_then(|result| {
        let bytes = serde_json::to_vec(&result).map_err(|_| "RESEARCH_OUTPUT")?;
        if bytes.len() > 256 * 1024 {
            return Err("RESEARCH_OUTPUT_LIMIT");
        }
        Ok(bytes)
    }) {
        Ok(bytes) => println!("{}", String::from_utf8(bytes).expect("JSON is UTF-8")),
        Err(code) => {
            eprintln!("{code}");
            std::process::exit(1);
        }
    }
}
