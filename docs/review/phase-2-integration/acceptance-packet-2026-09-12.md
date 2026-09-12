# Phase 2 closeout acceptance packet - 2026-09-12

Status: **coordination and decision input only; Phase 2 is not accepted.** This
is the current acceptance summary for P2-01 through P2-12. It does not merge a
pull request, post an issue message, close an issue, amend a budget or fixture,
change the S5 contract, activate production scanning, or grant owner
acceptance.

## Source review and document authority

The unpublished packet
`acceptance-packet-2026-09-10.md` was recovered from the existing
`agent1-phase2-closeout` worktree and reviewed before reuse. The older
`acceptance-prep-2026-09-08.md`, `continuation-2026-09-08.md`,
`checkpoint-2026-09-07.md`, and the dated reconciliation report remain
historical records. They are not rewritten here, and their old baselines,
open-PR descriptions, and planning checkboxes must not be read as current
GitHub state.

The active sources are now:

| Source | Authority in this closeout |
| --- | --- |
| [Live GitHub baseline](https://github.com/guilhermebmichelin-create/fruitboard) | Current branch, PR, issue, and CI state below |
| [Phase 2 execution plan](../../PHASE_2_EXECUTION_PLAN.md) | Binding contracts and provisional budgets; no silent budget change |
| [Reconciliation report](reconciliation-2026-09-08.md) | Preserved historical ledger and dated addenda |
| [This packet](acceptance-packet-2026-09-12.md) | Current evidence classes, decisions, coordination, and proposed updates |

The S5 regression and its preserved 2026-09-10 disposition are retained in
Agent 2's isolated branch. The September 12 driver, installed report,
checklist addendum, and index changes are now published there in draft PR
[#110](https://github.com/guilhermebmichelin-create/fruitboard/pull/110) at
head `e209b8a` (base S5 pin `19585da` plus one focused evidence commit). The
dirty main checkout's source copies remain untouched. The branch contents are
still evidence only: not merged and not owner-accepted.

Draft PR [#111](https://github.com/guilhermebmichelin-create/fruitboard/pull/111)
is at reviewed head `ef08522` with all ten required checks green. It is a mixed
product-and-evidence change, not an evidence-only PR: its local-NTFS report
records denied traversal, unchanged-bound `ResourceLimit`, and in-root
hardlink observations, while tested source `05fb35c` changes durable
product-owned diagnostics and desktop scan-console mapping for
`access_denied`, `resource_limit`, `unsupported`, and `unavailable`. The J
installed run used `05fb35c`; the current PR head is `ef08522`; neither is the
merged baseline `3ebac5f` or owner acceptance.

The #111 history also contains recovered S5 commit `de396e7`. It has the same
parent, tree, and stable patch ID as #110's `19585da`; it is preserved as
historical provenance, not counted as a second recovery experiment or duplicate
recovery commit. The isolated integration replay uses #110's `19585da` as the
canonical S5 patch and transplants only #111's post-recovery work.

The September 12 installed verification used reviewed commit `19585da`, whose
diff from `3ebac5f` is test/documentation only. The additive report records
queued state, queued disable/remove, and a genuine same-job restart recovery;
those observations are evidence only and are not yet merged or owner-accepted.

The separate #111 NTFS run used tested source `05fb35c153dd3f177b900292d39998da3774b5e4`
and is published at reviewed revision `ef085229471301043a50f4b668901d9c956fb407`;
the current PR head is `f8140349e436261bc95cc336e177f556bce19783`. That source
includes the focused product correction which persists fixed durable diagnostics
and maps them to typed desktop scan-console presentation; it is unmerged code,
not an evidence-only documentation change. The merged implementation boundary
remains `3ebac5f`, and owner acceptance remains a separate decision.

Draft PR [#112](https://github.com/guilhermebmichelin-create/fruitboard/pull/112)
is at `eace2e643f462e2cf2b8074ecd66007e2a21d074`. Its disposition is explicit:
the original `3ebac5f` current-main `Failed`/`Partial` artifact remains
unexplained and was not reproduced. It adds bounded, path-free
`partial_class` coverage and a sanitized benchmark protocol; later passes do
not establish the original cause or performance acceptance. It does not
change limits, budgets, safety checks, storage schema, or the S5 contract.

Agent 2 found no product or contract defect in the S5 follow-up and did not
rerun S5. Its local
validation passed storage tests (77), desktop feature-on unit tests (92),
desktop feature-on clippy, driver syntax, Markdownlint, privacy, and format
checks under Rust/Cargo `1.98.1`. The full desktop Cargo command reached 92
passing unit tests but failed its local Rustdoc phase with `E0463` missing
extern crates; `pnpm check` stopped at the unavailable Node `24.20.0`/`uv`
environment. GitHub PRs #110 (`e209b8a`) and #111 (`f8140349`) each have all
ten required checks green; #112's current head has successful Foundation and
Packaging Smoke runs. No acceptance or merge follows from these checks.

Agent 3's strict quiet-host preflight was completed fail-closed in draft PR
[#108](https://github.com/guilhermebmichelin-create/fruitboard/pull/108) at
`2561d3f`: DriveFS activity, an unverified Defender exclusion, sibling
activity, and missing 60-second CPU/disk idle proof prevented a qualifying
quiet-host run. The branch later published normal-configuration diagnostic
evidence at `a44653b`: before/candidate/current-main medians are 14,879 / 10,120
/ 14,982 ms and nearest-rank p95 values are 44,778 / 10,391 / 25,601 ms;
current-main has 9/10 authoritative iterations plus one retained partial
failure. The separate before, candidate, and current-main phase profiles
identify `next_entry` plus `read_metadata` at about 84% of scan wall time and
over 96% of timed filesystem-port time. These results are not quiet-host
qualification or acceptance evidence; the 10-second target, promotion, quota
changes, and historical classifications remain unchanged.

## Live GitHub baseline

The baseline was checked against GitHub before local work started:

| Item | Verified state |
| --- | --- |
| `origin/main` | `3ebac5f7a76c3425620ceba59e6078b32fb6cd85` (PR #91, merged 2026-09-10) |
| September 12 installed verification | `19585da7bef9ffdec06b71e0627df1b3f7ceb2f`, parent exactly `origin/main`; test/documentation-only diff |
| Foundation CI | [Run 34428758835](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34428758835), exact `3ebac5f`, success, all nine jobs |
| Foundation jobs | `docs-policy`, `client`, `rust-portable`, `migration`, `windows-foundation`, `security`, `filesystem-watcher-windows`, `enumeration-windows`, `scan-execution-windows` |
| Open pull requests | Draft #108 (`a44653b`, performance), #109 (closeout), #110 (`e209b8a`, restart/queued), #111 (`f8140349`, NTFS/product; reviewed report revision `ef08522`), and #112 (`eace2e6`, benchmark diagnostics); all remain unmerged |
| PR #99 | Closed, never merged; validation-only combined-build history at `9c211ad` |
| Open issues | #33, #36-#41, #47, #48, and #107 |

Main protection is unchanged. It still requires pull requests and ten named
contexts (`docs-policy`, `client`, `rust-portable`, `migration`,
`windows-foundation`, `security`, `windows-packaging-smoke`,
`enumeration-windows`, `filesystem-watcher-windows`, and
`scan-execution-windows`) with strict up-to-date branches. The nine-job
Foundation run above is not described as the ten-context protection result.

## Merged implementation and evidence classes

The following merged commits are provenance, not acceptance. The classification
is intentional:

| PR / merge commit | Classification and contribution |
| --- | --- |
| #82 / `05d39ff` | Merged watcher supervision, root mapping, reconnect, shutdown, and recovery implementation |
| #85 / `c05f4b6` | Merged durability, fault, atomicity, alias, and stale-publication implementation/tests |
| #90 / `f487aa1` | Merged blocker report for #47/#48; no platform qualification |
| #92 / `300c2a4` | Merged installed journey record; observations are from its recorded build and remain installed evidence, not current-head proof |
| #93 / `106335a` | Merged contended benchmark/profile evidence; not quiet-host qualification |
| #94 / `d342ec1` | Merged duplicate metadata-query cleanup; no end-to-end p95 pass established |
| #95 / `ee720af` | Merged Library actions/durable-state alignment |
| #96 / `326fb0a` | Merged bounded chunked-snapshot proposal only; not an approved scale design |
| #97 / `ff5d8ee` | Merged automated durable-queue and watcher-burst verification; not installed observation |
| #98 / `7e0e9f6` | Merged contended A/B validation and diagnostic cleanup; both sides miss the 10 s p95 target |
| #100 / `7980b75` | Merged packaging-smoke deadline/diagnostic fix |
| #101 / `67bdc76` and #102 / `892920d` | Merged historical installed/stall-failure records; not current success evidence |
| #103 / `55668da` | Merged exclusive installed-test lock helper |
| #104 / `cccaa67` | Merged queue-stall fix for failed retry while a root active slot is owned; automated regression passes |
| #105 / `f63a1d3` | Merged `smol-toml` 1.7.1 security remediation |
| #106 / `b2fb62c` | Merged canonical fixed installed report; S1-S4 and S6 pass, S5 is partial; docs-only |
| #91 / `3ebac5f` | Merged final reconciliation refresh; current live baseline, not a pending merge |

PR #99 is not a merge vehicle. Its green combined-build checks, like any
automated check, do not make its constituents safe or accepted. The Agent 2
branch `test/107-s5-restart-pin` at `19585da` is unpublished and is not listed
as a merged implementation.

## P2-01 through P2-12 evidence ledger

Installed records have their own provenance. The #92 journey was recorded on an
older merged baseline, and the #106 fixed validation tested the unmerged,
patch-identical candidate `ded02ac`, not `3ebac5f`. Neither is silently
relabeled as an installed run on the current head. The #110 queued/restart
record used `19585da`; the #111 NTFS record used unmerged product source
`05fb35c` and was published at `ef08522`. These tested-source, PR-head,
merged-code, and owner-acceptance boundaries remain separate.

| ID | Merged implementation | Automated tests/evidence | Installed evidence | Remaining acceptance |
| --- | --- | --- | --- | --- |
| P2-01 | #34/#35 picker, settings, and inline onboarding implementation | Client/Windows CI and #58 fake-adapter keyboard/narrow evidence | #92 picker cancel/selection, persistence, and restart; #54 picker record | Owner decision on inline onboarding and criterion acceptance |
| P2-02 | #59 reconciliation core, #64 enumeration, #70 worker | Deterministic reconciliation and Windows fixture tests | #92 add/modify/rename and remove/restore convergence | Platform scope and owner acceptance |
| P2-03 | #62/#70 safety path plus #85 fault cases | Partial/offline/cancel/resource-limit fault-injection tests; #111 adds focused diagnostic mapping tests | #92 cancellation and unavailable-root retention; #111 `ef08522` records local-NTFS denied traversal and unchanged-bound `ResourceLimit` passes, tested from unmerged `05fb35c` | Denied/limited traversal and owner acceptance |
| P2-04 | #60/#61/#70 durable state, #82 recovery, #104 active-slot fix | Migration/scan-execution gates, #97 convergence/stale fences, and #104 regression | #92 persistence/cancel/recovery; #101/#102 historical stall; #106 S1/S4/S6 pass and S5 successor path; #110 `e209b8a` queued/running/terminal states and genuine same-job restart recovery; #111 `de396e7` is tree-identical recovered S5 provenance, not new evidence | Installed S5 path disposition and owner acceptance |
| P2-05 | #60/#62/#85 lease, disable/remove, staging, and stale-publication paths | Lease, disable/remove, and publication tests | #92 disable-while-running retention; September 12 queued disable and queued remove cancel before publication | Installed evidence is pending review and owner acceptance |
| P2-06 | #62/#85 atomic publication, crash/rollback, backup, and migration paths | Crash, recovery, backup, and migration tests | #92 clean restart/interrupted recovery; #106 did not establish a genuine crash with a running lease; September 12 hard-kill copy proves a running lease, then same job/retry chain recovery | Correctly classified crash-recovery evidence and owner acceptance |
| P2-07 | #59/#62/#85 identity, alias, rename, and replacement paths | Alias and rename tests | #92 rename convergence; #111 `ef08522` true in-root hardlink-alias result on local NTFS, tested from unmerged `05fb35c` | #48 decision/qualification and owner acceptance |
| P2-08 | #73/#77/#78/#81/#82 IPC, native adapter, lifecycle, and supervision; #95 alignment | Feature-on CI and IPC/adapter/lifecycle/recovery tests; #104 regression; #111 diagnostic-mapping tests | #92 journey; #106 S1-S4/S6 installed observations; #110 `e209b8a` native/UI queued snapshot and queued invalidation; #111's unmerged `05fb35c` changes durable diagnostics and typed desktop error presentation; full criterion remains Partial | Full-criterion review remains Partial, not Complete |
| P2-09 | #68/#69/#71/#81/#82 watcher, coalescing, generation, and reconnect paths | #97 automated queue/burst/fence counts and watcher tests | #92 follow-ups; #106 isolated burst pass; real overflow timing and DriveFS remain unverified | Installed timing/overflow evidence, #47 decision, and owner acceptance |
| P2-10 | #81 static no-parser/no-content-I/O guards and fixture-byte equality | Dependency/privacy checks, static guards, and preservation tests | Native-adapter independence was observed; no runtime content-read spy was run | Owner chooses static boundary or runtime spy gate |
| P2-11 | #67/#72 harness, #86 rerun, #93 profile, #94 cleanup, #98 validation | F1 reproduced; historical contended warm p95 14,267 ms versus 10 s; Agent 3's strict quiet-host gate failed closed, while draft #108 `a44653b` records normal-config diagnostic medians 14,879 / 10,120 / 14,982 ms and p95 values 44,778 / 10,391 / 25,601 ms plus before/candidate/current-main profiles; current-main retains 9/10 authoritative iterations plus one `Failed`/`Partial` at 11,137 ms. #112 records the original Partial as unexplained and not reproduced, and adds bounded diagnostic coverage plus a sanitized protocol. Later passes do not establish its cause or performance acceptance; F3 remains unmeasured | No installed performance qualification | F1/F2/F3 decisions and any owner-approved remeasurement |
| P2-12 | #80 checkpoint plus merged implementation/CI map and #91 refresh | Current exact-head Foundation run is green; historical PR #99 remains validation-only | #92 and #106 records plus September 12 report, with exact `19585da` provenance preserved | Decisions above, then explicit P2-01-P2-12 owner acceptance |

The table deliberately does not use `Complete` as a synonym for “merged” or
“automated checks pass.” P2-08's older `Complete` wording is superseded by the
reconciled Partial disposition. No acceptance ID is promoted by this packet.

## S5: genuine crash recovery versus terminal follow-up

The #106 run must be read as two different state-machine paths:

1. It observed a process in `running`, but the harness had already sent a
   coalesced `scan_now` that returned `already_running`. That set
   `follow_up_requested`; the worker later finished the old run as terminal
   `interrupted` and created a queued successor.
2. `taskkill /F` happened after that terminal transition. After relaunch, the
   old job correctly stayed `interrupted` while the successor completed with
   committed rows. This is successor recovery from an already-terminal
   follow-up, not a crash recovery of a still-running lease.

Genuine crash recovery is different: restart finds a run still marked
`running`, `recover_interrupted_tx` marks that run interrupted with `restart`,
and the same job/retry chain is requeued for a fresh run. The existing
automated test `restart_requeues_the_interrupted_attempt_without_resetting_its_chain`
covers that state-machine rule. The #106 installed record does not prove that
path because its database copy had no running run before relaunch.

Agent 2's `19585da` pin replays the terminal follow-up path and asserts that the
old job is not resurrected and the successor is leaseable. It adds no product
fix. The September 12 installed record separately captures the genuinely
running-lease case: after an exact-PID hard kill, the same job and retry chain
reached attempt two, the old run became `interrupted/restart`, and a fresh run
completed with 8,000 published rows. It also records queued disable/remove
with `runId: null` before mutation. The report and driver are published in
draft PR #110, pending review and owner acceptance; they do not promote P2-04,
P2-05, P2-06, or P2-08.

Draft #111 repeats the recovered S5 tree in `de396e7`, but that commit has the
same parent, tree, and stable patch ID as `19585da`; it is not another recovery
run. The clean dependency strategy is to keep #110's `19585da` canonical,
drop `de396e7` when #111 is rebased, and replay #111's later harness, product,
and NTFS-report commits. Shared checklist/index files must be resolved by
unioning the #110 queued/restart rows with the #111 denied, ResourceLimit, and
hardlink rows. Empty formatting replays are skipped, while the historical SHAs
remain cited.

The #111 product portion is separately reviewable: tested source `05fb35c`
persists fixed worker/storage diagnostic codes and the desktop scan console maps
those durable codes to typed user-facing errors. It is not evidence-only and it
is not merged into `3ebac5f`; no owner acceptance follows from its green checks.

No additional S5 or NTFS run is requested for this stack. A future run would
require a newly identified evidence defect and separate owner authorization; it
must preserve the distinction between a terminal follow-up and a genuine
running-lease recovery, and must not invent a product/contract change.

## Ordered host windows

The host is a shared resource. Agent 3's strict quiet-host window is
non-qualifying, and its later normal-config measurements are diagnostic only;
the Agent 2 follow-up remains coordinated below:

| Order | Owner | Window and required controls |
| --- | --- | --- |
| 1 | Agent 3 | **Quiet-host gate failed closed at `2561d3f`; final normal-config evidence is published at `a44653b` / draft #108.** DriveFS, Defender verification, sibling activity, and idle-proof gates were ineligible for qualification. The later before/candidate/current-main timings and profiles are diagnostic only; no quiet-host claim or promotion follows. |
| 2 | Agent 2 | **Completed evidence follow-up at `e209b8a` / draft #110.** The existing S5 regression and September 12 installed artifacts are isolated and published; no product/contract defect was found and no S5 rerun was performed. Local focused checks pass; all ten required GitHub checks are green. Any future Agent 2 reproduction window takes exclusive priority over heavy validation. |
| 3 | Agent 1 / Agent 3 review | **Draft #111 at current head `f8140349` is the mixed NTFS/product follow-up.** Its J evidence used `05fb35c`; review the durable diagnostic persistence and typed desktop error mapping separately from the local-NTFS observations, then rebase it onto #110 without replaying `de396e7`. |
| 4 | Agent 2 independent review | **Review the frozen combined candidate after #112 is integrated.** Check that #111's durable diagnostic propagation remains intact alongside #112's `partial_class` reporting and sanitized benchmark protocol. This is a code/evidence gate only; it does not establish the original Partial's cause, performance acceptance, or Phase 2 acceptance. |

Before and throughout Agent 3's measurement, and during any Agent 2
reproduction window, pause heavy Rust, Tauri, Cargo, pnpm, packaging,
benchmark, and installed validation. Do not start a sibling build, installed
run, or other host-intensive task until the active window's samples and logs
are complete. If any quiet-host preflight fails, abort and record the window as
non-qualifying; do not relabel the contended data as quiet-host evidence.

Neither window may delete `storage/owner.lock`, kill an unrelated process, or
overwrite another worktree's smoke data. A lock acquisition failure is a
fail-closed result that records the foreign owner.

## Owner decisions prepared for the closeout

These are concrete choices, not decisions made by this packet.

### F1 fixture and F2 qualification

- F1-A: amend the baseline fixture definition to the exact 10,000 scanner
  observations represented by `custom-9995`, seed `0`, manifest
  `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a`.
  This is 9,995 FLP entries including the five hardlink primaries plus five
  alias locations; four non-FLP decoys are not scanner observations. Run the
  accepted protocol again after the owner records this fixture decision.
- F1-B: retain the accepted 10,000-FLP-plus-alias fixture and raise every
  coupled bound above 10,005, including worker observations, storage staging,
  path-byte checks, plan buffers, and corresponding tests, then remeasure.
  Raising only one constant is not a valid decision.
- F2: Agent 3's strict quiet-host gate failed closed, and draft #108's later
  normal-config timings remain diagnostic rather than qualification evidence.
  The historical 14,267 ms warm p95 is not rewritten. The current-main series
  also retains one `Failed`/`Partial` non-authoritative iteration at 11,137 ms.
  #112 records that the original Partial is unexplained and was not reproduced.
  Its bounded diagnostic coverage does not identify the original cause, and
  later passes do not establish performance acceptance. The owner must either
  authorize a new eligible quiet-host A/B or make a named host-class/target
  decision before any scoped optimization or remeasurement.

### F3 scale

Choose exactly one direction:

- bounded chunked snapshot based on merged proposal #96 (`326fb0a`), followed
  by design resolution for its six unresolved mechanisms and new correctness,
  cancellation, crash, memory, disk, and publication evidence;
- a coordinated increase to the 100,000-entry end-to-end limits, including
  worker, staging, path bytes, plan buffer, and publication/reconciliation
  fences, followed by private-memory, disk, latency, cleanup, and correctness
  measurement; or
- an explicit dated deferral that retains the current 10,000-entry contract
  and records that the 100,000-entry qualification remains unmeasured.

Merged #96 is a proposal, not authorization for any of these directions.

### Platform scope: #47 and #48

Do not repeat the unavailable-platform experiment. The merged blocker reports
already state what is missing:

- #47 lacks a Drive for Desktop Preferences capture of the active mode, a
  consented disposable synced location, and consent for cloud-synchronized
  create/rename/delete/cleanup plus pause/disconnect/resume. Both Mirror and
  Stream modes remain unverified; process or WMI metadata is not a mode
  capture.
- #48 lacks a genuine writable FAT32 volume. `C:` is NTFS; `D:` is a no-media
  RAW device; the system FAT32 partition has no drive letter; and `G:` is a
  DriveFS virtual mount whose WMI FAT32 report is not real FAT32 evidence. A
  disposable FAT32 USB volume or dedicated VHD, with drive letter, serial,
  filesystem, allocation unit size, and cleanup consent, is required.

For each issue the owner must authorize the exact prerequisite and run, or
post an explicit scope exclusion. Until then, DriveFS and non-NTFS identity
claims remain unverified; the NTFS-local implementation is not broadened by
inference.

### P2-10 runtime content-read boundary

The current evidence has static no-parser/no-content-I/O guards and fixture
source-byte equality. It does not include a runtime content-read spy. The owner
must choose either:

1. accept the static guards plus source-byte-equality boundary and document
   that runtime spying is not required for the filesystem-only MVP; or
2. require a runtime content-read spy gate, run it on the integrated
   filesystem-only path, and retain its implementation and log as P2-10
   evidence.

An installed native-adapter observation is not silently upgraded into a runtime
spy result.

### Inline Preferences onboarding

The implementation is merged and documented. The owner must either accept
inline Preferences onboarding as the Phase 2 entry path instead of a separate
first-run route, or defer the route decision to a named later phase. The
pending-decision note in the execution plan should be removed only after that
choice is recorded.

## Proposed issue updates (not posted)

These drafts are for the owner/maintainer. No GitHub message or issue mutation
was made by this task.

- **#33:** Keep the epic open. Link this packet after it is merged, state that
  implementation and evidence are present but F1/F2/F3, platform scope,
  P2-10, onboarding, S5 disposition, and owner acceptance remain open. Note
  that draft #111 is a mixed product-and-evidence change, not evidence-only.
- **#36, #37, #38, and #40:** Replace “implementation pending” wording with
  “implementation merged; acceptance evidence is classified in the 2026-09-12
  packet.” Keep each open until the owner accepts its criterion and any named
  installed/platform gap is resolved or explicitly scoped out.
- **#39:** Record that the filesystem-only/no-parser direction is implemented,
  then record the owner's static-boundary versus runtime-spy choice. Do not
  claim a runtime spy before it exists.
- **#41:** Keep the aggregator open and link the current packet, exact
  `3ebac5f` baseline, Foundation run `34428758835`, the published-but-unmerged
  `19585da` installed evidence, draft #108's fail-closed preflight and
  diagnostic `a44653b` normal measurements, draft #110's `e209b8a` Agent 2
  evidence head, draft #111's current `f8140349` NTFS/product head (reviewed
  report revision `ef08522`), and draft #112's `eace2e6` benchmark-diagnostic
  head. State that #112 leaves the original Partial unexplained and not
  reproduced, adds bounded diagnostic coverage, and does not establish its
  cause or performance acceptance. State that #111's installed J run used
  unmerged `05fb35c`, and that its durable diagnostic and desktop error-mapping
  changes need product review. Include the #110/#111 duplicate-S5 mapping, the
  #110 -> #111 -> #112 dependency, and the frozen combined-candidate Agent 2
  review. It is not a Phase 2 acceptance statement.
- **#47:** Request the missing DriveFS UI mode capture, disposable synced
  leaves, and cloud/pause-resume consent, or record an explicit owner scope
  exclusion. Do not report another blocked inventory as a run.
- **#48:** Request a genuine disposable FAT32 USB/VHD with recorded identity
  metadata and cross-volume consent, or record an explicit scope exclusion.
  Do not use DriveFS, the system partition, or the no-media device as a proxy.
- **#107:** Record that #106 demonstrated successor convergence after a
  terminal `follow_up_requested` transition, while the September 12 evidence
  separately demonstrates same-job recovery from a genuinely running lease.
  Agent 2 found no defect and published the isolated test/report artifacts in
  draft #110. Draft #111's `de396e7` is tree- and patch-identical S5 recovery
  provenance, not a second run; keep `19585da` canonical when rebasing the
  stack. No additional S5 or NTFS run is requested unless a specific new
  evidence defect is identified and separately authorized.
  Require a product/contract change only if review establishes a genuine
  defect.

No issue is closed, and no owner acceptance is inferred by these proposals.

## Dependency and merge order

Draft PRs #108, #109, #110, #111, and #112 are open. Their individual green
checks are not combined integration evidence. If the owner later advances the
stack, the safe sequence is:

1. Keep #108's `a44653b` performance record diagnostic. The quiet-host gate
   failed closed, the retained current-main `Failed`/`Partial` iteration is
   unexplained and was not reproduced by #112, and no target, quota, or
   acceptance claim follows.
2. Review and merge #110's `e209b8a` as the canonical queued/restart evidence
   carrier when the owner is ready. It contains `19585da`; do not treat its
   green checks or installed observations as owner acceptance.
3. Rebase #111 onto the resulting #110/main tip before merging it. Drop the
   tree- and stable-patch-identical recovered `de396e7`, retain its historical
   mapping to `19585da`, and replay the later harness, `05fb35c` product fix,
   and NTFS-report work. Resolve common checklist/index files by retaining all
   #110 queued/restart/disable/remove rows and adding #111's denied,
   ResourceLimit, and hardlink rows. Skip any now-empty formatting replay.
   Re-run the ten required checks on that rebased #111 head; the current
   `ef08522` checks are not combined evidence.
4. Rebase #112 onto the resulting #110+#111 product/evidence tip. Preserve
   #111's durable diagnostic propagation, add #112's bounded `partial_class`
   reporting and sanitized benchmark protocol, and run all ten required checks
   on the rebased #112 head. Do not treat the original Partial as explained or
   as performance acceptance.
5. Freeze the combined candidate SHA, publish it on the existing integration
   branch, and obtain an independent Agent 2 review against that exact SHA.
   Any follow-up edit must produce a new explicit SHA and repeat the review;
   no candidate SHA is implicit.
6. Rebase #109 onto the resulting merged baseline, refresh only live-head
   references and the reviewed candidate SHA, and merge the closeout packet
   last after explicit owner decisions. Any budget, fixture, platform,
   contract, or P2 acceptance change still requires its own decision and
   evidence.
7. Only after criterion-by-criterion owner acceptance consider a separate
   production-activation slice with default-feature, installed-smoke,
   capability, and rollback review.

## Cross-PR integration audit

The existing isolated checkout at
`%TEMP%\fruitboard-phase2-integration-20260912` started from verified
`origin/main` `3ebac5f7a76c3425620ceba59e6078b32fb6cd85`. Its prior synthetic
head `f7ebbf4e61ffef9501c4804763f1c6e469e6c486` replayed #110's canonical
`19585da` and #111's later harness, product, NTFS-report, and checklist/index
union work. #111's recovered `de396e7` was not replayed because it has the same
parent, tree, and stable patch ID as `19585da`; it remains historical
provenance.

The branch now cherry-picks #112's `eace2e6` as `879010d`. The result retains
#111's durable diagnostic propagation (`finish_scan_run_with_error` and its
closed durable vocabulary) while carrying #112's bounded path-free
`partial_class` through both worker paths and emitting only sanitized fixed
protocol fields. The checklist and installed-journey README retain the union
of #110's queued/restart/disable/remove entries and #111's denied,
`ResourceLimit`, hardlink, diagnostic, and report-index entries. No S5 or NTFS
experiment was rerun. The final candidate SHA is frozen only after these
metadata edits are committed and the existing branch is published; it is a
review candidate, not merged main or Phase 2 acceptance.

The pre-#112 combined validation at `f7ebbf4` is historical for #110/#111 and
does not validate #112. Fresh ten-context checks and an independent Agent 2
review are required on the frozen candidate. Any later candidate edit must
produce a new explicit SHA and repeat that review.

This packet itself is documentation/coordination only. It does not merge,
post, close, accept, or activate anything.

## Validation and non-actions

This packet branch is based on verified `origin/main` and the combined
candidate is the separate existing integration checkout. The results below
are the pre-#112 validation record; they are not a pass for the current
candidate:

- `git diff --check`: PASS.
- `pnpm.cmd lint:docs`: PASS (72 Markdown files, 0 issues).
- `node scripts/verify-repository-privacy.mjs`: PASS (279 files).
- `pnpm.cmd format:check`: PASS (Prettier and `cargo fmt --all --check`; the
  command was run with the installed Cargo bin directory on `PATH`).
- The package scripts reported the host's Node `26.4.0` versus the repository
  pin `24.20.0`; this is a docs-validation environment warning, not product
  or performance evidence.
- Agent 2's isolated branch passed `cargo test -p fruitboard-storage
  --locked --lib` (77), `cargo test -p fruitboard-desktop --features
  scan-console --locked --lib` (92), and feature-on desktop clippy; the full
  desktop doctest phase failed locally with `E0463` after the unit tests passed.
- GitHub draft PR #108 at `a44653b`, #110 at `e209b8a`, and #111 at
  `f8140349` each have all ten required checks green; #112 `eace2e6` has
  successful Foundation and Packaging Smoke runs, but its full ten-context
  result and the combined candidate checks are required after publication.
  Draft PR #109 is this documentation branch; its updated packet-only head
  remains subject to the normal required checks.
- No full build, installed run, performance run, issue mutation, merge, Phase
  2 acceptance, or production activation was performed by this documentation
  closeout. Draft PR #109 was opened for review; it is not a merge or
  acceptance action.
