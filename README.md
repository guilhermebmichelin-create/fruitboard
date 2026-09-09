# Fruitboard

A local-first FL Studio project library and production tracker for organizing,
analyzing, and finishing music.

## Project status

Fruitboard's **Phase 0 architecture is accepted**; the manual governance
exception recorded for the private GitHub Free repository was superseded on
2026-09-07 when the repository became public, and enforced branch protection is
enabled on `main` (2026-09-07; see [DEVELOPMENT.md](DEVELOPMENT.md#enforced-branch-protection)).
**Phase 1 is
accepted**: the reproducible Tauri/React shell now includes Rust-owned local
SQLite, one persisted startup-view preference, automated quality/security
gates, and a Windows packaging smoke. It still has no parser, project workflow,
sync, or PWA. Phase 2 proceeds under epic #33. The current merged baseline is
`0b7612db3570e6235d4d2c86a678dd9004264f30` (PR #89, fetched
`origin/main`, re-verified 2026-09-09 local time with no new merges). PR #82
merged native watcher supervision
and shutdown recovery; PR #85 added worker-level durability and publication
fault coverage for P2-03/P2-05/P2-06/P2-07; PR #86 added the newer benchmark
triage re-run (recorded at the pre-#85 `51f45af` baseline); and PRs #88/#89 recorded that DriveFS and FAT32/cross-volume
qualification remain unavailable. The latest
[Foundation CI run](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34293250231)
is green for that exact `origin/main` commit, including all nine Foundation
jobs and the enabled desktop tests and warnings-denied Clippy.

These are implementation, automated-test, measurement, and partial installed-app
results, not Phase 2 acceptance. A packaged feature-enabled Windows application
was built from the exact merged `main` commit above and installed evidence was
captured in the unmerged [PR #92 head](https://github.com/guilhermebmichelin-create/fruitboard/commit/49a5e649c688ae767c9189801dd033f2288b61f5)
[run record](https://github.com/guilhermebmichelin-create/fruitboard/blob/49a5e649c688ae767c9189801dd033f2288b61f5/docs/review/phase-2-integration/installed-journey/run-20260908.md).
Persistence, paging, watcher follow-up convergence, cancellation retention, and
interrupted-work recovery were observed. Durable queued observation and
independent watcher-burst counting remain unverified in that record. D1 native
labeling, D2 cancelled Retry, and D3 exhausted Retry repairs are verified in
the still-unmerged draft [PR #95 head `bf0aeac`](https://github.com/guilhermebmichelin-create/fruitboard/commit/bf0aeac38ac46dbb90990611b940429e232f67ef)
([Foundation run 34309511415](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34309511415),
[packaging run 34309511404](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34309511404);
`pnpm check`, 134 client tests, 88 feature-on desktop tests, and the
`run-20260909.md` installed replay), which remains open and unmerged; its
replay did not retain a durable queued snapshot and leaves ACL, DriveFS,
cross-volume/FAT32, 100,000-entry, and burst-timing cases outside its scope.
Automated durable-queue and watcher-burst fences are pinned in unmerged draft
[PR #97 head `aba1812`](https://github.com/guilhermebmichelin-create/fruitboard/commit/aba18121f6fa6d80d5fcc67be6985277154969f5)
([Foundation run 34339792661](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34339792661),
[packaging run 34339792759](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34339792759));
that automated evidence does not supply the installed queued snapshot or burst
timing, which remain unverified. PR #92's first packaging attempt recorded `The installed
sidecar smoke timed out` ([attempt 1](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102303731791));
[rerun attempt 2](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/34299680193/job/102314720612)
passed. Preserve that incident in the provenance, but do not infer a product
fix or acceptance from the rerun alone.

The native watcher supervisor is merged behind the feature-gated `scan-console`
path. PR #85 contains an older evidence-row statement that calls P2-08
complete; the current reconciliation treats that as superseded because no full
criterion acceptance was recorded. The unmerged [PR #90 report](https://github.com/guilhermebmichelin-create/fruitboard/commit/17e571742634eceb16f09b166edd3d77b57fcf61)
keeps #47/#48 blocked, and the unmerged [PR #93 report](https://github.com/guilhermebmichelin-create/fruitboard/commit/59faefc2806a725368a59e7b6fc9be7f863f4fec)
adds contended diagnostic profiling, not an idle-host performance pass. The
stacked [PR #94 head `588867b`](https://github.com/guilhermebmichelin-create/fruitboard/commit/588867bca154e798f189f6c99de8a2668005d141)
(base `docs/41-performance-followup-20260908`) reports a contended median move
with a p95/max outlier regression and has no exact-head CI yet; it needs
combined validation plus a quiet-host A/B rerun before any qualification claim.
Unmerged draft [PR #98 head `15b7f17`](https://github.com/guilhermebmichelin-create/fruitboard/commit/15b7f170c579d42c4f23f9619949236af4971ba2)
independently validates that candidate on the contended host (both sides fail
the 10 s p95; no qualification claimed) with diagnostic cleanup pending Agent 1
review; its CI was separately dispatched. F1/F3 scale decisions and #47/#48
platform prerequisites remain unresolved; the bounded-scan scale design is
proposed separately in draft
[PR #96](https://github.com/guilhermebmichelin-create/fruitboard/pull/96) as a
proposal only, not an implementation authorization (independent-review
correction with unresolved mechanisms and validation gates included). Owner
acceptance remains open. Production scanning remains hidden until P2-03
through P2-08 have integrated evidence. The [current reconciliation report](docs/review/phase-2-integration/reconciliation-2026-09-08.md)
and [installed-app journey checklist](docs/review/phase-2-integration/installed-app-journey-checklist.md)
track the remaining gates. Historical reports remain historical records.

Tracking starts at
[Phase 2 epic #33](https://github.com/guilhermebmichelin-create/fruitboard/issues/33)
(Phase 1 epic
[#10](https://github.com/guilhermebmichelin-create/fruitboard/issues/10)
is closed on acceptance).

## Development

The repository pins Node, pnpm/Corepack, Rust, and an isolated Python research
environment. Start with [DEVELOPMENT.md](DEVELOPMENT.md); after installing the
pinned tools, `pnpm check` is the single local verification entry point and
`pnpm dev` launches the Windows desktop shell.

CI mirrors that gate through stable per-area jobs, enforced as required checks
on `main` by branch protection (enabled 2026-09-07); see [the governance
details](DEVELOPMENT.md#github-governance).

The current shell's information hierarchy, responsive evidence, accessibility
coverage, and intentional limitations are recorded in the
[Issue #13 visual review](docs/review/issue-13/README.md). The first persisted
workflow has separate [Issue #16 interaction evidence](docs/review/issue-16/README.md).
The [Issue #18 Windows evidence](docs/review/issue-18/README.md) feeds the
[Phase 1 checkpoint](docs/PHASE_1_REVIEW.md).

Start with [the Phase 1 review brief](docs/PHASE_1_REVIEW.md); Phase 0 remains
available in its [accepted review brief](docs/PHASE_0_REVIEW.md).

## Architecture documents

- [Architecture](ARCHITECTURE.md)
- [Data model](DATA_MODEL.md)
- [FLP parser boundary](FLP_PARSER.md)
- [License and distribution intent](LICENSE_INTENT.md)
- [Synchronization](SYNC.md)
- [Security and privacy](SECURITY.md)
- [Development workflow](DEVELOPMENT.md)
- [Roadmap](ROADMAP.md)
- [Architecture Decision Records](docs/adr/README.md)

## Product principles

- FLP files are read-only inputs. Fruitboard never edits their binary content.
- Core desktop functionality is local-first and requires no account.
- Google Drive metadata sync is explicit and optional.
- Extracted, inferred, and manually entered data remain distinguishable.
- Automatic grouping and matching suggestions are reversible.
- No telemetry or third-party tracking is enabled by default.

Distribution and third-party parser licensing remain gated; see
[LICENSE_INTENT.md](LICENSE_INTENT.md).

## Repository

The canonical repository is public since 2026-09-07 at
`github.com/guilhermebmichelin-create/fruitboard`. Public visibility changes the
repository, not the product rules: privacy rules are unchanged (no telemetry or
third-party tracking), FLP files remain read-only inputs, private paths and
project data stay out of commits and logs, review evidence discipline still
applies, and the owner still merges every pull request manually.
