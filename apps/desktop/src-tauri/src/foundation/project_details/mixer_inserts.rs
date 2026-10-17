//! Selected saved record identities and inert names only.
use fruitboard_flp_parser::validation::{
    MixerInsertName, MixerInsertReason, SavedMixerInserts, validate_stored_mixer_inserts,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(super) enum Summary {
    Available { count: usize, items: Vec<Insert> },
    Unavailable { reason: &'static str },
    Unsupported { reason: &'static str },
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Insert {
    saved_insert_id: u16,
    name: Name,
}
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Name {
    Extracted { value: String },
    Unavailable { reason: &'static str },
}
fn reason(value: MixerInsertReason) -> &'static str {
    match value {
        MixerInsertReason::DataNotStored => "no_saved_section",
        MixerInsertReason::UnverifiedBuild => "unverified_build",
        MixerInsertReason::LayoutUnverified => "unverified_layout",
        MixerInsertReason::BindingUnverified => "unverified_binding",
        MixerInsertReason::RecordsAmbiguous => "ambiguous_records",
        MixerInsertReason::LimitExceeded => "limit_exceeded",
    }
}
pub(super) fn project(value: Option<&Value>, build: &str) -> Result<Summary, ()> {
    let Some(value) = value else {
        return Ok(Summary::Unsupported {
            reason: "not_saved",
        });
    };
    Ok(
        match validate_stored_mixer_inserts(value, build).map_err(|_| ())? {
            SavedMixerInserts::NotAdvertised => Summary::Unsupported {
                reason: "not_advertised",
            },
            SavedMixerInserts::Unavailable(code) => Summary::Unavailable {
                reason: reason(code),
            },
            SavedMixerInserts::Unsupported(code) => Summary::Unsupported {
                reason: reason(code),
            },
            SavedMixerInserts::Entries(items) => Summary::Available {
                count: items.len(),
                items: items
                    .iter()
                    .map(|item| Insert {
                        saved_insert_id: item.id(),
                        name: match item.name() {
                            MixerInsertName::Extracted(value) => Name::Extracted {
                                value: value.clone(),
                            },
                            MixerInsertName::Unavailable => Name::Unavailable {
                                reason: "name_not_saved",
                            },
                        },
                    })
                    .collect(),
            },
        },
    )
}
