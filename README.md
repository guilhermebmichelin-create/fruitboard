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
`b2fb62c36a057985ab0eba02458f037fcb96c215` (PR #106, merged 2026-09-10).
Since PR #89 (`0b7612d`), main has merged PR #90 (`f487aa1`), PR #100
(`7980b75`), PR #92 (`300c2a4`), PR #93 (`106335a`), PR #102 (`892920d`),
PR #96 (`326fb0a`; proposal only, not an approved scale design), PR #101
(`67bdc76`; historical stall baseline), PR #103 (`55668da`), PR #105
(`f63a1d3`; `smol-toml` 1.7.1 security fix for GHSA-7w5x-hrqm-74c2),
PR #94 (`d342ec1`), PR #98 (`7e0e9f6`), PR #95 (`ee720af`),
PR #97 (`ff5d8ee`), PR #104 (`cccaa67`; queue-stall fix, 2 files), and
PR #106 (`b2fb62c`; fixed installed validation, docs-only).
Current CI: Foundation `34427729043` on `b2fb62c` success (post-merge);
Foundation `34427283235` + Packaging `34427283311` on PR #106 head `9b86cbf`
success; Foundation `34426947016` on `cccaa67` success (post-merge);
Foundation `34425398811` on `ee720af` success (post-merge).

Implemented behavior, automated evidence, installed evidence, and acceptance
stay separate — not Phase 2 acceptance. P2-08 remains Partial (PR #85's older
`Complete` row is superseded; no full-criterion acceptance recorded).
Production scanning stays hidden until P2-03 through P2-08 have integrated
evidence.

Fixed installed validation lives in merged PR #106 (`b2fb62c`, canonical
`installed-journey/run-20260909-fixed-validation.md` with corrected
provenance; this PR carries no duplicate copy). S1–S4 PASS, S6 PASS, S5
PARTIAL (successor convergence; same-job not demonstrated; repro IDs in #106;
tracked as open issue #107). No acceptance claimed.

Remaining decisions only: F1 fixture, F2 qualification, F3 scope, platform
qualification/exclusions (#47/#48), P2-10 boundary, and final acceptance
(criterion-by-criterion P2-01–P2-12 including S5 disposition). History,
provenance, and the full P2 table are in the [current reconciliation
report](docs/review/phase-2-integration/reconciliation-2026-09-08.md)
and [installed-app journey checklist](docs/review/phase-2-integration/installed-app-journey-checklist.md)
track the remaining gates. Historical failed replays stay intact as history.

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
applies, and Agent 1 owns merges per the user's explicit agent-merge
authorization (owner retains acceptance and decision authority; branch
protection is unchanged).
