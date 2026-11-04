# Installed notes and enlarged-text reflow

Issue: [#397](https://github.com/guilhermebmichelin-create/fruitboard/issues/397).
Result: **The bounded installed note workflow passes on source
`1de0a6c4329ce5da1fffa7cc3a977f7d51ae1f31`, including the narrow-layout fix.**
This is Windows development evidence, not a Phase 3 exit, resource qualification
or public activation. [Sanitized results](results.json) retain the exact source,
configuration and artifact hashes. Later dependency merges do not automatically
inherit this installed result; normal changed-head CI remains required.

## Problem and correction

At 390 CSS pixels with a 32px root font, nested Library padding left the note
heading only **98.02px wide**, wrapping it across eight lines. The document had
no horizontal overflow, so the previous overflow-only check passed despite the
unusable text column. The independently retained old package fails the new
`check-installed-note-reflow.mjs` check at this boundary.

The fix reduces horizontal padding in narrow Library containers and the note
record indentation, using existing spacing tokens. The outer inset change is
scoped to a main region containing Library. Text size, vertical spacing, focus
controls and touch-target dimensions stay intact. At the same boundary, the
installed heading is **237px wide and three lines**. The new check requires at
least half the viewport width and no more than four heading lines there, as
well as the existing no-overflow/bounds checks. These are UI regression
criteria for this known disclosure, not performance budgets.

Visual review uses actual installed WebView2 content:

- [Before: 390px, enlarged text](screenshots/before-390-enlarged.png).
- [Corrected desktop](screenshots/chord-desktop.png).
- [Corrected 390px](screenshots/chord-390.png).
- [Corrected 390px, enlarged text](screenshots/chord-390-enlarged.png).

## Installed workflow

The unsigned analysis-jobs NSIS package was built with pinned tools and frozen
dependencies in the reused validation cache. A fresh review identity/profile
used independent byte copies of approved F24-F38 plus two existing controls:
named-empty F17 and the 2024 minimal F01. No database was seeded, personal
project root scanned, FL Studio save authored or new fixture published.

All seventeen cases passed the actual scan, supervised parser, full native
validation, immutable publication and authorized details path. Fifteen are note
cases: the initial seven, six clean pilot cases and two negative controls.
Library pages contain four entries; the runner uses real pagination to reach
every case and Enter to open each note disclosure. Full rendered numeric values,
saved cardinality/order and channel binding match the independently registered
expectations. The chord retains saved key order 67/64/60.

N09's unqualified velocity and N15's unexplained byte19 still refuse the whole
pattern while retaining their saved counts; no supported sibling is presented
as a complete sequence. F17 shows missing payload rather than zero notes. F01
explains the unverified note build rather than displaying invented notes.

After one graceful close and restart, all seventeen immutable snapshot IDs and
selected sequence states remain unchanged. A timestamp-only change to the owned
chord copy, followed by actual scanning, rejects its old row fingerprint.
Disabling the root rejects native details and removes visible note records.
All approved source and staged FLP bytes remain unchanged.

The new non-destructive CDP check passes at 1280/390px with 16/32px root-font
emulation. Disclosures use debugger-managed focus and actual browser Enter
events. This is keyboard/disclosure and reflow evidence, not a comprehensive
screen-reader, contrast or OS text-setting audit. A fresh profile does not
exercise an actual historical database upgrade; older-result explanations
retain their existing component/native test evidence.

Both successful-run launches accepted exactly one graceful close each, exited
zero within the existing ten-second deadline and passed strict eight-stage
shutdown trace validation. Uninstall removed only the owned installation and
registration. Its independently retained database is byte-identical through
uninstall; all **671 protected personal/Foundation Smoke profile files** and
all **36 approved source fixtures** match their baselines. No owned process or
host lease remains.

## Source and check boundaries

The note-value implementation is PR396's tested `882cfbe`. Integration commit
`7bb5674` incorporates the accepted docs/jsdom main history; its only file
differences from that tested runtime head are the client package manifest and
lockfile for jsdom 30.1.3. The layout/regression commit is `55d8758` above it.
Final source `1de0a6c` incorporates the completed dependency merge wave through
runtime integration `4f29035`: React Router 8.5.1 and lucide-react 1.66.0,
with main pinned at `e952779`. Compiled input changes since `55d8758` are
the client manifest/lockfile; native/parser sources are unchanged. The final
package is built from this exact integrated source. Uncommitted review documents
and the owner's two planning drafts are outside the compiled graph; the drafts'
original hashes remain preserved. Package configuration differs from production
only for the explicit review identity/window/unsigned analysis bundle.

Local frozen-install validation at the final integrated source `1de0a6c`
passes all **437 client tests in 37 files**. The same source passes production
frontend typecheck/build, exact toolchain checks, sidecar and NSIS package build,
then the complete installed workflow. The installed before/after regression and
full workflow above supplement PR396's prior parser/storage/native CI; they do
not replace normal full CI on this PR's final head. Scope is CSS, a dependency-free
installed layout probe and documentation/evidence. No Rust parser, schema,
protocol, note-value set or production capability changes in this fix.

Private raw attempts are retained. Three earlier orchestration failures exposed
an unsupported-result shape assumption, the four-row Library page size and
incomplete debugger keyboard input. The first stderr failure interrupted the
controller before its cleanup ran; verified operator recovery sent the owned
app its one graceful close, validated its trace and preserved/uninstalled it.
Later failures cleaned up automatically. Two earlier old-source successful
runs retain their limited overflow-only results; a separate old-source run
then demonstrates the new readability regression failing. None is erased or
substituted for the corrected-source successful run above. A corrected-layout
success at `55d8758` and a router-integrated success at `41c4e2d` are also
retained separately. The final run above repeats the complete workflow after
the owner confirmed the dependency merge wave was finished.

## Storage and handoff

Owner `/root` reused the existing `desktop-feature-tests` cache sequentially;
the primary development `target` is untouched. Absolute cache/evidence paths
and ownership are recorded in the private handoff. No new permanent compiler
cache was created. Installed runs hold the shared Foundation Smoke host lock
and use a dedicated review identity. They are correctness runs; no resource
timing or quiet-host qualification is claimed.

Before building, 1,764 verified, single-link September compiler-library/metadata
files were retired from the previous probe cache, reclaiming **2.39 GiB measured**.
All 1,624 protected hashes and source/worktree state were verified. Executables,
debug symbols, current outputs, primary/desktop caches, source and fixtures
were preserved. Reusing those retired old probe profiles requires recompilation.
Compiler hardlink aliases in the selected desktop cache were replaced by
independent, hash-verified copies before its write windows; no retained binary
is hardlinked to a writable cache.

The first package preflight passed at 34.01 GiB free with a 3 GiB allowance;
the fixed-source incremental window passed at 32.67 GiB with 0.75 GiB allowed.
Fresh installed preflights also passed. The router integration build passed at
31.96 GiB with a 0.50 GiB allowance. The settled dependency build passed at
31.92 GiB with a 0.50 GiB allowance; its successful uninstall observed 31.63 GiB
free. Capacity fluctuates and future work requires a new preflight
that maintains the 30 GiB reserve after its own output estimate. G: was unused.

All raw receipts, packages, independently copied binaries, databases, closed
profiles, fixture copies and inventories remain privately retained outside
source/compiler caches under `installed-notes-20261104`. Public results omit
private paths, process/root/snapshot identifiers and database contents. Cache
write windows and installed host leases are closed. Preserve the evidence and
owner drafts; no automatic cleanup service is installed.

## Next development work

Finish normal PR CI/review and reconcile any later main changes without
promoting these dated installed results. Continue Phase 3 with the planned
cross-build compatibility pilot: preregister exact builds/fields/expectations,
compare genuine saves and publish per-field findings before expanding support.
Automation #324, broader dependency resolution, mixer/arrangement coverage,
resource #259/#306 decisions and the full phase checkpoint remain open.
