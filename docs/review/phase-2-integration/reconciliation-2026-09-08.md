# Phase 2 status reconciliation - 2026-09-08 (refreshed 2026-09-09)

Status: **evidence reconciliation only; Phase 2 is not accepted.** This report
audits the current status documents against the fetched `origin/main`,
live GitHub PR/issue/CI records, merged source and tests, the benchmark re-run,
and the owner decisions that are actually recorded. It does not close an issue,
change a feature gate, amend a budget, or supply an owner decision.

This reconciliation does not edit the installed-app checklist, benchmark
evidence and raw JSON files, platform research, application code, or production
gates. The checklist observations are recorded in unmerged PR #92; the platform
reports are in unmerged PR #90; the diagnostic profile is in unmerged PR #93;
the stacked optimization is in unmerged PR #94; the D1–D3 repairs are in unmerged
PR #95; and the F1/F3 scale design is proposed in unmerged draft PR #96. Historical
checkpoint, continuation, acceptance-preparation, benchmark, post-merge, and
platform-research records remain historical records.

## 2026-09-09 refresh addendum

`origin/main` re-verified 2026-09-09 local time remains
`0b7612db3570e6235d4d2c86a678dd9004264f30` (PR #89); no new merges. Open
drafts are now #90, #91, #92, #93, #94, and #95 (plus new proposal draft #96);
open issues remain #33, #36–#41, #47, #48. Deltas since 2026-09-08:

- D1–D3 repairs are verified in still-unmerged draft PR #95 head `bf0aeac`
  (Foundation 34309511415 + packaging 34309511404 passed; `pnpm check`, 134
  client tests, 88 feature-on desktop tests, additive `run-20260909.md`
  installed replay). The replay retained no durable queued snapshot; ACL,
  DriveFS, cross-volume/FAT32, 100,000-entry, and burst-timing cases remain
  outside its scope. PR #95 remains open and unmerged.
- PR #92 packaging rerun attempt 2 passed
  ([job 102314720612](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102314720612));
  the original timeout ([attempt
  1](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102303731791))
  remains recorded. No product fix or acceptance is inferred.
- PR #94 head `588867b` (three commits on base
  `docs/41-performance-followup-20260908`) is stacked on #93 and has no
  exact-head CI yet. Its contended whole-scan median moved 11,735.5 ms to
  11,517 ms while p95/max worsened 12,188 ms to 17,682 ms on an outlier; it
  needs combined validation with #93 plus a quiet-host A/B rerun and is not a
  qualification.
- Durable queued and watcher-burst evidence is being addressed independently
  (including the `test/95-durable-queue-watcher-20260909` worktree) and is not
  claimed here; P2-04/P2-09 remain Partial.
- Platform prerequisites (#47 Mirror/Stream consent, #48 genuine FAT32
  volume) and F1/F3 scale decisions remain unresolved. The concrete
  bounded-scan proposal (F1 exact-10,000 fixture vs quota alternative; F3
  chunked snapshot with coverage ledger, fencing, bounds, atomic publication,
  cancel/crash/expiry/cleanup/backup, slices, adversarial tests, and
  quota/deferral comparison) is in draft
  [PR #96](https://github.com/guilhermebmichelin-create/fruitboard/pull/96)
  as a proposal only. No acceptance is promoted by this refresh.

## Executive result

- Native watcher supervision and shutdown recovery are implemented on `main`
  through PR #82. The old statement that watcher supervision was still open is
  stale.
- PR #85 adds merged worker-level fault, atomicity, restart/backup, alias, and
  stale-publication coverage. Its passing CI and tests strengthen P2-03,
  P2-05, P2-06, and P2-07, but they do not provide installed-app observations
  or owner acceptance.
- The P2-08 `Complete` statement introduced by PR #85 is not supportable
  as an acceptance claim. The reconciled status is Partial: native,
  automated-seam, and partial installed evidence exist, while D1/D2/D3 repairs
  (now verified in still-unmerged PR #95), durable queued observation,
  independent watcher-burst counting, and explicit full-criterion owner
  acceptance remain open as acceptance gates.
- PR #86 is the current performance evidence source. It reproduces F1, leaves
  F2 failing at a 14,267 ms warm p95 against the provisional 10 s target, and
  leaves F3 unmeasured. No budget or fixture decision was made. Stacked PR #94
  needs exact-head CI/combined validation and a quiet-host rerun; it is not a
  qualification.
- The installed journey was observed from a feature-enabled package built from
  `main` `0b7612d`: persistence, paging, watcher follow-up convergence,
  cancellation retention, and interrupted-work recovery were observed. The
  evidence record remains in unmerged PR #92; durable queued observation and
  independent watcher-burst counting remain unverified there and are addressed
  independently. D1–D3 repairs are verified in still-unmerged PR #95 (no
  durable queued snapshot retained). #47 and #48 remain open; PR #90 records
  their blockers, which are not qualifications.
- Unmerged PR #93 adds contended filesystem-port profiling. It identifies a
  measured cost center but is not an idle-host performance pass. PR #92's
  Foundation CI and packaging rerun attempt 2 are green; its first packaging
  attempt recorded `The installed sidecar smoke timed out`, which remains a
  provenance incident rather than a product-fix or acceptance result.
- No explicit full Phase 2 or P2-08 acceptance decision is recorded. Epic #33,
  child issues #36-#41, and platform follow-ups #47/#48 remain open.
  Production scanning stays hidden under the accepted P2-03 through P2-08
  gate. F1/F3 scale options are proposed in draft PR #96 without authorization.

## Audit basis

### Repository and merge state

On 2026-09-08 local time, `git fetch --prune origin main` completed and
`origin/main` resolved to (re-verified unchanged 2026-09-09 local time):

`0b7612db3570e6235d4d2c86a678dd9004264f30`

The latest first-parent sequence is:

| Commit | PR | Meaning |
| --- | --- | --- |
| `0b7612d` | [#89](https://github.com/guilhermebmichelin-create/fruitboard/pull/89) | #48 survey: no qualifying FAT32 target |
| `c8d8255` | [#88](https://github.com/guilhermebmichelin-create/fruitboard/pull/88) | #47/#48 executable manual plans |
| `e9464be` | [#87](https://github.com/guilhermebmichelin-create/fruitboard/pull/87) | Post-merge status documentation |
| `4218e41` | [#86](https://github.com/guilhermebmichelin-create/fruitboard/pull/86) | Benchmark triage re-run |
| `c05f4b6` | [#85](https://github.com/guilhermebmichelin-create/fruitboard/pull/85) | Durability close-out tests and storage fixes |
| `51f45af` | [#84](https://github.com/guilhermebmichelin-create/fruitboard/pull/84) | Acceptance-preparation matrix |
| `05d39ff` | [#82](https://github.com/guilhermebmichelin-create/fruitboard/pull/82) | Native watcher supervision and shutdown recovery |
| `e5e777d` | [#81](https://github.com/guilhermebmichelin-create/fruitboard/pull/81) | Native subscriptions and continuation lifecycle |

Live GitHub at the 2026-09-08 audit showed open issues (#33 plus #36 through #41,
plus #47 and #48). It also showed open draft PRs (#90 through #93); no reviews or
comments were recorded on those four PRs at that audit point. At the 2026-09-09
refresh, open issues are unchanged and open drafts are PRs #90 through #95 (plus new
proposal draft PR #96); all remain unmerged drafts. The exact issue and PR links are in the
[integration index](README.md) and the parent [Phase 2
epic](https://github.com/guilhermebmichelin-create/fruitboard/issues/33).

### CI provenance

The following records were checked through GitHub for the exact commit or PR
head named. `Foundation CI` has nine jobs; `windows-packaging-smoke`
is triggered on pull requests, schedules, and manual dispatch, not on a push
to `main`.

| Evidence target | Exact commit or head | Live result |
| --- | --- | --- |
| Current `origin/main` application/build baseline | [`0b7612db3570e6235d4d2c86a678dd9004264f30`](https://github.com/guilhermebmichelin-create/fruitboard/commit/0b7612db3570e6235d4d2c86a678dd9004264f30) | [Foundation run 34293250231](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34293250231): success, all nine jobs; this is the commit used for the installed application in PR #92 |
| PR #82 merge | `05d39ff074ab6c568d998175744551561928de89` | [Foundation run 34248128449](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34248128449): success, all nine jobs; Windows feature-on tests and warnings-denied Clippy passed |
| PR #85 merge | `c05f4b6a82c5b3469900a408b19b8c979bc4974b` | [Foundation run 34281613081](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34281613081): success, all nine jobs |
| PR #86 merge | `4218e413be7dc27422eab9a45252bcdd1ca74359` | [Foundation run 34290753717](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34290753717): success, all nine jobs |
| PR #87 merge | `e9464be404744a91071343c0e7d32f79f5485fa2` | [Foundation run 34291338301](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34291338301): success, all nine jobs |
| PR #88 head | `ddeac9025dcdd9f1f65c9236a3120ef4057420f1` | [Foundation run 34291358030](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34291358030) and [packaging run 34291358010](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34291358010): both success |
| PR #89 head | `289609511e1cb3359a2a30418a1fde56e8463dc9` | [Foundation run 34292846030](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34292846030) and [packaging run 34292846032](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34292846032): both success |
| PR #90 head (unmerged platform reports) | [`17e571742634eceb16f09b166edd3d77b57fcf61`](https://github.com/guilhermebmichelin-create/fruitboard/commit/17e571742634eceb16f09b166edd3d77b57fcf61) | [Foundation run 34296963183](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34296963183) and [packaging run 34296963219](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34296963219): both success |
| PR #92 head (unmerged installed record) | [`49a5e649c688ae767c9189801dd033f2288b61f5`](https://github.com/guilhermebmichelin-create/fruitboard/commit/49a5e649c688ae767c9189801dd033f2288b61f5) | [Foundation run 34299680195](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680195): success; [packaging attempt 1](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102303731791) failed with `The installed sidecar smoke timed out`, and [rerun attempt 2](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102314720612) passed; preserve the incident without inferring a product fix or acceptance |
| PR #93 head (unmerged diagnostic profile) | [`59faefc2806a725368a59e7b6fc9be7f863f4fec`](https://github.com/guilhermebmichelin-create/fruitboard/commit/59faefc2806a725368a59e7b6fc9be7f863f4fec) | [Foundation run 34301751666](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34301751666) and [packaging run 34301751625](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34301751625): both success; profile is contended shared-host diagnostics, not an idle-host pass |
| PR #94 head (unmerged, stacked on #93) | [`588867bca154e798f189f6c99de8a2668005d141`](https://github.com/guilhermebmichelin-create/fruitboard/commit/588867bca154e798f189f6c99de8a2668005d141) (base `docs/41-performance-followup-20260908`) | No exact-head CI reported at the refresh; needs combined validation with #93 plus a quiet-host A/B rerun. Contended whole-scan median 11,735.5 ms to 11,517 ms with p95/max 12,188 ms to 17,682 ms on an outlier; not a p95 win or 10 s qualification |
| PR #95 head (unmerged D1–D3 repairs) | [`bf0aeac38ac46dbb90990611b940429e232f67ef`](https://github.com/guilhermebmichelin-create/fruitboard/commit/bf0aeac38ac46dbb90990611b940429e232f67ef) | [Foundation run 34309511415](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34309511415) and [packaging run 34309511404](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34309511404): both success. Verified D1–D3 repairs with automated and installed-replay evidence; replay retained no durable queued snapshot; ACL/DriveFS/cross-volume/FAT32/100k/burst cases remain outside its scope; draft remains open and unmerged |
| PR #96 head (unmerged scale-design proposal) | [`3be014074b385747e51d6d6b859180315a5b80bc`](https://github.com/guilhermebmichelin-create/fruitboard/commit/3be014074b385747e51d6d6b859180315a5b80bc) | New draft proposal only (F1 exact fixture vs quota; F3 chunked snapshot); no implementation, quota, budget, or acceptance change; CI pending at the refresh |

The push run for merged commit `c8d8255` was cancelled when the next
`main` push arrived. Therefore the #88 PR-head checks are cited for that
docs-only change, while the successful exact-current-commit run above covers
the final tree. A green CI result establishes build and test evidence only; it
does not establish installed behavior, broad platform support, or owner
acceptance.

### Owner decisions and authority

The audit found these explicit decisions:

| Decision | Recorded basis | Scope |
| --- | --- | --- |
| Shared scanner contracts and provisional starting budgets accepted on 2026-09-06 | [PR #57](https://github.com/guilhermebmichelin-create/fruitboard/pull/57) body | Design/implementation baseline; not measured performance, implementation completion, or Phase 2 acceptance |
| UI-only Library seam accepted on 2026-09-07 | Owner comment on [PR #63](https://github.com/guilhermebmichelin-create/fruitboard/pull/63) | `LibraryScanAdapter`, page fencing, and fake-adapter harness only; native P2-08 journey not accepted |
| Branch protection enabled on 2026-09-07 | Merged [PR #74](https://github.com/guilhermebmichelin-create/fruitboard/pull/74) and live branch settings | Repository governance only |

No explicit owner post or review accepting the full P2-01 through P2-12
checkpoint was found. PR #82 explicitly says no acceptance issue is closed.
PR #85 explicitly says no acceptance ID status is promoted. PRs #88 and #89
explicitly preserve #47/#48 as unverified. An owner merge is therefore recorded
as a merge, not inferred to be acceptance.

The historical checkpoint has checked boxes for P2-01, the first P2-11 report,
and the hidden P2-12 journey. Its own legend defines checked boxes as
evidence-complete, says unchecked boxes block close, and states that epic close
still requires explicit owner acceptance. Those checkboxes are therefore
retained as historical evidence markers, not treated as owner posts or as
current acceptance of the criteria.

## Reconciled status summary

| ID | Status after reconciliation |
| --- | --- |
| P2-01 | Evidence present; not promoted |
| P2-02 | Partial |
| P2-03 | Partial |
| P2-04 | Partial |
| P2-05 | Partial |
| P2-06 | Partial |
| P2-07 | Partial |
| P2-08 | Partial; prior `Complete` claim disputed and not accepted |
| P2-09 | Partial |
| P2-10 | Partial |
| P2-11 | Measured; not qualified; F1-F3 decisions open |
| P2-12 | Pending |

No criterion is promoted to complete by this report.

## Criterion-by-criterion evidence ledger

The categories below are deliberately independent. In particular, automated
tests do not fill the installed-app, performance, platform, or owner-acceptance
columns.

### P2-01 - Existing root settings/picker remain safe and accessible

- **Implementation:** The #34/#35 picker, root settings, onboarding, failure,
  focus, keyboard, and narrow-layout work is merged through PRs #20-#32, #44,
  #52-#58. No newer merged change invalidates that implementation evidence.
- **Automated tests:** Client, documentation, and Windows foundation checks
  cover the merged settings surface. The rendered #35 evidence is explicitly
  fake-adapter evidence.
- **Installed-app observations:** Historical #34 installed-picker evidence is
  recorded under `docs/review/issue-34/`. The unmerged [PR #92 run record](https://github.com/guilhermebmichelin-create/fruitboard/blob/49a5e649c688ae767c9189801dd033f2288b61f5/docs/review/phase-2-integration/installed-journey/run-20260908.md)
  additionally records current-main installed picker cancellation, root
  selection, settings persistence, and clean restart survival. It does not
  itself establish criterion acceptance.
- **Performance qualification:** Not applicable to this criterion; no scanner
  benchmark qualifies it.
- **Platform qualification:** The picker record is Windows evidence for the
  picker slice only. It is not a DriveFS, FAT32, or scanner platform claim.
- **Owner acceptance:** Phase 1 is accepted, but no separate current Phase 2
  P2-01 acceptance line was found.
- **Reconciled status/gate:** Evidence present; not promoted. Review the
  current-main run record and obtain explicit P2-01 owner acceptance.

### P2-02 - Unchanged tree and add/modify/rename convergence

- **Implementation:** PR #59 (`c5264a2`) provides the deterministic
  reconciliation core; PR #64 (`57587b5`) provides bounded Windows
  enumeration; PR #70 (`b78e715`) composes the hidden worker.
- **Automated tests:** Deterministic tests include
  `unchanged_tree_is_idempotent_and_order_independent` and
  `every_incomplete_outcome_rejects_positive_and_absence_changes`.
  Windows enumeration and hidden-worker fixture tests cover add, modify,
  rename, missing, and restore cases. The exact current Foundation run is
  green.
- **Installed-app observations:** The unmerged #92 record observes native
  add, rename, metadata modification, remove, and restore follow-ups. The new
  path was Present while the old renamed path stayed Missing, and the restored
  path returned to Present. This was captured from the package built at
  `main` `0b7612d`; it is not a broader platform qualification.
- **Performance qualification:** No P2-02 performance qualification is
  claimed. The benchmark driver exercises a hidden worker and is P2-11
  evidence, not installed-app evidence.
- **Platform qualification:** Windows/NTFS fixture coverage exists in CI;
  #47/#48 remain outside that qualification.
- **Owner acceptance:** No explicit P2-02 owner acceptance was found.
- **Reconciled status/gate:** Partial. Review the installed convergence record,
  retain the broader platform limitation, and obtain owner acceptance.

### P2-03 - Non-authoritative traversal never marks files missing

- **Implementation:** PRs #62 and #70 establish staged, non-authoritative
  worker behavior. PR #85 (`c05f4b6`) adds the close-out coverage and
  storage support.
- **Automated tests:** The merged `p2_03_*` cases cover offline, root
  denial, partial disappearance, unsupported filesystem, cooperative and
  durable cancellation, resource limit, and staging/sink failure. Each seeds
  committed rows, compares rows plus the root success marker byte-for-byte,
  asserts no publication, and checks discarded staging. The exact merged
  evidence is [Foundation run 34281613081](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34281613081), with the current tree rechecked by [run 34293250231](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34293250231).
- **Installed-app observations:** The unmerged #92 record observes native
  cancellation and unavailable-root retention: the previous committed rows
  remained visible and no incomplete work published a replacement list. Denied
  and resource-limit installed cases were not run.
- **Performance qualification:** The cooperative stop samples in the
  benchmark are P2-11 worker measurements only; they do not qualify the
  installed cancellation or UI acknowledgement requirement here.
- **Platform qualification:** Fake-port fault injection and Windows CI are
  automated evidence. The ACL twin may be unavailable under elevated tokens,
  and DriveFS/non-NTFS behavior remains unverified.
- **Owner acceptance:** PR #85 says the close-out does not promote an
  acceptance ID. No explicit P2-03 acceptance was found.
- **Reconciled status/gate:** Partial. Review the installed retention record,
  keep denied/resource-limit coverage open, and obtain owner acceptance.

### P2-04 - Durable generations, deduplication, leases, backoff, and cancellation

- **Implementation:** PRs #60 and #61 establish the durable ledger and
  contract preservation; PR #70 composes the worker; PR #82 connects host
  lifecycle and shutdown recovery; PR #85 adds worker close-out cases.
- **Automated tests:** The `migration` lane covers state-machine and
  restart/backup transition matrices. The feature-enabled desktop host tests
  cover lifecycle recovery and joined loops. The exact-current and PR #85
  Foundation runs are green.
- **Installed-app observations:** The unmerged #92 record observes persisted
  root/settings state, native running/cancellation behavior, and restart
  recovery. A durable queued state was not captured there. Retry after
  cancellation was non-actionable (D2), and Retry after an exhausted
  unavailable-root chain was non-actionable (D3). Both repairs are verified in
  still-unmerged PR #95 (native `retryAvailable` fencing with `Scan now` for
  terminal chains, plus the additive `run-20260909.md` replay); PR #95 remains
  an open draft and its replay retained no durable queued snapshot.
- **Performance qualification:** The re-run reports cooperative worker stop
  p95 of 22 ms over six samples, but has no renderer and does not exercise the
  250 ms UI acknowledgement budget. It is not a complete P2-04 qualification.
- **Platform qualification:** Windows CI and injected lifecycle tests qualify
  only those tested seams; they do not qualify all filesystem modes.
- **Owner acceptance:** No explicit P2-04 owner acceptance was found.
- **Reconciled status/gate:** Partial. Review verified D1–D3 repairs in
  still-unmerged PR #95 (rebuilding from merged `main` where required),
  capture the durable queued observation through the independently addressed
  queue/watcher work, then complete the end-to-end review.

### P2-05 - Disable/remove while queued or running prevents stale publication

- **Implementation:** PRs #60 and #62 provide configuration invalidation and
  publication fencing. PR #85 adds disable, remove, queued-work, and fresh
  re-add coverage; PR #82 preserves notification ordering after durable root
  transactions.
- **Automated tests:** `p2_05_disable_invalidates_lease_and_staging_*`,
  `p2_05_remove_then_readd_*`, and
  `p2_05_queued_work_invalidated_*` cover lease/staging invalidation,
  no stale publication, fresh identity, and unchanged source markers. Green
  exact-commit CI verifies the merged tests.
- **Installed-app observations:** The unmerged #92 record observes disabling a
  running root cancelling the work while retaining prior rows, and removing an
  idle root without touching fixture files before re-adding a fresh tracked
  root. A specifically queued disable/remove observation is not recorded.
- **Performance qualification:** Not applicable; no timing result proves this
  concurrency property.
- **Platform qualification:** The source-marker assertions and fake durable
  harness are not a cross-filesystem qualification.
- **Owner acceptance:** No explicit P2-05 owner acceptance was found.
- **Reconciled status/gate:** Partial. Review the installed operation, add
  queued-operation coverage if required by the owner, and obtain acceptance.

### P2-06 - Atomic publication and restart/backup recovery preserve valid data

- **Implementation:** PR #62 supplies staging/publication and rollback
  boundaries. PR #85 adds close-out tests for crash and recovery boundaries.
- **Automated tests:** The merged cases
  `p2_06_crash_before_staging_*`,
  `p2_06_crash_after_staging_before_apply_*`,
  `p2_06_crash_during_apply_*`, and
  `p2_06_backup_recovery_*` assert no partial Library rows, ledger
  consistency, requeued convergence, and byte-identical committed data and
  marker after recovery. Existing migration fixtures remain covered. PR #85
  exact-merge CI is [run 34281613081](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34281613081).
- **Installed-app observations:** The unmerged #92 record observes committed
  rows and settings surviving clean restart, then an abruptly terminated
  installed process being recorded as `interrupted` and recovering to a
  completed attempt after relaunch. This is installed evidence from the
  current-main package, not a claim that all crash boundaries are covered.
- **Performance qualification:** Not applicable; a passing rollback test is
  not a performance result.
- **Platform qualification:** Storage and Windows CI evidence does not qualify
  DriveFS, FAT32, network, or other untested storage modes.
- **Owner acceptance:** No explicit P2-06 owner acceptance was found.
- **Reconciled status/gate:** Partial. Review the interrupted-work record and
  obtain owner acceptance for the full crash/recovery criterion.

### P2-07 - Hardlink aliases and uncertain identity preserve per-path presence

- **Implementation:** PR #59 establishes conservative identity decisions; PR
  #62 publishes alias-aware locations; PR #85 adds worker-level alias and
  replacement/rename close-out cases.
- **Automated tests:** `p2_07_hardlink_delete_one_*` verifies that
  deleting one alias leaves the other present and restores the missing path.
  The `p2_07_rename_replacement_*` cases preserve per-path history,
  mint fresh replacement records, and avoid Phase 4 grouping. Windows/NTFS
  fixture CI is green.
- **Installed-app observations:** The unmerged #92 record observes rename
  convergence with separate Present/Missing paths, but it does not establish an
  installed hardlink-alias run. The hardlink evidence remains automated and
  local-NTFS-oriented.
- **Performance qualification:** Not applicable.
- **Platform qualification:** The evidence is local-NTFS-oriented. PR #88
  supplies a manual #48 plan and PR #89 records no qualifying FAT32 target;
  neither qualifies FAT32, cross-volume, exFAT, ReFS, or network identity.
- **Owner acceptance:** No explicit P2-07 acceptance or non-NTFS scope
  exclusion was found.
- **Reconciled status/gate:** Partial. Add installed alias evidence if required
  by the owner and resolve #48 with evidence or an explicit owner decision;
  neither is supplied by this reconciliation.

### P2-08 - Scan/Cancel/Retry and Library list are usable, honest, and persistent

- **Implementation:** PR #73 adds feature-gated scan-console IPC; #77 fixes
  staged-batch responsiveness; #78 wires the native Library adapter; #81
  corrects event permissions and adds lifecycle coverage; #82 integrates the
  native watcher/scan host and shutdown recovery. These are merged behind
  `scan-console`.
- **Automated tests:** Native typed IPC, client adapter, pagination,
  cancellation, lifecycle, host recovery, and feature-enabled desktop tests
  are present. PR #82 and the exact merged Foundation run
  [34248128449](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34248128449)
  verify the feature-on desktop tests and warnings-denied Clippy; the current
  tree is green in [run 34293250231](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34293250231).
  Rendered captures remain fake-adapter evidence.
- **Installed-app observations:** The unmerged #92 record observes native
  `Scan now` running counters with no percentage, four-record Library paging,
  persisted settings and committed rows, cancellation/unavailable retention,
  watcher follow-up convergence, and interrupted-work recovery. Durable queued
  observation was not captured there. D1 (native Library mislabeled as fake),
  D2 (cancelled Retry rejected), and D3 (exhausted Retry not actionable)
  repairs are verified in still-unmerged PR #95 (native render context,
  `retryAvailable` fencing, `Scan now` for terminal chains, additive
  `run-20260909.md` replay with installer/executable SHAs and isolated data
  location). PR #95 remains an open draft; its replay retained no durable
  queued snapshot. The #92 record was captured from a package built at merged
  `main` `0b7612d`, not from an unmerged documentation head.
- **Performance qualification:** The benchmark driver uses the hidden worker,
  not the installed renderer path. It cannot qualify Library/UI latency or
  the installed journey.
- **Platform qualification:** Feature-enabled Windows CI verifies the native
  seam and test host only. It does not qualify DriveFS or non-NTFS behavior.
- **Owner acceptance:** There is a material authority conflict. The second
  commit carried by PR #85 says `P2-08 Pending -> Complete` using
  merge-train evidence and a claimed owner seam acceptance in PR #78. However,
  PR #85's body says no acceptance ID is promoted and owner review is required;
  PR #82 says no acceptance issue is closed; PR #63's explicit owner comment
  accepts only the UI-only Library seam; and the accepted plan requires
  integrated evidence for cross-issue criteria. No explicit owner post or
  review accepting the full P2-08 criterion was found.
- **Reconciled status/gate:** Partial, not Complete. Review verified D1/D2/D3
  repairs in still-unmerged PR #95 (rebuilding from merged `main` where
  required), capture durable queued state through the independently addressed
  work, retain the packaging-timeout incident, and obtain explicit
  full-criterion owner acceptance before promoting it.

### P2-09 - Watcher bursts, overflow, and event loss converge durably

- **Implementation:** PRs #68 and #69 provide bounded handle-bound watcher
  behavior; #71 connects watcher hints to durable follow-ups; #81 adds a
  deterministic lifecycle harness; #82 adds the native supervisor, root
  mapping, generation fencing, reconnect backoff, retained coverage hints,
  and joined host loops.
- **Automated tests:** Watcher coalescing, overflow/coverage-loss precedence,
  stale-generation dropping, delivery retention, shutdown recovery, empty-host
  joins, and disable/re-enable races are covered in the watcher, host, and
  recovery test files. Exact feature-on and watcher CI is green.
- **Installed-app observations:** The unmerged #92 record observes native
  watcher follow-ups for add/rename/metadata/remove/restore changes and
  convergence to committed Library results. It does not independently count a
  burst to prove the at-most-one queued-follow-up bound, and it does not record
  installed coverage-loss, overflow, or stale-generation cases. Those gaps are
  being addressed independently and are not claimed here.
- **Performance qualification:** No real OS overflow timing or installed
  watcher latency qualification is recorded.
- **Platform qualification:** Injected handles and Windows CI qualify
  deterministic seams. #47 DriveFS behavior, network shares, ACL revocation
  during a watch, and real overflow timing remain unverified.
- **Owner acceptance:** No explicit P2-09 owner acceptance or #47 scope
  exclusion was found.
- **Reconciled status/gate:** Partial. Independently count burst coalescing,
  run the remaining loss/overflow cases as scoped, and resolve the
  DriveFS/platform gate.

### P2-10 - No parsing, hydration, or source mutation in discovery

- **Implementation:** The filesystem-only worker and enumeration path are
  merged; PR #81 adds the repository no-parser/no-content-I/O policy guards
  and source-preservation assertions.
- **Automated tests:** `tests/no-parser.test.mjs` statically checks the
  manifests, forbidden content-read APIs, allowed-API pin, and preservation
  fixture pin. Separate enumeration and worker tests compare synthetic source
  bytes before and after. These are static source checks plus fixture equality,
  not a runtime content-read spy.
- **Installed-app observations:** No installed-app dependency or runtime
  content-read observation is recorded.
- **Performance qualification:** Not applicable; absence of parsing is not a
  speed result.
- **Platform qualification:** The checks cover the merged Windows-oriented
  discovery source policy; they do not broaden filesystem platform support.
- **Owner acceptance:** PR #57 records the filesystem-only direction and
  parser deferral as an accepted planning baseline. That does not by itself
  close P2-10, and the static-versus-runtime boundary remains an owner review
  question.
- **Reconciled status/gate:** Partial. Keep the limitation explicit and obtain
  the owner decision on whether the static guards plus fixture equality satisfy
  the criterion.

### P2-11 - Performance and resource limits are measured and safely enforced

- **Implementation:** PR #67 adds the fixture/methodology scaffold; PR #72
  adds the benchmark driver and first measured report; PR #86 adds the
  2026-09-08 re-run and profiling notes. These are evidence artifacts, not a
  budget amendment.
- **Automated tests:** Benchmark scaffold and repository policy tests pass in
  CI. The measured runs themselves are the documented protocol against the
  hidden worker, not a CI pass/fail qualification of an installed app.
- **Installed-app observations:** None. The driver uses a pinned release
  worker binary and disposable synthetic fixtures, not the packaged desktop
  UI.
- **Performance qualification:** The historical 2026-09-07 quota-fitting
  10,000-observation run measured median 12,875.5 ms and warm p95 32,876 ms.
  The current 2026-09-08 re-run was executed at the recorded `51f45af`
  benchmark baseline (before the later #85 merge), and uses the same fixture hashes: the accepted
  baseline still produces 10,005 observations and fails `ResourceLimit`;
  the quota-fitting run publishes 10,000 locations with first discovery 10,846
  ms, median 10,978 ms, and warm p95/max 14,267 ms. All ten warm iterations
  exceed the provisional 10 s target. Cooperative stop p95 is 22 ms, but
  working-set samples of about 53 MiB are from a 10,000-entry set and are not
  private-memory qualification for the 100,000-entry target. Because the
  recorded benchmark commit predates #85, this is the newest available
  measurement, not an exact-current-`origin/main` performance qualification.
  Unmerged [PR #93](https://github.com/guilhermebmichelin-create/fruitboard/blob/59faefc2806a725368a59e7b6fc9be7f863f4fec/docs/review/phase-2-integration/performance-followup/README.md)
  adds a filesystem-port diagnostic: 10.754 s of a 12.157 s warm scan (88.46%)
  on a contended shared host, with `next_entry` and `read_metadata` dominant.
  It is diagnostic cost-center evidence, not an idle-host rerun or performance
  pass, and it does not change the official benchmark result. Stacked unmerged
  [PR #94](https://github.com/guilhermebmichelin-create/fruitboard/blob/588867bca154e798f189f6c99de8a2668005d141/docs/review/phase-2-integration/performance-followup/optimization-20260909.md)
  removes one duplicate native attribute query with safety evidence, but its
  contended whole-scan comparison (median 11,735.5 ms to 11,517 ms; p95/max
  12,188 ms to 17,682 ms on an outlier) has no exact-head CI yet and needs
  combined validation plus a quiet-host A/B rerun; it is not an end-to-end p95
  win and does not qualify the 10 s target. Scale options are proposed in
  draft [PR #96](https://github.com/guilhermebmichelin-create/fruitboard/pull/96)
  without applying either F1 option or authorizing F3 implementation.
- **Platform qualification:** The re-run is a same-host, laptop-class
  Windows 11 measurement with background DriveFS, antivirus, agent, and
  sibling-build load. It is not the idle-host isolation required by the
  pending F2-rerun decision and transfers to no other host class.
- **Owner acceptance:** The starting budgets are owner-accepted as
  provisional in PR #57. F1 (quota versus fixture), F2 (budget/host/rerun or
  optimization), and F3 (100k qualification) have no posted owner decision.
  The [decision brief](budget-decision-brief.md) and [re-run report](benchmark-triage-20260908/rerun-report.md) quote the exact sentences still required.
- **Reconciled status/gate:** Measured, not qualified. Resolve F1-F3 with
  explicit owner decisions and any required re-measurement or plan amendment;
  do not promote the PR #93 diagnostic to an idle-host qualification.

### P2-12 - Complete visible journey works after integration

- **Implementation:** PR #80 records the historical checkpoint and PRs
  #81/#82/#85/#86/#88/#89 update the merged implementation and evidence
  boundary. The current index and this report are the status reconciliation,
  not a replacement for the journey record.
- **Automated tests:** The hidden worker/driver transcript in the historical
  checkpoint and the current exact-commit CI establish automated and
  contract-level evidence. They do not establish an installed-app journey.
- **Installed-app observations:** The unmerged [PR #92 checklist and run
  record](https://github.com/guilhermebmichelin-create/fruitboard/blob/49a5e649c688ae767c9189801dd033f2288b61f5/docs/review/phase-2-integration/installed-journey/run-20260908.md)
  record selection/cancel, persisted settings and paging, watcher convergence,
  cancellation retention, and interrupted-work recovery from a package built
  at merged `main` `0b7612d`. Durable queued observation and independent
  watcher-burst counting remain unverified there and are addressed
  independently. D1/D2/D3 repairs are verified in still-unmerged [PR
  #95](https://github.com/guilhermebmichelin-create/fruitboard/blob/bf0aeac38ac46dbb90990611b940429e232f67ef/docs/review/phase-2-integration/installed-journey/run-20260909.md)
  with an additive replay; that draft remains unmerged. Neither evidence
  document is merged and neither itself accepts P2-12.
- **Performance qualification:** P2-11 remains unresolved with F1-F3 open;
  no performance decision can complete P2-12.
- **Platform qualification:** #47 DriveFS and #48 FAT32/cross-volume remain
  open and unverified. The #89 no-target survey is not a pass.
- **Owner acceptance:** No explicit owner acceptance of the P2-12 checkpoint
  or Phase 2 was found. Issue #41 and epic #33 remain open.
- **Reconciled status/gate:** Pending. Review verified D1/D2/D3 repairs in
  still-unmerged PR #95 (rebuilding from merged `main` where required),
  capture the durable queued and independent burst observations through the
  independently addressed work, resolve performance and platform decisions
  (including the PR #96 scale proposal or dated deferral plus the required
  quiet-host rerun), and obtain the owner's criterion-by-criterion acceptance.

## Owner decision packet

The following are recommendations and consequences for the owner; none is an
approval or an acceptance decision.

1. **F1 — exact observations or coordinated quota (details in PR #96).**
   Recommend defining the accepted fixture as exactly 10,000 observations,
   including aliases: 9,995 FLP-named files plus five hardlink alias locations
   (`custom-9995`, seed 0, manifest
   `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a`; command
   `node scripts/run-benchmark.mjs --size custom --files 9995 --seed 0 --out
   <report>.json`). This is the narrowest change and preserves the
   10,000-record quota, but changes the fixture definition and requires a fresh
   accepted-protocol run (1 warm-up + 10 measured with median/max/p95,
   first-discovery, stop latency, and working-set reported). The alternative is
   a coordinated quota change above 10,005 across the worker, staging fences,
   storage checks, plan buffer, and tests; it preserves the 10,000-FLP fixture
   but requires implementation/configuration changes and remeasurement. Do not
   raise only one limit. Neither option is applied here; see PR #96 §1 for the
   full validation consequences.
2. **F3 — bounded 100,000-entry design or dated deferral (design in PR
   #96).** Recommend reviewing the bounded chunked-snapshot proposal (durable
   coverage ledger, generation/lease fencing across chunks, explicit
   memory/disk/path-byte/record bounds, one atomic authoritative publication,
   missing-file gating on complete coverage, cancel/crash/restart/expiry/
   cleanup/backup behavior, migration slices, and adversarial tests in PR #96
   §2) against the coordinated quota increase and the dated deferral. The
   chunked direction protects private-memory bounds but requires a new
   storage/protocol design with the listed tests. The lower-cost alternative is
   an explicit dated deferral that keeps the current 10,000-entry contract; the
   consequence is that the 100,000-entry target remains unqualified until that
   checkpoint. This reconciliation authorizes none of the three; PR #96 is a
   proposal only.
3. **#47 — exact DriveFS run and authorization.** Run the complete procedure
   once in **Mirror files** and once in **Stream files**, reading the active mode
   from Drive for Desktop Preferences. Use only the documented disposable
   leaves `G:\fruitboard-drivefs-<run-id>-enum`,
   `G:\fruitboard-drivefs-<run-id>-burst`, and
   `G:\fruitboard-drivefs-<run-id>-gap`. Before starting, the owner/operator
   must authorize cloud-synchronized synthetic create/rename/delete and cleanup;
   switching modes requires authorization for the full resync; the gap case
   additionally requires authorization to pause/disconnect and verify resume.
   No personal project contents may be traversed, opened, hashed, or parsed.
4. **#48 — genuine FAT32 alongside NTFS.** Recommend an already disposable,
   writable genuine FAT32 USB volume or dedicated VHD with a drive letter,
   alongside the existing disposable leaf on `C:` NTFS. Record source/type,
   volume serial, filesystem, and allocation unit size; use a leaf such as
   `<LETTER>:\fruitboard-48-fat32-<run-id>` and authorize synthetic operations
   and cleanup on both volumes. Do not substitute the system FAT32 partition,
   the `G:` DriveFS virtual mount, a network share, or the no-media USB device.
5. **Remaining acceptance after fixes and verification.** Review verified D1
   native labeling, D2 cancelled Retry, and D3 exhausted Retry in still-unmerged
   PR #95, rebuilding from the then-merged `main` where the owner requires it;
   retain PR #92's first-attempt sidecar timeout and green rerun attempt 2 in
   the provenance. Keep exact check links in the handoff. Capture a durable
   queued state and independently count watcher-burst coalescing through the
   independently addressed work. Resolve F2 with the owner-selected
   quiet-host A/B (including stacked PR #94 combined validation) or
   optimization/budget path, then close F1/F3 via the PR #96 proposal or dated
   deferral, complete any scoped #47/#48 runs (Mirror/Stream consent; genuine
   FAT32 volume) or explicit owner scope decisions without inventing consent or
   availability, and decide whether the P2-10 static no-parser evidence
   satisfies the criterion. Finally, obtain explicit acceptance for P2-01
   through P2-12; do not infer it from merged commits, green checks, or these
   recommendations. Refresh this reconciliation after sibling fix PRs land and
   before any final merge/acceptance review.

## Remaining gates and handoff

1. Keep the installed checklist and sibling evidence linked to their exact
   commits (including #95 `bf0aeac`, #94 `588867b`, and proposal #96
   `3be0140`); do not relabel unmerged observations or proposals as merged
   evidence until their PRs land.
2. Review verified D1/D2/D3 repairs in still-unmerged PR #95 (rebuilding from
   merged `main` where required), and capture the durable queued and
   independent watcher-burst gaps through the independently addressed work.
3. Resolve F1/F2/F3 and #47/#48 using the owner packet above and the PR #96
   scale proposal (or dated deferral); no platform exclusion is selected by
   the current reports, and no new scan protocol is authorized here.
4. Obtain explicit owner acceptance for each criterion or an explicitly
   recorded pending-at-close decision with a named follow-up. Do not infer
   acceptance from a merge, commit message, green CI, or a contributor's
   summary.

Until those gates are resolved, P2-03 through P2-08 remain behind the
production-scanning gate, the Phase 2 epic remains open, and no Phase 2
acceptance is declared.
