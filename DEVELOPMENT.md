# Development and delivery

Status: **Accepted workflow; Phase 1 will add executable tooling and CI**

## Working agreement

All product work follows:

```text
issue -> feature branch -> focused commits -> pull request -> CI -> review -> merge
```

Do not develop major features directly on `main`. Do not start a later phase
until the current phase review checkpoint is accepted. A phase may use multiple
small/medium PRs; each PR should represent one coherent concept and remain
revertible.

### Branches and commits

- Branch prefixes: `docs/`, `chore/`, `feat/`, `fix/`, `test/`, `spike/`.
- Include the issue number once issues exist, for example
  `feat/42-scan-root-persistence`.
- Keep formatting/refactors separate from behavior when practical.
- Commit generated lockfiles and migrations; do not commit generated build
  products, local databases, logs, credentials, personal paths, FLPs, or audio.
- Prefer squash merge for one-concept PRs; preserve separate commits only when
  they have lasting review value.

### Pull request requirements

Every PR uses [.github/pull_request_template.md](.github/pull_request_template.md)
and contains:

- summary and motivation;
- related issue;
- focused change list;
- test commands and evidence;
- screenshots/video for visual work, including relevant states and breakpoints;
- known limitations and intentionally deferred work;
- data/migration, security/privacy, accessibility, and documentation impact.

At least one review and all required checks are expected before merge. A PR may
be intentionally experimental only when labeled as a spike, produces a written
decision/result, and does not silently become production architecture.

## GitHub configuration proposed

Protect `main` with:

- pull requests required, one approving review, stale approvals dismissed after
  material changes;
- required status checks with branches up to date;
- resolved review conversations;
- no force pushes or deletion;
- linear history and signed commits/tags if practical;
- merge queue when parallel contribution warrants it;
- CODEOWNERS for Rust/native, parser, sync/security, and migrations once owners
  exist.

Verification on 2026-09-04 found this configuration unavailable for the current
private repository: both GitHub's branch-protection and repository-rulesets APIs
returned HTTP 403. The product owner declined GitHub Pro and accepted a manual
governance exception for the private GitHub Free repository. This does not claim
equivalent enforcement: the maintainer must still use issues, feature branches,
PR checklists, test evidence, and all available CI; avoid direct development,
force-pushes, and deletion of `main`; and request external review when an
eligible reviewer is available. Revisit enforced protection before regular
collaboration or public release.

Use labels by type (`epic`, `feature`, `bug`, `spike`, `docs`, `security`), area
(`desktop`, `scanner`, `parser`, `database`, `client`, `pwa`, `sync`), and phase.
Milestones represent phase review gates, not arbitrary dates.

The Phase 0 epic is GitHub issue #1, with review units #2–#5 and merged pull
requests #6–#9. After architecture acceptance, Phase 1 was created as epic #10
with issues #11–#18; the authoritative scope remains in
[ROADMAP.md](ROADMAP.md).

## Toolchain

### Workspace

- pnpm workspaces with one lockfile; use Corepack and pin `packageManager`.
- Cargo workspace with `rust-toolchain.toml` pinned to an accepted stable
  toolchain.
- Node 24 LTS for development/CI at the Phase 0 date. Node's release page marks
  installed Node 26 as Current, so it is not the production baseline yet.
- Python in an isolated environment for parser research/builds. Test and pin a
  supported non-EOL version (candidate 3.11) during P0-B; do not rely on the
  system Python.

Exact library versions are selected and locked in Phase 1, not copied from a
time-sensitive architecture document.

### Quality tools

| Area | Proposed tools |
| --- | --- |
| TypeScript/React | TypeScript strict, ESLint (`typescript-eslint`, React Hooks, JSX accessibility), Prettier |
| Unit/component tests | Vitest, React Testing Library, `user-event`, axe integration |
| Browser E2E/visual | Playwright with desktop and iPhone viewport projects; real iPhone manual matrix for platform behavior |
| Rust | `rustfmt`, Clippy with warnings denied in CI, built-in tests, `cargo-audit`/advisory scanning |
| Python sidecar | Ruff format/lint, mypy or Pyright, pytest, coverage, dependency audit |
| SQL | Numbered SQL migrations, SQL formatter/lint conventions, repository integration tests |
| Documentation | markdownlint, Mermaid render check, link checker with retries/allowlist |
| Supply chain | Dependabot/Renovate decision, dependency review, CodeQL, secret scanning, SBOM on release |

Do not add a monorepo task orchestrator until pnpm/Cargo commands become slow or
duplicated enough to justify it.

## Local prerequisites

Tauri's Windows prerequisites include Microsoft C++ Build Tools, WebView2, Rust,
and Node. The current development machine check on 2026-09-04 found Node 26.4.0
and Python 3.8.5, but no `rustc`, `cargo`, `py` launcher, or `corepack` on PATH.
Phase 1 setup must install/verify:

1. Microsoft C++ “Desktop development with C++” build tools;
2. current patched WebView2 runtime;
3. Rust stable MSVC through rustup;
4. Node 24 LTS plus pinned pnpm/Corepack setup;
5. an isolated, pinned Python 3.11 research environment; no PyFLP dependency in
   Phase 1;
6. the SQLite binding's embedded version is 3.51.3 or a documented official
   fixed backport before multi-connection WAL is enabled.

Document exact commands and verify them in `DEVELOPMENT.md` during the relevant
PR; do not make global machine changes as part of this architecture phase.

## Test strategy

### Pyramid

1. **Fast unit/domain tests (largest layer)**
   - filename normalization/version suffix parsing and candidate blocking;
   - grouping/export scoring, confidence bands, rejection memory;
   - duration/bar calculations and uncertainty rules;
   - rating/priority/progress/workflow invariants;
   - search query parsing/filter composition;
   - activity-session heuristic with fake clock/process/file signals;
   - sync operation canonicalization, idempotency, merge/conflict semantics;
   - path normalization/containment and log redaction.
2. **Component/accessibility tests**
   - keyboard and touch interactions, focus restoration, labels, live regions;
   - loading/empty/error/offline states and reduced motion;
   - domain views against fake platform ports.
3. **Integration/contract tests**
   - SQLite repositories, indexes, transactions, FTS, migrations/backups;
   - scanner against temporary directory fixtures and synthetic event streams;
   - parser protocol, process crash/timeout/restart, normalized golden results;
   - desktop/PWA sync adapters against a deterministic fake Drive service;
   - cross-runtime sync golden vectors.
4. **End-to-end tests (smallest layer)**
   - onboarding -> add root -> scan -> discovered project -> project detail;
   - metadata edit persists across restart;
   - corrupt FLP does not block another file;
   - later: open project, Kanban move, offline PWA edit, reconnect/conflict.
5. **Manual/platform qualification**
   - signed Windows installer/update/uninstall and WebView2 variants;
   - Google Drive mirrored/streamed/disconnected roots;
   - real FL Studio open/process/save behavior;
   - real installed iPhone PWA OAuth, offline storage, audio, and update flows;
   - future signed/notarized macOS build.

### Scanner test rules

- Abstract watcher input so deterministic tests feed create/modify/rename/remove,
  duplicate, overflow, error, and out-of-order sequences.
- Integration tests use unique temporary directories and await observable job
  states; never use arbitrary sleeps as correctness.
- Test burst writes, locked files, root removal/recovery, symlink/reparse loops,
  permission failures, Unicode/long paths, and reconciliation after dropped
  events.
- Hash every FLP fixture before and after scanning/parsing to prove the app did
  not mutate it.

### Parser fixture rules

- Prefer purpose-built minimal FLPs, upstream-public fixtures with compatible
  attribution/license, and malformed byte fixtures designed for the test.
- Maintain a manifest containing source/provenance, license, FL version,
  expected features, SHA-256, and privacy approval.
- `.gitignore` blocks FLP/audio by default. A fixture PR must explicitly
  force-add an approved file, update the manifest, and receive privacy review.
- Tests never enumerate the user's real project roots.
- Personal production projects are not CI fixtures unless the owner explicitly
  approves a minimized/sanitized derivative in writing.

### Coverage philosophy

Coverage thresholds apply to deterministic domain/core packages, not generated
bindings or trivial glue. Mutation/property testing is more valuable than line
coverage for matchers, query parsers, and merge rules. Every bug fix adds the
smallest regression test that would have caught it.

## CI proposal

CI is introduced incrementally with the code it can verify.

### Required pull-request checks

| Job | Runner | Scope |
| --- | --- | --- |
| `docs` | Ubuntu | Markdown, Mermaid, links, privacy pattern scan |
| `client` | Ubuntu | frozen install, lint, typecheck, unit/component tests, production build |
| `rust-core` | Ubuntu | format, Clippy, unit tests for portable crates |
| `windows-core` | Windows | native scanner/storage integration and Tauri compile |
| `parser` | Windows + pinned Python | lint/type/test, public fixtures, protocol smoke |
| `migration` | Ubuntu/Windows | create latest DB and upgrade supported historical snapshots |
| `e2e-web` | Ubuntu | critical client flows with fake ports; screenshots/artifacts on failure |
| `security` | Ubuntu | dependency review/advisories, secret/privacy scan, CodeQL as configured |

Avoid an expensive full installer build on every tiny PR until build duration is
measured. Run a scheduled/manual Windows packaging smoke and make it required on
release candidates. Add a macOS compile/package lane before declaring macOS
portability, not after platform-specific assumptions accumulate.

### Release workflow

- Trigger only from an approved version tag/manual protected environment.
- Re-run locked tests, build the parser, generate SBOM, build Tauri installers,
  sign, verify signatures/install, and publish a draft release.
- Signing/OAuth deployment credentials are environment secrets unavailable to
  forked/untrusted PR code.
- Record toolchain, dependency lock hashes, and artifact checksums.
- Never publish from `main` merely because a commit landed.

GitHub Actions matrices are useful for platform/runtime variation, and Tauri's
official action supports platform artifacts. Pin third-party actions to reviewed
SHAs even when documentation examples use floating major tags.

## Issue design

An issue is a reviewable outcome, not a single function or a whole phase. Each
feature issue should include:

- user problem/outcome and non-goals;
- acceptance criteria including states and accessibility;
- data/security/privacy impact;
- test expectations;
- dependencies/related ADRs;
- screenshots or diagrams when relevant.

Spikes have a time/size bound, explicit questions, artifacts, and a decision
deadline. A spike does not ship product behavior.

## Definition of done for a PR

- Acceptance criteria are met and later-phase scope is not pulled in.
- Automated tests pass and risk-appropriate manual evidence is attached.
- Error, empty, loading, and offline states are covered where applicable.
- Keyboard, focus, labels, contrast, scalable text, and reduced motion are
  considered for UI changes.
- Migrations have upgrade/backup tests and no unreviewed data loss.
- Logs and sync payloads contain no secret/personal data regression.
- Documentation/ADRs are updated with the code, not promised later.
- Known limitations and follow-up issues are explicit.

## Phase review checkpoint

At the end of a phase, prepare a short review issue or discussion containing:

- merged PRs and acceptance-criteria mapping;
- demo/screenshots and test/release evidence;
- unresolved bugs/risks and deliberate deferrals;
- documentation/ADR changes;
- proposed next-phase issue set.

Stop until the owner accepts the checkpoint.

## References

- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri GitHub Actions distribution](https://v2.tauri.app/distribute/pipelines/github/)
- [GitHub Actions Node build/test guidance](https://docs.github.com/en/actions/tutorials/build-and-test-code/nodejs)
- [GitHub Actions matrix syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)
- [GitHub CodeQL language support](https://docs.github.com/en/code-security/concepts/code-scanning/codeql/codeql-code-scanning)
- [GitHub protected branch availability](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)
- [GitHub repository rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets)
- [Node release status](https://nodejs.org/en/about/previous-releases)
