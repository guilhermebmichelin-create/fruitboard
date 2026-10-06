# Sample presence implementation sequence

Status: design accepted by PR279; policy slice #280 merged as PR281;
Windows slice #282 merged as PR283; app slice #284 merged as PR285 on 2026-10-06,
`d19587f52132f31065bbaab97a572d3c5487cf00`. All three bounded slices are delivered;
broader resolution remains open.
Planning source: merged PR268, `91261525383ce79adb090550329167ef347f7253`.
Start implementation from the reviewed design merge, record its actual SHA/tree
and validate each final PR head against that boundary. Do not start from an
unreviewed documentation branch or claim this plan is an implemented capability.

## Slice 1: Policy and typed reports

The [policy library review](../sample-presence-policy-280/README.md) implements
this slice without app activation, starting from main
`6054790e0a7b2f09ece4644b479f68acf6805b13`. Exact final-head checks are in its PR.
PR281 merged this slice on 2026-10-05 as
`aecf880e515540c3bce012c0508d81dba2c31e68`.

Owned files: a dedicated `crates/sample-presence/` library and its tests, root
`Cargo.toml`/`Cargo.lock` workspace entries only as needed. This is the proposed
home of the bounded metadata port; choosing another location requires the same
explicit boundary in the implementation PR.

Implement strict candidate syntax/root-component eligibility, the four outcomes,
fixed unchecked reasons, ordered per-channel reports and request/cancellation/
budget state. Inject root/parent capabilities, filesystem results and monotonic
time. The port has no content-read or mutation method. Test negative authority,
budget/deadline/blocked-worker behavior and duplicates rather than mirroring
internal helper code. This slice must not activate a native command or UI probe.

Local native checks are required for changed Rust: pinned fmt, warning-denied
all-target Clippy and the new crate's tests. Run required root checks and normal
updated-head CI. Existing accepted-source CI applies only to unchanged crates;
it cannot validate the new policy. Retain the source/tree and exact check logs.

## Slice 2: Windows metadata authority

The [Windows review](../sample-presence-windows-282/README.md) maps implementation
and real Windows evidence from that PR281 merge. The adapter is inactive in the
app. A narrow enumeration correction uses NT existing-only disposition rather
than the similarly named Win32 option; regression tests reproduce the former
creation behavior and verify that disappeared names remain absent.

Owned files: the new library's Windows implementation/tests and narrowly scoped
shared native helper changes only after their authority impact is reviewed.
Inspect `crates/filesystem-enumeration/src/lib.rs` and
`crates/analysis-execution/src/native.rs` as existing boundaries; do not silently
add content access or arbitrary path inputs to either one.

Implement explicit root/source qualification and exact-child handle-relative
metadata operations, attribute/identity/case checks, bounded owned guards,
authoritative absence and final revalidation. Add real Windows replacement/
reparse/rights tests and deterministic injected offline/recall/denial/time cases.
Confirm no recursion, hydration, network path, content read or sample write.

Run crate/native warning-denied Clippy and meaningful Windows tests. Any changed
shared helper also needs its existing enumeration/analysis tests and unchanged
held-source/source-publication behavior. Review native authority against every
case in [the matrix](authority-review.md) before integrating the command. Keep
source/binary hashes and independent evidence copies where a binary is retained.

## Slice 3: Authorized command and Library display

The [app review](../sample-presence-ui-284/README.md) records implementation
from PR283 merge `b8c065c3f10b6b64d96b8224fadcdbb4e19af3d6` and its validation
boundary. Broader resolution, G5 and Phase 3 remain open.

Owned files: desktop `foundation/mod.rs`, new sample-presence command/host module,
command registration/permission/capability files and desktop Cargo feature wiring;
the existing storage metadata read/fence API only if an additive read is needed;
client `platform/{contracts,tauri,fake}.ts`, `library/contracts.ts`, new strict
report decoder, `library/{ProjectSamples,ProjectDetailsPanel}.tsx` and relevant tests.
Name any additional file before changing it. No parser, snapshot schema, audio
capability, global filesystem permission or permanent result table is planned.

The host resolves references from the current authorized snapshot, releases its
database guard during I/O and captures/rechecks all source/root/session fences.
One worker, explicit cancellation/check-again and late-result correlation are
required. The renderer submits identities/fingerprint only; opening/refreshing
details remains a saved-results read. Keep default/PWA disabled behavior.

Run full pinned Windows `pnpm check`, changed native feature tests/Clippy,
actual command-to-metadata-port integration, strict decoder/race/a11y tests and
rendered desktop/narrow/200%-text states. Exercise a fresh isolated installed
development profile through check/cancel/stale/reinstall behavior; preserve the
owner's real profile. Use privately generated metadata-only target files and
record their hashes; new genuine public FLPs require their own exact approval.
Hash the full approved corpus before/after. Require final-head CI and owner PR
review before merge. Document rollback: remove command/view state, keep original
immutable metadata and reference display intact; no migration is needed.

## Host, cache and storage requirements for every slice

Every actual implementation/validation prompt must specify owned files/checkout,
exact baseline and final source boundary, local build need, required checks and
which unchanged-source CI evidence applies. It must record the absolute cache
path and a single owner in its private handoff. The current reusable validation
cache is `bounded-probes`, owned by Codex `/root`; use the absolute path recorded
in the task handoff, not a fresh multi-gigabyte cache for each slice.

Inspect existing worktrees/caches first. Before any native build or fixture
generation, run `scripts/preflight-build-storage.mjs` with explicit source,
build, cache and evidence paths and estimated additional output. Maintain at
least 30 GiB free on every used volume after that output. Set `CARGO_TARGET_DIR`
explicitly. Allocate one exclusive heavyweight window; never share a writable
Cargo target concurrently. Installed/performance tests use their existing
exclusive host coordination, with qualification workloads running alone.

Store logs, reports, private test files/databases and independent evidence
binaries outside disposable compiler caches. Never hardlink a retained binary
to writable Cargo output. Preserve private sources/profiles, corpus bytes,
dependencies/toolchains and prior evidence. Inventory/resolve/check any proposed
cleanup before acting; no blanket worktree/target deletion. Handoff must identify
cache ownership, reusable outputs, obsolete intermediates (if any) and retained
evidence with source/build provenance and hashes.

This design task itself is documentation-only: no local build, writable-cache
window or fixture generation is needed. Its private handoff records the current
absolute locations, capacity, doc checks and unchanged-product CI boundary.

## Completion boundary

The first delivered slice is an honest time-of-check report for permitted exact
absolute paths. It does not complete FL Studio dependency resolution. Relative
paths, placeholders, external-folder grants, moved-file search, cloud/DriveFS,
audio inspection and continuously current availability need separate qualified
scopes. Update G5's partial-delivery map when code ships; keep the broader
Phase 3 checkpoint and other compatibility/resource gaps open.
