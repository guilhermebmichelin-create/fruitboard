# Phase 2 residual disposition drafts - 2026-09-14

Status: **DRAFT - record only. No GitHub issue or pull request was commented on,
closed, reopened, edited, labeled, retargeted, or merged by this task.** This
file audits the still-open Phase 2 issues against the merged evidence and drafts
owner-ready disposition text. Posting any of it is an owner/maintainer action.

## Audit basis

- Issues audited: [#33](https://github.com/guilhermebmichelin-create/fruitboard/issues/33)
  (epic), [#36](https://github.com/guilhermebmichelin-create/fruitboard/issues/36),
  [#37](https://github.com/guilhermebmichelin-create/fruitboard/issues/37),
  [#38](https://github.com/guilhermebmichelin-create/fruitboard/issues/38),
  [#39](https://github.com/guilhermebmichelin-create/fruitboard/issues/39),
  [#40](https://github.com/guilhermebmichelin-create/fruitboard/issues/40).
- Acceptance authority: [acceptance-2026-09-14.md](acceptance-2026-09-14.md),
  the P2-01 through P2-12 table, the P2-08 gaps, and F1/F2/F3.
- Contract authority: [PHASE_2_EXECUTION_PLAN.md](../../PHASE_2_EXECUTION_PLAN.md).
- Evidence packet: [acceptance-packet-2026-09-12.md](acceptance-packet-2026-09-12.md),
  [README.md](README.md#live-review-status-2026-09-13),
  [qualification-evidence-20260913.md](qualification-evidence-20260913.md), and
  the [P2-08 current-head run](installed-journey/run-20260914-p2o8-retry-presentation.md).
- Merged PRs cited by the record, verified merged via `gh`: #59, #60, #64, #70,
  #82, #85, #92, #97, #104, #106, #110, #111, #112, #114, #119, and #121.
- Not re-litigated per task: #108/#113 (closed without merge; see
  [closeout-108-113.md](closeout-108-113.md)) and #107 (closed).

## Observed GitHub state at audit time

State is recorded, not decided. Where it has moved since the acceptance record,
the change is noted, not re-adjudicated.

<!-- markdownlint-disable MD060 -->

| Item            | Audited state                     | Note                                                                                          |
| --------------- | --------------------------------- | --------------------------------------------------------------------------------------------- |
| #33 epic        | OPEN (`epic`, `phase-2`)          | Kept open by the acceptance record                                                            |
| #36             | OPEN                              | Reconciler / filesystem-only metadata                                                         |
| #37             | OPEN                              | Watcher adapter / coalescing / overflow recovery                                              |
| #38             | OPEN                              | Durable queue / cancellation / retries / observability                                        |
| #39             | OPEN                              | Filesystem-only verification / parser deferral                                                |
| #40             | OPEN                              | Atomic persistence / missing-restored / Library list                                          |
| #41 aggregator  | CLOSED COMPLETED                  | Closed 2026-09-14T04:31:53Z; acceptance wording recorded by this documentation task           |
| #47             | OPEN (`phase-2`, `spike`)         | DriveFS host run unverified; stays open                                                       |
| #48             | OPEN (`phase-2`, `spike`)         | FAT32/cross-volume identity unverified; stays open                                            |
| #107            | CLOSED COMPLETED                  | Now closed 2026-09-15T01:46:45Z; not re-litigated per task                                    |
| #119            | MERGED 2026-09-14T17:35:04Z       | Merge `58538cd`; the acceptance record had recorded it as an open draft pending owner review  |
| Publication     | `origin/main` `23547c34`          | Documentation/dependency merges after the record's `c73fc1e`; measured source stays `69f27f6` |

<!-- markdownlint-enable MD060 -->

The scanner timing is unchanged: no merge after measured source
`69f27f64f26aa657182a9260cc8e78f28a5838fb` reran the qualification, so the
non-qualifying warm p95 `10,173 ms` against the unchanged `10,000 ms` target
still stands.

## Per-issue recommended disposition

`Close as accepted` means the issue's own acceptance wording is satisfied by the
2026-09-14 acceptance record and its cited merged evidence, with any carried
boundary explicitly assigned to an open tracker. `Keep open` means a residual
criterion is carried that is not yet evidenced or excluded.

<!-- markdownlint-disable MD060 -->

| Issue | Owned criteria                            | Merged evidence                                                     | Recommended disposition                                                                                              |
| ----- | ----------------------------------------- | ------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| #33   | Phase 2 epic tracker                      | #41 closed COMPLETED; #36-#40 remain open                           | **Keep open** as the epic tracker; no child acceptance is implied                                                     |
| #36   | P2-02/P2-03/P2-07/P2-10/P2-11             | #59, #64, #70, #85, #92, #97, #111, #114, #121                      | **Close as accepted**; local-NTFS boundary carried by open #47/#48, performance by F2/F3                              |
| #37   | P2-09                                     | #68/#69/#71/#81/#82 (and #82), #97, #92, #106                       | **Close as accepted**; OS-buffer overflow/timing and DriveFS watcher boundary carried by open #47                     |
| #38   | P2-04/P2-05/P2-08/P2-11                   | #60/#61/#70, #82, #85, #97, #104, #110, #111, #121                  | **Keep open** for the P2-08 controls gaps; P2-04/P2-05/P2-11 are accepted                                            |
| #39   | P2-10                                     | #81, plus the bounded Rust-parser spike proposal merged as #131     | **Close as accepted**; the bounded spike is the separate next step after acceptance                                   |
| #40   | P2-03/P2-05/P2-06/P2-07/P2-08/P2-11       | #62/#77/#85, #92, #104, #110, #111                                  | **Keep open** for the P2-08 Library/list gaps; the other owned criteria are accepted                                  |

<!-- markdownlint-enable MD060 -->

Owner-reversible alternative: if a dedicated P2-08 follow-up issue is opened
first, #38 and #40 could instead be closed as accepted with the gaps reassigned
to that follow-up. No such issue is proposed or created here.

### #33 - Phase 2 epic (keep open)

Draft comment body:

```markdown
Keep this epic open. Phase 2 is accepted with known gaps per
docs/review/phase-2-integration/acceptance-2026-09-14.md; the evidence
aggregator #41 is closed COMPLETED. Child issues #36/#37/#39 have their criteria
accepted, and #38/#40 keep the narrowed P2-08 residual. #47/#48 stay open and no
scope exclusion is applied. F1/F2/F3 and the performance non-qualification carry
forward unchanged. The bounded Rust-parser spike is the stated next step. No
budget, quota, fixture, target, or production-activation change follows.
```

### #36 - Incremental reconciler (close as accepted)

Draft comment body:

```markdown
Close as accepted. The owner accepted P2-02/P2-03/P2-07/P2-10/P2-11 on
2026-09-14. Merged #59/#64/#70 plus the installed #92 convergence and the #111
NTFS evidence cover the filesystem-only reconciler. The installed evidence is
local-NTFS; non-NTFS and cross-volume identity remain unverified under the open
#47/#48 and are not excluded. The performance non-qualification (F2/F3) is
carried unchanged. No new run or scope change is requested.
```

### #37 - Watcher adapter (close as accepted)

Draft comment body:

```markdown
Close as accepted. P2-09 is accepted on the merged #68/#69/#71/#81/#82 watcher
work plus the #97 burst/coverage-loss/fence counts, the #92 follow-ups, and the
#106 isolated burst convergence. Carried boundary: there is no controlled
OS-buffer overflow/timing record and no DriveFS watcher evidence; that platform
boundary stays with the open #47. No new run or scope change is requested.
```

### #38 - Durable queue (keep open for P2-08)

Draft comment body:

```markdown
Keep open for the narrowed P2-08 residual. P2-04/P2-05/P2-11 are accepted per
the 2026-09-14 record. P2-08 remains Partial: the typed access_denied,
resource_limit, unsupported, and worker_failed presentations were not produced
(they need the NTFS ACL/overflow/malformed cases), queued/running lack a keyboard
trace and axe run, and interrupted was observed only durably. D2/D3 converged on
the installed current head, so the recorded rejected-Retry behavior is no longer
exposed; the policy-versus-defect disposition and full-criterion acceptance
remain the owner's. No scope, budget, or target change is proposed.
```

### #39 - Filesystem-only verification and parser deferral (close as accepted)

Draft comment body:

```markdown
Close as accepted. P2-10 is accepted on the merged #81 static no-parser/
no-content-I/O guards and synthetic-fixture source-byte preservation. The
filesystem-only/no-parser deferral is recorded, and no runtime content-read spy
is claimed. The bounded independent Rust-parser research spike is the stated
next step after Phase 2 acceptance and is proposed in merged #131; it is not a
blocker on this issue and does not authorize a production parser adapter.
```

### #40 - Atomic persistence and Library list (keep open for P2-08)

Draft comment body:

```markdown
Keep open for the narrowed P2-08 Library/list residual. P2-03/P2-05/P2-06/P2-07/
P2-11 are accepted per the 2026-09-14 record, including the required hardlink
regression. P2-08 remains Partial for the Library criterion: the typed
presentations for access_denied, resource_limit, unsupported, and worker_failed
are unproduced, and the installed scan-console/native captures do not yet cover
every criterion state. The P2-08 gaps are carried with #38. No scope, budget, or
target change is proposed.
```

## P2-08 four-gap carryover

Carried unchanged from the acceptance record; assigned to the keep-open #38
(controls) and #40 (Library list):

1. The `access_denied`, `resource_limit`, `unsupported`, and `worker_failed`
   typed presentations are unproduced; producing them needs the NTFS ACL,
   overflow, or malformed-fixture cases that the #121 run explicitly did not
   rerun.
2. The transient `queued` and `running` states lack a keyboard trace and an axe
   run.
3. `interrupted` was observed only durably, not as a stable rendered state.
4. Full-criterion P2-08 acceptance was not promoted by the owner.

Recorded minor finding, not a defect claim: axe reported exactly one moderate
`region` violation per captured state (content not contained by a landmark) and
no critical or serious violations.

## No scope, budget, or target change proposed

- F1/F2/F3 are unchanged; the 10,000-entry contract and the `10,000 ms`
  warm-p95 target are unchanged, and the 100,000-entry F3 scale remains an
  unmeasured proposal (not an approved design).
- `custom-9995`/seed `0` and `repository-minimum` remain the approved, used
  fixture/profile; no historical strict-profile or A/B rerun is requested.
- #47/#48 remain open with no scope exclusion for DriveFS, FAT32, or
  cross-volume identity. Production scanning stays hidden; no Phase 3 issue or
  code is started.
- This audit adds no product, contract, migration, fixture, quota, or
  performance change.

## Observed state changes since the acceptance record

Recorded only; not re-litigated.

- #119 is now **MERGED** (`58538cd`). The acceptance record had recorded it as an
  open draft with owner disposition pending. Its feature-gated diagnostics
  remain diagnostic-only: they do not qualify performance or explain the
  historical Partial, and the `10,173 ms` non-qualifying warm p95 is unchanged.
- #107 is now **CLOSED COMPLETED** (2026-09-15T01:46:45Z); the record had
  recorded it as staying open. Per task, this is not re-adjudicated.
- `origin/main` advanced to `23547c34` through documentation and dependency
  merges after the record's `c73fc1e`; measured scanner source remains
  `69f27f6`.

## Non-actions

- No GitHub issue or pull request was commented on, edited, labeled, closed,
  reopened, retargeted, or merged.
- No issue disposition in this file has been posted or applied; the drafts are
  paste-ready text for the owner/maintainer.
- No local build, fixture generation, installed run, benchmark, or Cargo cache
  was used. Checks run for this file only: `pnpm lint:docs` and
  `git diff --check`.
