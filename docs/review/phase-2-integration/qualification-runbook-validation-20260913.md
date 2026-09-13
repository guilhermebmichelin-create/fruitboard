# Qualification runbook validation (2026-09-13)

Status: **validator correctness regression only; not performance evidence and
not a qualification run.** No benchmark, release build, host preflight,
Defender query, DriveFS action, process termination, host/security change, or
budget change was performed for this check.

## Live state and preserved reproductions

The live pre-edit state was verified before changing files:

- PR #114 was open at `802c20a7ce6bc16f553740e11251d4d3ca42fa5c`.
- Its then-base was the historical combined #113 candidate
  `e90b03cc0bddd1a449e817ea2d89708a85c82ef1`.
- The linked qualification worktree was the requested sibling worktree.
- Preserved inputs were read from the requested `fruitboard-review114-AR8mUM`
  temporary directory.
  The preserved JSON files were not edited.

Running the pre-edit validator against those inputs produced the confirmed
defects:

| Preserved case       | Individual measurements                              | Reported aggregate                                                        | Pre-edit result       | Corrected result at checked candidate `2d03abac5654a7f7dfc05b22ff5f9ce85784cc16`     |
| -------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------- | --------------------- | ------------------------------------------------------------------------------------ |
| `baseline`           | scan `100` x10; stop `100` x3; all attempt exits `0` | scan n=10/median/max/p95 `10/100/100/100`; cancellation n=3/`100/100/100` | exit `0`, `qualified` | exit `0`, `qualified`                                                                |
| `hidden-scan-miss`   | scan `100,100,100,100,100,100,100,100,100,20000`     | scan n=10/median/max/p95 `10/100/100/100`                                 | exit `0`, `qualified` | exit `1`, `non-qualifying`; derived max/p95 `20000`, aggregate mismatch, budget miss |
| `hidden-cancel-miss` | stop `100,100,2000`; scan `100` x10                  | cancellation n=3/median/max/p95 `3/100/100/100`                           | exit `0`, `qualified` | exit `1`, `non-qualifying`; derived max/p95 `2000`, aggregate mismatch, budget miss  |
| `nonzero-child`      | measured `iteration-0` exit `1`; all other exits `0` | scan/cancellation aggregates both reported all `100`                      | exit `0`, `qualified` | exit `1`, `non-qualifying`; failed attempt retained and marked exit `1`              |

The corrected summaries were written only to disposable temporary output paths.
The validator now derives timing statistics from qualifying validated records,
compares median, maximum, nearest-rank p95, and sample count with the report,
and applies budgets to those derived values. Cancellation uses
`stopLatencyMs`, not cancellation `scanMs`. The native driver contract was
confirmed in `crates/scan-execution/examples/benchmark.rs`: process exit `0`
means terminal `Published` or `Cancelled`; all other attempt exits are
non-qualifying. Summaries retain attempt `exitCode` and explain failed attempt
dispositions in `attemptFailures`.

## Regression and static checks

- `node --test tests/qualification-report.test.mjs`: **20 passed**. This
  committed suite uses disposable synthetic reports and cleanup and covers a
  valid pass, understated scan/cancellation summaries, each aggregate field,
  nonzero warm-up/measured/cancellation exits, missing and non-authoritative
  attempts, malformed numbers, fractional target misses, missing required
  metrics, and the scan-duration/stop-latency distinction.
- `node --test tests/benchmark-scaffold.test.mjs tests/synthetic-tree.test.mjs`:
  **17 passed**. These are the pre-existing benchmark/scaffold tests and are
  recorded separately; they are not coverage of the new qualification
  validator.
- `node --test "tests/*.test.mjs"`: **103 passed**; the normal repository
  JavaScript test glob discovered the new suite.
- `node --check scripts/validate-qualification-report.mjs` and
  `node --check tests/qualification-report.test.mjs`: **pass**.
- Prettier check for the changed package, validator, test, and runbook files:
  **pass**.
- `pnpm.cmd privacy:check`: **pass** (293 files).
- Root `pnpm.cmd lint`: Markdownlint, script checks, and recursive workspace
  lint reached the Rust stage; the command exited `1` because `cargo clippy`
  was unavailable in this environment.
- Root `pnpm.cmd format:check`: all Prettier checks passed; the command exited
  `1` because `cargo fmt` was unavailable in this environment.
- Root `pnpm.cmd test`: the repository JavaScript tests (**103 passed**) and
  workspace Vitest tests (**134 passed**) completed; the command exited `1`
  at the final Rust stage because `cargo` was unavailable. No benchmark was
  started.
- `git diff --check`: **pass**.

Report validation remains only a report-integrity gate. It does not prove host
eligibility, source provenance, or full Phase 2 acceptance; those preflight,
source/build, platform-scope, and acceptance requirements remain separate
runbook gates.

## Published CI evidence

Both existing workflow files were explicitly dispatched on the published
branch because the stacked PR base does not trigger the current workflow
events. GitHub reports terminal success for both runs, and both runs checked
the exact same head SHA:

| Workflow                | Run                                                                                             | Checked SHA                                | Result      |
| ----------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------ | ----------- |
| Foundation CI           | [34759328316](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34759328316) | `ace76b8c96c2776eebcf31d474d708f9f09d4c07` | **success** |
| Windows Packaging Smoke | [34759329996](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34759329996) | `ace76b8c96c2776eebcf31d474d708f9f09d4c07` | **success** |

Dispatch acceptance was not treated as a check result; each run was queried
after completion for its conclusion and head SHA.

## Source-stack coordination

PR #112 remains open at source-stack head
`9da0272fb2fefac9775028ed6bf8013459c10cad`; actual merged `origin/main` is
still the required future performance execution boundary. PR #113 remains open
at `e90b03cc0bddd1a449e817ea2d89708a85c82ef1`; its historical acceptance
documentation was excluded. #114's own commits were rebased onto the current
source-stack head, leaving only the runbook/validator/test changes above on
this branch.

No benchmark or performance qualification is claimed by this record.
