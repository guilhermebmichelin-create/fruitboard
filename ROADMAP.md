# Roadmap and issue/PR plan

Status: **Accepted; create only the next accepted milestone's issues**

The product roadmap remains broadly sound. The principal changes are:

1. add explicit parser compatibility, packaging, and GPL licensing gates before
   PyFLP becomes a shipped dependency;
2. treat DriveFS placeholder/watcher behavior and Windows file identity as
   scanner spikes, not assumptions;
3. define the sync operation/conflict protocol before implementing Google OAuth;
4. make the PWA's foreground-sync limitation explicit without adding a backend;
5. verify a patched SQLite build before enabling multi-connection WAL.

## Phase gates

| Phase | Outcome | Entry gate | Exit evidence |
| --- | --- | --- | --- |
| 0 Product & Architecture | Reviewed architecture and bounded uncertainties | Requirements | Accepted ADRs/review questions; PoCs scheduled |
| 1 Application Foundation | Installable/tested shell and durable foundation | Phase 0 accepted | CI, migrations, logging/errors, accessible shell; no scanner |
| 2 Scanner MVP | Roots discover and monitor FLPs safely | Watcher/parser gates relevant to MVP | Reconciliation/event tests; corrupt file isolation; status UI |
| 3 FLP Intelligence | Useful normalized FLP/dependency metadata | Parser matrix and license gate passed | Field reliability matrix; diagnostics; Plugin Explorer MVP |
| 4 Project Identity & Versions | Reversible logical grouping | Stable file/snapshot model | Suggestions/merge/split/reject with evidence tests |
| 5 Project Management | Core creative workflow | Project identity stable | Kanban, fields, notes, journal, tasks, timeline |
| 6 Project Workspace & Polish | Coherent deep workspace and OS actions | Management model stable | Responsive/a11y states; open FL/folder safety tests |
| 7 Audio Exports | Confirmable exports and desktop playback | Media threat model accepted | Matching tests; safe streaming/player flows |
| 8 Releases | Albums/EPs/collections | Project workflow stable | Ordered M:N releases and completion summary |
| 9 Analytics | Actionable local insights | Reliable source data | Metric definitions, query tests, restrained dashboard |
| 10 Work-Time Tracking | Honest prospective sessions | Signals validated on Windows | Heuristic audit, manual correction, limitation UX |
| 11 Google Drive Sync | Recoverable optional metadata sync | Protocol/security/encryption ADR accepted | OAuth, replay/conflict/recovery/large-log tests |
| 12 PWA | Installable iPhone companion | Sync stable; real-device PoC passed | Offline mutations, foreground sync, iPhone a11y/audio tests |
| 13 Advanced | Evaluated optional features | Usage evidence | Separate accepted issues/ADRs per feature |

Each phase ends with an explicit review issue/discussion and stops. “Exit
evidence” means merged, tested behavior and updated documentation—not a demo
branch.

## Phase 0 issue and PR structure

The current documentation can be reviewed as four logical, stackable units. If
the owner wants strict one-concept PRs before merge, split the commits/changes as:

### Epic: Phase 0 — Product and architecture

1. **#2 — System architecture and desktop framework decision** (PR #6)
   - architecture diagram, boundaries, repository layout, Tauri/Electron and
     frontend evaluation;
   - ADR-001 and ADR-006;
   - PR: `docs: propose system and shared-client architecture`.
2. **#3 — Local data, scanner, identity, and parser feasibility** (PR #7)
   - SQLite model, scanner/reconciliation, identity rules, research probe,
     parser protocol/PoCs and licensing gate;
   - ADR-002, ADR-003, ADR-004;
   - PR: `docs: define local data and FLP analysis boundaries`.
3. **#4 — Sync, PWA, security, and privacy architecture** (PR #8)
   - Drive operation log, conflict rules, browser auth limitation, threat model;
   - ADR-005 and security review gates;
   - PR: `docs: define optional sync and security model`.
4. **#5 — Delivery plan and repository governance** (PR #9)
   - roadmap, test/CI/tooling plan, privacy `.gitignore`, PR template;
   - PR: `docs: establish incremental delivery workflow`.

Suggested issue acceptance: documents exist, consequential uncertainty is
labeled, primary sources are linked, review questions are answered, and no
application code is added.

## Phase 1 issues and PRs

Created after Phase 0 acceptance under
[epic #10](https://github.com/guilhermebmichelin-create/fruitboard/issues/10).

### Epic: Phase 1 — Application foundation

1. **[#11 — Workspace and reproducible toolchains](https://github.com/guilhermebmichelin-create/fruitboard/issues/11)**
   - Node 24 LTS with pnpm/Corepack, `rust-toolchain.toml`, an isolated pinned
     Python 3.11 research environment, lockfiles, and SQLite 3.51.3-or-later
     WAL-fix verification;
   - create the proposed `docs/research/` location or remove it from the
     repository diagram;
   - PR: `chore: initialize reproducible workspace toolchains`.
2. **[#12 — Tauri 2 desktop shell and shared React client](https://github.com/guilhermebmichelin-create/fruitboard/issues/12)**
   - minimal local bundle, typed platform port, no broad capabilities;
   - PR: `feat: add Tauri shell with shared React client`.
3. **[#13 — Design tokens, routing, and accessible application shell](https://github.com/guilhermebmichelin-create/fruitboard/issues/13)**
   - light tokens, desktop navigation, route/error/loading/empty skeletons,
     keyboard/focus baseline and screenshots;
   - PR: `feat: add accessible light application shell`.
4. **[#14 — Rust command, logging, and error foundations](https://github.com/guilhermebmichelin-create/fruitboard/issues/14)**
   - structured local logging/redaction, stable error envelope, job/event
     skeleton; no scanner implementation;
   - PR: `feat: add native command and error infrastructure`.
5. **[#15 — SQLite schema and migration runner](https://github.com/guilhermebmichelin-create/fruitboard/issues/15)**
   - smallest Phase 1 subset of accepted model, patched SQLite verification,
     backup/migration tests;
   - PR: `feat: add SQLite schema and forward migrations`.
6. **[#16 — Project repository vertical slice](https://github.com/guilhermebmichelin-create/fruitboard/issues/16)**
   - one non-scanner repository/use case through typed IPC, proving boundaries;
   - PR: `test: prove client-to-SQLite application boundary`.
7. **[#17 — Pull-request CI and security baseline](https://github.com/guilhermebmichelin-create/fruitboard/issues/17)**
   - lint/type/test/build, Rust/Windows lanes, dependency/privacy checks;
   - add `.github/ISSUE_TEMPLATE/`, workflows, and `CODEOWNERS` with the actual
     maintainers and required-check names;
   - PR: `ci: enforce foundation quality gates`.
8. **[#18 — Foundation packaging smoke and checkpoint](https://github.com/guilhermebmichelin-create/fruitboard/issues/18)**
   - unsigned development artifact, startup/install notes, sizes/timings, Phase 1
     review evidence; not a public release;
   - PR: `test: add Windows foundation packaging smoke`.

Avoid putting the real scanner, parser, Drive OAuth, PWA service worker, project
cards, or Kanban in these issues.

PRs #6, #7, #8, and #9 were merged in that order with linear history, after
which the Phase 1 epic and these eight issues were created. Run P0-G and
P0-D/P0-E early and in parallel; P0-G may amend ADR-001, while P0-D and P0-E
must be complete before the Scanner MVP design is finalized.

## Phase 2 proposed issues and PRs

### Epic: Phase 2 — Scanner MVP

1. Scan-root repository and native folder picker.
2. Onboarding/root settings and root status UI.
3. Incremental reconciler with filesystem-only metadata.
4. Watcher adapter, event coalescing, and overflow recovery.
5. Durable scan queue, cancellation, retries, and observability.
6. Basic parser adapter after P0-A/B/C gates (or filesystem-only MVP if blocked).
7. Atomic project-file/snapshot persistence and missing/restored behavior.
8. Scanner integration/E2E test corpus and Phase 2 checkpoint.

The DriveFS watcher/placeholder and Windows file-identity spikes should be
separate `spike` issues that land written results before issues 3–6 finalize.

## Phase 3 proposed issues and PRs

### Epic: Phase 3 — FLP intelligence

1. Parser compatibility matrix and approved fixture manifest as a maintained
   test suite.
2. Project/arrangement metadata normalization and field provenance.
3. Duration/bar estimation with uncertainty.
4. Channel/pattern/automation/MIDI summaries.
5. Mixer and plugin normalization/classification.
6. Sample/reference resolution without content execution.
7. Parser diagnostics/repair-by-reparse UI (never FLP repair).
8. Plugin Explorer MVP and query indexes.
9. Performance/error qualification and Phase 3 checkpoint.

## Phase 4 proposed issues and PRs

### Epic: Phase 4 — Project identity and versions

1. Conservative filename normalization and candidate blocking.
2. File identity/exact duplicate detection.
3. Metadata similarity rules and versioned evidence scores.
4. Suggestion review UI: confirm/reject.
5. Manual merge/split with reversible history.
6. Version history UI and grouping regression suite.
7. Phase 4 checkpoint.

## Phase 5 proposed issues and PRs

### Epic: Phase 5 — Project management

1. Workflow-stage configuration with safe migration on delete.
2. Accessible Kanban drag/drop plus keyboard alternative.
3. Rating, priority, favorite, disposition, and progress.
4. Tags and combined filter/query model.
5. Persistent structured project note.
6. Timestamped journal and revision behavior.
7. Ordered tasks/checklists and completion summary.
8. Semantic activity-event production and timeline.
9. Backup/recovery and end-to-end workflow checkpoint.

## Later phase issue outlines

Keep these as outlines until the preceding design is informed by real usage.

- **Phase 6:** project-detail information hierarchy/wireframes; responsive
  states; artwork; metadata/versions/dependencies/tasks/timeline sections; safe
  open-in-FL/open-folder commands; accessibility/visual review.
- **Phase 7:** export enumeration; matcher; confirmation/override; safe local
  media protocol; player controls/accessibility; unavailable-file recovery.
- **Phase 8:** release model; ordered track editing; artwork/notes; completion
  semantics; releases views.
- **Phase 9:** metric definitions first; lifecycle/activity/plugin/work-time
  queries; dashboard/analytics views; large-library performance validation.
- **Phase 10:** Windows FL process/window signal spike; session state machine;
  idle/concurrency handling; manual adjustment; audit/accuracy study.
- **Phase 11:** sync protocol conformance suite; encryption ADR; desktop OAuth;
  Drive adapter; outbox/replay; conflict UI; compaction/recovery; security review.
- **Phase 12:** PWA build/deploy shell; IndexedDB repositories/migrations;
  browser OAuth; foreground sync; responsive touch navigation; offline/update
  UX; artwork/preview policy; real-iPhone qualification.
- **Phase 13:** one evaluated epic per advanced feature. Safe rename requires its
  own filesystem/reference research and threat review.

## Cross-cutting issue rules

- UI issues define user goal, information hierarchy, low-fidelity layout, and
  populated/loading/empty/error/offline states before implementation.
- Data-changing issues state migration, backup, sync, and tombstone impact.
- Scanner/parser issues state non-destructive guarantees and fixture privacy.
- Platform issues state Windows behavior and macOS portability implications.
- Every feature has test tasks inside the issue; avoid a late “add all tests” PR.
- Documentation/ADR updates ship with the decision or behavior they describe.
- Follow-up issues are created for known limitations; they do not authorize
  silently pulling later-phase work into the current PR.

## Phase 0 acceptance checklist

- [x] Product owner answers the five review questions in
  [docs/PHASE_0_REVIEW.md](docs/PHASE_0_REVIEW.md#review-questions).
- [x] ADR statuses are accepted or amended.
- [x] Product/distribution license direction is recorded; PyFLP remains blocked
  until GPL compatibility is resolved.
- [x] Phase 1 issue list is approved.
- [x] Phase 1 epic #10 and issues #11–#18 were created after final acceptance.
- [x] The owner accepts the documented GitHub Free manual-governance exception
  because private-repository protection is unavailable and GitHub Pro was
  declined.
- [x] No application feature implementation has entered Phase 0.
- [x] Phase 0 PRs were accepted and merged in order #6 → #7 → #8 → #9.

After this checklist is accepted, start only the first Phase 1 issue/branch.
