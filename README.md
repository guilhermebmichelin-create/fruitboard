# Fruitboard

A local-first FL Studio project library and production tracker for organizing,
analyzing, and finishing music.

## Current status - 2026-11-21

Reviewed merged baseline: [28fc508](https://github.com/guilhermebmichelin-create/fruitboard/commit/28fc508).
Phases 0 and 1 are accepted. The owner marked
[Phase 2 accepted with known gaps](docs/review/phase-2-integration/acceptance-2026-09-14.md),
and P2-08 remains Partial. Phase 3 FLP Intelligence is active. The proposed
resource budgets, Phase 3 exit, production activation and public distribution
still require separate decisions.

- **Windows development workflow:** the explicit analysis-jobs build supports
  local NTFS discovery, read-only FLP analysis and Library inspection.
  The default desktop leaves scanning and FLP analysis disabled. The
  [field reliability matrix](docs/review/flp-intelligence-readiness/field-reliability.md)
  records the supported fields and limitations across 46 fixtures and three
  exact saved builds. [PR #404](https://github.com/guilhermebmichelin-create/fruitboard/pull/404)
  displays the detected saved version when a build is unverified.
- **Library:** the development app displays saved project facts, channels,
  sample and plugin references, Plugin Explorer groups, pattern names and
  counts, qualified ordinary-note sequences and ordinary mixer record names.
  Explicit sample checks observe eligible absolute paths and project-relative
  candidates within the selected local NTFS root. Placeholder, external-folder
  and cloud resolution remain open.
  [Installed notes review](docs/review/installed-notes-397/README.md) ·
  [Installed sample checks](docs/review/project-relative-installed-20261028/README.md)
- **Saved automation counts:** Project Details displays Automation Clip and
  editable control-point totals for the qualified FL Studio 26.1.0.5530 /
  96 PPQ layout. The installed review checked confirmed zero, one clip with
  two points, an unverified-build explanation and persistence across restart.
  Curve targets, coordinates, Playlist placement and MIDI summaries remain open.
  [PR #429](https://github.com/guilhermebmichelin-create/fruitboard/pull/429) ·
  [Installed validation](https://github.com/guilhermebmichelin-create/fruitboard/actions/runs/44521556690)
- **Resources:** batching current Plugin Explorer reads reduced the observed
  native 2,000-entry p95 from 491 ms to 161 ms. Both windows exceeded the
  background CPU guard. A current-source repeat returned the expected results
  through 2,001 entries, with the 2,000-entry timing still outside the noise
  guard. Installed checks verified the 2,001-entry limit and a 64 MiB analysis
  job. Startup and whole-app resource qualification remain open.
  Separately, the scanner's historical warm p95 remains 10,173 ms against
  its 10,000 ms target.
  [PR #430](https://github.com/guilhermebmichelin-create/fruitboard/pull/430) ·
  [Phase 3 resource plan](docs/review/phase3-qualification-306/budget-proposal.md)
- **Repository:** GitHub requires pull requests and the ten configured checks
  on up-to-date branches. Secret scanning, push protection and Dependabot are
  enabled. CodeQL and dependency review are informational.
  [Development workflow](DEVELOPMENT.md#github-governance) ·
  [Security reporting](SECURITY.md#vulnerability-handling)

The [Phase 3 follow-through](docs/review/development-status-20261121/README.md)
maps open issues to merged evidence and records the next Windows checks,
the cross-build prerequisite and the decisions still needed for the checkpoint.
The [November 11 assessment](docs/review/development-status-20261111/README.md)
and [archived README status](docs/archive/readme-status-20260929.md) preserve
historical evidence at their recorded sources.

## Development

The repository pins Node, pnpm/Corepack, Rust, and an isolated Python research
environment. Start with [DEVELOPMENT.md](DEVELOPMENT.md); after installing the
pinned tools, `pnpm check` is the single local verification entry point and
`pnpm dev` launches the Windows desktop shell.

CI mirrors that gate through stable per-area jobs. The working agreement
requires ten checks before an owner merge, now enforced by GitHub on `main`;
see [the live governance verification and historical policy](DEVELOPMENT.md#github-governance).

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
