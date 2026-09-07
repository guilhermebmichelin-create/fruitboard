# Development and delivery

Status: **Active workflow; Phase 1 accepted, Phase 2 (epic #33) underway**

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

## GitHub governance

Protect `main` with:

- pull requests required, one approving review, stale approvals dismissed after
  material changes;
- required status checks with branches up to date;
- resolved review conversations;
- no force pushes or deletion;
- linear history and signed commits/tags if practical;
- merge queue when parallel contribution warrants it;
- CODEOWNERS for repository policy, Rust/native, security, and migrations. The
  current entries name the actual maintainer and must expand when eligible
  reviewers join.

Verification on 2026-09-04 found this configuration unavailable for the then
private repository: both GitHub's branch-protection and repository-rulesets APIs
returned HTTP 403. The product owner declined GitHub Pro and accepted a manual
governance exception for the private GitHub Free repository. That exception was
based on private-repository plan limits and is **superseded on 2026-09-07**: the
repository is now public, where branch protection and rulesets are available on
the current plan. Until the owner approves and implements the proposal below,
the manual practices remain in effect: the maintainer must still use issues,
feature branches, PR checklists, test evidence, and all available CI; avoid
direct development, force-pushes, and deletion of `main`; and request external
review when an eligible reviewer is available. The stable checks described below
run on every PR, but GitHub does not yet enforce them as required checks.
Enabling enforcement is a proposal only and has not been applied.

### Proposed enforced branch protection (pending owner approval)

Status: **Proposal pending owner approval. Nothing in this section is
implemented; the settings below have not been configured on GitHub.** With the
repository public since 2026-09-07, GitHub branch protection/rulesets are
available on the current plan, so the 2026-09-04 manual-governance exception no
longer reflects a platform limitation. Proposed configuration for `main`:

- Require pull requests before merging; no direct pushes to `main`.
- Require the status checks `docs-policy`, `client`, `rust-portable`,
  `migration`, `windows-foundation`, `security`, and
  `windows-packaging-smoke` to pass with branches up to date before merge.
- Require squash merges for one-concept PRs, matching the existing
  branch/commit convention.
- Disallow force pushes and branch deletion on `main`; keep linear history.
- Merge authority remains with the repository owner only (the existing
  CODEOWNERS maintainer); the owner performs every merge manually and no
  automation merges on their behalf.
- Optionally require resolved review conversations and stale-approval
  dismissal, as already described above.

This proposal does not change CI workflows, job names, or their commands; it
only proposes that GitHub enforce the checks that already run.

Use labels by type (`epic`, `feature`, `bug`, `spike`, `docs`, `security`), area
(`desktop`, `scanner`, `parser`, `database`, `client`, `pwa`, `sync`), and phase.
Milestones represent phase review gates, not arbitrary dates.

The Phase 0 epic is GitHub issue #1, with review units #2–#5 and merged pull
requests #6–#9. After architecture acceptance, Phase 1 was created as epic #10
with issues #11–#18; the authoritative scope remains in
[ROADMAP.md](ROADMAP.md). Phase 1 was accepted on 2026-09-06, epic #10 is
closed, and Phase 2 proceeds as epic #33 with issues #34–#41 and spikes #42
(P0-D) and #43 (P0-E).

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

| Tool     | Pin                                  | Purpose                                        |
| -------- | ------------------------------------ | ---------------------------------------------- |
| Node.js  | 24.20.0 LTS                          | JavaScript runtime and client tooling          |
| pnpm     | 11.25.0                              | Workspace package manager                      |
| Corepack | 0.36.0                               | Verifies and launches the pinned pnpm release  |
| Rust     | 1.98.1 MSVC                          | Native workspace, with `clippy` and `rustfmt`  |
| Python   | 3.11.16                              | Isolated parser research baseline only         |
| uv       | 0.12.9                               | Python runtime/environment and lock management |
| SQLite   | 3.53.2 embedded; 3.51.3 policy floor | Runtime checked; WAL remains disabled          |

The first application packages use these exact Phase 1 foundation versions:

| Dependency                | Pin             | Purpose                                        |
| ------------------------- | --------------- | ---------------------------------------------- |
| Tauri Rust / build        | 2.11.5 / 2.6.3  | Native desktop host and build integration      |
| Tauri JavaScript / CLI    | 2.11.1 / 2.11.4 | Typed invoke adapter and desktop commands      |
| Tauri shell plugin        | 2.3.6           | Rust-only inert sidecar packaging smoke        |
| UUID / regex              | 1.26.0 / 1.13.1 | Opaque native IDs and diagnostic redaction     |
| rusqlite / libsqlite3-sys | 0.40.2 / 0.38.2 | Bundled native SQLite and backup API           |
| React / React DOM         | 19.2.8          | Shared client rendering                        |
| Vite / React plugin       | 8.2.2 / 6.1.1   | Local development and production client bundle |
| TypeScript                | 6.0.3           | Strict shared-client compilation               |
| Vitest                    | 5.0.0           | Client and adapter contract tests              |

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

`pnpm check` verifies exact toolchain and embedded SQLite versions, repository
privacy, formatting, lint, strict TypeScript, Node/client/Rust tests, and the
integrated production build. Run toolchain verification only through
`pnpm check` or `pnpm verify:toolchains`; direct
`node scripts/verify-toolchains.mjs` invocation is unsupported because pnpm's
executable context is part of the version check.

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
preserving the original files and refusing a destination containing the
database or any `-journal`, `-wal`, or `-shm` companion, including empty files.
Choose a fresh location; do not delete companions to make recovery proceed.
There is no renderer recovery command yet. Storage startup failures stop native
initialization and emit only a fixed `storage_*` code; for `storage_busy`, close
the other instance; for `storage_newer_schema`, use a compatible application;
for schema/database failures, preserve all files before recovery. Installer
adoption/preservation checks remain in #18.

Issue #16 proves the first React-to-SQLite workflow with the startup-view
preference. On launch without an explicit hash route, the client loads the
saved value before creating the router; explicit deep links are preserved. Use
Preferences to choose Home, Library, Board, or Preferences. The control keeps
the previous value until Save preference succeeds and provides accessible
loading, success, retryable load-error, and save-error states. Native close and
reopen tests prove persistence in the same local database. The IPC payload
contains only schema version 1 and the route enum; paths and SQL remain native.
Scanner, parser, project storage, sync, and installer behavior are still
deferred. Desktop and narrow keyboard interaction recordings are documented in
[`docs/review/issue-16/`](docs/review/issue-16/README.md).

Issue #18 adds an explicitly unsigned, separately identified current-user NSIS
smoke package. It is not the production release configuration. On Windows, run:

```powershell
pnpm.cmd smoke:windows:foundation
```

The command builds the zero-dependency Rust probe from `Cargo.lock`, packages it
as a Tauri external binary, installs into a generated path containing spaces and
Unicode, measures cold/warm shell startup through a loopback-only WebView2 debug
port, closes the app, exercises fixed respond/fail/timeout/terminate modes,
uninstalls, verifies the database is byte-identical, reinstalls, verifies the
saved startup view, and uninstalls again. Generated installers and raw evidence
stay ignored. The committed summary and limitations are in
[`docs/review/issue-18/`](docs/review/issue-18/README.md).

The package uses WebView2's `downloadBootstrapper`, so installation needs
network access when the runtime is absent. Downgrades are refused and updater
artifacts are disabled. Phase 1 has no automatic updater: an approved replacement
build is installed manually under the same production identity only after
backup/recovery checks. Public distribution additionally requires protected
Windows signing and timestamp credentials, signatures over the application,
sidecars, installer and update metadata, SBOM generation, signature verification,
rollback qualification and SmartScreen/reputation review. Never add signing
material to repository files or pull-request jobs.

### Quality tools

| Area                 | Proposed tools                                                                                        |
| -------------------- | ----------------------------------------------------------------------------------------------------- |
| TypeScript/React     | TypeScript strict, ESLint (`typescript-eslint`, React Hooks, JSX accessibility), Prettier             |
| Unit/component tests | Vitest, React Testing Library, `user-event`, axe integration                                          |
| Browser E2E/visual   | Playwright with desktop and iPhone viewport projects; real iPhone manual matrix for platform behavior |
| Rust                 | `rustfmt`, Clippy with warnings denied in CI, built-in tests, `cargo-audit`/advisory scanning         |
| Python sidecar       | Ruff format/lint, mypy or Pyright, pytest, coverage, dependency audit                                 |
| SQL                  | Numbered SQL migrations, SQL formatter/lint conventions, repository integration tests                 |
| Documentation        | markdownlint, Mermaid render check, link checker with retries/allowlist                               |
| Supply chain         | Dependabot/Renovate decision, dependency review, CodeQL, secret scanning, SBOM on release             |

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

## Pull-request CI

`.github/workflows/foundation.yml` runs six stable, always-present jobs for
pull requests to `main`, pushes to `main`, and manual dispatch. Jobs do not use
path filters or job-level conditions, so a skipped check cannot look like a
successful quality signal.

### Foundation pull-request checks

| Job                  | Runner  | Scope                                                                       | Local equivalent                                                                                                                               |
| -------------------- | ------- | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs-policy`        | Ubuntu  | Markdown, script/policy tests, tracked and unignored-file privacy scan      | `pnpm privacy:check && pnpm lint:docs && pnpm lint:scripts`                                                                                    |
| `client`             | Ubuntu  | Frozen install, lint, typecheck, component tests, production web build      | `pnpm --recursive --if-present lint && pnpm typecheck && pnpm --recursive --if-present test && pnpm --filter @fruitboard/client build`         |
| `rust-portable`      | Ubuntu  | Rustfmt, warning-denied Clippy, portable SQLite storage tests               | `cargo fmt --all --check; cargo clippy -p fruitboard-storage --all-targets --locked -- -D warnings; cargo test -p fruitboard-storage --locked` |
| `migration`          | Ubuntu  | Latest creation, every supported upgrade, killed migration, backup recovery | `cargo test -p fruitboard-storage --locked`                                                                                                    |
| `windows-foundation` | Windows | Exact Node/Rust/Python/uv/pnpm/SQLite pins and complete production build    | `pnpm.cmd check`                                                                                                                               |
| `security`           | Ubuntu  | Privacy regressions, high-severity npm audit, RustSec advisory audit        | `pnpm privacy:check; pnpm audit --audit-level high; cargo audit`                                                                               |

The complete local pre-merge gate remains `pnpm.cmd check` on Windows with the
pinned toolchains. `cargo audit` requires the separately installed RustSec CLI;
CI installs the exact `cargo-audit` 0.22.2 release and runs it against
`Cargo.lock`. `.cargo/audit.toml` denies every new RustSec warning. Its explicit
17-advisory baseline covers 12 Tauri GTK3/WebKit transitive crates that are
inactive on the supported Windows target and five unmaintained Unicode helpers
through Tauri's `urlpattern`; none is a direct Fruitboard dependency. Revisit
the list on every Tauri update and before adding Linux support.

Workflow permissions default to read-only repository contents. Checkout does
not persist credentials, pull-request code receives no release/signing secrets,
and there is no `pull_request_target` path. All actions use reviewed immutable
commit SHAs. Dependency and build caches are keyed from committed locks and
contain only generated package/compiler data. CI uploads no artifacts, so it
cannot persist project data, local databases, logs, or personal paths.

GitHub dependency review, CodeQL, and GitHub secret protection were unavailable
on the private GitHub Free plan when this baseline was adopted. The repository
became public on 2026-09-07, so these GitHub-native products are now available
on the current plan. Enabling them is part of the [pending branch-protection
proposal](#proposed-enforced-branch-protection-pending-owner-approval) and has
not been done yet; until then, the baseline continues to use executable privacy
regressions, `pnpm audit`, RustSec, and weekly Dependabot updates for pnpm,
Cargo, and GitHub Actions, and never represents an unavailable or skipped
integration as a passing security check.

Bug and feature issue forms require bounded outcomes, acceptance/non-goals,
accessibility states, and a privacy confirmation. Security reports route to a
private advisory. CODEOWNERS names the actual current maintainer for repository
policy, native code, security policy, and migrations; it is an ownership signal,
not a substitute for independent review.

Avoid an expensive full installer build on every tiny PR until build duration is
measured. Run a scheduled/manual Windows packaging smoke and make it required on
release candidates. Add a macOS compile/package lane before declaring macOS
portability, not after platform-specific assumptions accumulate.

`.github/workflows/windows-packaging-smoke.yml` implements that separate smoke.
It runs when packaging inputs change, on a monthly schedule, or by manual
dispatch. The single Windows job has read-only repository permission, immutable
action SHAs, no secrets, and no artifact upload. The hosted runner uses
`pnpm.cmd smoke:windows:foundation:hosted` to exercise package, signature,
install, native launch/exit, sidecar, data-preservation, reinstall, and uninstall
behavior without claiming an interactive desktop. The complete local equivalent
is `pnpm.cmd smoke:windows:foundation`, which additionally proves WebView2
inspection, audio capability signals, timings, and graceful window close. The
six always-present Foundation CI jobs remain unchanged.

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
- [Tauri Windows installers](https://v2.tauri.app/distribute/windows-installer/)
- [Tauri external binaries](https://v2.tauri.app/develop/sidecar/)
- [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/)
