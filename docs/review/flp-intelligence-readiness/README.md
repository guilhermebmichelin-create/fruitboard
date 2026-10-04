# FLP intelligence readiness review

Status: **Review packet; budgets and Phase 3 exit are not accepted.**
Date: 2026-10-04. Related issue:
[#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259).
Reviewed application boundary: merged
[PR #261](https://github.com/guilhermebmichelin-create/fruitboard/pull/261),
`19b95362a66a82c91cba56fc8f2226c6ef899cc2` (tree identical to its tested head
`d32a6e358a6a91e3cc133dc3a3ce553334688690`).

## Owner summary

The development app can safely analyze selected local projects, retain saved
facts, explain missing or unsupported values, and group saved plugin references.
Its current evidence covers three exact saved FL Studio builds and a small
approved synthetic corpus. It supports ordinary project sizes, but that does
not establish support for every project or plugin.

The bounded development workflow is delivered. The broader Phase 3 roadmap is
not complete: pattern summaries still stop at the parser, sample availability
is unchecked, and external plugins, richer arrangements and whole-app resource
qualification remain gaps. Keep development inside FLP intelligence; this
packet does not authorize Phase 4, production activation or public distribution.

Use the [field reliability matrix](field-reliability.md) to distinguish real
saved-file evidence from constructed tests and from fields visible in the app.
The [gap ledger](gap-ledger.md) records what remains and the next focused change.

## Delivered behavior and evidence

| Surface                              | Merged delivery                  | Evidence and boundary                                                                                                                                                                                                                                                          |
| ------------------------------------ | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Read-only parser and authority       | PRs #224, #227, #229, #231, #233 | [Parser contract](../../../crates/flp-parser/README.md), approved corpus, Windows/Linux supervision tests; exact builds and per-field restrictions remain.                                                                                                                     |
| Windows development packaging        | PR #235                          | [Installed lifecycle](../parser-packaging-234/README.md): fixed sibling, reuse, crash recovery, rejection, missing binary and reinstall; unsigned development identity.                                                                                                        |
| Immutable metadata                   | PR #237                          | [Storage](../parser-metadata-236/README.md): bounded typed projection, source/publication fences, current/history distinction, atomic publication and rollback.                                                                                                                |
| Durable native analysis              | PR #239                          | [Worker](../analysis-jobs-238/README.md): local NTFS held-source observations and independent hashes, lease/session fencing, bounded attempts, cancellation and recovery.                                                                                                      |
| Library scalar facts and channels    | PRs #241, #243                   | [Facts](../library-project-details-240/README.md), [channels](../library-channels-242/README.md): authorized current results, explicit provenance, unsupported/older states.                                                                                                   |
| Analysis status and request controls | PRs #245, #247                   | [Status](../analysis-status-244/README.md), [requests](../analysis-request-246/README.md): durable attempt state, explicit Analyze/Retry, unchanged-input attempt limits.                                                                                                      |
| Saved sample and plugin references   | PRs #249, #251                   | [Samples](../library-samples-248/README.md), [plugins](../library-plugins-250/README.md): inert saved text and coverage; no embedded target is opened.                                                                                                                         |
| Installed Library workflow           | PR #253                          | [Installed record](../installed-library-details-252/README.md): real renderer/native/parser/storage, restart, source/root changes, unavailable parser, interrupted work and accessibility. Source is the earlier `2d5c769` boundary; not a current-head combined Explorer run. |
| Ordinary-size analysis               | PR #257                          | [Size policy](../analysis-size-255/README.md): 4,601,596/25,000,000/64 MiB constructed inputs, typed over-limit rejection, unchanged hashes; parser adapter 0.1.1.                                                                                                             |
| Saved Plugin Explorer                | PR #256                          | [Explorer](../plugin-explorer-254/README.md): exact provenance-aware groups, complete counts or limited state, bounded input/output, freshness and root isolation; native screenshots, not installed qualification.                                                            |
| Native resource observations         | PRs #260, #261                   | [Quiet before/after](../explorer-performance-259/README.md): full native Explorer plus serialization and real supervised worker; independent artifacts and source hashes. [Earlier characterization](../metadata-resources-258/README.md) retains its failed noise guard.      |

These rows identify the relevant merged delivery PRs, not every prerequisite.
The dated reports retain their original source and build boundaries. Current
CI tests code integration; it does not retrospectively rerun a historical
installed journey or expand GUI-verified compatibility.

## Phase 3 roadmap assessment

The [roadmap](../../../ROADMAP.md#phase-3-proposed-issues-and-prs) is broader
than the selected initial parser/development slice. A limited milestone review
must not be presented as the full phase exit.

| Proposed Phase 3 item                          | Current assessment                                                                | Remaining acceptance work                                                                                                |
| ---------------------------------------------- | --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| 1. Compatibility matrix and approved corpus    | Maintained narrow corpus and ordinary tests; three exact builds                   | Broader representative version/feature coverage; FL20/21, absent tempo and zero-channel genuine saves remain unverified. |
| 2. Metadata normalization and provenance       | Selected facts have typed validation, immutable storage and honest display        | Broader project/arrangement fields and independent field ground truth.                                                   |
| 3. Duration/bar estimates                      | Verified pattern-clip endpoint and explicitly low-confidence bars/nominal seconds | Richer arrangements, audio clips, tempo automation and tails; no finished-song duration claim.                           |
| 4. Channel/pattern/automation/MIDI summaries   | Channel labels/count/instruments displayed; pattern IDs/names parsed              | Pattern count/names are not retained by the application projection or displayed; automation/MIDI summaries are absent.   |
| 5. Mixer/plugin normalization                  | Saved top-level name/class/vendor and two verified instrument classes             | Mixer effects, nested coverage, broader external VST identity and classification.                                        |
| 6. Sample/reference resolution                 | Raw saved references displayed safely                                             | No existence, availability, relocation or dependency resolution.                                                         |
| 7. Diagnostics/reparse UI                      | Fixed analysis states and explicit retry/reanalysis                               | Broader compatibility/error qualification; no FLP modification or repair.                                                |
| 8. Plugin Explorer MVP                         | Delivered bounded saved-reference query and display                               | Current installed combined workflow and richer reference matrix; installed-plugin discovery remains separate.            |
| 9. Resource/error qualification and checkpoint | Quiet narrow measurements and this review packet                                  | Explicit budget decision, whole-app/UI qualification and owner checkpoint decision; scanner failures remain carried.     |

## Performance decision to review

The [complete proposal](../explorer-performance-259/budget-proposal.md) defines
the timed boundaries, excluded work, process counters and narrow constructed
matrix. Its proposed warm p95 limits are:

- Explorer native command plus outer serialization: 200 ms.
- Native worker: 1,000 ms through 25 MB; 3,000 ms at the 64 MiB bound.
- Supervised parse request: 2,000 ms; the hard 10-second deadline is unchanged.
- Worker parent and parser, separately: each 128 MiB peak working set and
  128 MiB peak private commit. This is not full desktop/WebView2 memory.

The quiet results meet these candidates on the recorded matrix; no target is
accepted by this packet. At 2,000 Explorer entries, warm p95 was 104.06 ms
after optimization versus 332.56 ms before. Worker p95 was about 404 ms at
25 MB and 1,164 ms at 64 MiB. Ten measured warm samples per case are small
single-host observations; OS caches were not flushed.

Accepting these budgets would establish targets for future validation at these
boundaries. It would not approve overall app latency/memory, all real projects,
the scanner's older failures, or a Phase 3 exit. Record any approved decision
with its date, scope, explicit limitations and evidence; preserve the original
proposal and raw samples rather than rewriting historical results as accepted.

## Owner decisions and next work

Decisions are currently **pending**, including after merging this documentation:

1. Accept or revise the specific native test budgets above.
2. Review the bounded development milestone with the listed limitations.
   This is separate from accepting the full Phase 3 exit.
3. Confirm the remaining Phase 3 scope and deliberate exclusions before any
   later-phase entry. No missing roadmap item is silently removed here.

The next recommended implementation is [G2 in the gap ledger](gap-ledger.md):
carry the already verified 2026 pattern count and stored names through typed
validation, immutable persistence and the authorized Library details view.
It can reuse approved F13 instead of inventing new FLP ground truth. Follow
with fresh-identity installed coverage and independently approved compatibility
fixtures. Keep production activation and later phases outside these changes.

The [development checkpoint rule](../../../DEVELOPMENT.md#phase-review-checkpoint)
requires an explicit owner acceptance before a later phase starts. Approval or
merge of an implementation/docs PR alone is not that checkpoint decision.

## Validation and retained evidence

This packet changes documentation only. Application/corpus/lockfiles are
byte-identical to merged PR261, whose exact head passed all 15 checks, including
Windows enabled native features, packaging smoke and CodeQL. No local native
rebuild, installer, app launch, benchmark or profile operation is needed.
Run pinned documentation, privacy, script and repository Node policy checks;
fresh final-head PR CI remains required before review and manual merge.

The existing reusable validation cache remains owned by Codex `/root` and is
not written by this task. No new compiler cache or cleanup is introduced.
Private check logs, exact source/CI provenance and storage/cache handoff remain
outside Git and outside compiler caches. Earlier binaries, fixtures, databases
and reports must remain; no personal project or profile is read or changed.
