//! Read-only local display projection. Saved JSON is never parser authority.
use super::{AppError, CommandEnvelope, CommandRuntime};
#[cfg(feature = "analysis-jobs")]
use crate::storage_failed;
use crate::{decode_request, invalid_request};
use fruitboard_storage::Database;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
#[cfg(feature = "analysis-jobs")]
mod analysis;
#[cfg(feature = "analysis-jobs")]
mod channels;
#[cfg(feature = "analysis-jobs")]
mod samples;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Request {
    schema_version: u64,
    root_id: String,
    location_id: String,
    expected_byte_size: String,
    expected_modified_at: String,
}

// No Debug: project values belong only in the explicitly authorized local view.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectDetails {
    root_id: String,
    location_id: String,
    #[serde(flatten)]
    content: Content,
}

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum Content {
    #[cfg(not(feature = "analysis-jobs"))]
    Disabled,
    #[cfg(feature = "analysis-jobs")]
    #[serde(rename_all = "camelCase")]
    NoCurrent {
        analysis: analysis::Summary,
        analysis_request: super::project_analysis::RequestInfo,
    },
    #[cfg(feature = "analysis-jobs")]
    #[serde(rename_all = "camelCase")]
    Available {
        snapshot_id: String,
        outcome: &'static str,
        facts: Vec<Fact>,
        channels: Vec<channels::Channel>,
        analysis: analysis::Summary,
        warnings: Vec<&'static str>,
        analysis_request: super::project_analysis::RequestInfo,
        #[serde(skip_serializing_if = "Option::is_none")]
        sample_references: Option<Vec<samples::SampleReference>>,
    },
}

#[cfg(feature = "analysis-jobs")]
#[derive(Serialize)]
struct Fact {
    key: &'static str,
    status: &'static str,
    value: Option<String>,
    explanation: Option<&'static str>,
}

pub(crate) fn handle_get_project_details(
    commands: &CommandRuntime,
    database: &Arc<Mutex<Database>>,
    request: Option<Value>,
    runtime_available: bool,
) -> CommandEnvelope<ProjectDetails> {
    commands.execute("get_project_details", || {
        let request: Request = decode_request(request)?;
        if request.schema_version != super::COMMAND_SCHEMA_VERSION
            || request.root_id.is_empty()
            || request.root_id.len() > 128
            || request.location_id.is_empty()
            || request.location_id.len() > 128
            || request
                .expected_byte_size
                .parse::<u64>()
                .ok()
                .is_none_or(|v| v.to_string() != request.expected_byte_size)
            || request.expected_modified_at.len() > 40
            || request.expected_modified_at.is_empty()
        {
            return Err(invalid_request());
        }
        read(database, request, runtime_available)
    })
}

fn read(
    database: &Arc<Mutex<Database>>,
    request: Request,
    runtime_available: bool,
) -> Result<ProjectDetails, AppError> {
    #[cfg(not(feature = "analysis-jobs"))]
    let content = {
        let _ = (database, runtime_available);
        Content::Disabled
    };
    #[cfg(feature = "analysis-jobs")]
    let content = {
        let db = database.lock().map_err(|_| storage_failed())?;
        current(&db, &request, runtime_available).map_err(|_| storage_failed())?
    };
    Ok(ProjectDetails {
        root_id: request.root_id,
        location_id: request.location_id,
        content,
    })
}

#[cfg(feature = "analysis-jobs")]
fn current(db: &Database, request: &Request, runtime_available: bool) -> Result<Content, ()> {
    // Resolve opaque IDs together, checking enabled local root and present
    // location under the shared database guard. No renderer path/file I/O.
    let input = match db.capture_metadata_input(&request.root_id, &request.location_id) {
        Ok(input) => input,
        Err(fruitboard_storage::StorageError::NotFound) => {
            return Ok(Content::NoCurrent {
                analysis: analysis::Summary::not_current(),
                analysis_request: super::project_analysis::RequestInfo::blocked("source_changed"),
            });
        }
        Err(_) => return Err(()),
    };
    if input.byte_size().to_string() != request.expected_byte_size
        || super::scan_console::unix_ns_to_rfc3339(input.modified_at_ns())
            != request.expected_modified_at
    {
        return Ok(Content::NoCurrent {
            analysis: analysis::Summary::not_current(),
            analysis_request: super::project_analysis::RequestInfo::blocked("source_changed"),
        });
    }
    let analysis = analysis::read(db, &input)?;
    let analysis_request = super::project_analysis::info(db, &input, runtime_available)?;
    let Some(snapshot) = db
        .current_metadata_snapshot(input.project_file_id())
        .map_err(|_| ())?
    else {
        return Ok(Content::NoCurrent {
            analysis,
            analysis_request,
        });
    };
    let header = snapshot.header();
    // Also fence against the displayed Library row. A later result must never
    // be attached to an older row while its list refresh is still outstanding.
    if header.input_byte_size.to_string() != request.expected_byte_size
        || super::scan_console::unix_ns_to_rfc3339(header.input_modified_at_ns)
            != request.expected_modified_at
    {
        return Ok(Content::NoCurrent {
            analysis: analysis::Summary::not_current(),
            analysis_request: super::project_analysis::RequestInfo::blocked("source_changed"),
        });
    }
    if header.projection_version != 1 {
        return Err(());
    }
    let (facts, channels, warnings, sample_references) =
        project(snapshot.payload_json().ok_or(())?)?;
    Ok(Content::Available {
        snapshot_id: header.id.clone(),
        outcome: header.outcome.as_str(),
        facts,
        channels,
        analysis,
        warnings,
        analysis_request,
        sample_references,
    })
}

#[cfg(feature = "analysis-jobs")]
type DisplayProjection = (
    Vec<Fact>,
    Vec<channels::Channel>,
    Vec<&'static str>,
    Option<Vec<samples::SampleReference>>,
);

#[cfg(feature = "analysis-jobs")]
fn project(payload: &str) -> Result<DisplayProjection, ()> {
    if payload.len() > fruitboard_storage::MAX_METADATA_JSON_BYTES {
        return Err(());
    }
    let value: Value = serde_json::from_str(payload).map_err(|_| ())?;
    if value["projectionVersion"] != 1 {
        return Err(());
    }
    let version = value["savedVersion"]
        .as_str()
        .filter(|s| {
            s.len() <= 64
                && s.split('.').count() == 4
                && s.split('.')
                    .all(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
        })
        .ok_or(())?;
    let channels = value["channelCount"]
        .as_u64()
        .filter(|v| *v <= 256)
        .ok_or(())?;
    let mut facts = vec![
        Fact {
            key: "savedVersion",
            status: "extracted",
            value: Some(version.into()),
            explanation: None,
        },
        scalar("baseTempoBpm", &value["baseTempoBpm"])?,
        Fact {
            key: "channelCount",
            status: "extracted",
            value: Some(channels.to_string()),
            explanation: None,
        },
    ];
    for key in [
        "projectCreatedLocal",
        "filesystemCreatedAtMs",
        "flStudioTimeSpentMs",
        "playlistPatternEndTick",
        "playlistPatternSpanBars",
        "playlistPatternNominalSeconds",
    ] {
        facts.push(scalar(key, &value[key])?);
    }
    let sample_references = samples::project(value.get("sampleReferences"), channels as usize)?;
    let channels = channels::project(&value, version, channels as usize)?;
    let warnings = match value.get("diagnostics") {
        None => Vec::new(),
        Some(value) => {
            let codes = value.as_array().filter(|v| v.len() <= 1024).ok_or(())?;
            if codes.iter().any(|v| v != "UNSUPPORTED_EVENT_255") {
                return Err(());
            }
            if codes.is_empty() {
                Vec::new()
            } else {
                vec!["unverified_events"]
            }
        }
    };
    Ok((facts, channels, warnings, sample_references))
}

#[cfg(feature = "analysis-jobs")]
fn scalar(key: &'static str, field: &Value) -> Result<Fact, ()> {
    let status = field["status"].as_str().ok_or(())?;
    let (status, value, explanation) = match status {
        "extracted" | "inferred" => {
            if field.get("reason").is_some() {
                return Err(());
            }
            let explanation = if status == "inferred" {
                match (key, field["method"].as_str(), field["confidence"].as_str()) {
                    (
                        "playlistPatternSpanBars",
                        Some("pattern-clip-span-at-verified-meter"),
                        Some("low"),
                    ) => Some(
                        "Low confidence. Pattern clip span calculated using the verified meter; excludes unverified clip kinds.",
                    ),
                    (
                        "playlistPatternNominalSeconds",
                        Some("constant-base-tempo-over-pattern-clips"),
                        Some("low"),
                    ) if field["assumptions"]
                        == serde_json::json!([
                            "tempo remains at base BPM",
                            "only verified pattern clips define span"
                        ]) =>
                    {
                        Some(
                            "Low confidence. Assumes tempo remains at base BPM and only verified pattern clips define span; excludes tempo automation and unverified clip kinds.",
                        )
                    }
                    _ => return Err(()),
                }
            } else {
                None
            };
            if (status == "inferred")
                != matches!(
                    key,
                    "playlistPatternSpanBars" | "playlistPatternNominalSeconds"
                )
            {
                return Err(());
            }
            let text = match key {
                "projectCreatedLocal" => {
                    let date = field["value"]
                        .as_str()
                        .filter(|v| local_date(v))
                        .ok_or(())?;
                    date.to_owned()
                }
                "filesystemCreatedAtMs" | "flStudioTimeSpentMs" | "playlistPatternEndTick" => {
                    let number = field["value"].as_u64().ok_or(())?;
                    let max = match key {
                        "flStudioTimeSpentMs" => 255_611_462_399_999,
                        "playlistPatternEndTick" => u32::MAX as u64,
                        _ => u64::MAX,
                    };
                    if number > max {
                        return Err(());
                    }
                    number.to_string()
                }
                _ => {
                    let number = field["value"]
                        .as_f64()
                        .filter(|v| v.is_finite() && *v >= 0.0)
                        .ok_or(())?;
                    let max = match key {
                        "baseTempoBpm" => 999.0,
                        "playlistPatternSpanBars" => u32::MAX as f64,
                        "playlistPatternNominalSeconds" => u32::MAX as f64 * 60.0,
                        _ => return Err(()),
                    };
                    if number > max || (key == "baseTempoBpm" && number < 1.0) {
                        return Err(());
                    }
                    number.to_string()
                }
            };
            (
                if status == "extracted" {
                    "extracted"
                } else {
                    "inferred"
                },
                Some(text),
                explanation,
            )
        }
        "unavailable" | "unsupported" => {
            if field.get("value").is_some() || field.get("method").is_some() {
                return Err(());
            }
            let reason = field["reason"].as_str().ok_or(())?;
            let explanation = reason_copy(key, status, reason).ok_or(())?;
            (
                if status == "unavailable" {
                    "unavailable"
                } else {
                    "unsupported"
                },
                None,
                Some(explanation),
            )
        }
        _ => return Err(()),
    };
    Ok(Fact {
        key,
        status,
        value,
        explanation,
    })
}

#[cfg(feature = "analysis-jobs")]
fn reason_copy(key: &str, status: &str, reason: &str) -> Option<&'static str> {
    match (key, status, reason) {
        ("baseTempoBpm" | "playlistPatternNominalSeconds", "unavailable", "BASE_TEMPO_ABSENT") => {
            Some("The project does not store a base tempo.")
        }
        (
            "projectCreatedLocal" | "flStudioTimeSpentMs",
            "unavailable",
            "PROJECT_INFO_NOT_STORED",
        ) => Some("The project does not store this project information."),
        (
            "projectCreatedLocal" | "flStudioTimeSpentMs",
            "unsupported",
            "PROJECT_INFO_LAYOUT_UNSUPPORTED" | "MULTIPLE_PROJECT_INFO_RECORDS",
        ) => Some("The saved project information layout is not verified."),
        ("projectCreatedLocal", "unsupported", "PROJECT_CREATION_DATE_INVALID") => {
            Some("The saved project creation date is invalid.")
        }
        ("flStudioTimeSpentMs", "unsupported", "PROJECT_TIME_SPENT_INVALID") => {
            Some("The saved FL Studio time counter is invalid.")
        }
        ("filesystemCreatedAtMs", "unavailable", "FILESYSTEM_CREATION_TIME_UNAVAILABLE") => {
            Some("The filesystem creation time could not be read.")
        }
        (
            "playlistPatternEndTick" | "playlistPatternSpanBars" | "playlistPatternNominalSeconds",
            "unavailable",
            "PLAYLIST_DATA_NOT_STORED" | "NO_PLAYLIST_PATTERN_CLIPS",
        ) => Some("No verified playlist pattern clips are stored."),
        (
            "playlistPatternSpanBars" | "playlistPatternNominalSeconds",
            "unsupported",
            "PPQ_UNVERIFIED" | "METER_UNVERIFIED",
        ) => Some("The saved timing resolution or meter is not verified."),
        (
            "playlistPatternEndTick" | "playlistPatternSpanBars" | "playlistPatternNominalSeconds",
            "unsupported",
            "PLAYLIST_CLIPS_UNVERIFIED_BUILD"
            | "MULTIPLE_ARRANGEMENTS_UNVERIFIED"
            | "PLAYLIST_CLIP_LAYOUT_UNVERIFIED"
            | "PLAYLIST_CLIP_KIND_UNVERIFIED"
            | "PLAYLIST_PATTERN_REFERENCE_UNVERIFIED"
            | "ZERO_LENGTH_PLAYLIST_CLIP_UNVERIFIED",
        ) => Some("Playlist pattern clips are not verified for this saved layout or build."),
        _ => None,
    }
}

#[cfg(feature = "analysis-jobs")]
fn local_date(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 23
        || [4, 7, 10, 13, 16, 19]
            .iter()
            .zip(*b"--T::.")
            .any(|(i, v)| b[*i] != v)
        || b.iter()
            .enumerate()
            .any(|(i, v)| ![4, 7, 10, 13, 16, 19].contains(&i) && !v.is_ascii_digit())
    {
        return false;
    }
    let n = |a, z| value[a..z].parse::<u32>().unwrap_or(u32::MAX);
    let year = n(0, 4);
    let month = n(5, 7);
    let day = n(8, 10);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    (1900..=9999).contains(&year)
        && day > 0
        && day <= days
        && n(11, 13) < 24
        && n(14, 16) < 60
        && n(17, 19) < 60
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::test_support::{FakeClock, FakeIdGenerator, RecordingLogSink};
    use serde_json::json;

    #[test]
    fn command_rejects_paths_unknown_fields_and_invalid_fingerprints() {
        let directory =
            std::env::temp_dir().join(format!("fruitboard-details-{}", uuid::Uuid::now_v7()));
        let db = Arc::new(Mutex::new(Database::open(&directory).unwrap()));
        let logs = Arc::new(RecordingLogSink::default());
        let runtime = CommandRuntime::new(
            Arc::new(FakeClock::new(100)),
            Arc::new(FakeIdGenerator::new(1)),
            logs.clone(),
        );
        let valid = json!({"schemaVersion":1,"rootId":"unknown","locationId":"unknown","expectedByteSize":"1024","expectedModifiedAt":"2026-01-02T00:00:00Z"});
        let result = serde_json::to_value(handle_get_project_details(
            &runtime,
            &db,
            Some(valid.clone()),
            true,
        ))
        .unwrap();
        assert_eq!(result["status"], "ok");
        #[cfg(not(feature = "analysis-jobs"))]
        assert_eq!(result["data"]["state"], "disabled");
        #[cfg(feature = "analysis-jobs")]
        assert_eq!(result["data"]["state"], "no_current");
        for (key, value) in [
            ("path", json!("C:\\Private\\project.flp")),
            ("parser", json!("other.exe")),
            ("schemaVersion", json!(2)),
            ("expectedByteSize", json!("01")),
            ("locationId", json!("")),
        ] {
            let mut request = valid.clone();
            request[key] = value;
            let result = serde_json::to_value(handle_get_project_details(
                &runtime,
                &db,
                Some(request),
                true,
            ))
            .unwrap();
            assert_eq!(result["error"]["code"], "invalid_request");
            assert!(!result.to_string().contains("Private"));
        }
        assert!(!format!("{:?}", logs.events()).contains("Private"));
        drop(db);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(feature = "analysis-jobs")]
    fn payload() -> Value {
        json!({"projectionVersion":1,"savedVersion":"26.1.0.5530","channelCount":3,
            "baseTempoBpm":{"status":"extracted","value":120},
            "projectCreatedLocal":{"status":"extracted","value":"2026-01-02T03:04:05.123"},
            "filesystemCreatedAtMs":{"status":"extracted","value":u64::MAX},
            "flStudioTimeSpentMs":{"status":"extracted","value":3661000},
            "playlistPatternEndTick":{"status":"extracted","value":768},
            "playlistPatternSpanBars":{"status":"inferred","value":2.0,"method":"pattern-clip-span-at-verified-meter","confidence":"low"},
            "playlistPatternNominalSeconds":{"status":"inferred","value":4.0,"method":"constant-base-tempo-over-pattern-clips","confidence":"low","assumptions":["tempo remains at base BPM","only verified pattern clips define span"]},
            "channelNames":{"status":"extracted","items":[{"status":"extracted","value":"Kick"},{"status":"extracted","value":"Bass"},{"status":"extracted","value":"Lead"}]},
            "untrustedExtension":"must not escape"})
    }

    #[test]
    #[cfg(feature = "analysis-jobs")]
    fn projection_preserves_status_assumptions_and_exact_integers_without_raw_extensions() {
        let (facts, _, _, _) = project(&payload().to_string()).unwrap();
        let serialized = serde_json::to_value(facts).unwrap();
        assert_eq!(serialized.as_array().unwrap().len(), 9);
        assert_eq!(serialized[4]["value"], u64::MAX.to_string());
        assert_eq!(serialized[7]["status"], "inferred");
        assert!(
            serialized[8]["explanation"]
                .as_str()
                .unwrap()
                .contains("excludes tempo automation")
        );
        assert!(!serialized.to_string().contains("Private"));
        assert!(!serialized.to_string().contains("untrustedExtension"));
        let mut value = payload();
        value["projectCreatedLocal"] =
            json!({"status":"unavailable","reason":"PROJECT_INFO_NOT_STORED"});
        value["playlistPatternSpanBars"] =
            json!({"status":"unsupported","reason":"METER_UNVERIFIED"});
        let facts = serde_json::to_value(project(&value.to_string()).unwrap().0).unwrap();
        assert_eq!(facts[3]["status"], "unavailable");
        assert!(facts[3]["value"].is_null());
        assert_eq!(facts[7]["status"], "unsupported");
    }

    #[test]
    #[cfg(feature = "analysis-jobs")]
    fn diagnostic_projection_collapses_known_warnings_and_contains_unknown_text() {
        let mut raw = payload();
        assert!(project(&raw.to_string()).unwrap().2.is_empty());
        raw["diagnostics"] = json!(["UNSUPPORTED_EVENT_255", "UNSUPPORTED_EVENT_255"]);
        assert_eq!(project(&raw.to_string()).unwrap().2, ["unverified_events"]);
        for invalid in [
            json!(["C:\\Private\\project.flp"]),
            json!([{"code":"UNSUPPORTED_EVENT_255"}]),
            json!([null]),
            json!("UNSUPPORTED_EVENT_255"),
            json!(vec!["UNSUPPORTED_EVENT_255"; 1025]),
        ] {
            raw["diagnostics"] = invalid;
            assert!(project(&raw.to_string()).is_err());
        }
    }

    #[test]
    #[cfg(feature = "analysis-jobs")]
    fn channel_projection_reads_old_snapshots_and_rejects_invalid_names_or_instrument_claims() {
        let raw = payload();
        let (_, channels, _, _) = project(&raw.to_string()).unwrap();
        let channels = serde_json::to_value(channels).unwrap();
        assert_eq!(channels[0]["name"]["value"], "Kick");
        assert_eq!(channels[0]["instrument"]["status"], "unsupported");
        assert!(
            channels[0]["instrument"]["explanation"]
                .as_str()
                .unwrap()
                .contains("not saved")
        );
        assert!(!channels.to_string().contains("Private"));
        let mut raw = payload();
        raw["channelGeneratorNames"] = json!({"items":[{"status":"extracted","value":"3x Osc"},{"status":"inferred","value":"Sampler","method":"sampler-generator-default-for-known-build","confidence":"high"},{"status":"unsupported","reason":"GENERATOR_CLASS_UNVERIFIED"}]});
        let channels = serde_json::to_value(project(&raw.to_string()).unwrap().1).unwrap();
        assert_eq!(channels[0]["instrument"]["value"], "3x Osc");
        assert_eq!(channels[1]["instrument"]["status"], "inferred");
        assert_eq!(channels[2]["instrument"]["status"], "unsupported");
        for (pointer, bad) in [
            ("/channelNames/items/0/value", json!("bad\u{0000}")),
            (
                "/channelGeneratorNames/items/0/value",
                json!("Private Plugin"),
            ),
            ("/channelGeneratorNames/items/1/confidence", json!("medium")),
            ("/channelGeneratorNames/items", json!([])),
            ("/channelNames/items", json!([])),
        ] {
            let mut bad_raw = raw.clone();
            *bad_raw.pointer_mut(pointer).unwrap() = bad;
            assert!(project(&bad_raw.to_string()).is_err(), "{pointer}");
        }
        raw["channelNames"]["status"] = json!("unavailable");
        raw["channelNames"]["reason"] = json!("CHANNEL_NAME_NOT_STORED");
        raw["channelNames"]["items"][1] =
            json!({"status":"unavailable","reason":"CHANNEL_NAME_NOT_STORED"});
        let channels = serde_json::to_value(project(&raw.to_string()).unwrap().1).unwrap();
        assert_eq!(channels[0]["name"]["value"], "Kick");
        assert_eq!(channels[1]["name"]["status"], "unavailable");
    }

    #[test]
    #[cfg(feature = "analysis-jobs")]
    fn projection_rejects_unknown_schema_invalid_dates_reasons_and_inferences() {
        for (key, value) in [
            ("projectionVersion", json!(2)),
            ("savedVersion", json!("C:\\Private\\version")),
            ("channelCount", json!(257)),
            ("baseTempoBpm", json!({"status":"extracted","value":0})),
            (
                "projectCreatedLocal",
                json!({"status":"extracted","value":"2026-02-30T00:00:00.000"}),
            ),
            (
                "flStudioTimeSpentMs",
                json!({"status":"unavailable","reason":"C:\\Private"}),
            ),
            (
                "playlistPatternSpanBars",
                json!({"status":"extracted","value":2}),
            ),
            (
                "playlistPatternNominalSeconds",
                json!({"status":"inferred","value":4,"method":"constant-base-tempo-over-pattern-clips","confidence":"low","assumptions":["wrong"]}),
            ),
        ] {
            let mut raw = payload();
            raw[key] = value;
            assert!(project(&raw.to_string()).is_err(), "{key}");
        }
    }

    #[test]
    #[cfg(feature = "analysis-jobs")]
    #[ignore = "requires an explicit independent copy of retained installed smoke data"]
    fn copied_installed_snapshot_projects_through_the_real_command() {
        let directory = std::path::PathBuf::from(
            std::env::var_os("FRUITBOARD_REVIEW_METADATA_COPY")
                .expect("explicit independent copy directory required"),
        );
        assert!(
            directory.join("storage/fruitboard.db").is_file(),
            "existing copied database required"
        );
        let db = Arc::new(Mutex::new(Database::open(&directory).unwrap()));
        let (request, expected_snapshot) = {
            let database = db.lock().unwrap();
            let roots = database.list_scan_roots().unwrap();
            let page = roots
                .iter()
                .find_map(|root| {
                    let page = database
                        .query_library(&fruitboard_storage::LibraryQuery {
                            scan_root_id: root.id.clone(),
                            page_size: 10,
                            cursor: None,
                            snapshot: None,
                        })
                        .ok()?;
                    page.locations
                        .first()
                        .map(|location| (root.id.clone(), location.clone()))
                })
                .expect("retained fixture location required");
            let snapshot = database
                .current_metadata_snapshot(&page.1.project_file_id)
                .unwrap()
                .unwrap();
            (
                json!({"schemaVersion":1,"rootId":page.0,"locationId":page.1.id,"expectedByteSize":page.1.byte_size.to_string(),"expectedModifiedAt":super::super::scan_console::unix_ns_to_rfc3339(page.1.modified_at_ns)}),
                snapshot.header().id.clone(),
            )
        };
        let runtime = CommandRuntime::new(
            Arc::new(FakeClock::new(100)),
            Arc::new(FakeIdGenerator::new(1)),
            Arc::new(RecordingLogSink::default()),
        );
        let result = serde_json::to_value(handle_get_project_details(
            &runtime,
            &db,
            Some(request),
            false,
        ))
        .unwrap();
        assert_eq!(result["status"], "ok");
        let details = &result["data"];
        assert_eq!(details["state"], "available");
        assert_eq!(details["snapshotId"], expected_snapshot);
        assert_eq!(details["facts"].as_array().unwrap().len(), 9);
        assert_eq!(details["facts"][0]["value"], "26.1.0.5530");
        assert_eq!(details["facts"][3]["status"], "extracted");
        assert_eq!(details["facts"][3]["value"].as_str().unwrap().len(), 23);
        assert_eq!(
            details["channels"].as_array().unwrap().len(),
            details["facts"][2]["value"]
                .as_str()
                .unwrap()
                .parse::<usize>()
                .unwrap()
        );
        assert_eq!(
            details["channels"][0]["instrument"]["status"],
            "unsupported"
        );
        assert!(
            details["channels"][0]["instrument"]["explanation"]
                .as_str()
                .unwrap()
                .contains("not saved")
        );
        assert!(details.get("payloadJson").is_none());
        assert!(!details.to_string().contains("Analysis Fixture"));
    }
}
