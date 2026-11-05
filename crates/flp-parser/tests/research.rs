#![cfg(feature = "compatibility-research")]
use fruitboard_flp_parser::research::{Event172Width, Profile, inspect};
use fruitboard_flp_parser::{MAX_FILE_BYTES, parse_bytes, sha256_hex};
use serde_json::json;
use std::fs;
use std::path::Path;

// Constructed defensive vectors only, not genuine new-build evidence.
fn project(build: &str, width: usize) -> Vec<u8> {
    let mut body = vec![199, (build.len() + 1) as u8];
    body.extend(build.bytes());
    body.push(0);
    body.push(172);
    body.extend([1, 1, 0, 0].into_iter().take(width));
    body.extend([192, 2, 0, 0]);
    body.push(156);
    body.extend(130_000_u32.to_le_bytes());
    body.extend([64, 0, 0, 21, 0]);
    let label: Vec<_> = "Compatibility Sampler\0"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    body.extend([203, label.len() as u8]);
    body.extend(label);
    let mut bytes = b"FLhd".to_vec();
    bytes.extend(6_u32.to_le_bytes());
    bytes.extend(0_u16.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(96_u16.to_le_bytes());
    bytes.extend(b"FLdt");
    bytes.extend((body.len() as u32).to_le_bytes());
    bytes.extend(body);
    bytes
}

#[test]
fn default_parser_stays_strict_even_with_the_research_feature() {
    let bytes = project("25.2.4.4960", 3);
    assert_eq!(parse_bytes(&bytes)["code"], "UNSUPPORTED_SAVED_VERSION");
    let profile = Profile::new("25.2.4.4960", Event172Width::Three).unwrap();
    let result = inspect(&bytes, &profile);
    assert_eq!(result["decoderOutcome"], "complete");
    assert_eq!(result["researchOnly"], true);
    assert_eq!(result["qualified"], false);
    assert_eq!(result["fieldObservations"]["baseTempoBpm"]["value"], 130.0);
    assert_eq!(
        result["fieldObservations"]["channelNames"]["value"],
        json!(["Compatibility Sampler"])
    );
    assert_eq!(result["fieldObservations"].as_object().unwrap().len(), 5);
    for absent in [
        "outcome",
        "protocolVersion",
        "schemaVersion",
        "patternNoteSequences",
        "inputFingerprint",
    ] {
        assert!(result.get(absent).is_none(), "{absent}");
    }
    assert_eq!(parse_bytes(&bytes)["code"], "UNSUPPORTED_SAVED_VERSION");
}

#[test]
fn exact_profile_does_not_admit_a_different_or_even_previously_supported_build() {
    let profile = Profile::new("25.2.4.4960", Event172Width::Three).unwrap();
    for build in ["25.2.4.4961", "26.1.0.5530"] {
        let result = inspect(&project(build, 3), &profile);
        assert_eq!(result["decoderCode"], "UNSUPPORTED_SAVED_VERSION");
        assert_eq!(result["fieldObservations"]["savedVersion"]["value"], build);
        assert_eq!(
            result["fieldObservations"]["baseTempoBpm"]["status"],
            "unsupported"
        );
    }
}

#[test]
fn profile_requires_a_canonical_exact_build_not_a_family_or_range() {
    assert!(Profile::new(&".".repeat(1024 * 1024), Event172Width::Three).is_err());
    for build in [
        "",
        "26",
        "26.1.x",
        "26.1.0",
        "26.01.0.5530",
        "26.1.0.5530-5539",
        "26.1.0.4294967296",
    ] {
        assert!(
            Profile::new(build, Event172Width::Three).is_err(),
            "{build}"
        );
    }
}

#[test]
fn successful_parsing_cannot_choose_between_ambiguous_framing_profiles() {
    let bytes = project("25.2.4.4960", 3);
    let one = inspect(
        &bytes,
        &Profile::new("25.2.4.4960", Event172Width::One).unwrap(),
    );
    let three = inspect(
        &bytes,
        &Profile::new("25.2.4.4960", Event172Width::Three).unwrap(),
    );
    assert_eq!(one["decoderOutcome"], "complete");
    assert_eq!(three["decoderOutcome"], "complete");
    assert_eq!(one["fieldObservations"], three["fieldObservations"]);
    assert_ne!(one["framingPrefix"], three["framingPrefix"]);
    assert_eq!(one["profile"]["event172Bytes"], 1);
    assert_eq!(three["profile"]["event172Bytes"], 3);
    assert_eq!(one["qualified"], false);
    assert_eq!(three["qualified"], false);
}

#[test]
fn opaque_state_is_excluded_and_framing_output_is_bounded() {
    let mut bytes = project("25.2.4.4960", 3);
    for _ in 0..100 {
        bytes.extend([0, 0]);
    }
    let size = (bytes.len() - 22) as u32;
    bytes[18..22].copy_from_slice(&size.to_le_bytes());
    let result = inspect(
        &bytes,
        &Profile::new("25.2.4.4960", Event172Width::Three).unwrap(),
    );
    assert_eq!(result["framingPrefix"].as_array().unwrap().len(), 32);
    assert_eq!(result["framingPrefixTruncated"], true);
    assert!(result.to_string().len() < 8192);
    assert!(result.get("pluginReferences").is_none());
}

#[test]
fn malformed_and_over_limit_bytes_keep_the_shared_decoder_rejections() {
    let profile = Profile::new("25.2.4.4960", Event172Width::Three).unwrap();
    let mut bytes = project("25.2.4.4960", 3);
    bytes.pop();
    assert_eq!(
        inspect(&bytes, &profile)["decoderCode"],
        "TRUNCATED_DATA_CHUNK"
    );
    let oversized = vec![0; MAX_FILE_BYTES as usize + 1];
    assert_eq!(
        inspect(&oversized, &profile)["decoderCode"],
        "FILE_SIZE_LIMIT"
    );
}

#[test]
fn approved_control_observations_match_the_unchanged_production_decoder() {
    let specification: serde_json::Value =
        serde_json::from_str(include_str!("../../../research/cross-build-controls.json")).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/parser-corpus");
    for control in specification["controls"].as_array().unwrap() {
        let bytes = fs::read(root.join(control["file"].as_str().unwrap())).unwrap();
        assert_eq!(sha256_hex(&bytes), control["sha256"]);
        let normal = parse_bytes(&bytes);
        let build = control["fields"]["savedVersion"]["value"].as_str().unwrap();
        let width = if build == "26.1.0.5530" {
            Event172Width::Three
        } else {
            Event172Width::Four
        };
        let research = inspect(&bytes, &Profile::new(build, width).unwrap());
        assert_eq!(research["decoderOutcome"], normal["outcome"]);
        for name in control["fields"].as_object().unwrap().keys() {
            assert_eq!(
                research["fieldObservations"][name], normal[name],
                "{build}/{name}"
            );
        }
        assert_eq!(sha256_hex(&bytes), control["sha256"]);
    }
}
