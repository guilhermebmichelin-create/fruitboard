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
sync, or PWA. Phase 2 proceeds under epic #33. The verified source baseline for
the 2026-09-13/14 documentation refresh was
`71848732216d4ab4e13e73c820b2b8e4d17bddbe`, which includes the squash merges
of #110 (`e2948f1`), #111 (`914d7bd`), the merged #112 (`00884ba`), and #114
(`7184873`); that pin is historical, not the current head. The
[Phase 2 integration index](docs/review/phase-2-integration/README.md) carries
the dated post-acceptance and publication addenda that record later heads and
merge state. Historical merge, tested-source, and CI references remain in the
integration index and dated reports.

Implemented behavior, automated evidence, installed evidence, and owner
acceptance stay separate. The owner accepted Phase 2 with known gaps on
2026-09-14 ([acceptance
record](docs/review/phase-2-integration/acceptance-2026-09-14.md)); P2-08
remains Partial (PR #85's older `Complete` row is superseded) and no
full-criterion P2-08 acceptance is recorded. That acceptance does not activate
production scanning; activation remains a separate owner action.

The canonical installed validation remains the merged PR #106 report
`installed-journey/run-20260909-fixed-validation.md`: S1–S4 PASS, S6 PASS,
and S5 is the terminal-follow-up/successor path. Merged #110 separately
publishes Agent 2's queued/running/terminal, queued disable/remove, and
genuine running-lease same-job evidence; it remains installed evidence and
does not close #107 or accept Phase 2. The #111 NTFS report remains tied to its
tested product source even though the PR's product and evidence changes are
now merged.

The [Phase 2 integration index](docs/review/phase-2-integration/README.md),
[execution plan](docs/PHASE_2_EXECUTION_PLAN.md), and [2026-09-12 acceptance
packet](docs/review/phase-2-integration/acceptance-packet-2026-09-12.md)
carry the current dependency map, evidence boundaries, and owner decision
list. Historical failed replays remain historical and performance remains
unqualified.

The source sequence is now merged #110 -> #111 -> #112 -> #114. #112's
diagnostic changes and #114's executable qualification runbook/validator are
in main, but no qualification run or acceptance is recorded. PR #113 remains
an older synthetic candidate. Agents prepare and verify; the owner retains Phase 2 acceptance,
scope, budget, and issue-closure authority.

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

The opt-in virtual Google Drive scan-root experiment is described in
[its safety and validation note](docs/research/drive-virtual-experimental.md).
It is manual-scan-only, does not infer missing files, and has not passed a
DriveFS host run. Production scanning remains inactive by default.

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
applies, and merge authority is exercised only for reviewed, checked pull
requests; agents prepare and review changes, while owner acceptance and
decision authority remain separate. A documentation merge does not accept
Phase 2 or authorize production scanning.
