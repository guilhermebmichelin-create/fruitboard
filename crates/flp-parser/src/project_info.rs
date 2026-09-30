//! Bounded interpretation of the saved project-info timestamp record.
//! Format facts are documented in docs/research/parser-project-facts-232.md.
use crate::field;
use serde_json::{Value, json};

pub(crate) const DAY_MS: f64 = 86_400_000.0;
// 9999-12-31 is the last representable civil date. No timezone is stored.
const LAST_DAY_EXCLUSIVE: f64 = 2_958_466.0;

#[derive(Default)]
pub(crate) struct ProjectInfo {
    record: Option<(Value, Value)>,
}

impl ProjectInfo {
    pub fn observe(&mut self, data: &[u8]) {
        let result = if self.record.is_some() {
            unavailable_pair("MULTIPLE_PROJECT_INFO_RECORDS")
        } else if data.len() != 16 {
            unavailable_pair("PROJECT_INFO_LAYOUT_UNSUPPORTED")
        } else {
            let created = f64::from_le_bytes(data[..8].try_into().expect("eight checked bytes"));
            let spent = f64::from_le_bytes(data[8..].try_into().expect("eight checked bytes"));
            let created = match local_date(created) {
                Some(value) => {
                    json!({"status":"extracted","value":value,"timezone":"unspecified-local"})
                }
                None => field(
                    "unsupported",
                    Value::Null,
                    Some("PROJECT_CREATION_DATE_INVALID"),
                ),
            };
            let spent = if spent.is_finite()
                && (0.0..LAST_DAY_EXCLUSIVE).contains(&spent)
                && (spent * DAY_MS).round() < LAST_DAY_EXCLUSIVE * DAY_MS
            {
                // Millisecond rounding avoids presenting binary-float artifacts.
                field("extracted", json!((spent * DAY_MS).round() as u64), None)
            } else {
                field(
                    "unsupported",
                    Value::Null,
                    Some("PROJECT_TIME_SPENT_INVALID"),
                )
            };
            (created, spent)
        };
        self.record = Some(result);
    }

    pub fn into_fields(self) -> (Value, Value) {
        self.record.unwrap_or_else(|| {
            let absent = field("unavailable", Value::Null, Some("PROJECT_INFO_NOT_STORED"));
            (absent.clone(), absent)
        })
    }
}

fn unavailable_pair(reason: &str) -> (Value, Value) {
    let unsupported = field("unsupported", Value::Null, Some(reason));
    (unsupported.clone(), unsupported)
}

fn leap(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

pub(crate) fn local_date(days: f64) -> Option<String> {
    if !days.is_finite() || !(2.0..LAST_DAY_EXCLUSIVE).contains(&days) {
        return None;
    }
    // Delphi day 2 is 1900-01-01. Calendar conversion deliberately applies no
    // machine timezone; treating this local wall time as UTC would invent data.
    let total_ms = (days * DAY_MS).round() as u64;
    let mut day = total_ms / 86_400_000 - 2;
    let mut year = 1900_u32;
    while year <= 9999 {
        let length = if leap(year) { 366 } else { 365 };
        if day < length {
            break;
        }
        day -= length;
        year += 1;
    }
    if year > 9999 {
        return None;
    }
    let lengths = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for length in lengths {
        if day < length {
            break;
        }
        day -= length;
        month += 1;
    }
    let time = total_ms % 86_400_000;
    Some(format!(
        "{year:04}-{month:02}-{:02}T{:02}:{:02}:{:02}.{:03}",
        day + 1,
        time / 3_600_000,
        time / 60_000 % 60,
        time / 1000 % 60,
        time % 1000
    ))
}
