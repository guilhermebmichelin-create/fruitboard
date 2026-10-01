# Explicit project analysis from Library details

Date: 2026-10-01. Related issue: #246. Baseline: merged PR #245
(`24ff211248fe27ef0dfa5e86d597050e36f29c77`).

## Delivered behavior

Project details offers **Analyze project**, **Analyze again**, or **Retry
analysis** when the development worker and captured source are eligible.
Opening or refreshing details still only reads saved state. Only an explicit
button activation requests work. Valid saved facts stay visible during the
request and its subsequent background status read. There is no automatic
polling or retry. Started attempts remain bounded to three for the same
captured source generation and reader version; requesting work does not count
as a started attempt.

Waiting/running work, an exhausted budget, unsupported unchanged builds,
changed sources, unverified identities, files above the existing 4 MiB limit,
a full 128-cell pending queue, and an unavailable worker receive fixed
explanations and a disabled action. A failed request requires a fresh details
read before another click. Older adapters and responses retain their existing
read-only behavior. The default build keeps metadata disabled.

## Native write and privacy boundary

One local main-window command accepts only opaque root/location IDs, displayed
byte size and nanosecond timestamp, and a native-issued action key. It accepts
no path, parser selection, job/lease identifier, SQL or process parameter.
The key is an optimistic state fence, not a secret or authorization grant.
It hashes captured source revisions, publication order, reader version and
the observed job cell. It is never displayed or logged.

The native command recaptures the approved enabled local source. One SQLite
IMMEDIATE transaction checks its revisions, the displayed action key,
eligibility, current attempts and queue capacity before inserting/replacing
the location's bounded job cell. Replays and stale screens fail closed;
requests cannot shorten pending backoff. Root disable/re-enable, restored
timestamps/identities, publication changes, and another location's key cannot
bypass these fences. Only a new captured source generation or reader version
receives a fresh attempt budget, following the existing discovery rule.

Reanalysis leaves the current good snapshot and immutable history intact.
The existing worker claims the queued request and independently rechecks the
approved handle identity, actual bytes and digest before publishing. Worker
availability is a startup/shutdown observation; queue acceptance does not
promise ongoing worker health or successful completion. No migration,
dependency, parser decoding, production activation, budget-reset or cancel
control is added. Rollback removes the control without removing saved data.

## Rendered evidence

The actual client and native response validator run with a synthetic transport
at desktop 1280 × 1800 and narrow 390 × 844. Checks cover one request for
repeated keyboard activation, retained facts, all blocked categories, safe
errors, expansion/close focus, reduced motion, 200% text and rendered
accessibility/contrast using unmodified styles. Independent component crops
omit navigation and skip-link overlays. This is browser evidence, not
installed-app or performance qualification.

- [Analyze again, desktop](ready-desktop.png)
- [Analyze again, narrow](ready-narrow.png)
- [Queued with retained facts, narrow](pending-narrow.png)
- [Attempt limit, narrow](attempt_limit-narrow.png)
- [Unavailable worker, narrow](runtime_unavailable-narrow.png)
- [Retry without saved facts, narrow](retry-narrow.png)
- [Contained request failure, narrow](error-narrow.png)

## Validation and handoff

Required: full pinned Windows `pnpm check`; enabled desktop tests and
all-target warning-denied Clippy; storage request/revision/budget/capacity
tests; existing-worker reanalysis; client privacy, explicit activation,
retained-fact, older-adapter and late-response tests; byte-identical approved
FLP fixtures; an independently copied older installed database through the
real read command; rendered checks above; final-head CI before manual merge.
No required check is waived.

Local full pinned `pnpm check` passes, including 254 client tests, workspace
checks and the desktop release build. Storage passes 135 ordinary tests;
its two explicit environment-dependent probes remain separate. All eight
new request tests pass. Enabled desktop tests pass 128 tests, with the
copied-database probe separately passing. Its 262,144-byte database hash
is unchanged. Enabled all-target Clippy, existing-worker reanalysis and
all rendered checks pass. All 12 approved FLPs match the merged baseline.
Final-head CI must pass before merge.

Metadata remains development-only. Broader parser support, live updates,
Plugin Explorer, installed/distribution and performance qualification remain
separate work. Owner review and manual merge are the next delivery decision.

The existing source worktree and reusable temporary validation cache are
reused. Codex `/root` owns the sequential heavyweight write window; absolute
paths and final capacity belong in the private handoff. The 4 GiB output
budget maintains at least 30 GiB free on every used volume. Private reports,
database copies, fixture hashes and independently copied binaries remain in
the external `fruitboard-review-evidence/analysis-request-20261001` directory,
outside compiler caches. Earlier evidence, source and the primary cache are
protected. No obsolete compiler intermediates are selected for deletion.
