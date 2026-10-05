# Maintained saved-field reliability matrix

Historical baseline: merged PR261, `19b95362a66a82c91cba56fc8f2226c6ef899cc2`.
Current integration adds issue #263 on merged PR262; see the
[saved patterns review](../library-patterns-263/README.md). Historical ground
truth and measured source boundaries below are unchanged.
This records the opt-in Windows development composition, not production enablement
or full Phase 3 acceptance. Changes to extraction, validation, persistence,
display or corpus coverage must update this matrix in the same PR.

## Saved-build coverage

| Exact saved build  | Approved genuine-save derivatives | Independently checked coverage                                                                                          |
| ------------------ | --------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| 24.1.0.4225        | F01/F04/F05                       | Minimal Sampler, stored name, tempos 120/140/141 BPM and absent sample references.                                      |
| 25.1.3.4922        | F06                               | Minimal Sampler, 130 BPM, displayed default name by labeled inference and absent sample references.                     |
| 26.1.0.5530        | F11/F12/F13/F14                   | Minimal save; explicit sample/name at 137 BPM; three named patterns/four placements; two channels with built-in 3x Osc. |
| FL Studio 20/21    | F02/F03 are not covered           | No runnable matching build or genuine-save fixture here; no supported-build claim.                                      |
| Other saved builds | No approved row                   | Typed `UNSUPPORTED_SAVED_VERSION`; neighboring build numbers do not inherit support.                                    |

The [manifest](../../../fixtures/parser-corpus/manifest.md) owns approval,
provenance, license, expected values and SHA-256. Twelve files are committed:
eight genuine-save derivatives and four approved robustness derivatives.
F07-F10 exercise failure/partial handling; they add no saved-build coverage.

## Field reliability and application reach

"App" below means typed validation, immutable persistence and current-result
Library display in `analysis-jobs`. A parser-only extension is not app support.
"Constructed" means defensive/formula tests with known synthetic values, not
additional FL Studio GUI ground truth. Status/provenance belongs to each field;
a complete parse does not make every field extracted or universally supported.

| Field                                        | Evidence and supported interpretation                                                                                                                                                                  | App reach                                                                                 | Important boundary                                                                                                                                                   |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Saved version/base BPM                       | GUI-checked exact builds/tempos above; [corpus tests](../../../crates/flp-parser/tests/corpus.rs)                                                                                                      | Stored and displayed                                                                      | Missing tempo is typed unavailable; genuine absent-tempo coverage is still missing.                                                                                  |
| Channel count                                | Header/event agreement; one-channel saves and two-channel F14                                                                                                                                          | Stored and displayed                                                                      | Constructed sparse/reordered IDs and inference cases test defenses; no approved zero-channel or large multi-channel fixture.                                         |
| Editable channel names                       | Stored 2024/F12 names; known 2025/2026 default Sampler by labeled inference                                                                                                                            | Stored and displayed per channel                                                          | Names are separate from instruments. Later Sampler numbering has medium confidence until an approved multi-Sampler save verifies it.                                 |
| Channel generator names                      | F14: extracted 3x Osc; verified 2026 Sampler inferred; [generator record](../../research/parser-generator-200-result.md)                                                                               | Stored and displayed                                                                      | Only the exact 2026 build and verified classes; older builds/unknown classes stay unsupported.                                                                       |
| Raw sample references                        | F12 explicitly stores the approved synthetic reference; other genuine saves have File `(none)`                                                                                                         | Stored and displayed as inert text                                                        | Does not establish existence, availability or missing dependency; no path resolution or audio read.                                                                  |
| Pattern count/names                          | F13: three distinct IDs/names, repeated placements do not inflate count; [pattern record](../../research/parser-pattern-count-184-result.md) and [tests](../../../crates/flp-parser/tests/patterns.rs) | Stored and displayed through bounded typed validation                                     | Exact 2026 build. Absent markers do not prove zero patterns; missing names retain an unavailable state.                                                              |
| Pattern placements/end tick                  | F13's four 88-byte records and endpoint 1536; [playlist record](../../research/parser-playlist-188-result.md)                                                                                          | Endpoint stored/displayed; raw placement list used for validation, not retained/displayed | Narrow verified layout. Other clip kinds, multiple arrangements, unknown pattern references and unsupported layouts do not become guessed lengths.                   |
| Pattern-span bars/nominal seconds            | Four-bar F13 endpoint; checked formula at 96 PPQ/4/4 and saved base BPM; [extent record](../../research/parser-extent-190-result.md)                                                                   | Stored and displayed as low-confidence inference                                          | Maximum clip end, not summed clips or rendered duration; no tempo automation, audio clips or tails.                                                                  |
| Embedded creation time/FL saved time counter | Existing corpus records plus constructed calendar/counter tests; [facts record](../../research/parser-project-facts-232.md)                                                                            | Stored and displayed                                                                      | No independent GUI ground truth for date/counter behavior. Creation timezone is unspecified-local; saved counter is not measured productive work.                    |
| File size/modified/filesystem-created time   | Native held-handle authority and independent source checks; filesystem creation optional                                                                                                               | Size/modified shown in Library; optional creation in details                              | File creation differs from embedded creation. Details are saved facts matching a scanned observation, not continuous live inspection.                                |
| Top-level plugin references                  | Verified built-in Sampler/3x Osc and bounded saved strings; recognized wrapper name/vendor handling has constructed tests                                                                              | Stored/displayed; Explorer groups exact name/class/vendor and provenance                  | No approved external VST wrapper fixture. No exhaustive mixer/nested coverage, installed-plugin inventory or plugin load. Empty coverage is not proof of no plugins. |
| Automation/MIDI/mixer summaries              | No implemented app summary or approved matching fixture                                                                                                                                                | Not delivered                                                                             | Pattern clips/names and top-level plugin references do not imply these capabilities.                                                                                 |

## Tests and independent checks

The ordinary [parser tests](../../../crates/flp-parser/tests/) exercise corpus,
patterns, generators, playlist, project facts, authorization, supervision and
resource limits. The [full typed projection](../../../crates/flp-parser/src/validation/project_facts.rs),
[private persisted projection](../../../crates/storage-sqlite/src/metadata/payload.rs)
and [details projection](../../../apps/desktop/src-tauri/src/foundation/project_details.rs)
define the application reach in the table. Unknown/unvalidated JSON does not
gain authority merely because a parser returned it.

The [installed Library record](../installed-library-details-252/README.md)
checks approved known values through real native components, plus recovery and
inert display. It predates the latest Explorer/size changes; retain that exact
source boundary. Constructed 25 MB/64 MiB opaque-state inputs and repeated
Sampler Explorer entries characterize size/resource policies, not diverse
plugin compatibility or new independent per-field ground truth.

Future fixture work needs preregistered expected values, matching-build GUI
inspection, exact sanitation/digest/privacy approval and unchanged-byte checks
under [the fixture rules](../../../DEVELOPMENT.md#parser-fixture-rules).
Do not change approved corpus bytes, substitute parser output for ground truth,
or enumerate the owner's personal project roots to fill a coverage row.
