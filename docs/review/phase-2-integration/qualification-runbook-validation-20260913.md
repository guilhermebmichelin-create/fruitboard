# Qualification runbook validation (2026-09-13)

Status: **runbook validation only; not performance evidence and not a
qualification run.** No benchmark, release build, host preflight, Defender
query, DriveFS action, or process termination was performed for this check.

## Checks performed

- Parsed all six PowerShell blocks in
  `qualification-runbook-20260913.md` with the PowerShell AST parser without
  invoking any block: **0 parse errors**.
- Parsed `scripts/validate-qualification-report.mjs` with `node --check`:
  **0 syntax errors**.
- Exercised the validator against disposable synthetic JSON reports. The
  `qualified` result below means only that the validator accepted the complete
  synthetic schema; it is not a benchmark result.

| Synthetic case          | Exit | Disposition      | Observed validation                      |
| ----------------------- | ---: | ---------------- | ---------------------------------------- |
| Complete passing report |    0 | `qualified`      | All required records and metrics valid   |
| Target miss             |    1 | `non-qualifying` | Warm p95 above 10,000 ms                 |
| Missing iteration       |    1 | `non-qualifying` | Measured count is not 10                 |
| `Partial` result        |    1 | `non-qualifying` | Non-authoritative measured observation   |
| Missing metrics         |    1 | `non-qualifying` | Missing working-set metric               |
| Malformed numeric data  |    1 | `non-qualifying` | Numeric string rejected                  |
| Cancellation failure    |    1 | `non-qualifying` | Non-terminal/invalid cancellation record |
| Boolean numeric         |    1 | `non-qualifying` | Boolean `scanMs` rejected                |
| Negative numeric        |    1 | `non-qualifying` | Negative memory rejected                 |
| Malformed record        |    1 | `non-qualifying` | Non-object iteration rejected            |
| `NaN` numeric           |    1 | `non-qualifying` | Non-finite timing rejected               |
| `Infinity` numeric      |    1 | `non-qualifying` | Non-finite timing rejected               |
| Fractional target miss  |    1 | `non-qualifying` | `10000.5` compared without rounding      |

- Used `buildPlan`, `writePlan`, `validateSyntheticManifest`, and
  `computeManifestHash` from the existing generator API in a disposable
  temporary directory, then removed that directory. Result: hardlinks
  `created`; 9,995 primary FLP files, 5 aliases, 4 other files, 10,000
  observations; canonical manifest SHA-256
  `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a`.
- Confirmed the #112 harness and native driver syntax with `node --check` and
  ran the focused generator/benchmark-scaffold tests: **17 passed**. These
  tests did not start the real benchmark; the scaffold test uses its
  environment-only fallback.
- Imported the actual #112 `parseBenchmarkArgs` function and exercised every
  option used by the runbook; the complete option surface and report-field
  markers were present. The parser accepts `--warmup`, but the execution path
  does not consume `parsed.warmup`; it uses the source constant
  `WARM_UP_RUNS = 1`, which is why the runbook omits that non-binding option.

The checked source-stack references were the historical combined candidate
`e90b03cc0bddd1a449e817ea2d89708a85c82ef1` and #112 head `9da0272`; their
benchmark harness, driver, and generator file blobs are identical. Neither
SHA is an execution boundary. A future run must record the exact merged
`origin/main` SHA at execution time.

## Final static checks

- Targeted Markdownlint: **pass**.
- Prettier check for the corrected runbook, validation record, and validator:
  **pass**.
- Repository privacy verification: **pass** (293 files).
- `git diff --check`: **pass**.

The temporary synthetic reports and generator fixture are outside the
repository and are not retained as benchmark evidence.
