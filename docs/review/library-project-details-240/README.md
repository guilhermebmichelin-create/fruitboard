# Library saved project details

Date: 2026-10-01. Related issue: #240. Baseline: merged PR #239
(`d26cd22fce95d5b082e1510b571d023473f21d12`).

## Delivered behavior

Each Library file has a keyboard-accessible Project details button. Expansion
loads nine selected saved facts: FL Studio version, base tempo, channel count,
project creation/local time, filesystem creation/UTC time, FL Studio's saved
time counter, verified playlist pattern endpoint, bar span and nominal seconds.
Separate status badges and reason/inference copy distinguish extracted,
inferred, unavailable and unsupported values. Local dates retain milliseconds
without inventing a timezone. Playlist estimates describe verified pattern
clips, and the saved counter does not measure independently tracked work.

Loading, unavailable-build, no-current-result and read-failure states explain
what happened. Refresh details rechecks saved data without triggering parsing.
Closing/unmounting ignores pending replies. Changed file/path fingerprints or
disabled/unsupported roots clear the open panel and discard earlier requests.
Expansion and closure preserve keyboard focus.

## Native and privacy boundary

`get_project_details` is a read-only permission for the local main window. Its
closed request contains schema version, opaque root/location IDs and the
displayed file size/modification fingerprint. It accepts no paths, SQL, parser
selection or parse action. The native database guard protects root/location
authorization and current-snapshot/fingerprint checks. Default builds report
unavailable; reads are enabled only with the existing development
`analysis-jobs` composition.

The versioned saved projection is untrusted display data, never a reconstructed
parser capability. A bounded allowlist validates scalar types, ranges, dates,
statuses, reasons and inference assumptions before serializing nine fixed fact
keys. No raw JSON, sample/plugin paths, extensions or diagnostic text crosses
this new boundary. The client checks response identity, key order/count,
status/value relationships and numeric/date bounds, and discards extra fields.
Integers remain decimal strings; out-of-range filesystem dates get explicit
display copy. Errors and logs contain fixed codes, never request/fact text.

Metadata remains immutable; no migration or parser/worker change. Missing,
changed, detached or disabled sources cannot return a stale current snapshot.
A negative attempt can retain a still-fresh last good result. The view states
that it describes the last scanned/read file, without claiming live freshness.

## Rendered evidence

These screenshots use the real client and native response validator with an
explicit synthetic transport and synthetic facts. They establish browser
layout/states, not installed-app or parser compatibility qualification.

- [Desktop details](details-desktop.png)
- [390 px details](details-narrow.png)
- [Unavailable and unsupported fields](coverage-narrow.png)
- [No current result](no_current-narrow.png)
- [Read failure](error-narrow.png)
- [Loading](loading-narrow.png)
- [Feature unavailable](disabled-narrow.png)

Desktop and narrow checks cover Enter activation, focus through expansion and
closure, reduced motion, 200% text, horizontal overflow and automated
accessibility including rendered contrast. Increasing text exposed the shell's
font-relative minimum width and a navigation label overflow; the minimum
viewport width now remains 320 CSS pixels and narrow labels can wrap.

## Validation boundary

Required checks: full pinned `pnpm check`, enabled desktop tests and
warning-denied all-target Clippy, rendered states and the final PR head's CI.
Native integration publishes approved fixture bytes through real validation
and storage, then checks selected-location authority, displayed fingerprint
and root/file invalidation through the real command. A separately invoked
ignored test reads an independent copy of retained installed smoke data; it
requires `FRUITBOARD_REVIEW_METADATA_COPY`, never production data. The copy's
before/after hash is checked in private evidence. Existing installed parser,
queue and package evidence from PR #239 applies to unchanged implementation.
No new installed-window or performance qualification is claimed.

Local validation passed at implementation
`5f3f96677142921930103299beea1e94ab23f388`: full pinned `pnpm check`
(178 client tests, workspace tests/lint/typechecks and desktop release build),
enabled desktop 122 tests with one explicit private-data test ignored by the
normal suite, and warning-denied feature Clippy. The ignored test was separately
invoked and passed against the independent installed snapshot copy; the
262,144-byte database retained its SHA-256 before/after. Rendered desktop/narrow
states, keyboard, reduced motion, 200% text and contrast/accessibility passed.
All approved corpus Git objects match the baseline. This validation summary
changes documentation only. The normal final-head CI remains required before
owner merge.

## Limits and handoff

Channel names, plugin/sample reference lists, history, progress and analysis
retry controls remain deferred. Unsupported parser builds/layouts do not gain
compatibility from this view. Parsing still requires explicit development
enablement; distribution, broader compatibility and performance gates remain
open. Owner review/merge is the next delivery step.

The existing source worktree and one temporary Cargo cache were reused; the
primary checkout/cache and earlier source/evidence remain protected. The sole
cache coordinator is Codex `/root`, with sequential heavyweight checks and
explicit `CARGO_TARGET_DIR`. A read-only storage preflight budgets 4 GiB further
output while preserving the 30 GiB reserve. Logs, copied database and private
provenance stay outside compiler caches/Git. No cleanup was selected or new
permanent build cache created; reusable outputs and retained evidence remain.
