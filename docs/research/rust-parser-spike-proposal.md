# Rust parser research spike proposal

- Status: **Proposed; not approved, not started. No parser selected.**
- Date: 2026-09-14
- Decision owner: product owner (spike approval, fixture approval, parser selection)
- Related: [ROADMAP.md parser selection before Phase 3](../../ROADMAP.md#parser-selection-before-phase-3),
  [ADR-002](../adr/002-flp-parser-process.md),
  [FLP_PARSER.md](../../FLP_PARSER.md), [research records](README.md), epic #33.
- Scope of this document: documentation only. It creates no issue, no fixture,
  no build, and no code. Execution requires the owner to approve the spike issue
  and a fixture manifest.

## Context

[ROADMAP.md](../../ROADMAP.md#parser-selection-before-phase-3) prescribes the
sequence followed here: complete the filesystem-only Scanner MVP, accept the
Phase 2 checkpoint, then run a bounded Rust-parser research spike before
selecting the production FLP parser. The owner accepted the Phase 2 checkpoint
with known gaps; those gaps remain with their existing follow-ups and are not
resolved, re-scoped, or re-argued by this proposal.

This document defines that spike and embeds a paste-ready issue draft. PyFLP
remains an optional gated candidate, not a requirement. No Phase 3
implementation is authorized by this document or by the spike it proposes.

**A spike ships no product behavior and selects nothing by itself.** Its only
output is recorded evidence and a recommendation; the product owner accepts,
rejects, or defers it as a separate decision.

## Time and size bound

| Bound          | Proposal                                                                                                                                                               |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Active work    | 15 calendar days from spike start (spike issue approved and fixture manifest owner-approved, whichever is later).                                                      |
| Hard stop      | 21 calendar days. Work stops and records what was measured; an incomplete spike closes as inconclusive rather than being extended silently.                            |
| Prototype size | One research-only read-only Rust binary behind the replaceable `FlpParser` boundary. No production workspace crate, no adapter wiring, no scanner or packaging change. |
| Field bound    | Four fields, staged two then two: saved FL Studio version and base tempo first; channel names and sample references only after the first stage meets its matrix.       |
| Version bound  | Up to six saved-version rows in the supported-version matrix.                                                                                                          |
| Corpus bound   | Up to twelve owner-approved fixtures, counting absent-field, single-property-difference, and malformed byte fixtures.                                                  |
| Dependency     | No new production dependency, no runtime requirement added to the application, and no fixture or prototype artifact committed outside the spike PR.                    |

If a bound is reached, the recorded default outcome is "no selection". A bound
change is a new owner decision, not something the spike grants itself.

## Explicit questions

1. **Saved FL version and base tempo.** Can an independent read-only Rust
   implementation extract the saved FL Studio version and the base tempo as
   declared by the file, per field, with absent or unreadable values labeled
   instead of guessed?
2. **Channel names and sample references.** After stage 1 passes, can it extract
   channel names and raw sample references (the reference as stored, without
   claiming a target is resolved, missing, or valid)?
3. **Supported-version matrix.** Which rows of the supported-version matrix are
   covered, which return `unsupported`, and what evidence distinguishes the two?
4. **Resource and packaging cost.** What are cold/warm start, per-file elapsed
   time, peak memory, and binary size, and does the prototype need any runtime
   beyond the pinned Rust toolchain?
5. **Boundary compliance.** Does the prototype hold the existing safety
   boundary: read-only input, no plugin/DLL/script loading, bounded input and
   output, typed per-file failure, and byte-identical input after every run?
6. **Fallback question.** If the Rust candidate falls short of the agreed
   matrix at manageable cost, can PyFLP meet the same owner-approved corpus and
   expected values under the existing research rules and gates? This question is
   evaluated only after the shortfall is recorded, and it selects nothing by
   itself.

## Initial field matrix

Status vocabulary is the accepted one in
[FLP_PARSER.md](../../FLP_PARSER.md): `extracted`, `unavailable`, `unsupported`,
or `failed`. Every absent field is recorded with a status and a reason; no value
is inferred.

| Order | Field                   | Recorded as                          | Rules                                                                                       |
| ----- | ----------------------- | ------------------------------------ | ------------------------------------------------------------------------------------------- |
| 1     | Saved FL Studio version | Version/build exactly as saved       | No mapping to "latest" or "equivalent"; unmatched formats are `unsupported` or `failed`.    |
| 2     | Base tempo              | BPM value as stored                  | Base/default tempo only; tempo automation, time signature, and duration are out of scope.   |
| 3     | Channel names           | One value per channel, in file order | Empty/deleted names and Unicode recorded verbatim; count mismatches recorded per item.      |
| 4     | Sample references       | Raw reference string per channel     | Raw strings only; never resolve paths, never claim a sample exists, is missing, or is safe. |

Stage rules:

- Stage 1 (fields 1-2) must meet the matrix on the approved corpus before stage
  2 begins. A failed stage 1 ends the spike with a written shortfall.
- Stage 2 results are recorded per field and per item; a partially covered
  matrix is a recorded limitation, not an average or a pass.

## Supported-version matrix

The rows are the version bands named in the existing research corpus list in
[FLP_PARSER.md](../../FLP_PARSER.md#required-parser-matrix-poc-p0-a). Fixture
availability is subject to the owner's fixture-approval process; a row the owner
cannot legally produce is recorded as "not covered", not substituted.

| Saved version band                  | Fixture (needs owner approval) | Matrix result                               |
| ----------------------------------- | ------------------------------ | ------------------------------------------- |
| Oldest producible baseline          | Purpose-built minimal FLP      | To record per fixture.                      |
| FL 20                               | Purpose-built minimal FLP      | To record per fixture.                      |
| FL 21                               | Purpose-built minimal FLP      | To record per fixture.                      |
| FL 2024                             | Purpose-built minimal FLP      | To record per fixture.                      |
| FL 2025/current                     | Purpose-built minimal FLP      | To record per fixture.                      |
| One adjacent unseen version, if any | Purpose-built minimal FLP      | Recorded as `unsupported` when not covered. |

Unsupported rows stay visible in the results. Omitting, aggregating, or
renaming a failed row is a matrix failure.

## Corpus and ground truth

- **Owner-approved fixtures only.** The fixture manifest process stays with the
  owner. This proposal does not authorize, reserve, or pre-approve any fixture.
  The accepted [parser fixture rules](../../DEVELOPMENT.md#parser-fixture-rules)
  apply: manifest entries with source/provenance, license, FL version, expected
  features, SHA-256, and privacy approval; purpose-built minimal FLPs or
  upstream-public fixtures with compatible attribution/license; no personal
  projects; tests never enumerate the user's real project roots.
- **Case families.** The approved corpus must include absent-field cases (the
  field is not present or not readable) and single-property-difference pairs
  (two fixtures differing in exactly one controlled property). Robustness byte
  fixtures for truncation, unknown events, malformed lengths, and resource
  limits are part of the corpus bound.
- **Ground truth is the pre-registered manifest.** Expected values are written
  before parsing. Neither parser is ground truth, and agreement between the Rust
  prototype and PyFLP is not evidence for either. Disagreement starts an
  investigation; it never resolves by majority or by trusting the more familiar
  parser. This restates
  [ROADMAP.md](../../ROADMAP.md#parser-selection-before-phase-3), items 1 and 5.

## Artifacts

1. **Read-only prototype.** A research-only Rust binary implementing the
   existing `FlpParser` shape (`describe`, `parse`, `health_check`) behind the
   replaceable boundary described in
   [FLP_PARSER.md](../../FLP_PARSER.md#adapter-shape). It contains no save,
   mutate, repair, plugin-load, DLL-load, or script-execution operation, and it
   lives only in the spike PR, never on a product code path.
2. **Provenance record.** Format-evidence sources, code provenance (including
   any referenced implementation and its license), fixture provenance and
   licenses, the pinned Rust 1.98.1 toolchain, and the exact source boundary the
   results apply to (commit SHA and binary hash where a binary is retained). Any
   retained binary is an independent copy with recorded provenance, never a
   hardlink into a writable build cache.
3. **Resource measurements.** Cold and warm start, per-file elapsed time, peak
   memory, and binary/package size, recorded with the host and exact source.
   These are single-host observations, not qualification or performance
   acceptance evidence.
4. **Per-field correctness record.** For every approved fixture: expected value
   from the manifest, observed value or status, warnings, and the fixture hash.
   Absent-field rows and the single-property-difference pairs are called out
   explicitly.
5. **Robustness record.** Truncated input, unknown events, malformed lengths,
   and resource-limit cases, each with a typed outcome; the harness must not
   crash, must not mark a file's bytes as changed (SHA-256 before and after),
   and must never load embedded plugins or scripts.
6. **Results and limitations record.** A `docs/research/` record stating the
   question, environment, inputs, observed result, limitations, and the decision
   input, following the directory's conventions. It distinguishes automated,
   synthetic-fixture evidence from anything manual or borrowed.
7. **Decision packet.** A short recommendation that maps each explicit question
   to evidence and states what was not covered. The packet is input to the
   owner's selection decision, not the selection itself.

## Decision criteria

Selection criteria are the accepted ones in
[ROADMAP.md](../../ROADMAP.md#parser-selection-before-phase-3), items 5 and 6.
This proposal adds no new gate, no new matrix, and no new license condition.

- **Rust candidate.** Adopt only if it meets the agreed initial matrix at
  manageable maintenance cost (item 5). Otherwise, evaluate PyFLP against the
  same owner-approved corpus and expected values under the existing research
  rules (item 5).
- **Both paths.** Select on reliability, packaging, and maintenance rather than
  avoiding GPL alone (item 6).
- **PyFLP path.** Production adoption still requires compatible licensing and
  the technical gates in [ADR-002](../adr/002-flp-parser-process.md): a
  representative FL-version/feature compatibility matrix, Windows
  package/startup/crash/signing/update tests, and an explicit decision that the
  product's distribution complies with PyFLP's GPL-3.0 obligations (also the
  licensing gate in [FLP_PARSER.md](../../FLP_PARSER.md#licensing-gate-p0-c)).
- **Rust path.** An independent Rust parser also needs compatibility, packaging,
  and provenance review (item 6).
- **Recording.** The production selection is recorded in ADR-002 and
  FLP_PARSER.md (item 6). A spike result that is not recorded there has not
  selected anything.

## Decision deadline

- The owner's selection decision (Rust, PyFLP, defer, or stop) is due within 10
  business days after the results and recommendation are posted, and no later
  than the 21-calendar-day hard stop.
- If the deadline passes without a decision, the spike closes as inconclusive
  and the parser stays unselected. Any resumed evaluation is a new owner
  decision with a new bound.
- No later-phase implementation follows automatically from a spike result.

## Explicit non-goals

- No product behavior, no production adapter, no scanner or packaging change,
  and no new production dependency.
- No parser selection by the spike or by this proposal.
- No Phase 3 scope: arrangements, plugin-state decoding, duration estimation,
  playlist/mixer normalization, diagnostics UI, and Plugin Explorer remain with
  the later phase (see [ROADMAP.md](../../ROADMAP.md#phase-3-proposed-issues-and-prs)).
- No fixture authorization; the owner's privacy manifest process is untouched.
- No ground truth from parser agreement, and no resolution of disagreement by
  trusting either parser.
- No claims about DriveFS, cross-volume, or FAT32 behavior; those stay with
  their existing scanner follow-ups.

## Paste-ready issue draft

The intended issue title is:

```text
Spike: bounded independent Rust FLP parser evaluation
```

Suggested labels: `spike`, `parser`. Suggested milestone: the next accepted
milestone, per the scheduling note in
[ROADMAP.md](../../ROADMAP.md#parser-selection-before-phase-3). Do not create
this issue, or the separate fixture-manifest PR it depends on, without the
owner's approval. No fixture is authorized by this draft.

```markdown
## Outcome

A written, evidence-backed answer to whether an independent read-only Rust
parser can extract the initial metadata matrix (saved FL Studio version and base
tempo, then channel names and sample references) from owner-approved synthetic
FLPs, at manageable maintenance cost. The spike recommends; the owner selects.
It ships no product behavior and selects no parser by itself.

## Non-goals

- No production `FlpParser` adapter, scanner change, or packaging change.
- No arrangements, plugin-state decoding, or duration estimation.
- No fixture approval: the owner approves any corpus through the existing
  privacy manifest process, in a separate PR.
- No Phase 3 implementation scope.
- No parser selection by this issue.

## Dependencies and related decisions

- ROADMAP.md#parser-selection-before-phase-3, items 1-6.
- ADR-002 and FLP_PARSER.md (existing boundary, safety contract, and PyFLP
  gates; unchanged by this spike).
- Blocked on the owner approving a fixture manifest; without it the spike does
  not start.

## Time and size bound

- Time: 15 calendar days of active work from issue start, hard stop at 21
  calendar days.
- Size: one read-only research prototype behind the `FlpParser` boundary, four
  staged fields, up to six version rows, up to twelve owner-approved fixtures,
  and no production crate or dependency change.

## Explicit questions

1. Can it extract the saved FL version and base tempo per field, with absent
   values labeled rather than guessed?
2. After that passes, can it extract channel names and raw sample references,
   without claiming a sample is resolved or missing?
3. Which supported-version matrix rows are covered, and which return
   `unsupported`?
4. What are cold/warm start, per-file time, peak memory, and binary size, with
   no runtime beyond the pinned Rust toolchain?
5. Does it keep the safety boundary: read-only, no plugin/script loading,
   bounded IO, typed per-file failure, byte-identical input?
6. If it falls short, can PyFLP meet the same approved corpus and expected
   values under the existing research rules and gates?

## Acceptance criteria

- Per-field results recorded against pre-registered manifest values for every
  approved fixture, including absent-field and single-property-difference
  cases.
- Truncation, unknown-event, malformed-length, and resource-limit cases
  recorded as typed outcomes with input hashes unchanged.
- Provenance, environment, and limitations recorded in docs/research/.
- Recommendation posted with a coverage statement naming what was not tested.
- Owner decision recorded by the decision deadline, or the spike closes as
  inconclusive.

## Evidence and privacy

- Read-only: the spike never writes, repairs, or uploads an FLP.
- No personal projects; fixtures enter the repository only through a separate
  owner-approved manifest with provenance, license, FL version, features,
  SHA-256, and privacy review.
- No personal paths or fixture contents in logs, issues, or pull requests.
- Measurements are single-host observations, not qualification evidence.
```

## What happens after this proposal

1. The owner reviews this record and either approves, edits, or declines the
   spike. Nothing runs before that decision.
2. If approved, the owner opens the spike issue and separately approves a
   fixture manifest through the existing privacy process.
3. The spike runs inside its time and size bound and publishes evidence plus a
   recommendation.
4. The owner decides; the decision and any selected parser's gates are recorded
   in ADR-002 and FLP_PARSER.md. Until then the production parser remains
   unselected and PyFLP remains absent from the application.
