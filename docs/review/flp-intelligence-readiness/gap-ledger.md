# FLP intelligence gaps and next implementation

Status: **Open gaps and proposals; no acceptance or later-phase authorization.**
Historical baseline: merged PR261. G2 was delivered by merged PR264;
see the [patterns review](../library-patterns-263/README.md). Issue #265 covers the
[current installed journey](../installed-combined-265/README.md) and corrects its
enlarged-text layout defect. Other gaps remain open. This ledger supplements the historical dated reports;
it does not erase their failures or promote their source boundaries.

## Current gaps

| ID  | Gap                                                                                 | Evidence/decision needed                                                                                                                                                                               | Effect on readiness                                                                                                                                                     |
| --- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1  | Native resource budgets remain proposed                                             | Explicit owner accept/revise decision on [the bounded proposal](../explorer-performance-259/budget-proposal.md), recorded with date/scope; #259 stays open                                             | Quiet measurements meet candidates, but approved-budget qualification is not claimed.                                                                                   |
| G2  | Saved pattern count/names integration delivered by merged PR264                     | Typed advertised-field validation, bounded immutable projection and authorized Library display using existing F13                                                                                      | G2 delivered; current installed integration is also covered by #265. Automation/MIDI remain G6.                                                                         |
| G3  | Compatibility/field ground truth is narrow                                          | Separately approved genuine fixtures: additional exact builds, absent tempo/zero channels, multi-Sampler provenance, richer arrangements and external wrapper metadata                                 | No broad supported-version/plugin or complete duration claim. Dates/time counter lack independent GUI qualification.                                                    |
| G4  | Installed combined workflow covered; whole-app resource decisions pending           | [Fresh installed journey and reflow regression](../installed-combined-265/README.md); still needs approved startup/UI/IPC/memory boundaries and representative/limit-scale measurements                | #265 passes the bounded current workflow and characterizes seven-entry installed resources. This does not accept budgets, qualify scale or replace historical failures. |
| G5  | Bounded sample presence delivered as PR285; broader resolution open                 | Merged explicit local metadata observations under ADR-007; relative, placeholder, external-folder and cloud resolution need separate accepted scope                                                    | Saved reference text alone proves no availability; delivered observations cover only eligible exact paths inside the selected local NTFS root.                          |
| G6  | Richer FLP summaries incomplete; note-count design #288 and qualified fixtures #290 | Implement bounded exact-build parsing/full validation, then immutable Library integration; broader automation/MIDI, mixer/nested plugins, tempo changes and arrangements need their own scope/evidence | F16/F17 qualify registered records and missing data; no note-count extraction/display is delivered yet. Phase 3 scope remains incomplete.                               |
| G7  | Production/distribution gates open                                                  | Explicit activation decision, representative compatibility/provenance review, license selection and protected signing/update/rollback process                                                          | Development feature, unsigned test build and green CI are not a public release.                                                                                         |
| G8  | Earlier Scanner MVP gaps and qualification failures                                 | Existing separate scanner evidence/decisions; retain unchanged targets and historical results                                                                                                          | Native metadata speedup does not qualify scanner performance or DriveFS/FAT32.                                                                                          |

G8 carries the [accepted Phase 2 record](../phase-2-integration/acceptance-2026-09-14.md)
and later [scanner recovery evidence](../phase-2-integration/installed-journey/run-20260930-scanner-recovery.md).
The latter narrows some earlier workflow gaps; it does not accept the old scanner
performance failures, 100,000-entry scale target, DriveFS host behavior or
FAT32/cross-volume identity. Preserve F1/F2/F3 and the previously recorded
non-qualifying 10,173 ms warm p95 versus the unchanged 10,000 ms target.
Issues [#47](https://github.com/guilhermebmichelin-create/fruitboard/issues/47),
[#48](https://github.com/guilhermebmichelin-create/fruitboard/issues/48)
and [#40](https://github.com/guilhermebmichelin-create/fruitboard/issues/40)
retain their actual dispositions. A stale issue body is not evidence of a new
implementation defect or of an accepted fix; use the linked merged source/tests.

## G2 implementation

Issue #263 implements the following criteria. After analyzing approved F13,
the Library can show its three saved patterns
with stored names. A project with missing/unsupported pattern data gets explicit
copy; older immutable snapshots stay readable without invented zero counts.

1. Add bounded typed validation for advertised `patternCount`/`patternNames`.
   Check exact supported build, count/ID uniqueness and range, names/statuses,
   correspondence where claimed, maximum counts/text/output and malformed
   relationships. Preserve unavailable/unsupported instead of guessing.
2. Add only these validated fields to the existing private metadata projection.
   Review payload version/backward reading before deciding if a migration is
   needed. Existing results are immutable; refresh must not rewrite or reparse
   them. Source/root/publication/lease authority and negative-result preservation
   remain required.
3. Extend the existing authorized details response and strict renderer decoder,
   then add a keyboard-accessible progressively disclosed pattern list. Define
   current/older/empty/missing/unsupported/loading/error states before coding;
   saved IDs/names remain inert text, never playback/plugin authority.
4. Validate the actual parser-to-storage-to-display path with approved F13,
   existing absent-marker saves and constructed malformed replies. Cover root/
   source changes, late replies, old snapshots, count/bounds failures and a11y.
   Hash every approved FLP before/after; no new fixture approval is needed for
   using the unchanged existing corpus.
5. Run required default/native feature checks and pinned Windows `pnpm check`,
   require exact updated-head CI, and retain narrow/native visual evidence.
   Use the same single owned cache, explicit storage preflight with expected
   output and exclusive heavyweight windows. Keep binaries/evidence independent.

Non-goals: raw playlist lists, MIDI/automation or note decoding, broader saved
build support, new fixtures, pattern editing/playback, public release, scanner
budget changes or later-phase features. This scoped delivery is not an instruction to an additional agent or an accepted
phase change. Final-head CI and owner review remain required.

## Qualification sequence

G2 is delivered and #265 covers G4's bounded current installed combined workflow
using fresh explicit review identities. Startup/UI/IPC and whole-app memory were
characterized; their target/boundary decisions and broader qualification remain
open. Future runs must declare boundaries, proposed/approved targets, noise
guards and exact source/binary hashes. Keep setup outside timing windows,
maintain the 30 GiB reserve after expected output, and never seed/swap the owner's
personal Foundation Smoke profile.

Plan G3 fixture additions separately from code: the owner approves exact public
bytes and expectations after sanitation. G5/G6 need their own scoped design and
native authority review. The full Phase 3 exit requires these criteria to be
met or explicitly revised/deferred by the owner in a checkpoint decision.

Issue #267 starts G3 with [an approved, independently registered three-Sampler
case](../../research/parser-multisampler-267-result.md). FL Studio shows duplicate
`Sampler` labels; Fruitboard's former numbering assumption failed that case.
The source correction uses an explicit new inference method and preserves older
immutable numbered results with an explanation. The owner approved the exact
F15 bytes and expectations on 2026-10-05; PR268 adds its pinned corpus regression.
This does not close
G3 or qualify additional saved builds, arrangements or external plugins.

Issue #259 remains open for G1 and the remaining qualification/checkpoint review.

Issue #278 provided the accepted G5 design review:
[ADR-007 and authority/implementation packet](../sample-presence-278/README.md).
It proposes explicit metadata-only checks of literal absolute saved paths within
the selected local-NTFS source root, with present/not-found/not-checked/no-reference
semantics and source/root/snapshot/session fences. Relative paths, placeholders
and additional folders need separate qualification/grants. No probe is implemented
by the design PR. PR279 accepted the design; [policy slice #280](../sample-presence-policy-280/README.md)
adds an inactive library with recording-port policy/lifecycle tests, merged in
PR281. [Windows slice #282](../sample-presence-windows-282/README.md) implements
the native metadata adapter, merged as PR283, including real NTFS rights and race
tests. [App slice #284](../sample-presence-ui-284/README.md) adds authorized
command/Library integration, merged as PR285 on 2026-10-06 at
`d19587f52132f31065bbaab97a572d3c5487cf00`. This delivers the three bounded
ADR-007 slices; G5's broader resolution work stays open. This does not qualify
every cloud provider or blocked kernel call.

Issue #288 proposes the next narrow G6 step:
[saved pattern note counts, authority review and fixture plan](../pattern-notes-288/README.md).
It defines per-pattern stored-record counts, explicit missing/unsupported/older
states, bounded additive projections and independent GUI qualification before
extraction. No note parser/UI or new fixture is delivered by this design.
The subsequent [fixture slice #290](../../research/parser-pattern-notes-290-result.md)
adds approved F16/F17 genuine saves and recorded expectations. A/B/C qualify
3/2/4 stored records, including slides and steps. A named empty pattern survives
with its note payload absent; explicit-zero payload support remains unqualified.
The demonstrated ordering/build is qualified; parser/full typed validation and
Library integration are separate next slices, and G6 remains open.
Automation/MIDI, mixer/nested plugins, tempo changes and richer arrangements
remain open; accepting a first note-count scope does not remove those roadmap items.
Merging this packet supplies a maintained evidence map; it does not close gaps,
accept budgets, accept Phase 3 or open a later-phase milestone.
