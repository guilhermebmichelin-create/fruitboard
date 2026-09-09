# Phase 2 status reconciliation - 2026-09-08

Status: **evidence reconciliation only; Phase 2 is not accepted.** This report
audits the current status documents against the fetched `origin/main`,
live GitHub PR/issue/CI records, merged source and tests, the benchmark re-run,
and the owner decisions that are actually recorded. It does not close an issue,
change a feature gate, amend a budget, or supply an owner decision.

The installed-app checklist, benchmark evidence and raw JSON files, platform
research, application code, and production gates were not edited. Historical
checkpoint, continuation, acceptance-preparation, benchmark, post-merge, and
platform-research records remain historical records.

## Executive result

- Native watcher supervision and shutdown recovery are implemented on `main`
  through PR #82. The old statement that watcher supervision was still open is
  stale.
- PR #85 adds merged worker-level fault, atomicity, restart/backup, alias, and
  stale-publication coverage. Its passing CI and tests strengthen P2-03,
  P2-05, P2-06, and P2-07, but they do not provide installed-app observations
  or owner acceptance.
- The P2-08 `Complete` statement introduced by PR #85 is not supportable
  as an acceptance claim. The reconciled status is Partial: native and
  automated seam evidence exists, while the installed journey and explicit
  full-criterion owner acceptance do not.
- PR #86 is the current performance evidence source. It reproduces F1, leaves
  F2 failing at a 14,267 ms warm p95 against the provisional 10 s target, and
  leaves F3 unmeasured. No budget or fixture decision was made.
- The installed-app observation table is still empty. #47 and #48 remain open;
  PR #89 records that this host has no qualifying writable FAT32/cross-volume
  target, which is a blocker, not a qualification.
- No explicit full Phase 2 or P2-08 acceptance decision is recorded. Epic #33,
  child issues #36-#41, and platform follow-ups #47/#48 remain open.
  Production scanning stays hidden under the accepted P2-03 through P2-08
  gate.

## Audit basis

### Repository and merge state

On 2026-09-08 local time, `git fetch --prune origin main` completed and
`origin/main` resolved to:

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

Live GitHub showed these open issues: #33, #36, #37, #38, #39, #40, #41,
issues #47 and #48. There were no open pull requests at the audit point. The exact
issue links are in the [integration index](README.md) and the parent
[Phase 2 epic](https://github.com/guilhermebmichelin-create/fruitboard/issues/33).

### CI provenance

The following records were checked through GitHub for the exact commit or PR
head named. `Foundation CI` has nine jobs; `windows-packaging-smoke`
is triggered on pull requests, schedules, and manual dispatch, not on a push
to `main`.

| Evidence target | Exact commit or head | Live result |
| --- | --- | --- |
| Current `origin/main` | `0b7612db3570e6235d4d2c86a678dd9004264f30` | [Foundation run 34293250231](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34293250231): success, all nine jobs |
| PR #82 merge | `05d39ff074ab6c568d998175744551561928de89` | [Foundation run 34248128449](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34248128449): success, all nine jobs; Windows feature-on tests and warnings-denied Clippy passed |
| PR #85 merge | `c05f4b6a82c5b3469900a408b19b8c979bc4974b` | [Foundation run 34281613081](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34281613081): success, all nine jobs |
| PR #86 merge | `4218e413be7dc27422eab9a45252bcdd1ca74359` | [Foundation run 34290753717](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34290753717): success, all nine jobs |
| PR #87 merge | `e9464be404744a91071343c0e7d32f79f5485fa2` | [Foundation run 34291338301](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34291338301): success, all nine jobs |
| PR #88 head | `ddeac9025dcdd9f1f65c9236a3120ef4057420f1` | [Foundation run 34291358030](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34291358030) and [packaging run 34291358010](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34291358010): both success |
| PR #89 head | `289609511e1cb3359a2a30418a1fde56e8463dc9` | [Foundation run 34292846030](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34292846030) and [packaging run 34292846032](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34292846032): both success |

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
  recorded under `docs/review/issue-34/`. It does not re-confirm the
  complete scanner journey at the current merged baseline. The current
  installed-app checklist table remains empty.
- **Performance qualification:** Not applicable to this criterion; no scanner
  benchmark qualifies it.
- **Platform qualification:** The picker record is Windows evidence for the
  picker slice only. It is not a DriveFS, FAT32, or scanner platform claim.
- **Owner acceptance:** Phase 1 is accepted, but no separate current Phase 2
  P2-01 acceptance line was found.
- **Reconciled status/gate:** Evidence present; not promoted. Re-confirm
  selection, cancellation, and restart survival in the combined installed
  journey or obtain an explicit owner decision on the existing evidence.

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
- **Installed-app observations:** No packaged-app add/modify/rename
  observation is recorded; the checklist row is empty.
- **Performance qualification:** No P2-02 performance qualification is
  claimed. The benchmark driver exercises a hidden worker and is P2-11
  evidence, not installed-app evidence.
- **Platform qualification:** Windows/NTFS fixture coverage exists in CI;
  #47/#48 remain outside that qualification.
- **Owner acceptance:** No explicit P2-02 owner acceptance was found.
- **Reconciled status/gate:** Partial. Run the installed convergence cases and
  retain the broader platform limitation.

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
- **Installed-app observations:** No installed cancellation, unavailable-root,
  denial, or resource-limit retention observation is recorded.
- **Performance qualification:** The cooperative stop samples in the
  benchmark are P2-11 worker measurements only; they do not qualify the
  installed cancellation or UI acknowledgement requirement here.
- **Platform qualification:** Fake-port fault injection and Windows CI are
  automated evidence. The ACL twin may be unavailable under elevated tokens,
  and DriveFS/non-NTFS behavior remains unverified.
- **Owner acceptance:** PR #85 says the close-out does not promote an
  acceptance ID. No explicit P2-03 acceptance was found.
- **Reconciled status/gate:** Partial. Add the integrated installed retention
  observation and owner review.

### P2-04 - Durable generations, deduplication, leases, backoff, and cancellation

- **Implementation:** PRs #60 and #61 establish the durable ledger and
  contract preservation; PR #70 composes the worker; PR #82 connects host
  lifecycle and shutdown recovery; PR #85 adds worker close-out cases.
- **Automated tests:** The `migration` lane covers state-machine and
  restart/backup transition matrices. The feature-enabled desktop host tests
  cover lifecycle recovery and joined loops. The exact-current and PR #85
  Foundation runs are green.
- **Installed-app observations:** No installed retry, cancellation, queue,
  lease, or restart observation is recorded.
- **Performance qualification:** The re-run reports cooperative worker stop
  p95 of 22 ms over six samples, but has no renderer and does not exercise the
  250 ms UI acknowledgement budget. It is not a complete P2-04 qualification.
- **Platform qualification:** Windows CI and injected lifecycle tests qualify
  only those tested seams; they do not qualify all filesystem modes.
- **Owner acceptance:** No explicit P2-04 owner acceptance was found.
- **Reconciled status/gate:** Partial. Complete the installed behavior and
  remaining end-to-end review.

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
- **Installed-app observations:** No disable/remove-while-running observation
  is recorded in the installed checklist.
- **Performance qualification:** Not applicable; no timing result proves this
  concurrency property.
- **Platform qualification:** The source-marker assertions and fake durable
  harness are not a cross-filesystem qualification.
- **Owner acceptance:** No explicit P2-05 owner acceptance was found.
- **Reconciled status/gate:** Partial. Add the installed operation and owner
  decision.

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
- **Installed-app observations:** No installed restart or interrupted-work
  recovery observation is recorded.
- **Performance qualification:** Not applicable; a passing rollback test is
  not a performance result.
- **Platform qualification:** Storage and Windows CI evidence does not qualify
  DriveFS, FAT32, network, or other untested storage modes.
- **Owner acceptance:** No explicit P2-06 owner acceptance was found.
- **Reconciled status/gate:** Partial. Add packaged restart/recovery evidence
  and owner review.

### P2-07 - Hardlink aliases and uncertain identity preserve per-path presence

- **Implementation:** PR #59 establishes conservative identity decisions; PR
  #62 publishes alias-aware locations; PR #85 adds worker-level alias and
  replacement/rename close-out cases.
- **Automated tests:** `p2_07_hardlink_delete_one_*` verifies that
  deleting one alias leaves the other present and restores the missing path.
  The `p2_07_rename_replacement_*` cases preserve per-path history,
  mint fresh replacement records, and avoid Phase 4 grouping. Windows/NTFS
  fixture CI is green.
- **Installed-app observations:** No installed hardlink or alias observation
  is recorded.
- **Performance qualification:** Not applicable.
- **Platform qualification:** The evidence is local-NTFS-oriented. PR #88
  supplies a manual #48 plan and PR #89 records no qualifying FAT32 target;
  neither qualifies FAT32, cross-volume, exFAT, ReFS, or network identity.
- **Owner acceptance:** No explicit P2-07 acceptance or non-NTFS scope
  exclusion was found.
- **Reconciled status/gate:** Partial. Add installed alias evidence and
  resolve #48 with evidence or an explicit owner scope decision.

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
- **Installed-app observations:** The Scan now, queued/running honesty,
  Library paging, restart survival, cancel/unavailable/retry, and watcher
  journey have no recorded packaged-app observations. The checklist table is
  intentionally empty.
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
- **Reconciled status/gate:** Partial, not Complete. Run the packaged journey
  and obtain explicit full-criterion owner acceptance before promoting it.

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
- **Installed-app observations:** No installed watcher burst, coverage-loss,
  overflow, stale-generation, or follow-up observation is recorded.
- **Performance qualification:** No real OS overflow timing or installed
  watcher latency qualification is recorded.
- **Platform qualification:** Injected handles and Windows CI qualify
  deterministic seams. #47 DriveFS behavior, network shares, ACL revocation
  during a watch, and real overflow timing remain unverified.
- **Owner acceptance:** No explicit P2-09 owner acceptance or #47 scope
  exclusion was found.
- **Reconciled status/gate:** Partial. Run the installed watcher journey and
  resolve the DriveFS/platform gate.

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
- **Platform qualification:** The re-run is a same-host, laptop-class
  Windows 11 measurement with background DriveFS, antivirus, agent, and
  sibling-build load. It is not the idle-host isolation required by the
  pending F2-rerun decision and transfers to no other host class.
- **Owner acceptance:** The starting budgets are owner-accepted as
  provisional in PR #57. F1 (quota versus fixture), F2 (budget/host/rerun or
  optimization), and F3 (100k qualification) have no posted owner decision.
  The [decision brief](budget-decision-brief.md) and [re-run report](benchmark-triage-20260908/rerun-report.md) quote the exact sentences still required.
- **Reconciled status/gate:** Measured, not qualified. Resolve F1-F3 with
  explicit owner decisions and any required re-measurement or plan amendment.

### P2-12 - Complete visible journey works after integration

- **Implementation:** PR #80 records the historical checkpoint and PRs
  #81/#82/#85/#86/#88/#89 update the merged implementation and evidence
  boundary. The current index and this report are the status reconciliation,
  not a replacement for the journey record.
- **Automated tests:** The hidden worker/driver transcript in the historical
  checkpoint and the current exact-commit CI establish automated and
  contract-level evidence. They do not establish an installed-app journey.
- **Installed-app observations:** The observation table in
  `installed-app-journey-checklist.md` is intentionally empty. No
  selection, scan, Library, restart, watcher, cancellation, or retry result is
  claimed here.
- **Performance qualification:** P2-11 remains unresolved with F1-F3 open;
  no performance decision can complete P2-12.
- **Platform qualification:** #47 DriveFS and #48 FAT32/cross-volume remain
  open and unverified. The #89 no-target survey is not a pass.
- **Owner acceptance:** No explicit owner acceptance of the P2-12 checkpoint
  or Phase 2 was found. Issue #41 and epic #33 remain open.
- **Reconciled status/gate:** Pending. Fill the installed record, resolve
  performance and platform decisions, and obtain the owner's
  criterion-by-criterion acceptance.

## Remaining gates and handoff

1. Run the packaged, feature-enabled Windows journey at a merged commit and
   record observations in the existing checklist. This report does not edit
   that checklist.
2. Resolve F1 by either raising the durable quota above 10,005 or redefining
   the baseline as exactly 10,000 observations including aliases.
3. Resolve F2 with the owner-selected budget/host/rerun/optimization path.
   The current re-run reduces variance but still misses the written target and
   was not isolated.
4. Resolve F3 by an owner-approved quota/chunking/defer decision before
   claiming the 100,000-entry memory qualification.
5. Land #47/#48 host evidence or record explicit owner scope exclusions.
   Current plans and the no-target survey do neither.
6. Obtain explicit owner acceptance for each criterion or an explicitly
   recorded pending-at-close decision with a named follow-up. Do not infer
   acceptance from a merge, commit message, green CI, or a contributor's
   summary.

Until those gates are resolved, P2-03 through P2-08 remain behind the
production-scanning gate, the Phase 2 epic remains open, and no Phase 2
acceptance is declared.
