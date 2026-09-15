# Parser fixture-manifest proposal

- Status: **Proposed; not approved. No fixture created, no corpus authorized, PyFLP absent.**
- Date: 2026-09-14
- Decision owner: product owner (fixture approval and spike start)
- Related: issue #138, [Rust parser research spike proposal](rust-parser-spike-proposal.md),
  [ROADMAP.md parser selection before Phase 3](../../ROADMAP.md#parser-selection-before-phase-3)
  items 1-6, [FLP_PARSER.md](../../FLP_PARSER.md),
  [ADR-002](../adr/002-flp-parser-process.md),
  [DEVELOPMENT.md parser fixture rules](../../DEVELOPMENT.md#parser-fixture-rules).
- Scope of this document: documentation only. It creates no fixture, no fixture
  bytes, no parser code, no build, and no GitHub issue, comment, or pull request.
  It prepares one artifact for owner decision: the fixture manifest that gates
  spike #138.

## Gating relationship

Issue #138 is owner-approved for filing, but spike execution is gated on a
separately owner-approved fixture manifest. This document prepares that manifest
so the owner can approve, edit, or decline it.

- The manifest is approved in its own PR, separate from the spike's start.
- The spike does not start until the manifest is owner-approved and merged.
- This proposal neither opens that PR nor approves any fixture.
- PyFLP stays absent: it is a gated fallback candidate only, evaluated against
  this same corpus after a recorded Rust shortfall, under
  [ADR-002](../adr/002-flp-parser-process.md) and the licensing gate in
  [FLP_PARSER.md](../../FLP_PARSER.md#licensing-gate-p0-c).

## Boundaries not crossed

- No fixture bytes, truncation, corruption, or generation run.
- No edits to `crates/`, [FLP_PARSER.md](../../FLP_PARSER.md), or
  [ADR-002](../adr/002-flp-parser-process.md).
- No GitHub mutation: no issue, pull request, comment, label, or milestone.
- No personal projects, personal paths, real project hashes, or fixture contents.

## Required manifest entry fields

Every committed fixture entry must carry the fields required by
[DEVELOPMENT.md](../../DEVELOPMENT.md#parser-fixture-rules). This proposal
defines the schema; it leaves every value unfilled.

| Manifest field      | Required content                                                            | Rule                                                          |
| ------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------- |
| Fixture ID          | Stable synthetic ID, e.g. `FIX-<family>-<nn>`                               | Never a filesystem path; no personal names                    |
| Source / provenance | How the bytes were produced, or the public source and retrieval reference   | Traceable to synthetic production or a licensed public source |
| License             | License or "owner-produced, no third-party content"                         | Compatible attribution where required                         |
| Saved FL version    | Version/build exactly as saved, mapped to one version row                   | Precise; no "latest" or "equivalent" mapping                  |
| Expected values     | Pre-registered value or declared absence for each of the four staged fields | Written before parsing; never back-filled from output         |
| SHA-256             | Digest of the exact committed bytes                                         | Recorded at force-add; re-checked before and after each run   |
| Privacy approval    | Owner sign-off record plus synthetic-only attestation                       | Required before any force-add                                 |

No SHA-256 value, fixture byte, or sample content appears in this proposal;
every field is populated only in the approved manifest PR.

## Proposed corpus allocation

Bound carried from issue #138: up to **twelve** owner-approved fixtures. The
table below is the proposed allocation. Ten slots are assigned; two are reserved
and remain empty unless the owner authorizes their use.

| Slot | Proposed ID    | Role / case family                                              | Version row                |
| ---- | -------------- | --------------------------------------------------------------- | -------------------------- |
| F01  | FIX-BASE-MIN   | Stage 1+2 baseline; base for the malformed byte derivatives     | Oldest producible baseline |
| F02  | FIX-FL20-MIN   | Version coverage; absent-field: base tempo not present          | FL 20                      |
| F03  | FIX-FL21-MIN   | Version coverage; absent-field: no channels / no sample refs    | FL 21                      |
| F04  | FIX-FL2024-A   | Version coverage; single-property-difference member A           | FL 2024                    |
| F05  | FIX-FL2024-B   | Single-property-difference member B (one property changed)      | FL 2024                    |
| F06  | FIX-FL2025-MIN | Version coverage                                                | FL 2025/current            |
| F07  | FIX-RB-TRUNC   | Robustness: truncation, typed outcome; derived from F01         | n/a (derivative)           |
| F08  | FIX-RB-UNKNOWN | Robustness: unknown event, typed outcome; derived from F01      | n/a (derivative)           |
| F09  | FIX-RB-MALFORM | Robustness: malformed length, typed outcome; derived from F01   | n/a (derivative)           |
| F10  | FIX-RB-LIMIT   | Robustness: resource limit (oversized strings/counts)           | n/a (derivative)           |
| F11  | Reserved       | Sixth version row (adjacent unseen version), only if producible | Adjacent unseen            |
| F12  | Reserved       | Owner-approved overflow only; a bound change is a new decision  | n/a                        |

Derivative fixtures are byte-level derivations of an approved base fixture; they
still count against the twelve-fixture bound and still need their own manifest
entry, digest, and privacy approval.

## Supported-version rows

Bound carried from issue #138: up to **six** version rows. A row the owner cannot
lawfully produce is recorded as "not covered", never substituted.

| Row | Saved version band              | Fixture slot   | Matrix result                           |
| --- | ------------------------------- | -------------- | --------------------------------------- |
| 1   | Oldest producible baseline      | F01            | To record per fixture                   |
| 2   | FL 20                           | F02            | To record per fixture                   |
| 3   | FL 21                           | F03            | To record per fixture                   |
| 4   | FL 2024                         | F04, F05       | To record per fixture                   |
| 5   | FL 2025/current                 | F06            | To record per fixture                   |
| 6   | Adjacent unseen version, if any | F11 (reserved) | Recorded `unsupported` when not covered |

## Pre-registered expected values

Status vocabulary is the accepted one in
[FLP_PARSER.md](../../FLP_PARSER.md#adapter-shape): `extracted`, `unavailable`,
`unsupported`, or `failed`. Expected values are written into the manifest before
any parse. Neither parser is ground truth, and agreement between parsers is not
evidence for either.

| Field                   | Expected entry                      | Absent/unreadable handling                                  |
| ----------------------- | ----------------------------------- | ----------------------------------------------------------- |
| Saved FL Studio version | Exact version/build string as saved | `unsupported` or `failed`; never mapped to another value    |
| Base tempo              | Numeric BPM value as stored         | `unavailable` with reason; never guessed                    |
| Channel names           | Ordered list, one value per channel | Missing/deleted labeled per item; Unicode recorded verbatim |
| Sample references       | Raw reference string per channel    | Raw only; never resolved, missing, or validity claims       |

The single-property-difference pair (F04, F05) exists to pin exactly one
controlled property change, so a parser that ignores that property is
distinguishable from one that reads it.

## Robustness cases as typed outcomes

Truncation, unknown-event, malformed-length, and resource-limit cases are each
recorded as a typed per-file outcome, never as a crash or a silent success. The
resource-limit fixture exercises oversized strings/counts in the input; it is a
data fixture, not a denial-of-service run against the host. For every case the
harness must not modify the input, must verify SHA-256 before and after, and must
never load embedded plugins or scripts.

## Provenance and license

- Allowed sources: purpose-built minimal FLPs produced synthetically for this
  corpus, upstream-public fixtures with compatible attribution/license, and
  malformed byte fixtures derived from an approved base.
- Personal production projects are excluded. A minimized/sanitized derivative
  would require explicit written owner approval and is not proposed here.
- Each entry records its license and, where applicable, attribution and source
  reference.

## SHA-256 and byte identity

Digests are recorded for the exact committed bytes at force-add and re-checked
before and after every parse. This proposal contains no real-project hash. Any
retained prototype binary is an independently copied file with recorded
provenance, never a hardlink into a writable build cache.

## Privacy approval and repository handling

- Fixtures and audio remain blocked by `.gitignore`; a fixture PR must
  force-add each approved file, update the manifest, and receive privacy review.
- Approval is owner sign-off on a synthetic-only corpus. Tests never enumerate
  the user's real project roots.
- No personal paths or fixture contents appear in logs, issues, or pull
  requests.
- `pnpm privacy:check` must pass on the manifest PR.

## What remains owner approval

1. This manifest proposal (approve, edit, or decline).
2. Creation of any fixture and the dedicated manifest PR, including each
   force-add, digest, and privacy sign-off.
3. Starting spike #138, only after the manifest is approved and merged.
4. Any change to the twelve-fixture, six-row, or 15/21-day bounds.
5. Parser selection and its recording in ADR-002 and FLP_PARSER.md.

## Traceability to issue #138

| #138 item                          | Manifest coverage                                                   |
| ---------------------------------- | ------------------------------------------------------------------- |
| Q1 version + tempo, absent labeled | F01-F06, with F02 covering absent tempo                             |
| Q2 channels + sample refs          | F01, F03, F04, F05; raw references only, no resolve/missing claim   |
| Q3 version matrix coverage         | Supported-version rows; F11 reserved for the sixth row              |
| Q4 runtime/memory/binary cost      | F10 resource limit plus per-file size recorded per entry            |
| Q5 safety boundary                 | Robustness cases, byte-identity re-checks, no plugin/script loading |
| Q6 PyFLP fallback                  | Same corpus and expected values reused; PyFLP absent until then     |

## What happens after this proposal

1. The owner reviews this record and approves, edits, or declines it.
2. If approved, the owner authorizes a dedicated manifest PR and the parented
   fixtures, with provenance, license, digests, and privacy review.
3. The owner starts spike #138 only after the manifest is merged.
4. The spike runs within its time and size bounds and publishes evidence plus a
   recommendation.
5. The owner records the decision in ADR-002 and FLP_PARSER.md. Until then the
   production parser stays unselected and PyFLP stays absent.

## Non-goals

- No fixture creation, force-add, or manifest PR.
- No parser code, build, or research run.
- No parser selection by this proposal.
- No Phase 3 scope: arrangements, plugin-state decoding, duration estimation,
  playlist/mixer normalization, diagnostics UI, and Plugin Explorer remain later.
- No increase to the fixture, version-row, or time bounds.
- No PyFLP or parser dependency added to the application.
