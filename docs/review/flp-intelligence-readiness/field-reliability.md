# Maintained saved-field reliability matrix

Historical baseline: merged PR261, `19b95362a66a82c91cba56fc8f2226c6ef899cc2`.
Current integration adds issue #263 on merged PR262; see the
[saved patterns review](../library-patterns-263/README.md). Historical ground
truth and measured source boundaries below are unchanged.
This records the opt-in Windows development composition, not production enablement
or full Phase 3 acceptance. Changes to extraction, validation, persistence,
display or corpus coverage must update this matrix in the same PR.

## Saved-build coverage

| Exact saved build  | Approved genuine-save derivatives | Independently checked coverage                                                                                      |
| ------------------ | --------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| 24.1.0.4225        | F01/F04/F05                       | Minimal Sampler, stored name, tempos 120/140/141 BPM and absent sample references.                                  |
| 25.1.3.4922        | F06                               | Minimal Sampler, 130 BPM, displayed default name by labeled inference and absent sample references.                 |
| 26.1.0.5530        | F11/F12/F13/F14/F15               | Minimal; explicit sample/name; three patterns/four placements; 3x Osc; cloned empty Samplers with duplicate labels. |
| FL Studio 20/21    | F02/F03 are not covered           | No runnable matching build or genuine-save fixture here; no supported-build claim.                                  |
| Other saved builds | No approved row                   | Typed `UNSUPPORTED_SAVED_VERSION`; neighboring build numbers do not inherit support.                                |

The [manifest](../../../fixtures/parser-corpus/manifest.md) owns approval,
provenance, license, expected values and SHA-256. Thirteen files are committed:
nine genuine-save derivatives and four approved robustness derivatives.
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
| Channel count                                | Header/event agreement; one-channel saves, two-channel F14 and three-channel F15                                                                                                                       | Stored and displayed                                                                      | Constructed sparse/reordered IDs and inference cases test defenses; no approved zero-channel or large multi-channel fixture.                                         |
| Editable channel names                       | Stored 2024/F12 names; labeled default Sampler inference; approved 2026 three-Sampler F15 (#267)                                                                                                       | Stored and displayed per channel                                                          | Names differ from instruments. New defaults repeat Sampler; old numbered snapshots explain reanalysis. F15 qualifies the clone path in the exact 2026 build.         |
| Channel generator names                      | F14: extracted 3x Osc; verified 2026 Sampler inferred; [generator record](../../research/parser-generator-200-result.md)                                                                               | Stored and displayed                                                                      | Only the exact 2026 build and verified classes; older builds/unknown classes stay unsupported.                                                                       |
| Raw sample references                        | F12 explicitly stores the approved synthetic reference; other genuine saves have File `(none)`                                                                                                         | Stored and displayed as inert text                                                        | Does not establish existence, availability or missing dependency; no path resolution or audio read.                                                                  |
| Pattern count/names                          | F13: three distinct IDs/names, repeated placements do not inflate count; [pattern record](../../research/parser-pattern-count-184-result.md) and [tests](../../../crates/flp-parser/tests/patterns.rs) | Stored and displayed through bounded typed validation                                     | Exact 2026 build. Absent markers do not prove zero patterns; missing names retain an unavailable state.                                                              |
| Pattern placements/end tick                  | F13's four 88-byte records and endpoint 1536; [playlist record](../../research/parser-playlist-188-result.md)                                                                                          | Endpoint stored/displayed; raw placement list used for validation, not retained/displayed | Narrow verified layout. Other clip kinds, multiple arrangements, unknown pattern references and unsupported layouts do not become guessed lengths.                   |
| Pattern-span bars/nominal seconds            | Four-bar F13 endpoint; checked formula at 96 PPQ/4/4 and saved base BPM; [extent record](../../research/parser-extent-190-result.md)                                                                   | Stored and displayed as low-confidence inference                                          | Maximum clip end, not summed clips or rendered duration; no tempo automation, audio clips or tails.                                                                  |
| Embedded creation time/FL saved time counter | Existing corpus records plus constructed calendar/counter tests; [facts record](../../research/parser-project-facts-232.md)                                                                            | Stored and displayed                                                                      | No independent GUI ground truth for date/counter behavior. Creation timezone is unspecified-local; saved counter is not measured productive work.                    |
| File size/modified/filesystem-created time   | Native held-handle authority and independent source checks; filesystem creation optional                                                                                                               | Size/modified shown in Library; optional creation in details                              | File creation differs from embedded creation. Details are saved facts matching a scanned observation, not continuous live inspection.                                |
| Top-level plugin references                  | Verified built-in Sampler/3x Osc and bounded saved strings; recognized wrapper name/vendor handling has constructed tests                                                                              | Stored/displayed; Explorer groups exact name/class/vendor and provenance                  | No approved external VST wrapper fixture. No exhaustive mixer/nested coverage, installed-plugin inventory or plugin load. Empty coverage is not proof of no plugins. |
| Automation/MIDI/mixer summaries              | No implemented app summary or approved matching fixture                                                                                                                                                | Not delivered                                                                             | Pattern clips/names and top-level plugin references do not imply these capabilities.                                                                                 |
| Ephemeral sample metadata observations       | ADR-007/PR285; actual NTFS rights/race and authorized-host tests, installed constructed host mapping plus unchanged F12 outside-root/F15 no-reference cases                                            | Explicit Library checks in `analysis-jobs`; no persisted availability                     | Exact eligible absolute paths inside selected local NTFS root only; time-of-check presence/absence, no audio/content reads or broader FL Studio resolution.          |

The [saved pattern note-count proposal](../pattern-notes-288/README.md), #288,
starts a narrow G6 scope with preregistered fixture and native authority reviews.
It adds no extraction/display support or approved note expectations to this
matrix. Note counts and automation/MIDI/mixer summaries remain undelivered.

## Tests and independent checks

G5's [sample presence design](../sample-presence-278/README.md), issue #278,
was accepted in PR279 for a separate ephemeral within-root metadata report.
The [policy slice #280](../sample-presence-policy-280/README.md) implements an
inactive library with injected-port tests, merged in PR281. The
[Windows slice #282](../sample-presence-windows-282/README.md) adds a metadata
adapter and actual NTFS rights/race tests, merged as PR283. The
[app slice #284](../sample-presence-ui-284/README.md), merged as PR285, adds
authorized ephemeral observations for eligible exact paths inside the selected
local NTFS root: present/not-found when checked, not-checked and no-reference.
Saved reference provenance remains unchanged and raw sample text stays inert.
Relative/placeholder/external-folder/cloud resolution remains unqualified.

Issue #267 [qualifies approved three-Sampler F15](../../research/parser-multisampler-267-result.md)
in the exact 2026 build. Duplicate GUI labels refute the old assumption that
unstored later labels should be numbered. New results retain literal `Sampler`
defaults with labeled inference; old immutable numbered results stay readable
with an explanation to reanalyze. The owner approved the exact F15 bytes and
registered expectations on 2026-10-05. The corpus table includes this clone path;
the exact supported builds remain unchanged.

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
