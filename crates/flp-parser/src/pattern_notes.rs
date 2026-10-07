//! F16/F17 qualify only the contiguous initial content series in 26.1.0.5530.
//! Payloads are borrowed; no per-note objects or raw values are retained.
use crate::{MAX_NOTE_RECORDS_PER_PATTERN, MAX_NOTE_RECORDS_TOTAL};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const RECORD_BYTES: usize = 24;
pub(crate) const COVERAGE: &str = "stored-pattern-note-records";

#[derive(Default)]
pub(crate) struct PatternNotes {
    started: bool,
    closed: bool,
    pending: Option<u16>,
    last_payload: Option<u16>,
    counts: BTreeMap<u16, Value>,
    candidates: usize,
    limit_exceeded: bool,
    binding_unverified: bool,
}
impl PatternNotes {
    pub(crate) fn observe(&mut self, event: u8, data: &[u8]) {
        if event == 224 {
            // Charge duplicates, malformed and unbound candidates before classifying.
            let count = data.len().div_ceil(RECORD_BYTES);
            match self.candidates.checked_add(count) {
                Some(total) if total <= MAX_NOTE_RECORDS_TOTAL => self.candidates = total,
                _ => self.limit_exceeded = true,
            }
            self.limit_exceeded |= count > MAX_NOTE_RECORDS_PER_PATTERN;
            let bound = if self.closed {
                None
            } else {
                self.pending.take().or(self.last_payload)
            };
            let Some(id) = bound else {
                self.binding_unverified = true;
                return;
            };
            let entry = if self.counts.contains_key(&id) {
                json!({"status":"unsupported","reason":"MULTIPLE_PATTERN_NOTE_PAYLOADS"})
            } else if data.is_empty() || !data.len().is_multiple_of(RECORD_BYTES) {
                // Genuine explicit-empty payload support remains unqualified.
                json!({"status":"unsupported","reason":"PATTERN_NOTE_LAYOUT_UNVERIFIED"})
            } else {
                json!({"status":"extracted","value":count})
            };
            if self.counts.len() < crate::MAX_PATTERNS || self.counts.contains_key(&id) {
                self.counts.insert(id, entry);
            } else {
                self.limit_exceeded = true;
            }
            self.last_payload = Some(id);
        } else if event == 65 && !self.closed {
            if self.pending.is_some() {
                // A second marker cannot replace an unresolved initial ID.
                self.closed = true;
                self.pending = None;
                self.last_payload = None;
            } else {
                self.started = true;
                self.pending = Some(u16::from_le_bytes([data[0], data[1]]));
                self.last_payload = None;
            }
        } else if self.started
            || !matches!(
                event,
                199 | 200
                    | 159
                    | 169
                    | 28
                    | 172
                    | 192
                    | 37
                    | 156
                    | 67
                    | 9
                    | 11
                    | 80
                    | 17
                    | 18
                    | 35
                    | 23
                    | 44
                    | 30
                    | 10
                    | 194
                    | 206
                    | 207
                    | 167
                    | 202
                    | 195
                    | 0
                    | 237
                    | 231
                    | 146
                    | 216
            )
        {
            // Any interruption ends authority; later property IDs cannot restore it.
            // Before the series, only header kinds observed in the F16 genuine
            // save/candidate pass. Registration (200) is skipped, never returned.
            self.closed = true;
            self.pending = None;
            self.last_payload = None;
        }
    }
    pub(crate) fn summary(&self, build: Option<&str>, ids: &BTreeSet<u16>) -> Value {
        let absent = |status, reason| json!({"status":status,"reason":reason,"coverage":COVERAGE});
        if build != Some("26.1.0.5530") {
            return absent("unsupported", "PATTERN_NOTES_UNVERIFIED_BUILD");
        }
        if self.limit_exceeded {
            return absent("unsupported", "PATTERN_NOTE_LIMIT_EXCEEDED");
        }
        if self.binding_unverified || self.counts.keys().any(|id| !ids.contains(id)) {
            return absent("unsupported", "PATTERN_NOTE_BINDING_UNVERIFIED");
        }
        if ids.is_empty() {
            return absent("unavailable", "PATTERN_DATA_NOT_STORED");
        }
        let items = ids
            .iter()
            .map(|id| {
                let count = self.counts.get(id).cloned().unwrap_or_else(
                    || json!({"status":"unavailable","reason":"PATTERN_NOTES_NOT_STORED"}),
                );
                json!({"patternId":id,"noteCount":count})
            })
            .collect::<Vec<_>>();
        if items
            .iter()
            .all(|item| item["noteCount"]["status"] == "extracted")
        {
            json!({"status":"extracted","coverage":COVERAGE,"value":items})
        } else {
            let unsupported = items
                .iter()
                .any(|item| item["noteCount"]["status"] == "unsupported");
            json!({"status":if unsupported {"unsupported"} else {"unavailable"},
                "reason":"PATTERN_NOTE_COUNTS_INCOMPLETE","coverage":COVERAGE,"items":items})
        }
    }
}
