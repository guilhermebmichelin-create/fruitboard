# Phase 2 acceptance record - 2026-09-14

Status: **Accepted by the owner with known gaps on 2026-09-14; the bounded
Rust-parser spike is the next step.**

The owner closed the Phase 2 evidence aggregator
[#41](https://github.com/guilhermebmichelin-create/fruitboard/issues/41) as
COMPLETED on 2026-09-14 and recorded Phase 2 as accepted with known gaps:
P2-08 stays Partial, issues #107/#47/#48 stay open, and the bounded
Rust-parser spike is the next step. The Phase 2 epic
[#33](https://github.com/guilhermebmichelin-create/fruitboard/issues/33)
remains open. Following the [Phase 1 acceptance
precedent](../PHASE_1_REVIEW.md), this is the explicit owner decision that the
dated evidence records could not make for themselves.

This record is documentation only. It does not promote P2-08, merge or close
any pull request or issue, post any GitHub comment, amend a budget, quota,
fixture, or scope, activate production scanning, or start Phase 3.

## Publication boundary

<!-- markdownlint-disable MD060 -->

| Item                    | Recorded value                                                                                                                                                                                                                                                                                                            |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Acceptance date         | 2026-09-14                                                                                                                                                                                                                                                                                                                |
| Phase 2 aggregator      | [#41](https://github.com/guilhermebmichelin-create/fruitboard/issues/41) closed COMPLETED at `2026-09-14T04:31:53Z`; no closing comment is retained                                                                                                                                                                       |
| Epic state              | [#33](https://github.com/guilhermebmichelin-create/fruitboard/issues/33) remains open                                                                                                                                                                                                                                     |
| Publication boundary    | `origin/main` `c73fc1e548364795dc0b94e96c4cb6cbd2d134da`, the merge of PR #123                                                                                                                                                                                                                                            |
| Merged advances         | #117 `9ebcc9ffcdd94300b9ad2af22281be4ec8c187db`; #121 `13c0ba2af88673746a96e787fae315d2eca920a0`; #120 `fea3a8763d039c9855a368f7072a4f41acef8d85`; #122 `3fcaa4ade3099105594b48eadc785731608c2824`; #123 `c73fc1e548364795dc0b94e96c4cb6cbd2d134da` (all documentation/evidence merges after the #118 boundary `adab234`) |
| Measured scanner source | `69f27f64f26aa657182a9260cc8e78f28a5838fb`; unchanged, and no timing was rerun after the documentation-only advances                                                                                                                                                                                                      |
| P2-08 installed run     | Executed once on installed `adab234b9b17b45f87d30460fa643818161e7cf3`; published by merged #121                                                                                                                                                                                                                           |
| Qualification result    | Warm nearest-rank p95 `10,173 ms` versus the unchanged `10,000 ms` target; non-qualifying ([dated evidence](qualification-evidence-20260913.md))                                                                                                                                                                          |

<!-- markdownlint-enable MD060 -->

## Criterion-by-criterion disposition (P2-01 through P2-12)

The owner accepted Phase 2 with the gaps named in this record. `Accepted`
below means the criterion is accepted as recorded in the dated evidence, with
any carried boundary or gap noted in the same row. It is not a new
performance, platform, or accessibility claim.

<!-- markdownlint-disable MD060 -->

| ID    | Disposition                                                 | Recorded basis and carried boundary or gap                                                                                                                                                                                                                                                                                                 |
| ----- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| P2-01 | Accepted                                                    | Picker/settings implementation #34/#35; #54 picker and #92 root/settings journey; #58 keyboard/narrow/axe fake-adapter evidence. The separate first-run route was not carried as a gap; the inline Preferences entry stands.                                                                                                               |
| P2-02 | Accepted                                                    | Reconciler #59, enumeration #64, worker #70; deterministic tests plus #92 installed add/modify/rename/remove/restore convergence. Carried boundary: installed evidence is local-NTFS; #47/#48 stay open.                                                                                                                                   |
| P2-03 | Accepted                                                    | Safety path #62/#70, fault cases #85, diagnostic hardening #111; partial/offline/denied/cancel/resource-limit tests plus #92 retention and #111 NTFS cases. Carried boundary: the #111 installed observations remain tied to tested source `05fb35c`, not rerun on merged `914d7bd`; mid-watch ACL, network, and DriveFS cases are absent. |
| P2-04 | Accepted                                                    | Durability #60/#61/#70, recovery #82, active-slot fix #104; migration/worker gates, #97 fences, #104 regression; #92 recovery and #110 queued/running/terminal same-job recovery. Carried: #107 stays open for the S5 disposition; no defect was found.                                                                                    |
| P2-05 | Accepted                                                    | Lease/disable-remove/staging/stale-publication #60/#62/#85; publication tests; #92 disable-while-running and #110 queued disable/remove. No stale-publication defect identified; installed evidence remains historical-source evidence.                                                                                                    |
| P2-06 | Accepted                                                    | Atomic publication/crash/rollback/backup/migration #62/#85; crash, recovery, backup, migration, and atomic-publication tests; #92 interrupted recovery and #110 genuine running-lease hard-kill/retry recovery.                                                                                                                            |
| P2-07 | Accepted                                                    | Identity/alias/rename/replacement #59/#62/#85; alias, rename, and per-location transition tests; #92 rename and #111 in-root hardlink aliases on local NTFS from `05fb35c`. Carried boundary: FAT32/cross-volume identity is unverified; #48 stays open.                                                                                   |
| P2-08 | **Partial**                                                 | D2/D3 converged on the installed current head and the typed `cancelled`/`unavailable` presentation was revalidated, but the full Scan/Cancel/Retry and Library criterion remains unproven and unpromoted. See the narrowed gaps below.                                                                                                     |
| P2-09 | Accepted                                                    | Watcher/coalescing/generation/reconnect #68/#69/#71/#81/#82; #97 burst/coverage-loss/fence counts; #92 follow-ups and #106 isolated burst convergence. Carried boundary: no controlled OS-buffer overflow/timing record and no DriveFS watcher evidence; #47 stays open.                                                                   |
| P2-10 | Accepted                                                    | Static no-parser/no-content-I/O guards and fixture equality #81; dependency/privacy checks, static guards, and source-byte preservation. No runtime content-read spy is claimed; that optional evidence-strength choice is not reopened here.                                                                                              |
| P2-11 | Accepted with performance non-qualification carried forward | Benchmark/validator work #67/#72/#86/#93/#94/#98/#114; the current-only qualification measured source `69f27f6` with 10/10 authoritative scans but warm p95 `10,173 ms` against the unchanged `10,000 ms` target. F1/F2/F3 remain as recorded below; no budget change follows.                                                             |
| P2-12 | Accepted as the phase checkpoint                            | Merged implementation/CI map #80 and the ten required checks on the #116 boundary (CI provenance only), plus the installed records in the dated ledger. No single current-head installed journey covers every criterion; the gaps in this record are carried forward.                                                                      |

<!-- markdownlint-enable MD060 -->

## P2-08 remains Partial (accepted known gap)

The merged #121 installed run on current head `adab234` revalidated the D2/D3
Retry paths and the #111 typed presentation. The current installed path no
longer exposes the rejected Retry: a terminal `cancelled` job has
`retryAvailable=false`, the UI offers an enabled `Scan now` that converges,
and after the unavailable-root retry budget is exhausted the restored root
also reconciles through `Scan now` while prior committed results stay visible.
The durable codes and UI agree for `cancelled` and `unavailable`, and the
installed console is `data-review-adapter="native"`.

The criterion remains Partial because these requirements were not produced:

- the `access_denied`, `resource_limit`, `unsupported`, and `worker_failed`
  typed presentations (producing them requires the NTFS ACL, overflow, or
  malformed-fixture cases that were not rerun);
- a keyboard trace and axe run for the transient `queued` and `running`
  states;
- a stable rendered `interrupted` state (it was observed only durably);
- full-criterion P2-08 acceptance, which the owner did not promote.

Recorded minor finding: axe reported exactly one moderate `region` violation
per captured state (content not contained by a landmark) and no critical or
serious violations. That is a finding for later work, not a claimed
accessibility defect.

## F1/F2/F3 and performance non-qualification carried forward

- F1: the 10,000 FLP-named baseline fixture produced 10,005 observations
  (including hardlink aliases) and ended `ResourceLimit`, so it was never
  authoritative. The owner-approved `custom-9995` fixture (exactly 10,000
  observations, seed `0`) was used for the qualification run. The plan's
  fixture/quota wording is not amended by this record.
- F2: the current-only qualification returned `non-qualifying` with warm
  nearest-rank p95 `10,173 ms` versus the unchanged `10,000 ms` target,
  measured from source `69f27f6` under the owner-approved
  `repository-minimum` profile. No historical strict-profile or A/B rerun is
  requested.
- F3: the 100,000-entry qualification set and its provisional `<=128 MiB`
  working-memory target remain unmeasured; merged #96's bounded
  chunked-snapshot work is proposal history, not an approved scale design.
- No performance qualification, budget, quota, or target change follows from
  this acceptance; performance remains unqualified.

## Issues explicitly left open

- [#107](https://github.com/guilhermebmichelin-create/fruitboard/issues/107):
  the S5 restart-contract follow-up. The terminal-follow-up/successor path and
  the genuine running-lease same-job recovery are both recorded, and no
  product or contract defect was found, but the issue keeps its open
  disposition.
- [#47](https://github.com/guilhermebmichelin-create/fruitboard/issues/47):
  the DriveFS host run remains unverified; merged #90 holds the blocker report
  and setup requirements.
- [#48](https://github.com/guilhermebmichelin-create/fruitboard/issues/48):
  cross-volume and FAT32 identity remain unverified; local-NTFS identity is
  the accepted scope.

## Observed dispositions recorded, not decided

- [#119](https://github.com/guilhermebmichelin-create/fruitboard/pull/119):
  still an open draft at `f64decd`, based on `adab234`. Its feature-gated
  phase diagnostics are diagnostic investigation only; they do not explain
  the historical Partial, qualify performance, or approve the proposed
  ancestor-validation fast path. The owner's disposition is pending.
- [#108](https://github.com/guilhermebmichelin-create/fruitboard/pull/108):
  GitHub now reports it closed without merge on 2026-09-14
  (`2026-09-14T10:32:34Z`), consistent with the recommendation published in
  merged #120.
- [#113](https://github.com/guilhermebmichelin-create/fruitboard/pull/113):
  GitHub reports it closed without merge on 2026-09-14
  (`2026-09-14T10:32:35Z`), consistent with the superseded-candidate
  recommendation published in merged #120.

These states are observed at recording time. This record does not close,
reopen, retarget, or relitigate any of them.

## What this record does not close or decide

- P2-08 is not promoted; it stays Partial with the narrowed gaps above.
- Issues #107, #47, and #48 are not closed and no scope exclusion is applied
  to DriveFS, FAT32, or cross-volume identity.
- F1/F2/F3 are not resolved; no budget, quota, fixture, or target is amended,
  and performance is not qualified.
- Production scanning is not activated by this acceptance.
- The bounded Rust-parser spike is the stated next step, but it is not started
  here; PyFLP remains an optional candidate, and no Phase 3 issue or code is
  started.
- No GitHub issue or pull request is commented on, edited, or mutated by this
  record.

## Checks and provenance

- Owner decision provenance: #41 is closed COMPLETED on 2026-09-14 with no
  closing comment; the acceptance wording is recorded by this documentation
  task as `Accepted with known gaps`.
- Evidence sources are the merged [acceptance packet](acceptance-packet-2026-09-12.md),
  the [integration index](README.md), the [qualification evidence](qualification-evidence-20260913.md),
  and the merged [#121 P2-08 run](installed-journey/run-20260914-p2o8-retry-presentation.md).
- No local build, fixture generation, installed run, benchmark, or Cargo
  cache was used. This change runs markdownlint, Prettier, the repository
  privacy check, and `git diff --check`; merged-main CI covers the unchanged
  code, and the PR's own required contexts are the publication gate.
