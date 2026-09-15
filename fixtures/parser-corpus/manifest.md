# Parser fixture corpus manifest (spike #138)

- Status: **Scaffold only. No fixture bytes committed; no digest recorded. Every slot is `not covered`.**
- Date: 2026-09-15
- Decision owner: product owner (fixture approval and spike start)
- Related: issue #138, [parser fixture-manifest proposal](../../docs/research/parser-fixture-manifest-proposal-20260914.md),
  [Rust parser spike proposal](../../docs/research/rust-parser-spike-proposal.md),
  [ROADMAP.md parser selection before Phase 3](../../ROADMAP.md#parser-selection-before-phase-3),
  [parser fixture rules](../../DEVELOPMENT.md#parser-fixture-rules),
  [repository proposal](../../ARCHITECTURE.md#repository-proposal).
- Scope of this file: the manifest required by
  [DEVELOPMENT.md parser fixture rules](../../DEVELOPMENT.md#parser-fixture-rules).
  It is the dedicated manifest PR for spike #138. It creates no fixture byte, no
  SHA-256 value, no parser code, and no build. It mutates no GitHub issue beyond
  referencing #138, and it does not start the spike.

## Generation-method decision

Allowed sources are the three named in the approved plan: purpose-built minimal
synthetic FLPs, upstream-public fixtures with compatible attribution/license, and
byte-derived robustness fixtures from an approved base. The method chosen for the
first corpus is **owner authoring in the installed FL Studio builds** (FL 20, 21,
2024, 2025, and 2026 are present on the development host).

The agent path is **not used**, for a recorded reason:

- FL Studio has no headless save/project-creation command, so a genuine
  "saved FL version" cannot be produced without hand-driven GUI work, which this
  task leaves to the owner.
- A from-scratch minimal-FLP writer would emit bytes whose validity cannot be
  confirmed without running parser or spike code (out of scope), so those bytes
  would be unverifiable ground truth. That is the "improvising bytes" case the
  task forbids.
- Upstream-public fixtures with compatible attribution/license are not usable
  here: the known public corpus (PyFLP) is GPL-3.0 and the repository does not
  accept GPL distribution; the FL Studio installation's own demo/template
  projects are Image-Line content and are not redistributable.
- Personal production projects and minimized/sanitized derivatives are excluded;
  no written owner approval exists.

Consequence: **no file is force-added and no digest exists yet**. Every assigned
slot is recorded `not covered` until the owner supplies genuine FL Studio saves;
nothing is substituted.

## Slot allocation and status

Bound carried from issue #138: up to twelve fixtures. Ten slots are assigned and
two are reserved. Every row below is `not covered`; the reserved rows stay
reserved unless the owner authorizes their use.

| Slot | Proposed ID    | Role / case family                                              | Version row                | Status      | Reason                                                          |
| ---- | -------------- | --------------------------------------------------------------- | -------------------------- | ----------- | --------------------------------------------------------------- |
| F01  | FIX-BASE-MIN   | Stage 1+2 baseline; base for the malformed byte derivatives     | Oldest producible baseline | not covered | Needs a genuine FL Studio save; no headless authoring available |
| F02  | FIX-FL20-MIN   | Version coverage; absent-field: base tempo not present          | FL 20                      | not covered | Needs a genuine FL Studio 20 save                               |
| F03  | FIX-FL21-MIN   | Version coverage; absent-field: no channels / no sample refs    | FL 21                      | not covered | Needs a genuine FL Studio 21 save                               |
| F04  | FIX-FL2024-A   | Version coverage; single-property-difference member A           | FL 2024                    | not covered | Needs a genuine FL Studio 2024 save                             |
| F05  | FIX-FL2024-B   | Single-property-difference member B (one property changed)      | FL 2024                    | not covered | Needs a genuine FL Studio 2024 save                             |
| F06  | FIX-FL2025-MIN | Version coverage                                                | FL 2025/current            | not covered | Needs a genuine FL Studio 2025/2026 save                        |
| F07  | FIX-RB-TRUNC   | Robustness: truncation, typed outcome; derived from F01         | n/a (derivative)           | not covered | No approved base: F01 is absent                                 |
| F08  | FIX-RB-UNKNOWN | Robustness: unknown event, typed outcome; derived from F01      | n/a (derivative)           | not covered | No approved base: F01 is absent                                 |
| F09  | FIX-RB-MALFORM | Robustness: malformed length, typed outcome; derived from F01   | n/a (derivative)           | not covered | No approved base: F01 is absent                                 |
| F10  | FIX-RB-LIMIT   | Robustness: resource limit (oversized strings/counts)           | n/a (derivative)           | not covered | No approved base: F01 is absent                                 |
| F11  | Reserved       | Sixth version row (adjacent unseen version), only if producible | Adjacent unseen            | reserved    | Only if the owner can lawfully produce it                       |
| F12  | Reserved       | Owner-approved overflow only                                    | n/a                        | reserved    | A bound change is a new owner decision                          |

## Supported-version rows

Bound carried from issue #138: up to six version rows. A row that cannot be
lawfully produced is recorded `not covered`, never substituted.

| Row | Saved version band              | Fixture slot   | Matrix result |
| --- | ------------------------------- | -------------- | ------------- |
| 1   | Oldest producible baseline      | F01            | not covered   |
| 2   | FL 20                           | F02            | not covered   |
| 3   | FL 21                           | F03            | not covered   |
| 4   | FL 2024                         | F04, F05       | not covered   |
| 5   | FL 2025/current                 | F06            | not covered   |
| 6   | Adjacent unseen version, if any | F11 (reserved) | reserved      |

## Required manifest-entry fields

Each entry added later must carry every field required by
[DEVELOPMENT.md](../../DEVELOPMENT.md#parser-fixture-rules) and the
[approved plan](../../docs/research/parser-fixture-manifest-proposal-20260914.md).
The schema is fixed here; the values stay unfilled until a fixture exists.

| Manifest field      | Required content                                                            | Rule                                                          |
| ------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Fixture ID          | Stable synthetic ID, e.g. `FIX-<family>-<nn>`                               | Never a filesystem path; no personal names                    |
| Source / provenance | How the bytes were produced, or the public source and retrieval reference   | Traceable to synthetic production or a licensed public source |
| License             | License or "owner-produced, no third-party content"                         | Compatible attribution where required                         |
| Saved FL version    | Version/build exactly as saved, mapped to one version row                   | Precise; no "latest" or "equivalent" mapping                  |
| Expected values     | Pre-registered value or declared absence for each of the four staged fields | Written before parsing; never back-filled from output         |
| SHA-256             | Digest of the exact committed bytes                                         | Recorded at force-add; re-checked before and after each run   |
| Privacy approval    | Owner sign-off record plus synthetic-only attestation                       | Required before any force-add                                 |

## Pre-registered expected values

Status vocabulary is the accepted one in
[FLP_PARSER.md](../../FLP_PARSER.md#adapter-shape): `extracted`, `unavailable`,
`unsupported`, or `failed`. Expected values are written into this manifest before
any parse; neither parser is ground truth.

| Field                   | Expected entry                      | Absent/unreadable handling                                  |
| ----------------------- | ----------------------------------- | ----------------------------------------------------------- |
| Saved FL Studio version | Exact version/build string as saved | `unsupported` or `failed`; never mapped to another value    |
| Base tempo              | Numeric BPM value as stored         | `unavailable` with reason; never guessed                    |
| Channel names           | Ordered list, one value per channel | Missing/deleted labeled per item; Unicode recorded verbatim |
| Sample references       | Raw reference string per channel    | Raw only; never resolved, missing, or validity claims       |

## Owner authoring recipe

To populate the corpus, produce each slot in the matching installed FL Studio
build and open a follow-up manifest PR. Do not import personal projects.

1. Open the row's FL Studio build (F01 oldest available, then 20, 21, 2024,
   and 2025/2026). Use `File > New` for a minimal, purpose-built project.
2. Realize the slot's case: F01 as the baseline; F02 with no base tempo; F03
   with no channels or sample references; F04/F05 as one controlled property
   difference; F06 as a plain version-coverage save.
3. Save explicitly from that build so the stored version string is the genuine
   saved version, then close FL Studio.
4. Derive F07-F10 as byte-level edits of the approved F01 base only, and record
   the exact derivation.
5. Compute each SHA-256 at force-add, fill every field above, replace the slot's
   `not covered` status, and request privacy sign-off in the PR.

## Provenance and privacy

- This file commits no third-party, personal, or generated FLP byte. There is
  nothing to attribute and no personal path or fixture content to review.
- `.gitignore` blocks `.flp`/audio. Any later fixture needs its own explicit
  force-add, its manifest entry, and privacy review in the PR that adds it.
- Tests must never enumerate the user's real project roots.

## Gating and next steps

1. This scaffold does not satisfy the fixture gate. It records the method
   decision and leaves every slot `not covered` rather than substituting bytes.
2. Spike #138 stays blocked until a corpus is populated from genuine FL Studio
   saves, reviewed, and merged; it does not start on this file.
3. Creating or force-adding any fixture, and starting the spike, remain
   owner-only decisions.
