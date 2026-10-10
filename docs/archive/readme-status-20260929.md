# Archived README status - through 2026-09-29

These dated blocks were moved from the root README on 2026-10-09. They retain
historical source, evidence and decision boundaries; their present-tense claims
are superseded by the [current README](../../README.md#current-status---2026-10-09).
Links now resolve from this archive. No historical performance or acceptance
result has been promoted to current qualification.

## Current status - 2026-09-29

This block supersedes the dated status sections below; they remain as
historical record.

- **Phases**: Phase 0 accepted 2026-09-04, Phase 1 accepted 2026-09-06, and
  Phase 2 accepted with known gaps on 2026-09-14
  ([acceptance record](../review/phase-2-integration/acceptance-2026-09-14.md)).
  Epic #33 is closed. Issues #38 and #40 remain open for the narrowed P2-08
  evidence residual; P2-08 itself stays Partial.
- **Parser**: the Rust FLP parser was selected on 2026-09-28
  ([ADR-002](../adr/002-flp-parser-process.md)) and the `flp-parser` crate
  landed with bounded extraction for three saved FL Studio builds. It is not
  yet wired into the scanner or packaged with the app.
- **Product surface**: the Tauri/React shell, the Rust-owned local SQLite
  library, scan roots, durable scan execution (feature-gated behind
  `scan-console`), and the Library list are implemented. Production scanning
  stays inactive by default until an explicit owner action. Kanban, search,
  project workflow, Google Drive sync, and the PWA are not implemented.
- **Performance**: unqualified; the measured warm nearest-rank p95 was
  10,173 ms against a 10,000 ms target on source `69f27f6`.
- **Repository**: public since 2026-09-07 (visibility re-verified 2026-09-29),
  branch protection with ten required checks, and the GitHub-native security
  products activated 2026-09-29: secret scanning, push protection, and
  Dependabot alerts, plus informational CodeQL and dependency review
  workflows (#148).

## Project status (superseded - see Current status above)

Fruitboard's **Phase 0 architecture is accepted**; the manual governance
exception recorded for the private GitHub Free repository was superseded on
2026-09-07 when the repository became public, and enforced branch protection is
enabled on `main` (2026-09-07; see [DEVELOPMENT.md](../../DEVELOPMENT.md#enforced-branch-protection)).
**Phase 1 is
accepted**: the reproducible Tauri/React shell now includes Rust-owned local
SQLite, one persisted startup-view preference, automated quality/security
gates, and a Windows packaging smoke. At that checkpoint the parser work,
project workflow, sync, and PWA had not started, and Phase 2 was tracked
under epic #33. The verified source baseline for
the 2026-09-13/14 documentation refresh was
`71848732216d4ab4e13e73c820b2b8e4d17bddbe`, which includes the squash merges
of #110 (`e2948f1`), #111 (`914d7bd`), the merged #112 (`00884ba`), and #114
(`7184873`); that pin is historical, not the current head. The
[Phase 2 integration index](../review/phase-2-integration/README.md) carries
the dated post-acceptance and publication addenda that record later heads and
merge state. Historical merge, tested-source, and CI references remain in the
integration index and dated reports.

Implemented behavior, automated evidence, installed evidence, and owner
acceptance stay separate. The owner accepted Phase 2 with known gaps on
2026-09-14 ([acceptance
record](../review/phase-2-integration/acceptance-2026-09-14.md)); P2-08
remains Partial (PR #85's older `Complete` row is superseded) and no
full-criterion P2-08 acceptance is recorded. That acceptance does not activate
production scanning; activation remains a separate owner action.

The canonical installed validation remains the merged PR #106 report
`installed-journey/run-20260909-fixed-validation.md`: S1–S4 PASS, S6 PASS,
and S5 is the terminal-follow-up/successor path. Merged #110 separately
publishes Agent 2's queued/running/terminal, queued disable/remove, and
genuine running-lease same-job evidence; it remains installed evidence and
does not close #107 or accept Phase 2. The #111 NTFS report remains tied to its
tested product source even though the PR's product and evidence changes are
now merged.

The [Phase 2 integration index](../review/phase-2-integration/README.md),
[execution plan](../PHASE_2_EXECUTION_PLAN.md), and [2026-09-12 acceptance
packet](../review/phase-2-integration/acceptance-packet-2026-09-12.md)
carry the current dependency map, evidence boundaries, and owner decision
list. Historical failed replays remain historical and performance remains
unqualified.

The source sequence is now merged #110 -> #111 -> #112 -> #114. #112's
diagnostic changes and #114's executable qualification runbook/validator are
in main, but no qualification run or acceptance is recorded. PR #113 remains
an older synthetic candidate. Agents prepare and verify; the owner retains Phase 2 acceptance,
scope, budget, and issue-closure authority.

Tracking starts at
[Phase 2 epic #33](https://github.com/guilhermebmichelin-create/fruitboard/issues/33)
(Phase 1 epic
[#10](https://github.com/guilhermebmichelin-create/fruitboard/issues/10)
is closed on acceptance).

## Post-#152 follow-up status — 2026-09-21 (superseded — see Current status above)

The current publication boundary is `main` at
`73775453e9e4021c7b6b4ade381fd87300f47c32`, the squash merge of #152. Its
product tree matches the reviewed #152 head
`79da7e97f7b8481eab1f01661fe54ab0b33d98aa`. The merged fix covers the explicit
Retry and cancellation-finalization races; this follow-up records additional
UI evidence and narrowly scoped accessibility/focus corrections without
reopening that native implementation.

The feature-enabled installed candidate exercised ten native local-NTFS
scenarios, visible root/settings/library controls, real Windows keyboard input,
Scan now, Cancel, focus retention, and committed-row preservation. The retained
raw evidence is outside Git under the `post152-autonomous-followup-20260921`
evidence root; its final driver result is `pass: true`, qualified for local NTFS
only. FAT32, DriveFS, network, cross-volume, and performance support remain
unclaimed.

Phase 2 remains accepted with known gaps. Issues #38 and #40 stay open for the
narrowed P2-08 evidence residual, and production scanning remains a separate
owner decision. This follow-up does not activate repository security products,
add parser code or fixtures, or record a new acceptance decision.
