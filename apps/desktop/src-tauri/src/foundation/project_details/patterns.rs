//! Saved local display data; names never become parser/path/process authority.
use fruitboard_flp_parser::validation::{PatternSupport, SavedPatterns, validate_stored_patterns};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(super) enum Summary {
    Available { count: usize, items: Vec<Pattern> },
    Unavailable { reason: &'static str },
    Unsupported { reason: &'static str },
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Pattern {
    pattern_id: u16,
    name: Name,
}
#[derive(Serialize)]
struct Name {
    status: &'static str,
    value: Option<String>,
}

pub(super) fn project(value: Option<&Value>, build: &str) -> Result<Summary, ()> {
    let Some(value) = value else {
        return Ok(Summary::Unsupported {
            reason: "not_saved",
        });
    };
    Ok(
        match validate_stored_patterns(value, build).map_err(|_| ())? {
            SavedPatterns::Unsupported(reason) => Summary::Unsupported {
                reason: match reason {
                    PatternSupport::NotAdvertised => "not_advertised",
                    PatternSupport::UnverifiedBuild => "unverified_build",
                },
            },
            SavedPatterns::Unavailable => Summary::Unavailable {
                reason: "no_stored_patterns",
            },
            SavedPatterns::Entries(items) => Summary::Available {
                count: items.len(),
                items: items
                    .iter()
                    .map(|item| Pattern {
                        pattern_id: item.id(),
                        name: Name {
                            status: if item.name().is_some() {
                                "extracted"
                            } else {
                                "unavailable"
                            },
                            value: item.name().map(str::to_owned),
                        },
                    })
                    .collect(),
            },
        },
    )
}
