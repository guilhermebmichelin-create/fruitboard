# Fruitboard

A local-first FL Studio project library and production tracker for organizing,
analyzing, and finishing music.

## Current status - 2026-10-09

Source baseline: `266327058fbf3f6dfd2cfa2fedd51ceb09ea3fa2` (merged PR #301).
The review dated 2026-10-08 used the earlier `b27b761` baseline.

- **Phases**: Phase 0 accepted 2026-09-04, Phase 1 accepted 2026-09-06, and
  Phase 2 accepted with known gaps on 2026-09-14
  ([acceptance record](docs/review/phase-2-integration/acceptance-2026-09-14.md)).
  Epic #33 is closed. Issues #38 and #40 carry the narrowed P2-08 evidence
  residual; P2-08 remains Partial. Phase 3 exit and resource budgets remain
  unaccepted; later phases are not authorized by these deliveries.
- **Parser and analysis**: Rust was selected on 2026-09-28
  ([ADR-002](docs/adr/002-flp-parser-process.md)). The real parser is packaged
  in the unsigned Windows development smoke, with durable native analysis
  in the explicit `analysis-jobs` composition. The default desktop has no
  scanner or automatic FLP content reads. The [readiness packet](docs/review/flp-intelligence-readiness/README.md)
  records the implemented workflow and its narrow compatibility.
- **Library**: the app displays current saved facts, channels, sample/plugin
  references, a saved Plugin Explorer, pattern names/counts and
  [pattern note-record counts](docs/review/library-pattern-notes-294/README.md)
  (PR #295). [Explicit sample checks](docs/review/sample-presence-ui-284/README.md)
  (PR #285) observe metadata only for eligible literal absolute paths inside
  the selected local NTFS root. Relative, placeholder, external-folder and
  cloud resolution remain open.
- **Mixer**: approved F18-F21
  [independent qualification](docs/research/parser-mixer-inserts-298-result.md)
  (PR #299) and bounded extraction/full native validation (PR #301) are
  delivered for the exact FL Studio 26.1.0.5530 eighteen-record layout.
  [Parser contract](crates/flp-parser/README.md#initial-native-result-validation)
  counts sixteen ordinary saved records, with honest missing-name states.
  Mixer storage projection and Library display remain open.
- **Product scope**: the Tauri/React shell, Rust-owned SQLite library, scan
  roots and feature-gated durable scanning are implemented. Production
  activation is still a separate owner decision. Kanban, search, project
  workflow, Google Drive sync and the PWA are not implemented.
- **Performance**: scanner qualification remains unaccepted; the historical
  warm nearest-rank p95 was 10,173 ms against a 10,000 ms target on source
  `69f27f6`. Later [native analysis/Explorer observations](docs/review/explorer-performance-259/README.md)
  do not replace scanner or whole-app qualification.
- **Repository**: public since 2026-09-07, with ten required branch-protection
  checks. Secret scanning, push protection and Dependabot are enabled;
  validity checks and non-provider patterns remain off. CodeQL and dependency
  review remain informational. Private vulnerability reporting is enabled
  (verified 2026-10-09); use the [security policy](SECURITY.md#vulnerability-handling).

The [archived README status](docs/archive/readme-status-20260929.md) preserves
the superseded status blocks, source pins, installed evidence and links.
Dated evidence remains tied to its tested source; a current documentation
update does not rerun an installed journey or accept a phase.

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
