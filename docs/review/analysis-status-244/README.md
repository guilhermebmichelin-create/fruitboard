# Project analysis status and explanations

Date: 2026-10-01. Related issue: #244. Baseline: merged PR #243
(`80b65f250c2b2683b90510ad1a54438679b1dff2`).

## Delivered behavior

Library Project details explains the last recorded analysis state: waiting,
reading, finished, unsupported, failed, cancelled or changed source. Recorded
attempts show the existing three-attempt limit. Missing reports are explicit;
older adapters and snapshots never imply an invented successful attempt.
The view explains interruptions, unavailable files, reader failures, unsupported
builds and supported parser limits using fixed text. Unknown categories receive
an explicit unclassified explanation. Saved partial results can show one bounded
warning about unverified events while retaining supported facts and channels.

Status describes a saved observation, not live worker health or a completion
percentage. Refresh reads saved results; it never starts or retries analysis.
A terminal problem can coexist with a separate valid snapshot, whose facts
remain visible and clearly distinguished from that attempt.

## Read and privacy boundary

The existing command still accepts only opaque root/location IDs and the displayed
row fingerprint. The storage reader checks captured source/root/file/location
revisions, adapter/schema and attempt bounds. A terminal snapshot must match
that location's source and its job outcome. Another alias's attempt is not
attributed to the selected location. Publication order remains a commit fence,
not a new source observation; reading never revives or schedules work.

The fingerprint guard now runs before reading status even when there are no
current facts. Missing/disabled/unauthorized sources or old displayed rows do
not reveal their old attempt. The database guard keeps status and saved facts
within the same native read. Existing scan-snapshot and late-response fences
clear the entire panel when its source changes.

Only allowlisted state, bounded attempt count and fixed explanation categories
reach the client. No job/lease IDs, raw codes, paths, diagnostics or arbitrary
parser text are added to IPC or logs. Saved diagnostic codes are collapsed to
one supported warning; malformed/unknown values fail closed. Old projections
without diagnostics remain readable. No migration, parser decoding, scheduling,
dependency, capability or production feature change.

## Rendered evidence

The actual client and native response validator use an explicitly synthetic
transport. Browser evidence covers desktop 1280 × 1800 and narrow 390 × 844,
all reported states, older adapters, retained facts, disabled/error/loading,
keyboard focus, reduced motion and 200% text. Rendered accessibility and
contrast checks use unmodified styles. Independent component crops omit
navigation and skip-link overlays so the complete details section is visible.
These checks are not installed-app or performance qualification.

- [Finished, desktop](complete-desktop.png)
- [Finished, narrow](complete-narrow.png)
- [Waiting, narrow](queued-narrow.png)
- [Unsupported build, narrow](unsupported-narrow.png)
- [Failed attempt with valid saved facts, narrow](retained-narrow.png)

## Validation and handoff

Required: full pinned Windows `pnpm check`; enabled `analysis-jobs` desktop
tests and all-target warning-denied Clippy; storage source/alias/snapshot
ownership regressions; client bounds/privacy/refresh/late-response tests;
unchanged approved fixture hashes; the rendered checks above; an independent
copy of an older installed database through the real command; final-head CI.
Native tests exercise actual failed and unsupported parser replies through
publication and the authorized command, including safe fixed explanations.

Local full pinned `pnpm check` passes with 225 client tests, 127 repository
tests, workspace lint/type/tests and the desktop release build. Storage passes
127 tests with the added source/alias/snapshot guards. Enabled desktop tests
pass 126 tests; their ordinary suite ignores the explicit copied-data probe,
which separately passes against an independent older installed database copy.
Its 262,144-byte database hash is unchanged. Enabled all-target Clippy and all
rendered checks pass. Approved FLP bytes match the merged baseline. Final-head
CI must pass before manual merge; no required gate is waived.

Metadata remains development-only. Reparse/retry/cancel controls, live updates,
broader parser support, Plugin Explorer, installed/distribution and performance
qualification remain separate work. Owner review and manual merge are required.

The existing source worktree and single temporary validation cache are reused.
Codex `/root` owns the sequential heavyweight write window; the private handoff
records its absolute path. The 4 GiB output budget preserves at least 30 GiB
free on the used volume. Private logs, database copies, hashes and independent
binaries remain outside compiler caches in the external
`fruitboard-review-evidence/analysis-status-20261001` directory. Earlier evidence,
fixtures and the primary development cache remain protected. No obsolete
intermediates are selected for deletion; reusable outputs remain with the owner.
