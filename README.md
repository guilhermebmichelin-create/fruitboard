# Fruitboard

A local-first FL Studio project library and production tracker for organizing,
analyzing, and finishing music.

## Project status

Fruitboard's **Phase 0 architecture is accepted**; the manual governance
exception recorded for the private GitHub Free repository was superseded on
2026-09-07 when the repository became public, and enforced branch protection is
now proposed (pending owner approval; see [DEVELOPMENT.md](DEVELOPMENT.md#proposed-enforced-branch-protection-pending-owner-approval)).
**Phase 1 is
accepted**: the reproducible Tauri/React shell now includes Rust-owned local
SQLite, one persisted startup-view preference, automated quality/security
gates, and a Windows packaging smoke. It still has no scanner, parser, project
workflow, sync, or PWA. Phase 2 proceeds under epic #33: scan-root storage with
a native folder picker (#34), root settings with inline Preferences onboarding
(#35), the bounded reconciliation reference core (#36), and the durable
execution foundation (#38) and the fenced staging/publication storage boundary
for #40 have landed. Filesystem traversal, watcher integration, the Library
journey, production scanning, and parser work remain ahead.

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

CI mirrors that gate through six stable jobs. Repository policy currently
relies on manual merge governance; enforced branch protection is available on
the now-public repository and is proposed for adoption, pending owner approval;
see [the governance details](DEVELOPMENT.md#github-governance).

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
