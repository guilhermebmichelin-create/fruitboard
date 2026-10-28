# Development status and next step

Assessment date: 2026-10-27. Merged baseline: `ce4331e9004f1a7f363f45106adaa192d1f635fb`
([PR334](https://github.com/guilhermebmichelin-create/fruitboard/pull/334)).
Active development: draft
[PR348](https://github.com/guilhermebmichelin-create/fruitboard/pull/348), issue
[#347](https://github.com/guilhermebmichelin-create/fruitboard/issues/347).

## Application and phase status

Phases 0 and 1 are accepted. Phase 2 is accepted with known gaps; P2-08
remains Partial. Phase 3 FLP Intelligence is active. Its exit and resource
budgets are unaccepted, so the next step stays inside this phase.

The explicit development build can scan selected local NTFS roots, analyze
projects, retain immutable metadata and display saved facts, channels,
sample/plugin references, Plugin Explorer groups, pattern names/counts,
note-record counts and qualified ordinary mixer names. The default desktop
does not activate scanning or FLP-content analysis automatically.
The [readiness packet](../flp-intelligence-readiness/README.md) records exact
field/build limits. The approved parser corpus remains 21 files across three
exact saved builds. Automation extraction, broader compatibility and later
project-management phases remain unfinished.

## Progress since the October 17 assessment

- PR329 added development-only shutdown phase diagnostics; PR331 added strict
  trace verification. The October 17 recommendation to implement #327 is
  therefore superseded. Diagnostics do not explain the earlier close timeout.
- PR333 delivered the project-relative sample-candidate design/native review.
  The owner accepted ADR-008's precise interpretation on 2026-10-21, recorded
  in the active feature branch. PR348 implements that accepted scope but is
  unmerged and remains a draft pending validation.
- PR334 delivered current installed resource evidence. Open
  [PR335](https://github.com/guilhermebmichelin-create/fruitboard/pull/335)
  records a subsequent stopped remainder run. Its 15 checks passed, but the
  startup CPU guard failed. The remaining startup/memory/refusal and native
  resource windows remain unqualified; merging a report accepts no budget.
- Open [PR350](https://github.com/guilhermebmichelin-create/fruitboard/pull/350)
  originally patched markdown-it with 15 successful checks on `430bc06`.
  The October 27 rerun then found new high/moderate advisories in indirect
  development dependencies zod and undici. PR350 now also pins their patched
  releases, zod 4.6.6 and undici 8.12.2, at `1f4cacb`. Its updated-head checks
  are required again. The prerequisite is included in PR348's branch and
  remains separately available for owner merge.

These are dated source and tracker observations, not new installed or
performance evidence. Historical measurements keep their original sources,
corpora and failures, including the unexplained close timeout and scanner
10,173 ms p95 against its unchanged 10,000 ms target.

## Next bounded step: unblock the existing sample feature

PR348 head `efcf06428643f0acf8c30e999ccef4a0ea3dcc39` had 14 successful checks
and one failing required `windows-foundation` check in
[run 39704559578](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/39704559578).
The failure occurred in two constructed relative-reference host tests.
Running one test alone with the existing binary exposed `InvalidSchema` at
metadata publication, before sample checking began. The full-suite panic
redaction had hidden that setup error's location.

The raw parser fixture incorrectly supplied `status: extracted` with
per-channel `items`. A fully extracted parser list must instead contain
`value`, an array of strings; only the storage projection converts it to
per-channel `items`. Correct the fixture without relaxing production
validation or changing the expected sample outcomes. The existing tests then
exercise the nested project-folder base, path-free results, unchanged file
bytes and root/snapshot/cancel/deadline/shutdown/admission fences.

Once fixture publication succeeded, the delivery test exposed a second setup
mistake: it waited for a deadline after `check` had already published the
library result. That retired control no longer transitions to a deadline, so
the test hung and the owned test process was stopped. Split that case into a
prepublication deadline test: hold the active worker, observe the deadline
and busy admission, then release it and verify that the late native check
returns `Deadline`. Keep the root/snapshot/cancel/shutdown cases after library
publication, before transport delivery. Shutdown correctly returns `stale`
for new work; other still-running cases return `busy`. The delivery helper's
pause is bounded to avoid an indefinite wait on assertion failure.

The feature correction owns those host tests and this status/handoff
documentation. The newly discovered security prerequisite is isolated in
PR350 and merged into the feature branch. Neither changes product runtime
code, parser/storage contracts or a phase gate.

## Validation of this correction

- Pinned Windows `cargo test -p fruitboard-desktop --features analysis-jobs
--locked`: 169 passed, zero failed, four existing ignored; main and doc-test
  targets also passed. This includes both formerly failing host cases and the
  separate prepublication deadline regression.
- `cargo clippy -p fruitboard-desktop --features analysis-jobs --all-targets
--locked -- -D warnings` passed. The corrected test source is commit
  `838622c`; subsequent status edits change documentation only.
- Scoped Markdown lint, Prettier, repository privacy and `git diff --check`
  passed. All 21 approved fixture SHA-256 values match the retained baseline.
- The patched JavaScript dependencies pass frozen installation and
  `pnpm audit --audit-level moderate`; one low-severity finding remains below
  the unchanged threshold. Client ESLint and TypeScript checks passed on the
  security prerequisite. Local Vitest attempts stalled before reporting test
  results, including a bounded worker/fork retry; their owned processes were
  stopped and logs retained. No client test pass is claimed for the dependency
  update; updated-head client CI remains required.
- Local `pnpm check` and installed validation are not claimed. This correction
  changes test code and documentation only. The earlier Windows gate passed
  its default integrated check and scan-console tests before failing the
  analysis-jobs tests; that historical result applies only to unchanged code.
  The corrected PR still needs all required updated-head CI, including the
  pinned full Windows gate and feature Clippy. It remains a draft.

## Completion order and remaining decisions

1. Validate the corrected host tests and the feature-enabled native suite;
   retain the actual commands, source diff and results outside compiler caches.
2. Push the correction to PR348 and require its normal updated-head CI.
   Earlier passing checks apply to unchanged code only. They do not prove the
   correction or substitute for required checks on the new head.
3. Complete PR348's outstanding feature-enabled installed inspection, including
   truthful scope/results, keyboard/focus, narrow and enlarged-text states,
   and unchanged fixture/profile evidence. Keep the PR a draft until its
   acceptance criteria and review are satisfied. Constructed references prove
   Fruitboard policy/UI behavior, not FL Studio resolution or compatibility.
4. The owner merges reviewed, checked PRs under
   [the working agreement](../../../DEVELOPMENT.md#enforced-branch-protection).
   No agent merge is authorized. Then continue #259/#306 resource work only
   when the storage and all-process quietness guards pass.

No Phase 4 entry, G5 completion, production activation, license/distribution
approval or resource budget acceptance follows from this correction. Broader
relative/placeholder/external/cloud resolution and G3/G6 evidence still need
their separately scoped work and decisions.

## Local validation and storage boundary

The existing reusable `desktop-feature-tests` validation cache remains solely
owned by `/root`. Its absolute path and the retained evidence directory are
recorded in the private ownership receipt and task handoff. The primary
`target` cache is preserved. Private evidence is retained outside source and
compiler caches in the dated `development-status-20261027` evidence directory.
Earlier implementation/review and pending
installed-run preparation remain in the sibling `project-relative-20261021`
evidence directory.

C: initially had approximately 30.93 GiB free; G: had approximately 12.39 GiB
and is not used. Later capacity rose to approximately 34.7 GiB without any
task-owned cleanup; no reclaimed-space claim follows. A full build/installer
needs a new capacity preflight and an
honest output allowance, retaining 30 GiB afterward. Do not start it simply
because a small incremental test fits. Reuse the existing cache sequentially,
with one owner and no concurrent heavyweight work; installed testing also
requires its exclusive host lock. Resource qualification runs alone.

The cache preflight detected Cargo-created internal compiler hardlink aliases.
The aliases were inventoried, verified to remain inside this cache and replaced
with independent hash-verified byte copies before the build window. This
preserves outputs and is not disk reclamation. Cargo may recreate internal
aliases, so future writes require a fresh preflight. No worktree, profile,
database, fixture, dependency, toolchain or retained binary is deleted. No new
per-PR compiler cache is created.
