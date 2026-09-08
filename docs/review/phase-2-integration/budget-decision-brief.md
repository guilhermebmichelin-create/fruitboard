# Budget + governance decision brief — Phase 2 integration (Wave 4 / Agent D)

Date: 2026-09-07. Base: `origin/main` `74203a2`. Branch: `chore/owner-decisions-packet`.
Scope: verification + this brief only. No code changes. PR #74 was **not**
merged by this agent (see §1).

Related: issue #66 (proposal, MERGED) → PR #74 (enforcement + Dependabot
scoping); Dependabot PRs #27 (`@types/node`) and #28 (`typescript`), both
CLOSED; benchmark `benchmark-2026-09-07.md` + `-baseline-raw.json` +
`-quota-raw.json` (P2-11 evidence for #41); budgets in
`docs/PHASE_2_EXECUTION_PLAN.md` (provisional, owner-accepted).

Status: **decision packet**. Nothing in this brief amends a budget, a quota,
a fixture definition, or a GitHub setting. Every amendment below needs an
explicit owner sentence from §5.

## 1. Branch protection (#66 → #74) — verified ENABLED

PR #74 (`chore: enable enforced branch protection and scope dependabot
majors`, `chore/branch-protection-enabled` → `main`) was already
**MERGED** when this packet ran: merged at `2026-09-07T23:44:03Z` by
`guilhermebmichelin-create`, merge commit `1b6f65e`. This agent did not
merge it.

### 1.1 Verification commands (PowerShell-safe) and observed output

```powershell
gh pr view 74 --json number,title,state,baseRefName,headRefName,url
gh pr view 74 --json files --jq ".files[].path"
gh pr view 74 --json statusCheckRollup
gh api repos/guilhermebmichelin-create/fruitboard/branches/main/protection
gh api repos/guilhermebmichelin-create/fruitboard/branches/main/protection/required_status_checks --jq .
gh api repos/guilhermebmichelin-create/fruitboard/branches/main/protection/enforce_admins --jq .
gh api repos/guilhermebmichelin-create/fruitboard/branches/main/protection/required_pull_request_reviews --jq .
gh api repos/guilhermebmichelin-create/fruitboard --jq "{allow_squash_merge, allow_merge_commit, allow_rebase_merge, visibility}"
```

Observed (2026-09-07 run):

- `gh pr view 74`: `number 74`, `state MERGED`, `baseRefName main`,
  `headRefName chore/branch-protection-enabled`.
- Files in #74: `.github/dependabot.yml`,
  `.github/workflows/windows-packaging-smoke.yml`, `DEVELOPMENT.md`,
  `README.md`, `ROADMAP.md`, `SECURITY.md`, `tests/packaging-policy.test.mjs`.
- `statusCheckRollup`: 10 CheckRuns, all `COMPLETED / SUCCESS`:
  `docs-policy`, `client`, `rust-portable`, `migration`,
  `windows-foundation`, `security`, `windows-packaging-smoke`,
  `enumeration-windows`, `filesystem-watcher-windows`,
  `scan-execution-windows`.
- `branches/main/protection.required_status_checks`: `strict: true`
  (branches up to date), exactly the 10 contexts above.
- `required_pull_request_reviews`: `required_approving_review_count: 0`,
  `dismiss_stale_reviews: false`, `require_code_owner_reviews: false`,
  `require_last_push_approval: false`.
- `enforce_admins.enabled: true` (include-admins).
- `required_linear_history.enabled: true`.
- `allow_force_pushes.enabled: false`; `allow_deletions.enabled: false`.
- `required_conversation_resolution.enabled: false`;
  `required_signatures.enabled: false`; `lock_branch.enabled: false`.
- Repo: `allow_squash_merge: true`, `allow_merge_commit: true`,
  `allow_rebase_merge: true`, `visibility: public`.

### 1.2 Requirement matrix

| Requirement | Expected | Observed | Verdict |
| --- | --- | --- | --- |
| Required PRs before merge | yes | PRs required (protection present, reviews object exists) | PASS |
| 10 required checks, strict | `docs-policy`, `client`, `rust-portable`, `migration`, `windows-foundation`, `security`, `windows-packaging-smoke`, `enumeration-windows`, `filesystem-watcher-windows`, `scan-execution-windows`, branches up to date | exact 10 contexts, `strict: true` | PASS |
| Squash / linear | squash convention for one-concept PRs + linear history | `required_linear_history: true`; `allow_squash_merge: true` (see note) | PASS with note |
| No force-push | disallow | `allow_force_pushes: false` | PASS |
| No deletion | disallow | `allow_deletions: false` | PASS |
| Include-admins | enforce on admins | `enforce_admins: true` | PASS |
| Zero approvals | `required_approving_review_count: 0` (single-maintainer; self-approval unsatisfiable) | `0` | PASS |

Squash note: GitHub has no "squash-only" branch-protection flag; it is a
repository setting. `allow_squash_merge` is `true`, but `allow_merge_commit`
and `allow_rebase_merge` are also `true`, so squash is a documented
convention (`DEVELOPMENT.md` "Require squash merges for one-concept PRs"),
not a GitHub-enforced merge-button lock. Linear history **is** enforced.
If the owner wants squash-only, that is a separate repo-settings decision.

Wording check (#74 vs pre-#74 base `74203a2`): `DEVELOPMENT.md` replaces
"Proposed enforced branch protection (pending owner approval)" with
"Enforced branch protection / Enabled on 2026-09-07 by owner decision",
lists the 10 checks above, records zero-approval rationale, and keeps
owner-only manual merge. `README.md`, `ROADMAP.md`, `SECURITY.md` drop the
"pending proposal" phrasing and point at the enforced section; the
packaging smoke trigger drops its `paths:` filter so the required check
reports on every PR (with the `packaging-policy` test updated to assert
`doesNotMatch /paths:/`). All verified via `gh pr diff 74` and
`git show origin/main:<file>`.

### 1.3 Contingency — exact steps if protection were NOT enabled (do not run now)

Protection **is** enabled; this subsection is the owner runbook only:

- Click path: repository → Settings → Branches → Branch protection rules →
  `main` → Require a pull request before merging; Require status checks to
  pass before merging (+ Strict: branches up to date) and add the 10 checks
  above; Require linear history; Do not allow bypassing the above settings
  (Include administrators); leave required approvals at 0 for the
  single-maintainer reason; block force pushes and deletions.
- API equivalent (`gh api`, one example per area; owner token with admin):
  `PUT repos/{owner}/{repo}/branches/main/protection` with
  `required_status_checks: {strict: true, contexts: [<10 above>]}`,
  `enforce_admins: true`, `required_pull_request_reviews: null`-equivalent
  (`required_approving_review_count: 0`),
  `required_linear_history: true`, `allow_force_pushes: false`,
  `allow_deletions: false`, `required_conversation_resolution: false`.

## 2. Dependabot #27 / #28 (both CLOSED) — #74 scoping verified

| PR | Title | State | Closed |
| --- | --- | --- | --- |
| #27 | `deps: bump @types/node from 24.13.3 to 26.4.1` | CLOSED | 2026-09-07T18:08:43Z |
| #28 | `deps: bump typescript from 6.0.3 to 7.0.2` | CLOSED | 2026-09-07T18:08:47Z |

PR #74 adds exactly the requested scoping to `.github/dependabot.yml`
(verified via `gh pr diff 74` and `git show origin/main:.github/dependabot.yml`):

```yaml
    # @types/node must track the pinned Node 24.20.0 runtime (DEVELOPMENT.md
    # toolchain policy); TypeScript 7 is postponed until typescript-eslint
    # supports it and the Phase 2 checkpoint has passed (owner decision,
    # 2026-09-07).
    ignore:
      - dependency-name: "@types/node"
        versions: [">=25"]
      - dependency-name: "typescript"
        versions: [">=7"]
```

Rationale verified on base `74203a2`: Node pin is `24.20.0 LTS`
(`tools/toolchain-policy.json`, `package.json` engines,
`DEVELOPMENT.md` toolchain table); TypeScript pin is `6.0.3` with
`typescript-eslint 8.69.0` in `pnpm-lock.yaml` (no TS7 support claimed).
Local `pnpm.cmd lint:docs` even warns the shell runs Node `v26.4.0`
against the `24.20.0` want — evidence the major drift is real, not
theoretical. The agent did not reopen, recreate, or comment on #27/#28.

### 2.1 Exact owner comment drafts (post as-is; agent posts nothing)

PR #27 — recreate scoped to the pinned 24.x line:

```text
Owner decision 2026-09-07: @types/node must track the pinned Node 24.20.0
runtime (see DEVELOPMENT.md toolchain policy and tools/toolchain-policy.json).
@dependabot ignore @types/node major versions >=25 is now configured, so this
26.x bump stays closed.

@dependabot recreate with @types/node 24.x — please open a 24.x-only update
if one is available; 25+ will remain ignored until the Node pin itself moves
in a focused toolchain PR.
```

PR #28 — postpone TS7 until tooling + checkpoint:

```text
Owner decision 2026-09-07: TypeScript 7 is postponed until typescript-eslint
supports TS7 and the Phase 2 checkpoint has passed. `typescript>=7` is now in
the Dependabot ignore list, so this 7.0.2 bump stays closed with no action.

Revisit after the Phase 2 checkpoint in a focused toolchain PR (regenerate
locks, run the full gate, update DEVELOPMENT.md + toolchain policy together).
Do not reopen here.
```

## 3. F2 warm-p95 miss triage (benchmark 2026-09-07)

Source: `docs/review/phase-2-integration/benchmark-2026-09-07.md` (Run B,
quota-fitting fixture `custom-9995`: 9,995 FLP + 5 hardlink aliases =
10,000 observations, seed `0`, manifest
`a4760a28…6d08a`) + `benchmark-2026-09-07-quota-raw.json`. Recomputed here
with `node -e` over `iterations[].scanMs` (n = 10, nearest-rank p95 = max
by construction at n = 10).

### 3.1 Recomputed statistics (matches report)

| Metric | Value |
| --- | --- |
| Sorted scan windows (ms) | 11686, 11911, 12294, 12710, 12833, 12918, 12924, 23528, 31304, 32876 |
| Median | 12875.5 ms (~12.9 s) |
| Max | 32876 ms (~32.9 s) |
| Nearest-rank p95 | 32876 ms (ceil(0.95 × 10) = 10th value = max) |
| Warm-up first discovery | 11886 ms (~11.9 s), Published / Complete / 10,000 locations |
| Budget `warm-p95 <= 10 s` | measured 32876 ms → **FAIL (F2)** |
| Budget `first discovery <= 30 s` | measured 11886 ms → PASS |
| Cooperative stop p95 `<= 1 s` | max 8 ms over 6 samples → PASS (wide margin) |
| Incremental memory `<= 128 MiB` | ~53 MiB on the 10k set → PASS with set caveat (F3) |

Iteration shape: iterations 1–4 are `32876, 12710, 23528, 31304` (three of
the first four at 23.5–32.9 s); iterations 5–10 are stable at
`11686–12924` ms. Warm-up (11.9 s) sits inside the stable band, so the
spike is not a first-run effect. Run A (accepted `baseline` preset) never
evaluates latency: all 11 runs end `Failed / ResourceLimit` at the quota
boundary (10.5–21.8 s to failure) — see F1.

### 3.2 Hypothesis ranking (no budget amended)

| Rank | Hypothesis | Evidence for / against |
| --- | --- | --- |
| (a) Background contention — first | Report machine profile: active developer laptop (i7-10750H, 2019), Google DriveFS sync service, Defender real-time scanning, agent tooling; fixture freshly generated so DriveFS indexing + AV plausibly hit the first iterations hardest; early variance (11.7–32.9 s) collapsing to a stable 11.7–12.9 s band fits transient contention, and `sampleCount` (237/215/267 on slow iters vs ~111–121 stable) shows longer windows, not a different code path | For: timing shape + documented load. Against: not isolated (no idle rerun, no per-phase breakdown) |
| (b) Per-file handle + ancestor-validation cost on laptop hardware — second | Production path per report: handle-bound NTFS traversal, `NtCreateFile` per entry + ancestor validation chain per directory over ~10k files / 1,025 dirs; even the stable band (median 12.9 s) exceeds the 10 s budget by ~29%, so a structural per-file cost remains after contention fades | For: stable band still misses. Against: no profiling split (enumerate vs stage vs commit) to quantify |
| (c) `synchronous = FULL` per-batch commit — third | 512-record hard batch bound × ~20 batches per 10k run multiplies any per-commit fsync cost; plausible contributor, listed third in the report | For: mechanism exists. Against: zero direct evidence in this wave (no `NORMAL` vs `FULL` comparison, no commit timing) |

### 3.3 F1 (blocks everything) and F3 (100k unmeasurable)

- **F1**: accepted `baseline` preset = 10,000 FLP files + 5 hardlink
  aliases = **10,005 observations** vs durable staging quota
  `MAX_STAGED_RECORDS = 10,000`. All Run A iterations stop at
  `ResourceLimit`, staging discarded, nothing published. Enforcement
  behaved as designed; the counts did not account for aliases as separate
  locations. Owner must either raise the quota above 10,005 or redefine
  the fixture as exactly 10,000 observations including aliases. The plan
  row "Baseline fixture 10,000 FLP-named files" is invalid as literally
  implemented.
- **F3**: the 100,000-entry qualification set cannot complete under the
  10,000-record quota, so the `<= 128 MiB on the 100k set` budget is
  **unmeasurable today**. The ~53 MiB figure is a 10k-set measurement with
  that caveat. The 100k set stays unmeasurable until the quota changes.

### 3.4 Owner options (WITHOUT amending budgets in this packet)

| Option | Action | Cost / consequence |
| --- | --- | --- |
| A | Raise `warm-p95` for laptop-class, and/or add a desktop-class host class | Accepts current speed on this laptop; requires plan amendment + host definition; does not fix F1 |
| B | Optimize the handle/metadata path (batch opens, ancestor-chain caching, commit batching) then re-measure | Real fix candidate; engineering cost; needs profiling first; does not fix F1 |
| C | Re-run on an idle host with cache control (kill DriveFS sync pause, AV exclusion for fixture volume, no agent load, fixed power, documented cache state) | Cheapest isolator for hypothesis (a); if stable band drops under 10 s, contention was decisive; still leaves F1 |
| D | Fix F1 first: raise quota above 10,005 or define the fixture as exactly 10k including aliases | Prerequisite for any authoritative baseline/100k claim; quota change needs owner amendment; do D before judging A–C on the accepted fixture |

Recommendation: **D first** (nothing authoritative can be claimed on the
accepted fixture until the quota/fixture contradiction is resolved),
**then C** (cheap, isolates the leading hypothesis), **then** decide
between A (re-budget/host-class) and B (optimize) on clean data. Do not
re-budget on contended numbers.

## 4. Budget table (current vs measured — no amendments made)

| Budget (plan) | Target | Measured 2026-09-07 | Verdict |
| --- | --- | --- | --- |
| Baseline first discovery | <= 30 s | 11886 ms (quota-fitting 10k set; accepted set not evaluable) | PASS with fixture caveat |
| Unchanged warm reconciliation p95 | <= 10 s | p95 32876 ms, median 12875.5 ms | FAIL (F2) |
| Cooperative worker stop p95 | <= 1 s | max 8 ms (6 samples) | PASS |
| Incremental private memory | <= 128 MiB on 100k set | ~53 MiB on 10k set | PASS with set caveat (F3) |
| Batch / staging | <= 512 records / <= 256 MiB | 512-record bound observed; quota enforcement demonstrated by F1 | No violation observed; not directly observable via worker API |

## 5. Exact owner approval sentences (copy-paste; each amends the plan only when posted)

Post the sentence(s) for the decision(s) taken. Each is effective only as
an owner post; this brief amends nothing.

```text
F1-quota: I approve amending docs/PHASE_2_EXECUTION_PLAN.md to raise the
durable staging quota (MAX_STAGED_RECORDS / WorkerConfig::default
max_observations) from 10,000 to [OWNER FILLS: e.g. 10,500], because the
accepted baseline fixture produces 10,005 observations (10,000 FLP files +
5 hardlink aliases) and every Run A iteration ends ResourceLimit. Record the
new quota, the alias-counting rule, and the re-measurement requirement.
```

```text
F1-fixture (alternative to F1-quota, pick one): I approve amending
docs/PHASE_2_EXECUTION_PLAN.md to define the baseline fixture as exactly
10,000 observations including hardlink aliases (e.g. 9,995 FLP files + 5
aliases, seed 0), instead of 10,000 FLP-named files plus aliases. The
"10,000 FLP-named files" row is superseded by this definition.
```

```text
F2-budget: I approve amending docs/PHASE_2_EXECUTION_PLAN.md warm
reconciliation budget from p95 <= 10 s to [OWNER FILLS: value + host class,
e.g. p95 <= 15 s on laptop-class i7-10750H / <= 10 s on desktop-class],
citing benchmark-2026-09-07 (median 12875.5 ms, p95 32876 ms, n = 10) as
evidence. No code change is authorized by this sentence alone.
```

```text
F2-host (alternative or companion to F2-budget): I approve adding a
desktop-class reference host to docs/PHASE_2_EXECUTION_PLAN.md and
re-measuring the warm-p95 budget there before any re-budget; the
laptop-class result (p95 32876 ms) stands as reported and is not rewritten.
```

```text
F2-rerun: I approve an idle-host re-run per benchmark-2026-09-07 §Limitations
(DriveFS paused, AV exclusion for the fixture volume, no agent load, AC +
high-performance, documented cache state, 1 warm-up + 10 measured) to
isolate background contention before any optimize/re-budget decision. Report
median/max/nearest-rank p95 the same way; do not filter iterations.
```

```text
F2-optimize: I approve profiling and optimizing the scan handle/metadata
path (per-file opens, ancestor validation chain, synchronous=FULL commit
batching) against the quota-fitting 10k set, then re-measuring warm-p95 on
the same host class. Budgets stay as written until the new evidence lands.
```

```text
F3-100k: I acknowledge the 100,000-entry qualification set stays
unmeasurable under the current 10,000-record staging quota, and I approve
[OWNER FILLS: raising the quota / staging the 100k set in bounded chunks /
deferring the 100k qualification with a dated checkpoint] in
docs/PHASE_2_EXECUTION_PLAN.md. The ~53 MiB figure remains a 10k-set
measurement and is not claimed against the 100k budget.
```

Dependabot/protection sentences (for the record; #74 already merged by the
owner — repost only if a revert/re-decision is intended):

```text
GOVERNANCE: I confirm enforced branch protection on main as merged in PR #74
(10 strict required checks, include-admins, linear history, no force-push or
deletion, zero required approvals for the single-maintainer repo) and the
Dependabot scoping (@types/node >=25 and typescript >=7 ignored; Node tracks
24.20.0; TS7 postponed until typescript-eslint support + Phase 2 checkpoint).
```

## 6. Verification evidence for this packet

- `gh pr view 74 --json files,statusCheckRollup`: 7 files (§1.1); 10/10
  checks `SUCCESS` (§1.1).
- `pnpm.cmd lint:docs`: `0 issues` across 44 files including this brief
  (engine warning only: shell Node v26.4.0 vs pinned 24.20.0).
- `node --test tests/packaging-policy.test.mjs`: `6 pass, 0 fail` on base
  `74203a2` (pre-#74 workflow still carries the `paths:` filter; the
  post-#74 assertion update ships in #74 itself).
- F2 recomputation: `node -e` over
  `benchmark-2026-09-07-quota-raw.json` →
  `{n: 10, median: 12875.5, max: 32876, p95: 32876}` — matches the report's
  `statistics` block exactly.
- `gh api .../branches/main/protection` (+ `required_status_checks`,
  `enforce_admins`, `required_pull_request_reviews` sub-paths) outputs as
  quoted in §1.1; `allow_force_pushes` / `allow_deletions` /
  `required_linear_history` read from the top-level protection document
  (dedicated sub-paths return HTTP 404 on this API version).
- `gh pr view 27 / 28 --json number,title,state,closedAt`: both CLOSED
  2026-09-07; drafts in §2.1 are for the owner to post — not posted here.
