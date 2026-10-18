# Packaging-smoke shutdown phase diagnostics

Issue: [#327](https://github.com/guilhermebmichelin-create/fruitboard/issues/327)

The `packaging-smoke` feature supports two ways to opt into a separate
shutdown trace beside the selected evidence file. The existing `seed`,
`verify`, and `parser-missing` modes keep their scheduled smoke behavior and
also emit the trace. For an interactive installed run, set
`FRUITBOARD_FOUNDATION_SMOKE_MODE=shutdown-diagnostics` and provide an
absolute `.json` path in `FRUITBOARD_FOUNDATION_SMOKE_OUTPUT`. This mode only
arms shutdown diagnostics: it does not schedule a self-test, modify
preferences, or request the app to exit. For `seed.json`, the trace is
`seed.shutdown-stages.jsonl`. The validated output path is retained in native
state; shutdown does not re-read environment variables. Without the feature
or one of these valid explicit configurations, no shutdown trace is created.

Use a fresh evidence output path for each launch. The trace is opened in append
mode, so reusing an output path also reuses its `*.shutdown-stages.jsonl` file.

Each flushed JSONL record has exactly two fields:

```json
{ "stage": "sample_checks_start", "elapsedMs": 0 }
```

Elapsed milliseconds start at shutdown entry. With `analysis-jobs`, the fixed
stage sequence is `shutdown_start`, `sample_checks_start`,
`sample_checks_end`, `analysis_host_start`, `analysis_host_end`,
`scan_console_start`, `scan_console_end`, and `shutdown_end`. Without that
feature, the two analysis-host records are omitted. At most eight records are
written per shutdown with `analysis-jobs`, or six without it. The `elapsedMs`
difference between a phase's start and end records estimates how long its
existing shutdown call took.

The trace contains only those fixed stage names and elapsed time. It does not
record a path, command argument, database value, record identifier, or error
text. Writes are best-effort and flushed after each record; an open, write, or
flush error does not interrupt shutdown. The underlying order remains sample
checks, optional analysis host, then scan-console host. No shutdown timeout,
join, cancellation, or lifecycle behavior is changed.

This is diagnostic evidence, not a fix or a qualification result. A start
record without its matching end may mean that the process stopped during the
phase, or that writing the end record failed. It does not identify the cause
of a hang or prove that shutdown completed.

## Installed smoke validation

The follow-up validator in [issue #330](https://github.com/guilhermebmichelin-create/fruitboard/issues/330)
checks a fresh trace after the exact owned `seed`, `parser-missing`, and
`verify` processes exit successfully. It rejects missing, incomplete,
oversized, duplicated, reordered, unknown, or malformed records. The final
report keeps only the fixed stage count and phase durations; the raw JSONL
trace remains beside the private native smoke evidence. The hosted workflow
exercises both the six-stage package and eight-stage `analysis-jobs` variants.
These scheduled smoke exits validate trace plumbing on the installed app; they
do not reproduce the earlier 1,000-entry interactive close delay or qualify
resource budgets.

The full-scale #306 resource runner does not currently collect these shutdown
records. To capture them during a separately authorized installed resource
run, launch the feature-enabled development app with
`FRUITBOARD_FOUNDATION_SMOKE_MODE=shutdown-diagnostics` and a fresh absolute
`.json` path in `FRUITBOARD_FOUNDATION_SMOKE_OUTPUT`, then close the app
normally. The `*.shutdown-stages.jsonl` sibling is append-only: choose a new
output path for every launch and preserve even incomplete traces. Keep the
existing close deadline and resource guards; the trace alone is not a
qualification result.

Focused native tests:

```powershell
cargo test -p fruitboard-desktop --features packaging-smoke shutdown_diagnostics --locked
cargo test -p fruitboard-desktop --features analysis-jobs shutdown_diagnostics --locked
```

Updated-head CI and the installed packaging-smoke evidence remain required for
review. The source tests cover fixed record shape and order, omitted analysis
markers when that feature is off, diagnostics-only versus scheduled-mode
configuration, invalid-output rejection, no-request behavior, and continuation
when the trace destination cannot be written.
