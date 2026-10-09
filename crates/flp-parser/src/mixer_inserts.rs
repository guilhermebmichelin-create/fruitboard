//! Independent observer of the approved F18-F21 complete saved section.
//! Positions are local to this exact layout, not persistent numeric identities.
use serde_json::{Value, json};
use std::io::{self, Write};

pub(crate) const BUILD: &str = "26.1.0.5530";
pub(crate) const COVERAGE: &str = "explicit-saved-mixer-insert-records";
const SECTION_RECORDS: usize = 18;
const ORDINARY_RECORDS: usize = 16;
const SELECTED_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Default)]
enum Phase {
    #[default]
    Seeking,
    Record {
        position: usize,
        step: usize,
    },
    Tail(usize),
    Complete,
    Invalid,
}

#[derive(Default)]
pub(crate) struct MixerInserts {
    phase: Phase,
    guard: usize,
    seen: bool,
    candidates: usize,
    reason: Option<&'static str>,
    names: Vec<Option<String>>,
    previous_name: bool,
}

impl MixerInserts {
    fn reject(&mut self, reason: &'static str) {
        // Continue charging candidates after a rejected record/section. Stronger
        // ambiguity/limit evidence must never disappear behind an earlier error.
        let rank = |code| match code {
            "MIXER_INSERT_LIMIT_EXCEEDED" => 4,
            "MIXER_INSERT_RECORDS_AMBIGUOUS" => 3,
            "MIXER_INSERT_LAYOUT_UNVERIFIED" => 2,
            _ => 1,
        };
        if self.reason.is_none_or(|old| rank(reason) > rank(old)) {
            self.reason = Some(reason);
        }
        self.phase = Phase::Invalid;
        self.names.clear();
    }

    pub(crate) fn observe(&mut self, id: u8, data: &[u8]) {
        if id == 42 {
            self.candidates = self.candidates.saturating_add(1);
            if self.candidates > crate::MAX_MIXER_INSERT_CANDIDATES {
                self.reject("MIXER_INSERT_LIMIT_EXCEEDED");
            }
        }
        if id == 204
            && (self.previous_name || matches!(self.phase, Phase::Record { step: 2.., .. }))
        {
            self.reject("MIXER_INSERT_RECORDS_AMBIGUOUS");
        }
        self.previous_name = id == 204;

        // A bare event 103 never authorizes a count. The complete contiguous
        // entry guard was observed independently in each of the four saves.
        let guard_matches = match self.guard {
            0 => id == 242 && data.len() == 28,
            1 => id == 100 && data == [0, 0],
            2 => id == 29 && data == [1],
            3 => id == 39 && data == [1],
            4 => id == 40 && data == [0],
            5 => id == 31 && data == [0],
            6 => id == 38 && data == [1],
            _ => false,
        };
        if id == 103 {
            if self.seen {
                self.reject("MIXER_INSERT_RECORDS_AMBIGUOUS");
            } else {
                self.seen = true;
                if self.guard != 7 || data != [18, 0] || self.reason.is_some() {
                    self.reject("MIXER_INSERT_BINDING_UNVERIFIED");
                } else {
                    self.phase = Phase::Record {
                        position: 0,
                        step: 0,
                    };
                }
            }
            self.guard = 0;
            return;
        }
        self.guard = if guard_matches {
            self.guard + 1
        } else if id == 242 && data.len() == 28 {
            1
        } else {
            0
        };

        match self.phase {
            Phase::Seeking => {
                if matches!(id, 42 | 204) {
                    self.reject("MIXER_INSERT_BINDING_UNVERIFIED");
                }
            }
            Phase::Invalid => {}
            Phase::Complete => self.reject("MIXER_INSERT_BINDING_UNVERIFIED"),
            Phase::Tail(step) => {
                let expected = [(225, 6924), (133, 4), (243, 8), (47, 1)];
                if (id, data.len()) != expected[step] {
                    self.reject("MIXER_INSERT_BINDING_UNVERIFIED");
                } else {
                    self.phase = if step == 3 {
                        Phase::Complete
                    } else {
                        Phase::Tail(step + 1)
                    };
                }
            }
            Phase::Record { position, step } => self.record(position, step, id, data),
        }
    }

    fn record(&mut self, position: usize, step: usize, id: u8, data: &[u8]) {
        let special = position == 0 || position == SECTION_RECORDS - 1;
        if step == 1 && id == 204 {
            if data.len() > 8192 {
                self.reject("MIXER_INSERT_LIMIT_EXCEEDED");
                return;
            }
            let Ok(name) = crate::utf16_text(data) else {
                self.reject("MIXER_INSERT_LAYOUT_UNVERIFIED");
                return;
            };
            if name.is_empty() {
                self.reject("MIXER_INSERT_LAYOUT_UNVERIFIED");
                return;
            }
            if !special {
                self.names[position - 1] = Some(name);
            }
            self.phase = Phase::Record { position, step: 2 };
            return;
        }
        let matches = match step {
            0 => id == 42 && data == [0],
            1 | 2 => {
                if id == 236 && data.len() != 12 {
                    self.reject("MIXER_INSERT_LAYOUT_UNVERIFIED");
                    return;
                }
                let flag = if special { 0x0c } else { 0x4c };
                id == 236 && data == [0, 0, 0, 0, flag, 0, 0, 0, 0, 0, 0, 0]
            }
            3..=12 => id == 98 && data == [(step - 3) as u8, 0],
            13 => id == 235 && data == [u8::from(!special)],
            14 => id == 165 && data == [3, 0, 0, 0],
            15 => id == 166 && data == [1, 0, 0, 0],
            16 => id == 49 && data == [0],
            17 => id == 154 && data == [255; 4],
            18 => id == 147 && data == if position == 0 { [0; 4] } else { [255; 4] },
            _ => false,
        };
        if !matches {
            self.reject("MIXER_INSERT_BINDING_UNVERIFIED");
            return;
        }
        if step == 0 && !special {
            // At most sixteen selected slots. Never allocate from an unchecked
            // advertised count or retain any borrowed body/termination payload.
            self.names.push(None);
        }
        self.phase = if step == 18 {
            if position == SECTION_RECORDS - 1 {
                Phase::Tail(0)
            } else {
                Phase::Record {
                    position: position + 1,
                    step: 0,
                }
            }
        } else {
            Phase::Record {
                position,
                step: if step == 1 { 3 } else { step + 1 },
            }
        };
    }

    pub(crate) fn summary(self, build: Option<&str>) -> (Value, Value) {
        if build != Some(BUILD) {
            return pair("unsupported", "MIXER_INSERT_UNVERIFIED_BUILD");
        }
        if let Some(reason) = self.reason {
            return pair("unsupported", reason);
        }
        if !self.seen && self.guard != 0 {
            return pair("unsupported", "MIXER_INSERT_BINDING_UNVERIFIED");
        }
        if !self.seen {
            return pair("unavailable", "MIXER_INSERT_DATA_NOT_STORED");
        }
        if !matches!(self.phase, Phase::Complete) || self.names.len() != ORDINARY_RECORDS {
            return pair("unsupported", "MIXER_INSERT_BINDING_UNVERIFIED");
        }
        if ORDINARY_RECORDS > crate::MAX_MIXER_INSERTS {
            return pair("unsupported", "MIXER_INSERT_LIMIT_EXCEEDED");
        }
        let incomplete = self.names.iter().any(Option::is_none);
        let entries = self
            .names
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                let name = match name {
                    Some(value) => json!({"status":"extracted","value":value}),
                    None => json!({"status":"unavailable","reason":"MIXER_INSERT_NAME_NOT_STORED"}),
                };
                json!({"savedInsertId":index + 1,"name":name})
            })
            .collect::<Vec<_>>();
        let count = json!({"status":"extracted","value":ORDINARY_RECORDS,"coverage":COVERAGE});
        let names = if incomplete {
            json!({"status":"unavailable","reason":"MIXER_INSERT_NAMES_INCOMPLETE","coverage":COVERAGE,"items":entries})
        } else {
            json!({"status":"extracted","coverage":COVERAGE,"value":entries})
        };
        // JSON escaping can expand bounded text. Count serialization without
        // creating another output buffer, before selecting the pair.
        if !selected_fits(&count, &names) {
            return pair("unsupported", "MIXER_INSERT_LIMIT_EXCEEDED");
        }
        (count, names)
    }
}

fn pair(status: &str, reason: &str) -> (Value, Value) {
    let value = json!({"status":status,"reason":reason,"coverage":COVERAGE});
    (value.clone(), value)
}
struct OutputBudget(usize);
pub(crate) fn selected_fits(count: &Value, names: &Value) -> bool {
    serde_json::to_writer(&mut OutputBudget(0), &(count, names)).is_ok()
}
impl Write for OutputBudget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self
            .0
            .checked_add(bytes.len())
            .filter(|n| *n <= SELECTED_BYTES)
            .ok_or_else(|| io::Error::other("selected output limit"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_counter_checks_overflow_and_the_exact_ceiling_without_allocation() {
        let mut budget = OutputBudget(SELECTED_BYTES - 1);
        assert_eq!(budget.write(&[0]).unwrap(), 1);
        assert!(budget.write(&[0]).is_err());
        let mut overflow = OutputBudget(usize::MAX);
        assert!(overflow.write(&[0]).is_err());
    }
}
