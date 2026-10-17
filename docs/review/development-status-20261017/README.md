# Current Phase 3 status and next step

Date: 2026-10-17. Assessment issue:
[#326](https://github.com/guilhermebmichelin-create/fruitboard/issues/326).

This read-only assessment uses merged `main` at
`65969d093806d63f1aba101e8fa0d1e16cfd0735`
([PR #325](https://github.com/guilhermebmichelin-create/fruitboard/pull/325)).
PR325 records the automation research boundary; it does not change application
behavior. Current application source includes
[PR #321](https://github.com/guilhermebmichelin-create/fruitboard/pull/321),
which consolidated current Plugin Explorer metadata reads under the existing
freshness checks. Its before/after measurements were similar and both failed
the CPU quietness guard, so they establish neither a speedup nor resource
qualification. Current parser evidence includes approved F22 from
[PR #320](https://github.com/guilhermebmichelin-create/fruitboard/pull/320)
and F23 from
[PR #323](https://github.com/guilhermebmichelin-create/fruitboard/pull/323).

This update does not accept a phase, budget, compatibility claim, fixture,
filesystem authority, production activation, or release. The dated installed
and performance reports retain their tested source boundaries. In particular,
the October 14 report remains a record of its 19-file cohort and original
outcomes; this current summary does not rewrite that history.

## Delivered workflow

In the explicit development build, a user can select a local NTFS root, scan
projects, request analysis, and view the saved facts in the Library. Delivered
fields include project facts, channels, inert sample and plugin references,
Plugin Explorer groups, pattern names and counts, pattern note-record counts,
and qualified ordinary mixer record names. The
[readiness review](../flp-intelligence-readiness/README.md) maps each surface
to its evidence and exact-build limits.

Explicit sample checks report metadata observations only for eligible literal
absolute paths inside the selected local NTFS root. Relative paths,
placeholders, external folders, and cloud references remain unresolved. The
approved parser corpus contains 21 files across three exact FL Studio builds;
this is a narrow set of cases, not support for all projects or plugins.

F23 is an approved genuine save for FL Studio 26.1.0.5530 with one automation
clip and three GUI-visible control points. The parser still exposes no
automation summary. [Issue #324's research result](../../research/parser-automation-counts-324-result.md)
confirms the files' outer event framing but does not establish a saved object,
placement, point-count, or reference grammar. Extraction and Library display
remain future work that requires independent differential saves and the
owner's approval of any new public fixture packet.

The default desktop does not scan or read FLP content automatically.
Production activation remains a separate owner decision. Project grouping,
Kanban, search, broader project workflow, Google Drive sync, and the PWA are
not delivered.

## Phase and acceptance status

Phases 0 and 1 were accepted on 2026-09-04 and 2026-09-06. Phase 2 was
accepted with known gaps on 2026-09-14; P2-08 remains Partial with the specific
evidence gaps in the
[acceptance record](../phase-2-integration/acceptance-2026-09-14.md). The live
Phase 2 epic #33 is closed, while issue
[#40](https://github.com/guilhermebmichelin-create/fruitboard/issues/40)
remains open. Tracker state does not promote P2-08.

Phase 3, FLP Intelligence, is active. Its bounded local analysis and Library
workflow is delivered, but resource targets, remaining scope decisions, and
the Phase 3 exit are not accepted. The
[checkpoint](../phase3-qualification-306/checkpoint.md) recommends continuing
inside Phase 3. No later-phase work is authorized by these deliveries.

## Remaining Phase 3 gates

- **G1/G4 resource and error qualification:** issues
  [#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259)
  and [#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306)
  remain open. Resource proposals remain candidates; the current Explorer and
  installed windows are characterization after failed CPU guards. Do not
  treat the 200 ms native comparison or any whole-app candidate as an accepted
  budget.
- **G3 compatibility:** F22 adds one approved cloned-Sampler case for exact
  build 25.1.3.4922. Other builds and fields without independent GUI ground
  truth remain unsupported or unqualified; the
  [gap ledger](../flp-intelligence-readiness/gap-ledger.md) lists examples.
- **G5 sample resolution:** the delivered check covers eligible absolute
  paths inside the selected root. Relative-path semantics need a separate
  ADR-007 review and accepted authority boundary before implementation.
- **G6 richer intelligence:** automation/MIDI, mixer effects or nested
  plugins, tempo changes, and richer arrangements remain open. For automation,
  #324's evidence plan requires independently observed differential saves;
  public fixture bytes and expectations need exact owner approval before
  parser extraction.
- **G7/G8:** activation, license, and distribution decisions remain open.
  Phase 2 scanner failures and unverified DriveFS/FAT32 behavior remain
  separate from Phase 3 metadata results; retain the historical 10,173 ms p95
  against the unchanged 10,000 ms scanner target.

## Next bounded step

The next bounded step is [issue #327](https://github.com/guilhermebmichelin-create/fruitboard/issues/327),
which prepares shutdown-phase diagnostics for the open #306 resource review.
The October 14 installed attempt completed its 1,000-entry scan/analysis
preparation but did not exit within the harness's ten-second graceful-close
wait. A separate debugging repeat closed normally in 87.3, 69.0, and 60.1 ms
for the 19-, 100-, and 1,000-entry cases. It did not reproduce the timeout,
identify its cause, or demonstrate a product fix. See the
[dated repeat report](../resource-repeat-20261014/README.md#shutdown-reproduction-and-owned-recovery).

Issue #327 scopes fixed, allowlisted start/end markers and monotonic elapsed
time for sample checks, the feature-enabled analysis host, the scanner host,
and overall shutdown in the explicit development smoke. It preserves shutdown
ordering, the ten-second close criterion, and safe best-effort diagnostics. It
does not change behavior or claim to fix the timeout. If the timeout recurs,
the captured phase times may show where shutdown stopped; a normal close in a
later run still would not explain the earlier failure.

After diagnostics are ready, repeat the remaining native and installed
resource windows only when both the host-noise and storage preconditions pass.
Keep the existing candidate boundaries and guards, include every unrelated
process (including T3), and maintain at least 30 GiB free after expected
output. Re-declare the exact current source and corpus for the repeat: the
issue #306 packet measures PR305, while PR321 subsequently changed the
current-metadata read path and PR320/PR323 advanced the approved parser corpus.
A valid repeat can inform a separate owner budget decision; it does not accept
one automatically.

## Reliability issues whose fixes are already merged

GitHub currently lists issues
[#220](https://github.com/guilhermebmichelin-create/fruitboard/issues/220),
[#221](https://github.com/guilhermebmichelin-create/fruitboard/issues/221),
[#222](https://github.com/guilhermebmichelin-create/fruitboard/issues/222),
and [#223](https://github.com/guilhermebmichelin-create/fruitboard/issues/223)
as open. Their reported behaviors were addressed in merged
[PR #225](https://github.com/guilhermebmichelin-create/fruitboard/pull/225),
and the [application-fixes review](../application-fixes-20260929.md) maps each
change to regression coverage:

- **#220:** scanner runtime health is reported separately from build capability, and work is rejected after the worker stops.
- **#221:** the client validates Library root relationships, timestamps, and state-dependent job/run identifiers.
- **#222:** Library state is reread and reconciled when live-event attachment fails.
- **#223:** publication lookup is bounded by paths and identities observed in the completed scan while preserving atomic updates.

This status record documents the live discrepancy; it does not close or otherwise change those issues.

## Baseline CI context

At the time of this assessment, Foundation CI run
[39078524191](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/39078524191)
for baseline SHA `65969d093806d63f1aba101e8fa0d1e16cfd0735` was in progress:
nine jobs had succeeded, and `windows-foundation` was still running. CodeQL
run [39078524180](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/39078524180)
had completed successfully for JavaScript/TypeScript and Rust. No Windows
Packaging Smoke run appeared for that SHA in the captured run list. These are
baseline observations only; they do not show that every required context
completed, qualify an installed workflow, or satisfy checks for a changed PR
head.

This documentation update changes no application, corpus, dependency, schema, workflow, or CI code. It requires the repository's documentation, privacy, and policy checks plus normal updated-head CI. No local application build, installed test, fixture generation, performance run, cache write, or cleanup is part of this status assessment.
