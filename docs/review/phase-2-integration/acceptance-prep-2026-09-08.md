# Phase 2 acceptance preparation — 2026-09-08

Status: **planning and evidence-matrix preparation only.** This file audits
the P2-01 through P2-12 acceptance criteria against merged evidence, drafts
the #47/#48 manual evidence plans, and lists the owner-only questions. It
does not promote any acceptance ID, amend any budget or quota, record any
scope exclusion, activate production scanning, or claim installed-app
evidence.

- Baseline: `origin/main` `7a0b6b6` (PR #83, docs-only post-merge status for
  `05d39ff`, merged 2026-09-08T17:52:57Z).
- Worktree/branch: `docs/acceptance-prep-7a0b6b6`, created from
  `origin/main` in a separate worktree. File ownership for this task is this
  file only: `docs/review/phase-2-integration/acceptance-prep-2026-09-08.md`.
  Sibling-owned files (`continuation-2026-09-08.md`,
  `next-agent-wave-2026-09-08.md`, `installed-app-journey-checklist.md`,
  `docs/PHASE_2_EXECUTION_PLAN.md`, `apps/desktop/**`, `crates/**`,
  workflows, lockfiles) are not touched.
- Open issues: epic #33; slices #36–#41; platform follow-ups #47 (DriveFS
  host run) and #48 (cross-volume/FAT32 identity). Prior spikes #42/#43 are
  closed with partial findings recorded in `docs/research/p0-d-drivefs-watcher.md`
  and `docs/research/p0-e-file-identity.md`; those findings are explicitly
  not blanket qualification.

## Evidence classes (kept separate)

| Class | What counts | Current state |
| --- | --- | --- |
| Merged | A commit on `main` plus its CI provenance (Foundation run URL for the exact merged commit) | Up to `7a0b6b6`; strongest scanner evidence is `05d39ff` with Foundation run `34248128449` (`success`) |
| Local | Unmerged branches/worktrees (`wave/*`, `journey/*`, `fruitboard-spike-triage`, this branch before merge) | Never cited as merged evidence |
| Installed-app | Observations from the packaged installed Windows app recorded in the sibling checklist | Empty: the sibling checklist observation table is intentionally blank; owned by the sibling journey agent on `journey/installed-app-7a0b6b6` |
| Benchmark | Executed protocol with pinned build, machine profile, seeds/hashes, raw JSON | First measured report only; provisional budgets unchanged; findings F1–F3 open |
| Acceptance | Explicit owner decision per criterion | None taken by this file |

Production scanning stays hidden until P2-03 through P2-08 have integrated
evidence, per the accepted execution plan. Nothing below changes that gate.

CI provenance note: at the time of writing, the push run for `7a0b6b6`
(`34259738925`) was still `in_progress` and is therefore not claimed as
evidence. The cited Foundation evidence is the post-merge run for `05d39ff`
(`34248128449`, `success`, including the Windows foundation job with
feature-on scan-console tests and warnings-denied Clippy) and the PR-head
packaging-smoke run for PR #82 (`34236211276`, `success`), because
`windows-packaging-smoke` runs on `pull_request`, not on post-merge push.
Commit `7a0b6b6` itself (PR #83) is docs-only: it changes the execution
plan status section, the continuation addendum, and the next-wave handoff
record, with no `apps/**` or `crates/**` behavior change.

## Acceptance matrix (P2-01 through P2-12)

Criterion text is abbreviated; the binding text is in
`docs/PHASE_2_EXECUTION_PLAN.md` ("Acceptance ownership and evidence").

| Criterion | Merged evidence (commit / PR / CI) | Missing evidence | Owner decision needed |
| --- | --- | --- | --- |
| P2-01 — root settings/picker safe and accessible | Picker/repository PRs #20–#32, #44, #52–#54 with installed-picker evidence in `docs/review/issue-34/`; settings/onboarding PRs #55/#56/#58 with rendered desktop/narrow keyboard evidence in `docs/review/issue-35/README.md` (fake adapter labeled separately) | Installed-app re-confirmation of selection/cancel and restart survival at the combined commit — **blocked on the sibling journey agent** (`journey/installed-app-7a0b6b6`, checklist steps 2–3, table empty) | Accept as evidenced, or require the sibling journey rows first |
| P2-02 — unchanged tree idempotent; add/modify/rename converge | PR #59 (`c5264a2`, 17 reconciliation tests, `rust-portable`); PR #64 (`57587b5`, 43 enumeration tests, 2 ignored, `enumeration-windows`); PR #70 (`b78e715`, hidden worker NTFS tests incl. add/modify/rename/missing/restore, `scan-execution-windows`) | Installed-app convergence run (add/rename/modify on the disposable leaf) — **blocked on the sibling journey agent** (checklist step 7) | Accept Partial as sufficient for close, or require the journey row |
| P2-03 — partial/offline/denied/cancelled/limited traversal never marks files missing | PR #62 (`857de7d`, quota enforcement, staging invisible until publication); PR #70 fault injection (denial mid-traversal, offline retry chain, durable + cooperative cancellation, both cancel/commit orderings); PR #77 (`3ea6bb7`, short per-batch transactions keep statuses/pages responsive) | Integrated before/after committed-rows comparison from the installed app (cancel + unavailable-root retention, retry convergence) — **blocked on the sibling journey agent** (checklist step 10); NTFS ACL twin skips loudly under elevated tokens, portable fake remains the gate | Accept as Pending-at-close with a named follow-up, or require the integrated gates first |
| P2-04 — generations, dedup, leases, backoff, cancellation obey contracts | PR #60 (`7870377`, durable ledger) + PR #61 (`82bc651`, contract preservation) with state-machine tests (`migration`); PR #70 worker (lease expiry/replacement, restart recovery, coalescing, quota limits); PR #77 short-transaction concurrency fix | Installed-app cancellation/retry behavior plus any remaining fake-clock coverage — **partially blocked on the sibling journey agent** (checklist steps 4, 10) | Accept Partial as sufficient for close, or require the remaining coverage first |
| P2-05 — disable/remove while queued/running prevents stale publication | PR #60 disable/remove invalidation with fresh re-add identity; PR #62 publication rejects stale/cancelled runs without touching committed rows; PR #82 (`05d39ff`) host notifies after the durable transaction returns (config acknowledgement never waits on the supervisor under lock) | Installed-app disable/remove-while-running observation — **blocked on the sibling journey agent** (no dedicated checklist step; covered via steps 3, 6, 9 if scheduled) | Accept Partial as sufficient for close, or require a dedicated journey row |
| P2-06 — atomic publication and restart/backup recovery preserve valid data | PR #62 atomic publication with rollback tests (`publication_rolls_back_visible_rows_and_ledger_on_apply_failure`, migration/backup recovery tests); PR #77 keeps staging/publication lease-fenced inside short transactions | Installed-app restart-survival and interrupted-work recovery observations — **blocked on the sibling journey agent** (checklist steps 3, 6); crash-before/after-apply fixtures remain crate-level | Accept as Pending-at-close with a named follow-up, or require the journey rows first |
| P2-07 — hardlink aliases and uncertain identity preserve per-path presence | PR #59 hardlink regression (`hardlink_removal_preserves_surviving_alias_identity`); PR #62 alias-aware publication (`publication_marks_missing_and_restores_each_alias_without_collapsing_it`); identity policy per P0-E local-NTFS findings | Installed-app alias observation; non-NTFS identity stays unverified — **blocked on the sibling journey agent and on #48** (see plan below) | State #48 limits; accept as Pending-at-close with a named follow-up, or require the gates first |
| P2-08 — Scan/Cancel/Retry and Library list usable, honest, persistent | PR #73 (`177cecf`, six scan-console commands behind `scan-console`, feature-off typed `unavailable`); PR #78 (`9d8545c`, native Library scan adapter/capability seam); PR #81 (`e5e777d`, listen/unlisten-only capability correction + deterministic lifecycle harness) with Foundation run [34218497618](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34218497618); PR #82 (`05d39ff`) supervisor integration with post-merge Foundation run [34248128449](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34248128449) ([Windows job](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34248128449/job/102135306455) incl. feature-on tests + Clippy); fake-adapter client evidence in `docs/review/phase-2-library/` (labeled separately) | Full native journey: Scan now queued/running honesty, Library paging, restart survival, cancel/unavailable/retry against the installed app — **blocked on the sibling journey agent** (checklist steps 4–6, 10); single-connection read-concurrency limitation documented in `docs/review/phase-2-ipc/README.md` §5 | Accept as Pending-at-close (native flip + concurrency revisit land after #33, production stays hidden until P2-03–P2-08 integrated), or require the journey rows first |
| P2-09 — watcher bursts/overflow/event loss converge through durable reconciliation | PR #68 (`4219e0c`) + PR #69 (`01bd2dd`) watcher foundation, bounded coalescer, typed outcomes (`filesystem-watcher-windows`); PR #71 (`b148777`) durable follow-up adapter (burst/overflow/stale-generation/`watch_ended` semantics, `scan-execution-windows`); PR #81 lifecycle harness (5 scenarios); PR #82 supervisor (starts/stops watches per configured root, generation fencing, bounded reconnect backoff, one retained hint per root, explicit join) with run `34248128449` | Installed-app watcher follow-up observation (burst coalescing, coverage-loss full reconciliation, stale-generation drops, hints never marking files missing) — **blocked on the sibling journey agent** (checklist step 9); DriveFS watcher behavior unverified — **see #47 plan** | Accept as Pending-at-close with named follow-ups (host follow-up journey + DriveFS scope), or require the gates first |
| P2-10 — no parsing, hydration, or source mutation in discovery | PR #81 static no-parser/no-content-I/O policy guards (`tests/no-parser.test.mjs`, 4 tests: manifest dependency check, forbidden-API source policy, allowed-API pin, preservation-fixture pin) plus separate synthetic-fixture source-byte preservation assertions in enumeration and scan-execution NTFS tests; PR #70 dependency/content-read evidence (`cargo tree`, `ReadFile`/`NtReadFile`/`parse_flp`/`hydrat` absence) | Owner review of the static-vs-runtime boundary: the guards are static source checks plus fixture byte equality, not a runtime content-read spy — recorded, not missing; no installed-app dependency | Accept as Pending-at-close with a named follow-up, or require a runtime-spy gate first (owner-only scope call) |
| P2-11 — performance/resource limits measured and safely enforced | PR #67 (`31bb383`, generator + methodology + harness scaffold); PR #72 (`0935271`, benchmark driver + first measured report [`benchmark-2026-09-07.md`](benchmark-2026-09-07.md) + raw JSONs) with CI run [34163769541](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34163769541); PR #79 (`7eca4cc`, [`budget-decision-brief.md`](budget-decision-brief.md)) | Findings F1 (10,005 baseline observations vs 10,000 staging quota — baseline never authoritative), F2 (warm p95 32,876 ms vs provisional 10 s target), F3 (100,000-entry set unmeasurable under current quota) — **blocked on the owner plus the sibling triage agent** (`fruitboard-spike-triage` worktree: profiling / isolated re-run evidence) | Owner decisions F1/F2/F3 (exact sentences below); no budget, quota, or fixture change is made by this file |
| P2-12 — complete visible journey works after integration | PR #80 (`dbabb50`, [`checkpoint-2026-09-07.md`](checkpoint-2026-09-07.md): PR-to-ID map, hidden worker/driver journey transcript, P2-10 verification, F1–F3 table, owner checklist); PR #83 (`7a0b6b6`, status snapshot for `05d39ff`); preparation-only [`installed-app-journey-checklist.md`](installed-app-journey-checklist.md) (empty table) | The journey itself (installed-app observations at the combined commit), all F1–F3 and #47/#48 decisions, and every unchecked owner box — **blocked on the sibling journey agent, the sibling triage agent, and the owner** | Owner acceptance per line; epic #33 stays open until then |

Summary of the matrix: crate-level and contract-level work is merged with
green CI for P2-02 through P2-10 (Partial or Pending per the standing rule
that cross-issue criteria need integrated evidence); the first measured
P2-11 report is recorded with F1–F3 open; and every criterion that needs a
packaged-app observation (P2-01 through P2-09, P2-12) is blocked on the
sibling journey agent whose table is still empty. No row is promoted by
this file.

## Manual evidence plan for #47 (DriveFS host run)

Parent: #33. Follows #42 (partial result in
`docs/research/p0-d-drivefs-watcher.md`). Until this plan is executed or
the owner records an explicit scope exclusion, DriveFS behavior stays
labeled unverified and no Drive claim may be made for #36/#37.

Research rules followed (P0-D precedent): state the question,
environment, inputs, observed result, limitations, and resulting decision
under `docs/research/`; use disposable synthetic trees with recorded seed
and manifest hash; never add private files, credentials, personal paths,
or raw diagnostic dumps; an ignored fixture or an unavailable environment
is not a pass.

### Question (#47 DriveFS)

Can the scanner rely on watcher events and placeholder metadata on
DriveFS-backed roots — mirrored versus streamed enumeration, placeholder
versus hydrated metadata, hydration side effects of metadata-only reads,
and burst/rename/disconnect watcher fidelity?

### Exact host setup (#47 — record all of it)

- Windows 11 host with Drive for Desktop installed, signed in, and
  actively syncing. Record: Windows build, Drive for Desktop version,
  mount identity (drive letter or mount path), and the tested mode
  (**mirrored** vs **streamed**) — run every operation in both modes
  unless the owner excludes one.
- Pinned repository baseline: record `git rev-parse HEAD`, the
  `.tools` toolchain versions (`node --version`, `rustc --version`,
  `uv.exe --version`), and the probe command used
  (`scripts/research-fs-probe.ps1 -Mode Watcher`, plus any Rust
  handle-bound watcher harness with its exact `cargo test` invocation).
- No personal Drive content is traversed. All trees are disposable
  synthetic fixtures created under a run-specific temp directory (for
  example `%TEMP%\fruitboard-drivefs-<run-id>`), generated with
  `scripts/generate-synthetic-tree.mjs` with a recorded `--seed` and the
  printed manifest SHA-256. Record the seed, hash, file/alias counts,
  and the relative scan root. Remove the trees after the run.

### Disposable fixtures

- Small deterministic leaf (10 FLP-named synthetic files, one seed) for
  the enumeration/metadata half, placed once under a mirrored root and
  once under a streamed root.
- Burst set (50 created files, one rename, one delete, 10 spaced
  appends — mirroring the P0-D local load) for the watcher half, per
  mode.
- Disconnect fixture: a small leaf whose Drive root can be paused or
  disconnected and reconnected by the operator without touching any
  personal data.

### Operations (#47 DriveFS)

1. Enumerate the mirrored leaf and the streamed leaf; record per-file
   presence, size/mtime availability, and placeholder-vs-hydrated state
   without reading file contents.
2. Perform metadata-only reads (open handle, query size/mtime/identity)
   on one placeholder file per mode and record whether hydration was
   triggered (state change, network activity, or size/state difference
   afterwards). Do not hash, parse, or hydrate deliberately.
3. Replay the burst/rename/delete/append load per mode with the probe
   script; record delivered Created/Changed/Renamed/Deleted counts and
   any Error (overflow) event.
4. Attempt a genuine overflow with a faster generator or real burst load
   (the P0-D 500-file/4 KiB attempt did not reproduce overflow locally);
   record whether overflow was actually observed — `not reproduced` is a
   result, not a pass.
5. Disconnect (or pause sync) mid-watch, then reconnect; record hint
   delivery across the gap and the recovery reconciliation outcome.

### Observations to record (per mode, in a table)

- Enumerated file count vs generated count; placeholder metadata
  available Y/N per field; hydration side effect observed Y/N with the
  exact triggering operation.
- Watcher event counts per operation phase; coalescing observed;
  overflow observed Y/N with the generator parameters that produced it
  (or `not reproduced` with parameters).
- Disconnect/reconnect behavior: hints delivered, lost, or followed by
  coverage-loss; reconciliation result (authoritative or not).
- Environment block: Windows build, DriveFS version, mode, seed, manifest
  hash, commit, toolchain versions.

### Limitations (#47 — must be stated in the findings)

- Single host and single DriveFS version; results are not transferable
  to other versions, modes, or machines.
- OS-tool probes versus the Rust `notify`/handle-bound watcher are
  inference unless the Rust harness itself ran on the Drive root — label
  which ran.
- Sleep/resume and multi-machine sync races are out of scope unless
  explicitly exercised.

### Decision output (#47 DriveFS)

Commit written findings to `docs/research/` with a support/fallback
matrix (which Drive modes are supported, which fall back to
manual/full-reconciliation-only, which are excluded), or return the
explicit owner scope-exclusion sentence for §Owner questions. Do not
reinterpret an unavailable Drive host as a passing result.

## Manual evidence plan for #48 (cross-volume/FAT32 identity)

Parent: #33. Follows #43 (partial result in
`docs/research/p0-e-file-identity.md`). Until this plan is executed or
the owner records an explicit scope exclusion, non-NTFS identity stays
labeled unverified and the #36 reconciler identity policy is NTFS-local
only.

Research rules followed (P0-E precedent): same documentation,
disposability, seed/hash, and no-private-data rules as #47; OS-tool
results mapped to Rust file-identity APIs are labeled inference; an
ignored fixture or an unavailable environment is not a pass.

### Question (#48 identity)

Where is (volume serial, file ID) trustworthy for continuing a file
locator outside local NTFS — same-file moves across volumes, FAT32
file-ID semantics, and placeholder hydration effects on identity?

### Exact host setup (#48 — record all of it)

- Windows 11 host with two writable volumes: one NTFS and one FAT32
  (USB stick, attached VHD, or dedicated partition — record which, plus
  cluster size). Record `fsutil` availability, Windows build, and both
  volume serials (serials are environment identifiers for the run, not
  personal data). The P0-E run had no second writable volume; this plan
  requires one — without it the run cannot happen and stays unverified.
- Same pinned baseline, toolchain, seed/hash, and disposability rules as
  #47. One synthetic file per operation row, plus the hardlink-alias
  pair on NTFS and the FAT32 hardlink-attempt (FAT32 has no hardlinks —
  record the exact failure mode).

### Operations (#48 identity)

1. On FAT32, replay the P0-E operation table row by row (rename in
   directory, move within the volume, overwrite in place, copy to a new
   path, replacement save via temp + move-over, delete + recreate under
   the same name) using `fsutil queryfileid` plus volume serial; record
   identity Stable vs New ID per row.
2. Move one synthetic file across volumes (NTFS → FAT32 and FAT32 →
   NTFS); record the identity before and after — a cross-volume move is
   a copy + delete at the filesystem level, so a New ID is the expected
   honest result; record what was actually observed.
3. Attempt the NTFS hardlink-alias survival case on FAT32; record the
   failure mode and the reconciler consequence (aliases cannot exist
   there; each path is its own file).
4. Spot-check size/mtime granularity on FAT32 (2-second timestamp
   granularity is the known FAT limitation — confirm or refute on this
   host, do not assume it).
5. If a DriveFS host is available in the same session, re-check identity
   stability across a placeholder hydration transition; otherwise record
   hydration effects as not run.

### Observations to record (tables)

- FAT32 operation × identity table in the same shape as the P0-E NTFS
  table, plus serial stability across all operations.
- Cross-volume move table (direction × before/after identity × honest
  classification: same file vs copy + delete).
- FAT32 granularity note (observed timestamp behavior) and hardlink
  failure mode.
- Environment block: Windows build, volume filesystem types and serials,
  seed, manifest hash, commit, toolchain versions.

### Limitations (#48 — must be stated in the findings)

- Host- and filesystem-implementation-specific; FAT32 behavior here does
  not transfer to exFAT, ReFS, or network filesystems.
- OS-tool (`fsutil`) results mapped to Rust identity APIs are inference
  unless the Rust enumerator itself ran on the FAT32 volume — label
  which ran.
- Network shares, ACL revocation mid-watch, and real overflow timing are
  separate unverified items, not covered by this run.

### Decision output (#48 identity)

Commit written findings to `docs/research/`, extending the P0-E fallback
policy (where identity may continue a locator, where the reconciler must
fall back to path-continuity-only with explicit uncertainty, and the
FAT32/cross-volume exclusion list), or return the explicit owner
scope-exclusion sentence for §Owner questions.

Neither #47 nor #48 is run by this planning task: this environment has
no authenticated DriveFS mount, no second writable/FAT32 volume was
provisioned for the run, and DriveFS/cross-volume fixtures on this host
would be ignored-or-unavailable — which is not a pass. The plans above
are ready for a genuinely provisioned host.

## Owner question list (owner-only; nothing below is decided here)

Budget sentences are quoted from
[`budget-decision-brief.md`](budget-decision-brief.md) §5; each takes
effect only as an owner post.

- F1-quota: "I approve amending docs/PHASE_2_EXECUTION_PLAN.md to raise
  the durable staging quota (MAX_STAGED_RECORDS /
  WorkerConfig::default max_observations) from 10,000 to [OWNER FILLS:
  e.g. 10,500], because the accepted baseline fixture produces 10,005
  observations (10,000 FLP files + 5 hardlink aliases) and every Run A
  iteration ends ResourceLimit. Record the new quota, the
  alias-counting rule, and the re-measurement requirement."
- F1-fixture (alternative to F1-quota, pick one): "I approve amending
  docs/PHASE_2_EXECUTION_PLAN.md to define the baseline fixture as
  exactly 10,000 observations including hardlink aliases (e.g. 9,995
  FLP files + 5 aliases, seed 0), instead of 10,000 FLP-named files
  plus aliases. The "10,000 FLP-named files" row is superseded by this
  definition."
- F2-budget: "I approve amending docs/PHASE_2_EXECUTION_PLAN.md warm
  reconciliation budget from p95 <= 10 s to [OWNER FILLS: value + host
  class, e.g. p95 <= 15 s on laptop-class i7-10750H / <= 10 s on
  desktop-class], citing benchmark-2026-09-07 (median 12875.5 ms, p95
  32876 ms, n = 10) as evidence. No code change is authorized by this
  sentence alone."
- F2-host (alternative or companion to F2-budget): "I approve adding a
  desktop-class reference host to docs/PHASE_2_EXECUTION_PLAN.md and
  re-measuring the warm-p95 budget there before any re-budget; the
  laptop-class result (p95 32876 ms) stands as reported and is not
  rewritten."
- F2-rerun: "I approve an idle-host re-run per benchmark-2026-09-07
  §Limitations (DriveFS paused, AV exclusion for the fixture volume, no
  agent load, AC + high-performance, documented cache state, 1 warm-up +
  10 measured) to isolate background contention before any
  optimize/re-budget decision. Report median/max/nearest-rank p95 the
  same way; do not filter iterations."
- F2-optimize: "I approve profiling and optimizing the scan
  handle/metadata path (per-file opens, ancestor validation chain,
  synchronous=FULL commit batching) against the quota-fitting 10k set,
  then re-measuring warm-p95 on the same host class. Budgets stay as
  written until the new evidence lands."
- F3-100k: "I acknowledge the 100,000-entry qualification set stays
  unmeasurable under the current 10,000-record staging quota, and I
  approve [OWNER FILLS: raising the quota / staging the 100k set in
  bounded chunks / deferring the 100k qualification with a dated
  checkpoint] in docs/PHASE_2_EXECUTION_PLAN.md. The ~53 MiB figure
  remains a 10k-set measurement and is not claimed against the 100k
  budget."
- Platform scope exclusions (each needs evidence or one explicit owner
  sentence before any blanket platform claim): DriveFS mirrored/streamed
  modes (#47); FAT32 and cross-volume identity (#48); network-share
  roots; ACL revocation during a watch; real OS buffer-overflow timing.
  An ignored fixture or an unavailable environment is not a pass.
- P2 pending-at-close candidates: accept P2-03/P2-06/P2-07/P2-09/P2-10
  as Pending-at-close each with a named follow-up, or require their
  integrated fault-injection/restart/alias/watcher/no-parser gates
  first; accept P2-02/P2-04/P2-05 Partial as sufficient for close, or
  require the remaining coverage first; accept P2-08 as Pending-at-close
  (native flip + concurrency revisit land after #33, production scanning
  stays hidden until P2-03 through P2-08 have integrated evidence).
- Merge of this planning branch (one new file, no production or status
  changes) at owner discretion.

## What is deliberately left unclaimed

- No acceptance ID is promoted: P2-02/P2-04/P2-05 stay Partial and
  P2-03/P2-06/P2-07/P2-08/P2-09/P2-10/P2-12 stay Pending; P2-11 stays
  Measured with F1–F3 open; P2-01 keeps its prior standing subject to
  the sibling journey rows.
- No budget, quota, fixture definition, or platform scope is amended or
  excluded.
- No installed-app evidence is claimed: the sibling checklist table is
  empty and this file does not fill it.
- No DriveFS, cross-volume, FAT32, network-share, ACL, real-overflow,
  or 100,000-entry qualification is claimed; spikes #42/#43 remain
  partial findings, not qualification.
- No local-only result (this branch, `wave/*`, `journey/*`,
  `fruitboard-spike-triage`) is described as merged, and no benchmark
  sample is converted into an approved budget.
- Production scanning stays hidden; no parser, grouping, Kanban,
  playback, sync, PWA, or Phase 3 work is started or authorized.

## Validation for this planning file

- `pnpm lint:docs` (markdownlint-cli2) passes for the changed file.
- `git diff --check` reports no whitespace errors.
- Scope check: `git status --short` and `git diff --stat` show only
  `docs/review/phase-2-integration/acceptance-prep-2026-09-08.md`.
