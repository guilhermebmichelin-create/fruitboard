//! Exact saved-reference groups for one committed Library root. No file I/O.
use super::{CommandEnvelope, CommandRuntime};
use crate::{decode_request, invalid_request};
use fruitboard_storage::Database;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};

#[cfg(feature = "analysis-jobs")]
use {
    crate::storage_failed,
    fruitboard_storage::{ExplorerMetadata, ExplorerRead},
    std::collections::BTreeMap,
};

#[cfg(feature = "analysis-jobs")]
const MAX_REFERENCES: usize = 8_192;
#[cfg(feature = "analysis-jobs")]
const MAX_GROUPS: usize = 1_024;
#[cfg(feature = "analysis-jobs")]
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Request {
    schema_version: u64,
    root_id: String,
}

// No Debug: saved text is private local display data.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Response {
    root_id: String,
    #[serde(flatten)]
    content: Content,
}
#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum Content {
    #[cfg(not(feature = "analysis-jobs"))]
    Disabled,
    #[cfg(feature = "analysis-jobs")]
    Unavailable,
    #[cfg(feature = "analysis-jobs")]
    Limited,
    #[cfg(feature = "analysis-jobs")]
    #[serde(rename_all = "camelCase")]
    Ready {
        entries: Vec<Entry>,
        groups: Vec<Group>,
        reference_count: usize,
        unnamed_reference_count: usize,
    },
}
#[cfg(feature = "analysis-jobs")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    #[serde(flatten)]
    record: super::scan_console::PublishedFileLocation,
    metadata_state: &'static str,
    unnamed_reference_count: usize,
}
#[cfg(feature = "analysis-jobs")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Group {
    plugin: Value,
    reference_count: usize,
    matches: Vec<Match>,
}
#[cfg(feature = "analysis-jobs")]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Match {
    location_id: String,
    reference_count: usize,
}

pub(crate) fn handle_get_plugin_explorer(
    commands: &CommandRuntime,
    database: &Arc<Mutex<Database>>,
    request: Option<Value>,
) -> CommandEnvelope<Response> {
    commands.execute("get_plugin_explorer", || {
        let request: Request = decode_request(request)?;
        if request.schema_version != super::COMMAND_SCHEMA_VERSION
            || request.root_id.is_empty()
            || request.root_id.len() > 128
            || request.root_id.contains('\0')
        {
            return Err(invalid_request());
        }
        #[cfg(not(feature = "analysis-jobs"))]
        let content = {
            let _ = database;
            Content::Disabled
        };
        #[cfg(feature = "analysis-jobs")]
        let content = {
            let db = database.lock().map_err(|_| storage_failed())?;
            match db.read_plugin_explorer(&request.root_id) {
                Ok(read) => project(read).map_err(|_| storage_failed())?,
                Err(fruitboard_storage::StorageError::NotFound) => Content::Unavailable,
                Err(_) => return Err(storage_failed()),
            }
        };
        Ok(Response {
            root_id: request.root_id,
            content,
        })
    })
}

#[cfg(feature = "analysis-jobs")]
fn add_references(
    items: &[Value],
    location_id: &str,
    groups: &mut BTreeMap<String, Group>,
) -> Result<usize, ()> {
    let mut unnamed = 0;
    for item in items {
        if item["name"]["value"].as_str().is_none() {
            unnamed += 1;
            continue;
        }
        let plugin = serde_json::json!({
            "name":item["name"], "className":item["className"], "vendor":item["vendor"]
        });
        // Exact text and complete per-field provenance define identity. No
        // aliasing, case folding, guessed vendors or logical-project dedupe.
        let key = serde_json::to_string(&plugin).map_err(|_| ())?;
        let group = groups.entry(key).or_insert_with(|| Group {
            plugin,
            reference_count: 0,
            matches: Vec::new(),
        });
        group.reference_count += 1;
        if let Some(last) = group
            .matches
            .last_mut()
            .filter(|m| m.location_id == location_id)
        {
            last.reference_count += 1;
        } else {
            group.matches.push(Match {
                location_id: location_id.to_owned(),
                reference_count: 1,
            });
        }
    }
    Ok(unnamed)
}

#[cfg(feature = "analysis-jobs")]
fn project(read: ExplorerRead) -> Result<Content, ()> {
    let (root, sources) = match read {
        ExplorerRead::Unavailable => return Ok(Content::Unavailable),
        ExplorerRead::Limited => return Ok(Content::Limited),
        ExplorerRead::Ready { root, entries } => (root, entries),
    };
    let mut entries = Vec::with_capacity(sources.len());
    let mut groups = BTreeMap::new();
    let mut reference_count = 0;
    let mut unnamed_reference_count = 0;
    for source in sources {
        let mut unnamed = 0;
        let metadata_state = match source.metadata {
            ExplorerMetadata::Missing => "missing",
            ExplorerMetadata::Stale => "stale",
            ExplorerMetadata::NoAnalysis => "no_analysis",
            ExplorerMetadata::Current(snapshot) => {
                match super::project_details::explorer_plugins(snapshot.payload_json().ok_or(())?) {
                    Err(_) => "unavailable",
                    Ok(None) => "older",
                    Ok(Some(plugins)) => {
                        let items = plugins["items"].as_array().ok_or(())?;
                        reference_count += items.len();
                        if reference_count > MAX_REFERENCES {
                            return Ok(Content::Limited);
                        }
                        unnamed = add_references(items, &source.location.id, &mut groups)?;
                        unnamed_reference_count += unnamed;
                        if groups.len() > MAX_GROUPS {
                            return Ok(Content::Limited);
                        }
                        "current"
                    }
                }
            }
        };
        entries.push(Entry {
            record: super::scan_console::to_published_file_location(&source.location, &root),
            metadata_state,
            unnamed_reference_count: unnamed,
        });
    }
    let content = Content::Ready {
        entries,
        groups: groups.into_values().collect(),
        reference_count,
        unnamed_reference_count,
    };
    // Leave room for the root ID and enclosing response, including escaped
    // control characters, so the client receives at most its 2 MiB ceiling.
    if serde_json::to_vec(&content).map_err(|_| ())?.len() > MAX_RESPONSE_BYTES - 1_024 {
        return Ok(Content::Limited);
    }
    Ok(content)
}

#[cfg(test)]
mod tests;

#[cfg(all(test, windows, feature = "analysis-jobs"))]
mod resource_probe;
