# P2-08 dedicated follow-up issue proposal - 2026-09-15

Status: **DRAFT - record only.** No GitHub issue or pull request was created,
opened, commented on, edited, labeled, closed, reopened, retargeted, assigned,
or merged by this task. This file drafts owner-ready issue text for one
dedicated P2-08 follow-up covering the four narrowed gaps. Creating, posting,
labeling, or scheduling that issue is an owner/maintainer action only.

## Purpose and source boundary

The 2026-09-14 acceptance record keeps P2-08 **Partial** with four narrowed
gaps. The residual disposition draft leaves those gaps with the keep-open
issues #38 (controls) and #40 (Library/list) and names an owner-reversible
alternative: open a dedicated P2-08 follow-up first, then reassign the gaps and
close #38/#40 as accepted. This proposal drafts that optional follow-up issue.
It makes no decision and changes no evidence.

Sources validated for this draft:

- [acceptance-2026-09-14.md](acceptance-2026-09-14.md) - the P2-08 Partial
  disposition, its four narrowed gaps, and the axe minor finding.
- [merged #121 P2-08 run](installed-journey/run-20260914-p2o8-retry-presentation.md)
  - the current-head (`adab234`) installed retry/presentation/accessibility
    record and the explicitly unproduced typed codes.
- [residual-disposition-drafts-2026-09-14.md](residual-disposition-drafts-2026-09-14.md)
  - the #38/#40 keep-open rationale and the owner-reversible alternative.
- Open issues [#38](https://github.com/guilhermebmichelin-create/fruitboard/issues/38)
  and [#40](https://github.com/guilhermebmichelin-create/fruitboard/issues/40),
  confirmed read-only via `gh`.

## Observed GitHub state at draft time

State is recorded, not decided. If either issue has moved since this draft, that
is an observation for the owner and is not re-adjudicated here.

<!-- markdownlint-disable MD060 -->

| Issue | Title                                                                | Observed state | Draft note                                                       |
| ----- | -------------------------------------------------------------------- | -------------- | ---------------------------------------------------------------- |
| #38   | Durable scan queue, cancellation, retries, and observability         | OPEN           | Keep-open rationale for the P2-08 controls gaps                  |
| #40   | Atomic project-file/snapshot persistence and missing/restored behavior | OPEN         | Keep-open rationale for the P2-08 Library/list gaps            |

<!-- markdownlint-enable MD060 -->

## Why a dedicated follow-up is proposed

The residual disposition draft records #38/#40 as **keep open** for the P2-08
residual because their other owned criteria are already accepted. A dedicated
issue would give the four gaps one owned outcome instead of parking them on two
issues whose primary criteria are done, and would let #38/#40 close as accepted
with the residual explicitly reassigned. This is optional: keeping #38/#40 open
is the recorded default, and no scope exclusion for #47/#48 is implied by
either choice.

## Proposed issue

### Suggested title

`[Phase 2] P2-08 follow-up: complete the scan-console typed presentations and accessibility evidence`

### Suggested labels

`phase-2`, `enhancement`, `accessibility`. Labels are a recommendation only; no
label was applied.

### Proposed body (paste-ready draft)

````markdown
## User problem and outcome

Phase 2 is accepted with known gaps, and P2-08 remains Partial because four
narrowed requirements were not produced by the merged #121 current-head
installed run. Until they are evidenced or explicitly excluded, Scan/Cancel/
Retry and Library cannot be treated as fully usable, honest, and persistent.

Outcome: one installed current-head evidence pass covers the four gaps below,
or the owner records an explicit exclusion per gap, so P2-08 can be accepted or
deliberately scoped.

## Proposed scope

This issue owns only the four carried gaps, with these acceptance criteria.

### Gap 1 - typed presentations for the four unproduced codes

Accepted when one installed feature-enabled run on the current head produces
each of `access_denied`, `resource_limit`, `unsupported`, and `worker_failed`,
and for each one: the durable terminal code and the rendered client copy agree;
`retryAvailable` is correct; prior committed results are preserved with no
false-missing rows or partial publication; and the capture carries source/build
provenance. Candidate producers named by the #121 run are the NTFS ACL denial,
the overflow case against the unchanged 10,000-observation bound, and a
malformed/unsupported fixture. The 10,000-observation contract and the
`10,000 ms` target are unchanged.

### Gap 2 - keyboard and axe evidence for the transient states

Accepted when the transient `queued` and `running` scan-console states each have
a current-head installed keyboard focus trace and an axe run under the
production CSP, in addition to the existing screenshots. Every reported
violation is triaged; the recorded one-moderate-`region`-per-captured-state
finding is either resolved or explicitly accepted by the owner as a non-defect.

### Gap 3 - a stable rendered interrupted state

Accepted when `interrupted` is captured as a stable rendered installed state on
the current head (desktop and narrow screenshots, accessibility tree, keyboard
trace) and the rendered presentation agrees with the durable interrupt code
(`follow_up_requested`), rather than being observed only durably.

### Gap 4 - explicit full-criterion disposition

Accepted when the owner records an explicit P2-08 disposition on the evidence
produced by gaps 1-3: either full-criterion acceptance (promote P2-08) or a
written exclusion of any gap not evidenced. This is an owner decision and
cannot be produced by an agent or by CI.

## Non-goals and deferrals

- No change to the 10,000-observation contract, the `10,000 ms` warm-p95
  target, the `custom-9995`/seed `0` fixture, or the `repository-minimum`
  profile. F1/F2/F3 are unchanged and performance remains unqualified.
- No production activation: production scanning stays hidden.
- No scope exclusion for #47 (DriveFS) or #48 (FAT32/cross-volume). Local NTFS
  remains the accepted platform scope for this issue.
- No reopening of the accepted P2-01..P2-07, P2-09..P2-12 rows or the D2/D3
  policy-versus-defect decision. No product fix is implied unless the run finds
  a specific defect.
- No Phase 3 work and no parser work; the bounded Rust-parser spike is tracked
  separately.

## Data, privacy, and security impact

- Installed evidence uses synthetic fixtures only. No personal project paths,
  FLPs, audio, credentials, or unsanitized diagnostics are committed.
- Native authority, the SQLite schema, migrations, logging, and retention are
  unchanged; there is no migration or data-loss risk.
- Raw screenshots, logs, and database copies stay outside Git; only a redacted
  summary and hashes are committed.

## Accessibility and application states

- The issue adds the missing keyboard and axe coverage for `queued` and
  `running`, and a stable rendered `interrupted` capture.
- Existing states (loading, empty, error, offline, narrow, reduced motion) are
  unchanged and are not re-evidenced by this issue.
- The carried finding is one moderate `region` violation per captured state
  (content not contained by a landmark) with no critical or serious violations.

## Dependencies and sequencing

- Depends on the merged #121 run and the [2026-09-14 acceptance
  record](https://github.com/guilhermebmichelin-create/fruitboard/blob/main/docs/review/phase-2-integration/acceptance-2026-09-14.md).
- Produces the evidence required before any P2-08 promotion.
- Does not block the bounded Rust-parser spike.
- Optional to the recorded default: the residual disposition draft keeps #38
  (controls) and #40 (Library/list) open instead, and reassigns the gaps to
  this issue only if it is opened first.

## Privacy confirmation

- [x] This proposal contains no private project files, credentials, personal
  paths, or unsanitized diagnostics.
````

## Gap-to-acceptance mapping

Restating the four gaps exactly as carried in the acceptance record, each with
its acceptance wording and residual owner.

<!-- markdownlint-disable MD060 -->

| # | Carried gap (acceptance-2026-09-14)                                                              | Acceptance wording (accepted when...)                                                                                                                                                                       | Residual owner     |
| - | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| 1 | `access_denied`, `resource_limit`, `unsupported`, `worker_failed` typed presentations unproduced | One installed feature-enabled current-head run produces all four; durable code and rendered copy agree, `retryAvailable` is correct, prior committed results are preserved, and provenance is recorded.       | #38 controls       |
| 2 | No keyboard trace and no axe run for the transient `queued` and `running` states                 | Both transient states have a current-head installed keyboard trace and an axe run under the production CSP; every violation is triaged, and the one moderate `region` finding is resolved or accepted.       | #38 / #40          |
| 3 | `interrupted` observed only durably, with no stable rendered state                              | `interrupted` is captured as a stable rendered installed state (desktop, narrow, AX tree, keyboard trace) and agrees with the durable `follow_up_requested` code.                                            | #38 / #40          |
| 4 | Full-criterion P2-08 acceptance not promoted by the owner                                        | The owner records an explicit disposition on gaps 1-3: promote P2-08, or write an exclusion for any gap not evidenced.                                                                                      | Owner only         |

<!-- markdownlint-enable MD060 -->

The recorded minor finding carries with gap 2: axe reported exactly one moderate
`region` violation per captured state (content not contained by a landmark) and
no critical or serious violations. It is a later-work finding, not a claimed
accessibility defect.

## Non-goals of this proposal

- No budget, quota, fixture, target, or scope change. The 10,000-observation
  contract and the `10,000 ms` warm-p95 target are unchanged, and the
  100,000-entry scale remains an unmeasured proposal.
- No production activation; production scanning stays hidden.
- No promotion of P2-08 and no closure, edit, label, or comment on any issue or
  pull request.
- No scope exclusion for #47 (DriveFS) or #48 (FAT32/cross-volume).
- No Phase 3 issue or code, and no parser adapter work.

## What remains owner-only

- Creating, posting, labeling, and scheduling the proposed follow-up issue.
- Choosing between the dedicated issue and the recorded default of keeping #38
  and #40 open.
- The gap 4 full-criterion disposition, if and when gaps 1-3 are evidenced.
- Any reassignment of the residual from #38/#40 to a new issue.

## Checks and provenance

- This is a documentation-only task. No local build, fixture generation,
  installed run, benchmark, accessibility rerun, Cargo build, or Cargo cache was
  used.
- Applicable existing CI: Foundation CI `success` on `origin/main` `274d155`
  covers all unchanged code. No code change is proposed, so that result carries.
- Checks run for this file only: `pnpm.cmd lint:docs` scoped to the new file and
  `git diff --check`.
- Evidence sources are the files and issues named under "Purpose and source
  boundary"; issue states were observed read-only via `gh` at draft time.

## Build, storage, evidence, and cleanup handoff

- Build cache: none used, none created, none written. The primary development
  cache (`target/`) and the reusable temporary validation cache were not touched;
  this task owns neither, and their owners are unchanged.
- Capacity: the only working volume used is `C:`, measured at about 60.66 GiB
  free; the 30 GiB reserve holds. `G:` was measured at about 14.99 GiB free,
  below the reserve, and must not be used. Nothing was placed on `G:`.
- Evidence: the new file
  `docs/review/phase-2-integration/p2o8-followup-proposal-2026-09-15.md` inside
  the checkout is the only evidence. No binaries, databases, screenshots, or
  logs were produced; no external evidence location was used.
- Cleanup: no cleanup was performed and nothing was deleted. All worktrees,
  `target/`, untracked local-only files, and the Temp superseded-drafts backup
  were left untouched. Rebuild cost is zero.
- Owner decisions still required: whether to open the dedicated P2-08 follow-up
  (versus keeping #38/#40 open) and, separately, the gap 4 disposition.
