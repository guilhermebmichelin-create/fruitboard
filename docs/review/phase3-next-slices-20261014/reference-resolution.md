# G5 proposal: bounded project-relative sample references

Status: **Proposal only. No accepted design change, implementation authorization,
G5 acceptance or Phase 3 acceptance.** This document does not change
[ADR-007](../../adr/007-sample-presence-boundary.md). Relative references and
dot/parent components remain outside its accepted eligibility rules until the
owner accepts a scoped amendment after a separate native-authority review.

## Recommendation

Consider one first expansion: let the existing explicit sample check examine a
literal relative reference using the current FLP file's parent directory as
Fruitboard's stated candidate base, but only when the resulting ordinary file
stays inside that project's already selected, enabled, qualified local NTFS
root. Keep the existing absolute-path behavior unchanged. Do not add folder
grants, environment or FL Studio placeholder expansion, search folders, cloud
lookup, audio reads or a persistent availability index.

This is the smallest useful extension because the native check already captures
the selected root, current source file, validated snapshot and user-triggered
request. The current source path supplies a deterministic candidate base, and
the existing metadata port can walk exact children below its held root. A
relative string alone does not establish what FL Studio does with it, though.
The existing [client test](../../../apps/client/src/library/sampleReferences.test.ts)
contains the synthetic value `..\Samples\Kick.wav`, and the
[policy test](../../../crates/sample-presence/tests/policy.rs) deliberately
reports it as unchecked; neither is FL Studio ground truth. A separate
owner-approved genuine saved reference and independent GUI/path evidence are
required before calling this FL Studio's resolution rule.
If that evidence cannot establish the rule, any shipped behavior must describe
it only as Fruitboard's explicit candidate-base rule, with the owner accepting
that product meaning.

The proposal is an amendment to the eligibility boundary in ADR-007, not an
assumption that the selected folder grant already defines the meaning of a
relative string. The selected root limits filesystem authority; the owner must
separately approve the base and normalization semantics.

## Proposed reference rule

The renderer still sends only the existing opaque request identifiers and
displayed fingerprint. The native host reads the saved references and current
source path from the authorized snapshot. For a relative reference, begin with
the directory containing that exact current FLP, then process ordinary
backslash-separated components in order. A normal component adds one child;
`.` stays at the current directory; `..` removes one component. If `..` would
move above the selected root, return the existing fixed `outside_root`
unchecked result before querying a candidate. Never pass `.` or `..` to a
Windows path API. After normalization, inspect the exact resulting child chain
from the already qualified root.

This defines a deterministic Fruitboard candidate base. It does not assert
that every FL Studio version uses the same base. A confirmed FL Studio rule can
be claimed only for the exact build(s) covered by approved GUI evidence.

The relative parser should reuse the existing conservative component checks:
reject empty or trailing components, alternate `/` separators, drive-relative
and rooted paths, URLs, UNC/device namespaces, streams, wildcards, reserved
names, ambiguous trailing dots/spaces, short-name spellings, controls and
bidi formatting characters. Preserve spelling; do not normalize Unicode or
guess component case. Keep the existing size, component, operation, handle,
deadline and report caps. The current absolute-path parser continues rejecting
`.` and `..` exactly as it does now.

Relative references that normalize to the selected root itself name a
directory, not an ordinary sample file, so they stay `not_checked` with the
existing `not_regular_file` reason. A normalized relative path outside the root
gets `outside_root`; malformed relative syntax gets the existing fixed syntax
reason; placeholders get `unresolved_placeholder`. No new outcome or raw OS
error is needed.

## User-facing meaning and examples

Keep the current explicit **Check saved samples** action. The report remains an
ephemeral metadata observation with the existing timestamp and per-channel
ordering. `present` means an ordinary file was observed at the permitted
candidate path when checked. `not_found` means an authoritative absence at that
exact candidate path. Neither says that audio is playable, content matches, or
the complete FL Studio dependency set is available. An unsupported or unsafe
reference remains **Not checked**, never a missing warning.

For example, with selected root `C:\FruitboardFixtures` and current project
`C:\FruitboardFixtures\Songs\Demo.flp`, the proposed Fruitboard candidate base
is `C:\FruitboardFixtures\Songs`:

| Saved reference                                                         | Proposed handling                                      | Reason                                                |
| ----------------------------------------------------------------------- | ------------------------------------------------------ | ----------------------------------------------------- |
| `Samples\Kick.wav`                                                      | Check `Songs\Samples\Kick.wav` under the selected root | Ordinary relative child path                          |
| `..\Shared\Kick.wav`                                                    | Check `Shared\Kick.wav` under the selected root        | Parent step stays within the root                     |
| `C:\FruitboardFixtures\Songs\Samples\Kick.wav`                          | Keep current exact-absolute-path behavior              | Existing ADR-007 eligibility                          |
| `..\..\Outside\Kick.wav`                                                | Not checked; no candidate lookup                       | Normalization would leave the selected root           |
| `%FLStudioData%\Kick.wav` or `$HOME\Kick.wav`                           | Not checked; do not expand                             | Placeholder/environment expansion is out of scope     |
| `C:\AudioLibrary\Kick.wav`                                              | Not checked; no candidate lookup                       | Outside the selected root                             |
| `\\server\share\Kick.wav`, `\Samples\Kick.wav`, or `C:Samples\Kick.wav` | Not checked                                            | UNC, rooted and drive-relative forms stay unsupported |
| A linked, offline or recall-on-access child                             | Not checked                                            | Do not follow, hydrate or query a substitute          |

These are synthetic public example paths, not owner data or fixture paths. The
current UI says relative paths remain unchecked in
[SamplePresenceControls.tsx](../../../apps/client/src/library/SamplePresenceControls.tsx).
If this proposal is accepted, change that explanation to name the actual
Fruitboard candidate base in plain language, such as `Relative paths are checked
from this project's folder and only within the selected folder.` Only say
`FL Studio resolves these paths from the project folder` after the GUI evidence
establishes that claim. Keep the explanation visible and available to assistive
technology. Preserve the explicit check/cancel actions, keyboard operation,
focus restoration, status announcement and non-color status meaning already
covered by the app contract. Do not return resolved paths or add clickable,
open or play actions.

## Authority and lifecycle fences

Any implementation must retain the existing authority described by
[ADR-007](../../adr/007-sample-presence-boundary.md) and the delivered
[Library integration](../sample-presence-ui-284/README.md):

- The root is the current project's enabled, selected, qualified local NTFS
  folder. A relative reference never selects a root or grants access to another
  configured root.
- The native host obtains the current FLP path and sample projection from the
  authorized current snapshot. Renderer input remains opaque IDs and the
  displayed size/modified fingerprint; it supplies no path, base, reference,
  folder list or retry budget.
- Resolve components in memory against the source path already qualified under
  the root. Do not call `canonicalize`, use the process working directory, open
  `..`, or reopen an untrusted joined path. Walk only exact next children from
  the held root using the existing no-follow/no-recall metadata port. Do not
  enumerate folders or search FL Studio locations.
- Retain local NTFS, case-mode, identity, ordinary-file, ancestry, reparse,
  offline/recall, access-denied and authoritative-absence rules. An absent
  intermediate component is `not_found` only when the trusted parent reports
  exact-name absence. Errors and uncertain metadata remain `not_checked`.
- Reuse the root/location/source/snapshot/session fences, source identity/size/
  high-resolution time and current parent/name binding. Root removal, source
  replacement, a newer snapshot, session shutdown or cancellation discards the
  whole provisional report. Renderer row/fingerprint/request-generation fences
  still discard late replies. Reports remain session-local and nonpersistent.
- Preserve the one-worker gate and current 256-channel, 4,095 UTF-16-unit
  reference, 64-component, 2,048-operation, 132-handle, two-second and 64 KiB
  bounds. Count the full normalized path under the same depth cap. Do not add a
  queue or cross-request cache. Existing limits remain proposed operational
  stops, not newly accepted performance budgets.

Metadata-preserving content edits remain undetectable without reading or
hashing the FLP. The report is still a time-of-check observation. No schema,
snapshot, sync, telemetry, raw log or debug field should retain the candidate
base, resolved path, volume/file identity or raw OS error.

## Scope comparison

| Option                                                                 | User value                                                                           | New authority or uncertainty                                                                                                                | Recommendation              |
| ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- |
| Project-directory-relative candidates constrained to the existing root | Covers co-located samples and parent-relative paths that stay inside the chosen root | Adds one explicit base and safe dot/parent normalization; needs native review and genuine GUI ground truth                                  | First bounded proposal      |
| Additional external-folder grants                                      | Could check factory/user libraries outside the selected project root                 | Needs a distinct user grant, picker and revocation model, persisted authority, multiple-root precedence, path privacy review and new limits | Keep open as a later design |
| Placeholder/environment mapping                                        | Could map saved symbolic names to FL Studio, user or environment folders             | Needs exact per-version mapping, precedence and installation/user-data discovery; can silently broaden access or cross cloud boundaries     | Keep open as a later design |
| Search all configured roots or FL Studio search folders                | Might find relocated or duplicated samples                                           | Adds fallback/search semantics, ambiguous matches, wider access and unpredictable work                                                      | Exclude from this slice     |

Relative resolution would not settle relocation, duplicate-name precedence,
external library availability, DriveFS behavior or full dependency availability.
Those remain open G5 work in the
[gap ledger](../flp-intelligence-readiness/gap-ledger.md).

## Acceptance table for a future implementation

| Area                                   | Acceptance evidence required                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Boundary that stays open                                                                                                          |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Design and semantics                   | Owner accepts the exact candidate base, dot/parent normalization, root escape behavior and whether the result is described as Fruitboard mapping or FL Studio mapping; scoped native-authority review is accepted before code                                                                                                                                                                                                                                                                                                                                        | ADR-007 is unchanged until that review and acceptance                                                                             |
| Policy                                 | Tests cover child and parent paths, `.` normalization, root-level files, root escape with zero candidate queries, malformed/placeholder/absolute/outside-root forms, duplicate slots and 64-component limits                                                                                                                                                                                                                                                                                                                                                         | No environment expansion, forward-slash compatibility or guesses                                                                  |
| Windows authority                      | Real NTFS tests prove exact-child-only access, case-sensitive behavior, no reparse traversal, parent/name binding, source/root replacement fences, access denied versus authoritative absence, and unchanged source/sample bytes. Port review and injected offline/recall cases prove the adapter stops before opening those candidates.                                                                                                                                                                                                                             | Cloud/provider and blocked kernel behavior remain unqualified                                                                     |
| Freshness and cancellation             | Host tests cover disabled/removed root, source replacement/save, superseding snapshot, cancel/deadline, shutdown, stale row and late transport reply; results are discarded as a whole                                                                                                                                                                                                                                                                                                                                                                               | Existing undetectable identity/size/time-preserving content edits remain a stated limit                                           |
| Genuine reference and GUI ground truth | For an FL Studio compatibility claim, the owner approves a genuine save with a relative reference; exact displayed/raw spelling and target are independently recorded; a controlled GUI open proves the base, including when process working directory differs. Use a harmless generated sample found only at the proposed target and confirm FL Studio actually loads it, not only that the raw string is displayed. If the owner approves an app-defined base without that compatibility claim, install evidence verifies Fruitboard's stated interpretation only. | Compatibility evidence applies only to the exact FL Studio build(s) tested; existing synthetic contract strings do not qualify it |
| Installed UI and accessibility         | Fresh installed Windows app shows the base and root scope clearly, distinguishes present/not-found/not-checked, supports keyboard check/cancel and focus restoration, announces status, and remains usable at narrow width and enlarged text                                                                                                                                                                                                                                                                                                                         | Does not qualify every FL Studio version, cloud provider or full dependency resolver                                              |
| Privacy and reporting                  | IPC/response/log checks prove no path/base/raw OS error or new filesystem capability crosses the existing boundary; reports stay ephemeral                                                                                                                                                                                                                                                                                                                                                                                                                           | No persistent index, sync payload, telemetry or audio read                                                                        |
| Phase/G5 status                        | Update the G5 gap ledger only after an owner-reviewed implementation PR and evidence; record remaining gaps separately                                                                                                                                                                                                                                                                                                                                                                                                                                               | No implicit G5 closure or Phase 3 acceptance                                                                                      |

The current review packet already records the report states, request/worker
lifecycle, exact-child authority and installed UI behavior in its
[policy](../sample-presence-policy-280/README.md),
[Windows](../sample-presence-windows-282/README.md) and
[app](../sample-presence-ui-284/README.md) reviews. An implementation should
extend those contracts rather than create a second report lifecycle.

## Likely implementation order and files

1. Review and accept a narrow ADR-007 amendment first. Do not code a relative
   base while the authority review still says that base is a guess.
2. Extend policy parsing and execution in
   [path.rs](../../../crates/sample-presence/src/path.rs),
   [execution.rs](../../../crates/sample-presence/src/execution.rs) and
   [lib.rs](../../../crates/sample-presence/src/lib.rs). Add injected-port tests
   in [policy.rs](../../../crates/sample-presence/tests/policy.rs). Keep
   absolute-path eligibility and existing report vocabulary stable.
3. Extend the Windows adapter and adversarial tests in
   [native.rs](../../../crates/sample-presence/src/native.rs) and
   [native/tests.rs](../../../crates/sample-presence/src/native/tests.rs).
   Validate any change to the source-parent authority and full normalized-depth
   accounting in the native review.
4. Review host and renderer freshness without adding renderer path authority:
   [host.rs](../../../apps/desktop/src-tauri/src/foundation/sample_check/host.rs),
   [host tests](../../../apps/desktop/src-tauri/src/foundation/sample_check/host/tests.rs),
   [samplePresence.ts](../../../apps/client/src/library/samplePresence.ts),
   [SamplePresenceControls.tsx](../../../apps/client/src/library/SamplePresenceControls.tsx)
   and their tests. No new permission or command should be needed; stop for a
   separate design if implementation requires broader filesystem authority.
5. Update the policy, Windows and app review notes, then update the G5 ledger
   with only the demonstrated scope. Keep extension work separate from any
   later external-folder or placeholder proposal.

The existing [parser fixture rules](../../../DEVELOPMENT.md#parser-fixture-rules) require exact owner
approval, provenance/license, expected fields, SHA-256 and privacy review before
committing a new FLP. First establish the relative mapping with a controlled
synthetic sample and an exact-build FL Studio GUI record. Do not use personal
projects or audio in CI. A constructed path string or fake metadata port proves
policy only, not FL Studio semantics. Retain private raw GUI/database/log
evidence outside Git; use only synthetic public paths in review documents.

## Owner decisions still needed

- Is the proposed base the containing directory of the exact current FLP, and
  may a relative path climb to another folder as long as it remains within the
  already selected root?
- Should the owner require genuine GUI proof of FL Studio's base rule before
  implementation, or explicitly approve an app-defined candidate-base rule
  whose UI says “Fruitboard checks from this project's folder” without claiming
  FL Studio would load the same target?
- Which exact FL Studio build and owner-approved synthetic fixture should
  establish relative spelling and base behavior? F12 is the currently approved
  positive raw-reference case, but the current record does not establish a
  relative-reference mapping.
- Is the current selected-root metadata authority acceptable for every
  normalized candidate within that root? Any request for additional folders,
  placeholders, environment expansion, cloud mounts or search behavior needs a
  separate scoped decision.

Until those decisions and reviews are recorded, relative paths remain
`not_checked`, ADR-007 remains unchanged, G5 stays open and this proposal must
not be described as an approved design or completed feature.
