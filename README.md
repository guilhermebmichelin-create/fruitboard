# Fruitboard

A local-first FL Studio project library and production tracker for organizing,
analyzing, and finishing music.

## Current status - 2026-10-27

Merged baseline: `ce4331e9004f1a7f363f45106adaa192d1f635fb`
([PR #334](https://github.com/guilhermebmichelin-create/fruitboard/pull/334),
installed resource evidence). The application includes the current-metadata
read-path change from [PR #321](https://github.com/guilhermebmichelin-create/fruitboard/pull/321),
merged as `803ef43`. It consolidates current Plugin Explorer reads under
existing freshness checks; its measurements did not establish a speedup or
qualify resource limits. The [current status assessment](docs/review/development-status-20261027/README.md)
records the phase gates, evidence boundaries, live tracker discrepancies, and
next work.

- **Phases**: Phase 0 was accepted 2026-09-04, Phase 1 on 2026-09-06, and
  Phase 2 accepted with known gaps on 2026-09-14
  ([acceptance record](docs/review/phase-2-integration/acceptance-2026-09-14.md)).
  P2-08 remains Partial. Phase 3 FLP Intelligence is active; its budgets and
  exit are not accepted. Later phases, production activation, and public
  distribution remain separate decisions.
- **Parser and analysis**: Rust was selected on 2026-09-28
  ([ADR-002](docs/adr/002-flp-parser-process.md)). The parser is packaged in
  the unsigned Windows development build, and durable analysis is available
  through the explicit `analysis-jobs` composition. The default desktop does
  not scan or read FLP content automatically. See the
  [readiness review](docs/review/flp-intelligence-readiness/README.md) for
  exact field and build limits.
- **Library**: the development workflow displays saved facts, channels, inert
  sample/plugin references, Plugin Explorer groups, pattern names/counts,
  note-record counts, and qualified mixer record names. Explicit sample checks
  observe metadata only for eligible literal absolute paths inside the
  selected local NTFS root; relative, placeholder, external-folder, and cloud
  resolution remain open. See the
  [sample-check review](docs/review/sample-presence-ui-284/README.md) and
  [mixer slice](docs/review/library-mixer-inserts-304/README.md).
  Draft [PR #348](https://github.com/guilhermebmichelin-create/fruitboard/pull/348)
  implements the owner-accepted same-root project-relative candidate rule;
  it remains unmerged, with required checks and installed validation pending.
- **Compatibility and automation**: the approved parser corpus now has 21
  files. [PR #320](https://github.com/guilhermebmichelin-create/fruitboard/pull/320)
  adds the exact-build F22 cloned-Sampler case; [PR #323](https://github.com/guilhermebmichelin-create/fruitboard/pull/323)
  publishes F23 GUI evidence. F23 shows one automation clip with three visible
  control points, but the saved event framing does not establish an object or
  point-count grammar. Automation extraction and display remain unqualified;
  [issue #324 research](docs/research/parser-automation-counts-324-result.md)
  defines the evidence still needed.
- **Resource and scanner status**: scanner qualification remains unaccepted;
  the historical warm nearest-rank p95 was 10,173 ms against the unchanged
  10,000 ms target on source `69f27f6`. The
  [current resource review](docs/review/phase3-qualification-306/README.md) is
  characterization, not qualification. The
  [October 14 follow-up](docs/review/resource-repeat-20261014/README.md)
  records an unexplained installed shutdown timeout and a successful debugging
  repeat that did not establish its cause or a product fix. Shutdown diagnostics
  and trace validation have since shipped in PR329/PR331. The
  [current installed review](docs/review/resource-current-20261018/README.md)
  adds guarded IPC/UI observations. Open
  [PR #335](https://github.com/guilhermebmichelin-create/fruitboard/pull/335)
  records a stopped remainder run after a failed CPU guard. Resource decisions,
  remaining measurements and the Phase 3 checkpoint stay open.
- **Reliability tracker**: GitHub still lists
  [#220](https://github.com/guilhermebmichelin-create/fruitboard/issues/220),
  [#221](https://github.com/guilhermebmichelin-create/fruitboard/issues/221),
  [#222](https://github.com/guilhermebmichelin-create/fruitboard/issues/222),
  and [#223](https://github.com/guilhermebmichelin-create/fruitboard/issues/223)
  as open, although their scanner lifecycle, response-validation,
  event-recovery, and bounded-publication fixes shipped in
  [PR #225](https://github.com/guilhermebmichelin-create/fruitboard/pull/225).
  The [review note](docs/review/application-fixes-20260929.md) maps each fix
  to its tests. This status update does not change issue states.
- **Repository**: public since 2026-09-07. `main` protection requires pull
  requests and the ten configured checks, with branches up to date, linear
  history, and no force pushes or deletion. Secret scanning, push protection,
  and Dependabot are enabled; CodeQL and dependency review are informational.
  Use the [security policy](SECURITY.md#vulnerability-handling) for
  vulnerability reports.

The [archived README status](docs/archive/readme-status-20260929.md) preserves
the superseded status blocks, source pins, installed evidence and links.
The [October 17 assessment](docs/review/development-status-20261017/README.md)
preserves that date's next-step recommendation, now superseded by this update.
Dated evidence remains tied to its tested source; a current documentation
update does not rerun an installed journey or accept a phase.

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
