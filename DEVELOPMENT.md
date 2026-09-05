# Development and delivery

Status: **Active workflow; Phase 1 adds executable tooling and CI incrementally**

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

- pnpm workspaces use one committed lockfile and Corepack verifies the exact
  package manager from `package.json`.
- The Cargo workspace uses a committed lockfile and the exact toolchain in
  `rust-toolchain.toml`.
- The Python 3.11 research environment is isolated in `.venv` and locked by uv.
  It has no PyFLP dependency.
- `tools/toolchain-policy.json` is the machine-readable source for cross-file
  pin checks and the SQLite WAL safety floor.

Pins selected and verified on 2026-09-04:

| Tool | Pin | Purpose |
| --- | --- | --- |
| Node.js | 24.20.0 LTS | JavaScript runtime and client tooling |
| pnpm | 11.25.0 | Workspace package manager |
| Corepack | 0.36.0 | Verifies and launches the pinned pnpm release |
| Rust | 1.98.1 MSVC | Native workspace, with `clippy` and `rustfmt` |
| Python | 3.11.16 | Isolated parser research baseline only |
| uv | 0.12.9 | Python runtime/environment and lock management |
| SQLite | 3.53.2 embedded; 3.51.3 policy floor | Runtime checked; WAL remains disabled |

The first application packages use these exact Phase 1 foundation versions:

| Dependency | Pin | Purpose |
| --- | --- | --- |
| Tauri Rust / build | 2.11.5 / 2.6.3 | Native desktop host and build integration |
| Tauri JavaScript / CLI | 2.11.1 / 2.11.4 | Typed invoke adapter and desktop commands |
| UUID / regex | 1.26.0 / 1.13.1 | Opaque native IDs and diagnostic redaction |
| rusqlite / libsqlite3-sys | 0.40.2 / 0.38.2 | Bundled native SQLite and backup API |
| React / React DOM | 19.2.8 | Shared client rendering |
| Vite / React plugin | 8.2.2 / 6.1.1 | Local development and production client bundle |
| TypeScript | 6.0.3 | Strict shared-client compilation |
| Vitest | 5.0.0 | Client and adapter contract tests |

Dependency versions remain exact in their generated lockfiles. Updating a pin
requires a focused PR that regenerates locks, runs the full check, and updates
this table and the machine-readable policy together.

### Fresh Windows setup

Install Node 24.20.0, rustup, and uv 0.12.9 from their official distributions,
then run these commands from the repository root:

```powershell
node --version
npm.cmd install --global corepack@0.36.0
corepack.cmd enable
corepack.cmd install
rustup toolchain install 1.98.1 --profile minimal --component clippy --component rustfmt --target x86_64-pc-windows-msvc
uv python install 3.11.16
uv sync --frozen --python 3.11.16
pnpm.cmd install --frozen-lockfile
pnpm.cmd check
pnpm.cmd dev
```

Use the `.cmd` launchers on Windows so restrictive PowerShell execution policy
does not select blocked `npm.ps1`/`pnpm.ps1` shims. On macOS/Linux, use the same
commands without `.cmd`; the Windows Rust target check is platform-conditional.
Every subprocess receives encoded argument arrays, so repository paths with
spaces are supported.

`pnpm check` verifies exact toolchain and embedded SQLite versions, formatting, lint, strict
TypeScript, Node/client/Rust tests, and the integrated production build. Run
toolchain verification only through `pnpm check` or `pnpm verify:toolchains`;
direct `node scripts/verify-toolchains.mjs` invocation is unsupported because
pnpm's executable context is part of the version check.

`pnpm dev` starts Vite and the Tauri development shell. `pnpm build` produces
the local client bundle and optimized native executable. Installer bundling is
disabled in Issue #12; unsigned installer evidence remains owned by Issue #18.
The client can also be checked independently with `pnpm typecheck`, while the
root lint and test commands include both TypeScript and Rust packages.

The Issue #13 client uses hash-based data routes because the desktop loads a
bundled static document. Shared light-theme values live in
`packages/ui/src/tokens.css`; application CSS consumes semantic variables rather
than declaring component colors or spacing. Keyboard tests cover skip-link and
route focus, and axe checks cover initial, loading, empty, and error states.
The visual review and agreed desktop/narrow viewport evidence live in
`docs/review/issue-13/`. Dark mode remains a deliberate future theme, and the
off-Windows/PWA browser target is not selected until the later PWA entry exists.

Issue #14 wraps native command results in schema version 1 with a native
correlation ID and either data or one of six stable user-safe errors. The client
must reject missing, future, or malformed envelopes instead of guessing. Local
command diagnostics use allowlisted JSONL records in Tauri's app log directory;
the default rotation is 1 MiB per file, five files total, and 14 days. There is
no diagnostic export, telemetry, crash reporting, scanner job, or parser process
in this slice. Command errors log fixed diagnostic codes, and arbitrary context
must pass through the redacted `SafeDiagnostic` type before it can enter an
event. A diagnostic containing a filesystem, request, or FLP path is replaced
as a complete value so spaces and punctuation cannot leave private suffixes in
the persisted log. The active log recovers its start timestamp from the first
record after a restart. Rust tests use fake clocks/IDs/errors/log sinks, while
client and repository policy tests keep serialization and error vocabulary
aligned.

Issue #15 initializes the native database on startup. The linked runtime probe
is part of `pnpm check`, and can also run independently:

```powershell
pnpm.cmd verify:sqlite:embedded
cargo test -p fruitboard-storage --locked -- --nocapture
```

The command blocks versions below 3.51.3 unless an exact official fixed
backport has first been reviewed and added to the policy. The operating-system
`sqlite3` executable is not evidence for the version embedded by the app.

The standalone storage test command requires Rust and the native C compiler
only; it does not build Tauri/WebView2 and is the portable lane for Issue #17.
It prints the embedded runtime version and generates synthetic database
fixtures in temporary directories. Tests exercise v0/v1, a synthetic future
migration, process termination during a transaction, and backup recovery.

The database lives in Tauri's local application-data directory under `storage/`.
Keep the application closed for any operator maintenance. Do not delete the
`owner.lock` file to bypass another running process. SQLite owns rollback-journal
recovery; copying a live database is unsupported. Completed backups end in
`.backup.db`; `.pending.db` files are unfinished and cannot be restored.

Native `Database::recover_to` restores a checked backup into a fresh location,
preserving the original files and refusing an existing database destination.
There is no renderer recovery command yet. Storage startup failures stop native
initialization and emit only a fixed `storage_*` code; for `storage_busy`, close
the other instance; for `storage_newer_schema`, use a compatible application;
for schema/database failures, preserve all files before recovery. Installer
adoption/preservation checks remain in #18.

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
and Node. The global development-machine check on 2026-09-04 found Node 26.4.0,
Python 3.8.5, and SQLite 3.33.0, but no `rustc`, `cargo`, `py` launcher, or
`corepack` on `PATH`. Development verification therefore uses the pinned,
isolated toolchains rather than silently falling back to those global versions.
Install or verify:

1. Microsoft C++ “Desktop development with C++” build tools;
2. current patched WebView2 runtime;
3. Rust stable MSVC through rustup;
4. Node 24.20.0 LTS plus pinned pnpm/Corepack setup;
5. the isolated Python 3.11.16 research environment; no PyFLP dependency in
   Phase 1;
6. the SQLite binding's embedded version is 3.51.3 or a documented official
   fixed backport before multi-connection WAL is enabled.

The Issue #12 production command compiles the native Windows executable and the
development command opens its locally bundled client in the installed WebView2
runtime. It does not produce an installer; the explicit install/uninstall smoke,
artifact measurements, and packaging decision remain Issue #18.

## Test strategy

### Pyramid

1. **Fast unit/domain tests (largest layer)**
   - command envelopes, fixed error serialization, job transitions, panic
     containment, retention, and diagnostic redaction;
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
   - Rust/TypeScript command schema, error vocabulary, and capability alignment;
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
- [Node.js 24.20.0 distribution](https://nodejs.org/dist/v24.20.0/)
- [Corepack documentation](https://github.com/nodejs/corepack#readme)
- [pnpm installation](https://pnpm.io/installation)
- [Rust 1.98.1 channel manifest](https://static.rust-lang.org/dist/channel-rust-1.98.1.toml)
- [Python 3.11.16 release](https://www.python.org/downloads/release/python-31116/)
- [uv 0.12.9 release](https://github.com/astral-sh/uv/releases/tag/0.12.9)
