# Resource qualification runner preparation

Status: **Guard and unattended runner prepared; quiet measurements pending.**
Date: 2026-10-11. Related issues
[#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306) and
[#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259).
Application baseline: merged PR307, `4fadc55`; its application code is unchanged
from the retained PR305 artifacts built at `0e86b8e`.

## What changed

PR307 preserved the measurements and withdrew six incorrect quiet-pass labels
after review found omitted FL Studio CPU activity. The next step is valid quiet
measurement. This preparation makes the assessment rule reviewable in
[assess-resource-noise.mjs](../../../scripts/assess-resource-noise.mjs), with
portable regression tests, and connects the private Windows collectors to it.
No application, parser, database schema, dependency, CI or budget change is made.

The guard examines every observed process, without a name filter. The fixed
limits remain initial CPU load at most 20% and at most 0.5 CPU-seconds per
unrelated process across the window. System/service CPU is counted
conservatively. Instrumentation and measured application exclusions require
both a process ID and its recorded UTC start identity. A familiar process name,
an old `ReviewOwned` flag, or a reused process ID cannot supply an exemption.
Malformed evidence fails closed. The collector remains responsible for proving
the declared identities and retaining its observations.

Boundary snapshots still omit terminated or inaccessible processes; this guard
does not claim otherwise. A guard pass also does not approve budgets, establish
a timing/memory target pass or accept Phase 3.

## Validation

Five focused tests cover the omitted FL Studio case and unknown process names,
the exact existing thresholds, process-ID reuse, untrusted legacy flags and
malformed/ambiguous evidence. The original samples and receipts are unchanged.

A retained native Explorer binary exercised the updated Windows collector and
the shared assessor while T3 was active. The assessor reported **failure** for
an observed 1.1875 CPU-seconds from T3 against the 0.5 limit. This is a plumbing
check and characterization, not quiet qualification. Its raw evidence and
source/artifact provenance remain private, outside compiler caches and Git.
The installed collector and coordinator pass syntax checks; no installed
measurement or unattended qualification is claimed by this preparation.

Run the portable checks with the pinned Node environment:

```text
node --test tests/resource-noise-guard.test.mjs
node --test "tests/*.test.mjs"
pnpm lint:scripts
pnpm privacy:check
```

Normal checks on the updated PR head remain required. Existing exact-head CI
covers the unchanged application source, not benchmark validity. No redundant
local application rebuild is needed for this script/test/documentation scope.

## Unattended run and evidence boundary

The private prepared runner starts after a two-minute delay, allowing T3,
FL Studio and unrelated applications to be closed. It checks known workload
presence before setup; classification still uses all observed process deltas.
It does not close unrelated applications. Each attempt has a unique evidence
directory and refuses to overwrite a started attempt.

It verifies source/artifact/guard hashes and reruns the read-only storage
preflight before generating fixtures or installing the review app. The single
existing validation cache remains owned by Codex/root; no build or cache writes
are planned. The preflight passed with 32.70 GiB free and 1.50 GiB estimated
output, leaving 31.20 GiB above the 30 GiB reserve. The plumbing check produced
retained databases; the later launch repeats the capacity check.

Closed prior worker databases are inventoried and retained through checked
PowerShell moves. The installed stage holds the shared host lock and archives
the old owned review profile with matching before/after file hashes. That
profile includes unfinished 2,001-entry work and must not contaminate timers.
A fresh profile uses the existing review package identity; it is not a newly
built or newly named product. Personal and Foundation Smoke profiles remain
protected by independent before/after inventories.

The installed matrix prepares all nineteen approved corpus files and the
100/1,000/2,000-entry roots through actual scanning, supervision and publication
before latency/memory timers. It repeats native Explorer/worker, installed
Explorer IPC/UI, mixer refresh, populated startup and whole-app memory using
the [same proposals](../phase3-qualification-306/budget-proposal.md). The
2,001-entry refusal runs last outside timers. Cleanup uninstalls only the owned
review identity and retains databases, fixtures, binaries, logs and all earlier
evidence. A new source or guard hash requires reviewed preparation again.

The private handoff records absolute cache/evidence paths, ownership, prepared
launcher, hashes and retention duties. After a run, independently review guards,
all samples, profile cohort, source hashes and cleanup before publishing a
sanitized result. Workload closure is coordination, not budget/phase approval.
Issues #259/#306 and the broader Phase 3 gaps remain open.
