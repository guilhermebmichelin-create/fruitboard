# Dependabot triage - seven open PRs (2026-09-14)

Local-only triage note. Not committed, not pushed. No GitHub mutation of any
kind: no branch pushed, no PR merged, closed, edited, commented, or labeled.

Method: (1) `gh pr checks` for the current head of each PR plus failing-job
logs; (2) diff scope from `gh pr view --json files` and per-file patches versus
the DEVELOPMENT.md pin tables, `tools/toolchain-policy.json`, and
`tests/ci-policy.test.mjs`; (3) no local repro reached - every failure is fully
explained by the CI logs, so no worktree was created, no build ran, and no
compiler cache was created or used.

Base for all CI merge-ref runs: current `main` = `c73fc1e` (the pull/N/merge
refs in the logs merge each head into `c73fc1e`).

## Verdict summary

| PR  | Component                          | Verdict              | Why                                                                |
| --- | ---------------------------------- | -------------------- | ------------------------------------------------------------------ |
| 126 | typescript-eslint 8.69.0 -> 8.70.0 | MERGE                | All checks green; not a documented pin.                            |
| 129 | lucide-react 1.41.0 -> 1.45.0      | MERGE                | All checks green; not a documented pin.                            |
| 130 | @types/node 24.13.3 -> 24.13.4     | MERGE                | All checks green; satisfies dependabot Node 24 constraint.         |
| 124 | setup-uv 10.0.1 -> 10.1.0 (SHA)    | HOLD-until-companion | CI red: new action SHA not allowlisted; fail-closed by design.     |
| 125 | uuid 1.26.0 -> 1.26.1              | HOLD-until-companion | CI green, but uuid is an exact documented pin (pin-update rule).   |
| 127 | react-dom + @types/react-dom       | HOLD                 | CI red: react-dom 19.3.0 vs react 19.2.8 mismatch; also a pin.     |
| 128 | vite 8.2.2 -> 8.3.0                | HOLD-until-companion | CI green, but vite is an exact documented pin (pin-update rule).   |

Pin-table check correction to the task premise: vite 8.2.2 and React/React DOM
19.2.8 are **not** unpinned semver. They are exact entries in the DEVELOPMENT.md
pin table (and exact in `apps/client/package.json`), so the pin-update rule
applies. `@types/node`, `typescript-eslint`, and `lucide-react` have no
pin-table entry, so green Dependabot PRs for those can merge normally.
`tools/toolchain-policy.json` carries only node/pnpm/corepack/rust/python/uv/
sqlite; it has no uuid, react, or vite fields.

## Per-PR detail

### #124 - deps: bump astral-sh/setup-uv from 10.0.1 to 10.1.0

- Head: `f5f476a5ab747b38bf79de0f377629f238c55afc`; merge state BLOCKED.
- CI (exact head): FAIL `docs-policy` (17s)
  <https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34836992133/job/103952945924>,
  FAIL `windows-foundation` (2m30s)
  <https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34836992133/job/103952945919>;
  `client`, `migration`, `rust-portable`, `security`, `enumeration-windows`,
  `filesystem-watcher-windows`, `scan-execution-windows`, and
  `windows-packaging-smoke` pass.
- Failure cause (same in both jobs):
  `tests/ci-policy.test.mjs:63` -
  `astral-sh/setup-uv@bec219d24cd3e171d82865faccec33120bb574f4 must be
  explicitly reviewed and allowlisted`. The allowlist at
  `tests/ci-policy.test.mjs:50-55` still names
  `astral-sh/setup-uv@20cfd1bf945f4377ade1205e4dbc17946fc9a30d`.
- Diff scope: `.github/workflows/foundation.yml` only (+1/-1), the `uses:` SHA
  line. The `version: 0.12.9` input is unchanged, so the pinned uv toolchain
  (DEVELOPMENT.md table, `tools/toolchain-policy.json` `"uv": "0.12.9"`) is
  still what gets installed. No lockfiles.
- Pin impact: uv pin itself unchanged; the setup-uv action is third-party
  execution governed by the fail-closed CI allowlist. The test asserts the
  allowlist set equals the workflow's action set exactly
  (`tests/ci-policy.test.mjs:68`), so allowlisting the new SHA alone while the
  workflow still uses the old SHA would also fail; both must change together.
- Verdict: HOLD-until-companion. CI cannot go green as-is.
- Next action: **owner** (or a follow-up agent under owner direction) makes the
  explicit action-review decision and lands one focused change that bumps the
  workflow SHA (`.github/workflows/foundation.yml:192`) and the allowlist
  (`tests/ci-policy.test.mjs:50-55`) together, then re-runs `docs-policy` and
  `windows-foundation`; #124 is then closed/superseded rather than merged.
  A follow-up agent may prepare that change once the owner approves the new
  action SHA. Non-blocking observation: the windows job logs a cosmetic
  `npm error EEXIST ... yarn` during the global corepack install; the job
  continues and the failure is only the allowlist assertion.

### #125 - deps: bump uuid from 1.26.0 to 1.26.1

- Head: `c128ddd8c69945d248acc7fa78b26c7433683473`; merge state CLEAN.
- CI (exact head): all 10 checks pass, including `windows-foundation` (13m2s)
  <https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34837005670/job/103952990526>
  and `windows-packaging-smoke` (5m53s).
- Diff scope: `Cargo.toml` (`uuid = "=1.26.1"`), `Cargo.lock` (+2/-2).
- Pin impact: uuid 1.26.0 is an exact documented pin
  (DEVELOPMENT.md "UUID / regex | 1.26.0 / 1.13.1"; `Cargo.toml:27`
  `uuid = { version = "=1.26.0" }`). Per the pin rule, updating a pin requires
  a focused PR that regenerates locks, runs the full check, and updates the
  table and machine-readable policy together. `tools/toolchain-policy.json` has
  no uuid entry today.
- Verdict: HOLD-until-companion even though CI is green.
- Next action: **owner** decides whether 1.26.1 is wanted; a **follow-up agent**
  then authors the focused pin-update PR: DEVELOPMENT.md table entry
  1.26.0 -> 1.26.1 (and `tools/toolchain-policy.json` if the pin set is
  expanded to cover uuid), lock regeneration, and full `pnpm.cmd check` on
  Windows. #125 is then closed/superseded.

### #126 - deps: bump typescript-eslint from 8.69.0 to 8.70.0

- Head: `b64f4508d657461bfb2374c1d22c3d3175d9a0dd`; merge state CLEAN.
- CI (exact head): all 10 pass, including `windows-foundation` (5m40s).
- Diff scope: `apps/client/package.json`, `pnpm-lock.yaml` (dev dependency).
- Pin impact: none. Not in the DEVELOPMENT.md pin table or
  `tools/toolchain-policy.json`; not in `.github/dependabot.yml` ignore list.
  Lockfile regenerated in the PR; exact-head `client`/`windows-foundation`
  green covers lint, typecheck, and tests.
- Verdict: MERGE. Existing green CI is the needed evidence; no local build.

### #127 - deps: bump react-dom and @types/react-dom

- Head: `f855df96aad6b4ac61533211c880ad63317afab8`; merge state BLOCKED.
- CI (exact head): FAIL `client` (32s)
  <https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34837067662/job/103953195390>,
  FAIL `windows-foundation` (2m23s)
  <https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34837067662/job/103953195237>;
  other jobs pass.
- Failure cause (same in both jobs): all 13 client Vitest suites fail at import
  with `Error: Incompatible React versions: The "react" and "react-dom" packages
  must have the exact same version. Instead got: - react: 19.2.8 -
  react-dom: 19.3.0`.
- Diff scope: `apps/client/package.json` - `react-dom` 19.2.8 -> 19.3.0 and
  `@types/react-dom` 19.2.7 -> 19.3.0; `react` stays 19.2.8; `pnpm-lock.yaml`.
- Pin impact: React / React DOM 19.2.8 is an exact documented pin
  (DEVELOPMENT.md table). Even a green react-dom-only bump would require the
  focused pin-update process; this PR is additionally internally inconsistent.
- Verdict: HOLD (failing CI plus pin rule). Do not merge.
- Next action: **owner** decides whether to adopt React 19.3.0 at all. If yes,
  a **follow-up agent** authors a combined `react` + `react-dom` +
  `@types/react-dom` bump with lock regeneration, DEVELOPMENT.md table update,
  and full `pnpm.cmd check`. Otherwise close/supersede #127. Local repro is not
  needed - the log identifies the exact mismatch.

### #128 - deps: bump vite from 8.2.2 to 8.3.0

- Head: `d62fa2e6e620334d8b2808e7ce6e2f125324e49c`; merge state CLEAN.
- CI (exact head): all 10 pass, including `windows-foundation` (5m50s).
- Diff scope: `apps/client/package.json` (`vite` 8.2.2 -> 8.3.0),
  `pnpm-lock.yaml`.
- Pin impact: Vite 8.2.2 is an exact documented pin (DEVELOPMENT.md
  "Vite / React plugin | 8.2.2 / 6.1.1"). The PR regenerates the lock and is
  green, but does not update the pin table, so it does not satisfy the
  pin-update rule. `tools/toolchain-policy.json` has no vite field.
- Verdict: HOLD-until-companion (CI green, bookkeeping missing).
- Next action: **owner** confirms 8.3.0; a **follow-up agent** authors the
  focused pin-update PR (DEVELOPMENT.md table 8.2.2 -> 8.3.0, lock update, full
  `pnpm.cmd check`), then #128 is closed/superseded. Or the owner may
  explicitly waive the pin rule and merge #128 plus a docs follow-up - that is
  an owner call, not an agent one.

### #129 - deps: bump lucide-react from 1.41.0 to 1.45.0

- Head: `bac7fa7fa209294dc446b306e048206c8f82631b`; merge state CLEAN.
- CI (exact head): all 10 pass, including `windows-foundation` (6m5s).
- Diff scope: `apps/client/package.json`, `pnpm-lock.yaml` (runtime
  dependency).
- Pin impact: none. Not in the DEVELOPMENT.md pin table or
  `tools/toolchain-policy.json`.
- Verdict: MERGE. No local build; exact-head CI is the evidence.

### #130 - deps: bump @types/node from 24.13.3 to 24.13.4

- Head: `f531282f10a3a1c1d8e88d4eaf8a81c83bf34598`; merge state CLEAN.
- CI (exact head): all 10 pass, including `windows-foundation` (6m14s).
- Diff scope: `apps/client/package.json`, `pnpm-lock.yaml` (dev dependency).
- Pin impact: not in the DEVELOPMENT.md pin table or
  `tools/toolchain-policy.json`. `.github/dependabot.yml:17-19` requires
  @types/node to track the pinned Node 24.20.0 runtime and ignores `>=25`;
  24.13.4 satisfies that constraint.
- Verdict: MERGE. No local build; exact-head CI is the evidence.

## Evidence and handoff

- CI evidence is the exact current head of each PR (check runs created for the
  pull/N/merge refs on `main` = `c73fc1e`); no code changed on any head after
  these runs. For #124 and #127 the logs alone explain the failures, so step 3
  (local repro) was not reached.
- No worktree created, no build run. Cache: **no cache created or used**
  (`CARGO_TARGET_DIR=%TEMP%\opencode\validation-target` was not touched, and it
  was not claimed as an exclusive window because no build ran).
- C: free space: 62.62 GiB before -> 62.60 GiB after (rounding/OS noise; no
  build output). The 30 GiB reserve was never at risk. G: untouched.
- No deletion performed. Pre-existing untracked `cache-inventory-20260914.md`
  left untouched.
- No branch pushed; no PR merged, closed, or otherwise mutated.
- This note is local-only and untracked: `dependabot-triage-20260914.md`.
