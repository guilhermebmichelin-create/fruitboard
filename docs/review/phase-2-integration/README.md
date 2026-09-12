# Phase 2 integration evidence index (#41)

Status: **current summary verified 2026-09-12; Phase 2 is not accepted.** This
index separates merged implementation, automated tests, installed observations,
and owner acceptance. The [current acceptance packet](acceptance-packet-2026-09-12.md)
owns the closeout decisions and coordination. The dated checkpoint,
reconciliation, benchmark, and journey reports remain historical evidence and
are not rewritten here.

## Live baseline

| Item | Current state |
| --- | --- |
| `main` | `3ebac5f7a76c3425620ceba59e6078b32fb6cd85` (PR #91) |
| Foundation CI | [Run 34428758835](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34428758835), exact current head, success, all nine Foundation jobs |
| Open pull requests | Draft #108 (`a44653b`, performance), #109 (closeout), #110 (`e209b8a`, restart/queued), and #111 (`f8140349`, NTFS/product; reviewed report revision `ef08522`); all remain unmerged |
| PR #99 | Closed without merge; validation-only combined-build history |
| Open issues | #33, #36-#41, #47, #48, #107 |

The ten required branch-protection contexts remain unchanged. The current
Foundation run has nine jobs; `windows-packaging-smoke` is a separate required
context and is not folded into the nine-job count.

## Merged contributions

These records establish provenance, not owner acceptance:

| PR / commit | Classification |
| --- | --- |
| #82 / `05d39ff` | Watcher supervision, root mapping, reconnect, shutdown, and recovery implementation |
| #85 / `c05f4b6` | Durability, fault, atomicity, alias, and stale-publication implementation/tests |
| #90 / `f487aa1` | #47/#48 blocker report; no platform qualification |
| #92 / `300c2a4` | Installed journey record from its recorded baseline |
| #93 / `106335a` | Contended benchmark/profile evidence |
| #94 / `d342ec1` | Duplicate metadata-query cleanup |
| #95 / `ee720af` | Library/durable-state alignment |
| #96 / `326fb0a` | Bounded chunked-snapshot proposal only |
| #97 / `ff5d8ee` | Automated durable-queue/watcher-burst verification |
| #98 / `7e0e9f6` | Contended A/B validation; no 10 s p95 qualification |
| #100 / `7980b75` | Packaging-smoke deadline and diagnostics fix |
| #101 / `67bdc76`, #102 / `892920d` | Historical installed/stall-failure records |
| #103 / `55668da` | Exclusive installed-test lock helper |
| #104 / `cccaa67` | Queue-stall fix and automated regression |
| #105 / `f63a1d3` | `smol-toml` 1.7.1 security remediation |
| #106 / `b2fb62c` | Canonical fixed installed report; S5 partial; docs-only |
| #91 / `3ebac5f` | Final reconciliation refresh and current baseline |

PR #99 (`9c211ad`) is closed and never merged. The Agent 2 branch
`test/107-s5-restart-pin` at `19585da` is unpublished and is not merged
evidence. A merged commit, green automated check, or installed observation does
not by itself provide owner acceptance.

## Reviewed but unmerged heads

| PR / head | Classification |
| --- | --- |
| #108 / `a44653b` | Agent 3 normal-configuration performance diagnostics; the strict quiet-host preflight failed closed and no qualification is claimed |
| #109 | This closeout packet and coordination index |
| #110 / `e209b8a` | Agent 2 queued/running/terminal, queued disable/remove, and genuine same-job restart evidence; no product fix |
| #111 / `f8140349` | Mixed product-and-evidence change: local-NTFS denied traversal, unchanged-bound `ResourceLimit`, and in-root hardlink evidence, plus the durable-diagnostic and desktop scan-console correction at tested source `05fb35c`; reviewed report revision `ef08522`; it is not evidence-only |

The #111 branch also carries recovered S5 commit `de396e7`. Its tree and
stable patch ID are identical to #110's `19585da`, so it is historical
provenance, not a second recovery result. The local integration replay treats
PR #110's `19585da` as the canonical recovery patch and does not duplicate it.

## P2-01 through P2-12

The #92 installed journey was recorded on an older merged baseline. The #106
fixed validation tested unmerged candidate `ded02ac`, patch-identical to the
fix in #104, not the current `3ebac5f`. The September 12 queued/restart report
was verified from `19585da`, whose diff from `3ebac5f` is test/documentation
only. The queued-state, queued disable/remove, and genuine same-job restart
observations are published in draft #110 at `e209b8a`, but remain unmerged and
owner-unaccepted. The separate NTFS report in draft #111 was published at
reviewed revision `ef08522`, and the current head is `f8140349`; it was tested
from unmerged source `05fb35c`. That source changes durable
product-owned diagnostics and desktop scan-console mapping for `access_denied`,
`resource_limit`, `unsupported`, and `unavailable`. The installed observations
and that product correction must not be relabeled as merged-main evidence or
as evidence-only documentation.

| ID | Merged implementation | Automated evidence | Installed evidence | Remaining |
| --- | --- | --- | --- | --- |
| P2-01 | #34/#35 picker, settings, and inline onboarding | Client/Windows CI; #58 fake-adapter keyboard/narrow evidence | #92 picker, cancel, persistence, restart; #54 picker record | Onboarding decision and owner acceptance |
| P2-02 | #59 core, #64 enumeration, #70 worker | Deterministic and Windows fixture tests | #92 add/modify/rename and remove/restore | Platform scope and owner acceptance |
| P2-03 | #62/#70 safety path and #85 fault cases | Partial/offline/cancel/resource-limit fault tests | #92 cancellation and unavailable-root retention; #111 `ef08522` local-NTFS denied traversal and unchanged-bound `ResourceLimit` pass, tested from unmerged `05fb35c` | Denied/limited traversal and owner acceptance |
| P2-04 | #60/#61/#70 durability, #82 recovery, #104 active-slot fix | Migration/worker gates; #97 fences; #104 regression | #92 persistence/cancel/recovery; #106 S1-S4/S6 pass, S5 successor path; #110 `e209b8a` queued/running/terminal states and genuine same-job restart; #111 `de396e7` is tree-identical recovered S5 provenance, not new evidence | S5 disposition and owner acceptance |
| P2-05 | #60/#62/#85 lease, disable/remove, staging | Lease and stale-publication tests | #92 disable-while-running retention; September 12 queued disable/remove | Evidence review and owner acceptance |
| P2-06 | #62/#85 atomic publication and crash/backup paths | Crash, recovery, backup, migration tests | #92 restart/recovery; September 12 genuine running-lease hard-kill recovery | Correct crash-path review and owner acceptance |
| P2-07 | #59/#62/#85 identity, alias, rename paths | Alias and rename tests | #92 rename; #111 `ef08522` true in-root hardlink alias result, local NTFS only, tested from unmerged `05fb35c` | #48 decision/qualification |
| P2-08 | #73/#77/#78/#81/#82 UI/native/lifecycle; #95 alignment | Feature-on CI and IPC/adapter/lifecycle tests; #111 tests durable diagnostic mapping | #92 journey; #106 S1-S4/S6; #110 `e209b8a` native/UI queued snapshot and invalidation; #111's unmerged `05fb35c` changes durable diagnostics and typed desktop error presentation | Full criterion remains Partial |
| P2-09 | #68/#69/#71/#81/#82 watcher/coalescing/reconnect | #97 automated queue/burst/fence tests | #92 follow-ups; #106 isolated burst pass | Overflow timing and #47 scope |
| P2-10 | #81 static no-parser/content-I/O guards and fixture equality | Dependency/privacy, static, preservation checks | Native-adapter independence only; no runtime spy | Static-versus-runtime decision |
| P2-11 | #67/#72/#86/#93/#94/#98 benchmark work | F1 reproduced; contended warm p95 14,267 ms vs 10 s; #108 normal-config current-main retains 9/10 authoritative iterations plus one `Failed`/`Partial` at 11,137 ms; Agent 3's quiet-host window failed closed; F3 unmeasured. This Partial is an explicit open finding pending Agent 2's disposition; #111 does not relabel or dispose it | No installed performance qualification | F1/F2/F3 decisions |
| P2-12 | #80 checkpoint, merged map, #91 refresh | Run 34428758835 passes all nine Foundation jobs | #92/#106 plus September 12 report with `19585da` provenance | Decisions, then explicit owner acceptance |

P2-08's historical `Complete` wording is not carried forward: the current
disposition is Partial because the full installed/owner boundary was not
established. No P2 acceptance ID is promoted here.

## S5 disposition

The #106 record observed a running process, but the harness had already issued
an `already_running` coalesced follow-up. The old run therefore finished as
terminal `interrupted/follow_up_requested` before `taskkill`; after relaunch,
the queued successor completed and the old job stayed interrupted. That is the
expected terminal-follow-up path.

A genuine crash is the separate case where restart finds a run still marked
`running`; recovery marks it `restart` and requeues the same job/retry chain.
The current automated state-machine test covers that rule. Agent 2's
`19585da` pin covers the terminal-follow-up path and adds no product fix. The
September 12 installed report separately records a genuinely running lease at
exact-PID kill, same-job/retry-chain recovery at attempt two, queued
disable/remove with `runId: null`, and terminal queued/running states. Those
artifacts are isolated in draft #110 at `e209b8a`; Agent 2 found no defect and
no contract change is proposed. Draft #111's `de396e7` has the same parent,
tree, and stable patch ID as `19585da`, so the integration plan retains only
one recovery patch while preserving both historical SHAs. Do not request
another S5 run without a specific evidence defect.

## Remaining decisions and coordination

The owner decisions are collected in the [closeout packet](acceptance-packet-2026-09-12.md):

- F1: use the exact 10,000-observation `custom-9995` fixture, or raise all
  coupled limits above 10,005 and remeasure.
- F2: run a quiet-host A/B before any target or budget change; retain the
  contended result as diagnostic. The #108 current-main series retains one
  `Failed`/`Partial` non-authoritative iteration at 11,137 ms; this is an
  explicit open finding pending Agent 2's disposition, not a pass and not
  something #111's diagnostic fix disposes.
- F3: choose bounded chunked snapshots, a coordinated 100,000-entry limit
  increase, or a dated deferral retaining the 10,000-entry contract.
- #47: supply DriveFS UI mode capture, disposable synced leaves, and cloud/
  pause-resume consent, or explicitly exclude the modes.
- #48: supply a genuine disposable FAT32 USB/VHD with recorded identity, or
  explicitly exclude FAT32/cross-volume support. DriveFS and system partitions
  are not substitutes.
- P2-10: accept static guards plus fixture equality, or require and run a
  runtime content-read spy.
- Inline onboarding: accept inline Preferences as the Phase 2 entry path, or
  defer the route choice to a named later phase.

Agent 3's first quiet performance window failed closed before measurement in
draft #108 at `2561d3f`; no quiet-host qualification was claimed. Its later
normal-config current-main series retains the `Failed`/`Partial` finding above,
which remains open pending Agent 2's disposition. Agent 2's follow-up is
isolated in draft #110 at `e209b8a`, with no S5 rerun and no product defect
identified. No heavy Cargo, Rust, Tauri, pnpm, packaging, benchmark, or
installed validation was run during Agent 2's reproduction window. Prebuild
artifacts outside a measurement window and pause those heavy tasks throughout
any future measurement. Do not overlap windows or relabel a failed quiet
preflight as qualification. Do not request another S5 run without a specific
evidence defect.

## Standing gates

- #47/#48 remain unverified until their prerequisites exist or the owner makes
  an explicit scope decision.
- Historical reports retain their original baselines and outcomes; they are
  not current-state claims.
- Production scanning remains hidden until P2-03 through P2-08 have integrated
  evidence and the owner accepts Phase 2.
- The epic and child issues remain open. No message, merge, closure, acceptance,
  or production scanning action is authorized by this index.
