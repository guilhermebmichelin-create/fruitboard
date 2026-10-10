# Independent verification of the application review

Date: 2026-10-09. Reviewed report: `b27b761` (PR #299).
Implementation baseline: `266327058fbf3f6dfd2cfa2fedd51ceb09ea3fa2`
(PR #301), its immediate successor. Original-source claims were checked
against their surrounding code and callers; the successor changes the mixer
parser and validation, not the cited scanner/client error paths.

## Assessment

The report identifies useful maintenance work, but its urgent panic list
substantially overstates reachable failures. Searching for `expect`,
`unreachable!`, or poison recovery is a starting point, not proof of a bug.
Several cited operations have guards that establish their invariants; other
recommendations would undo documented privacy or preservation behavior.

Confirmed corrections in this change are watcher pending-generation guards,
verification of the opened watch root, Windows storage hardlink guards,
commit-driven keyboard focus, SQLite companion privacy checks, current status
documentation, and a usable private vulnerability reporting channel. An
independent audit additionally found three actual moderate documentation-tool
advisories; compatible pinned updates and a moderate-or-higher CI threshold
address those. No broader application architecture rewrite is justified by
the report alone.

The architectural strengths are supported: native ownership of filesystem
and SQLite operations, typed validated command envelopes, durable scan fences,
bounded read-only parser inputs, exact dependency/toolchain pins, privacy
regressions, and an explicit RustSec exception list. These do not establish
production activation, parser sandbox qualification, or performance acceptance.

A live governance check found a further discrepancy: GitHub currently reports
`main` unprotected, the classic protection endpoint returns HTTP 404, and the
repository has no rulesets. The ten CI contexts exist and remain required by
the working agreement, but they are not currently enforced by GitHub. The
2026-09-07 documented enablement is a historical record; when or why protection
changed is unknown. Current documentation is corrected. Restoring the recorded
controls remains an owner governance decision; no protection settings were
changed during this review.

## Verification of the twelve urgent claims

| Claim                                                    | Independent conclusion and action                                                                                                                                                                                                                                                                                                                                                                                                                         |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Enumeration panics on non-UTF8 names                  | **Incorrect.** `validate_entry_name` converts with `to_str().ok_or(InvalidUnicode)` before the cited conversion. Invalid names record `InvalidEntryName` and continue. The immutable entry cannot become non-Unicode between these operations.                                                                                                                                                                                                            |
| 2. Publication can panic at `one candidate`              | **Unsupported.** The owned `BTreeSet` is checked for `len() == 1` immediately before consuming its first element. No mutation intervenes. Returning a storage error would add an unreachable branch.                                                                                                                                                                                                                                                      |
| 3. Parser/supervisor expects are input-triggered crashes | **Unsupported.** Header length, four-byte event widths, and 88-byte playlist records establish the `le_u32` slice length. Successful `ensure_running` installs a process; piped handles follow explicit `Command` configuration; shutdown takes the process it already borrowed. Spawn and I/O failures already return typed errors.                                                                                                                      |
| 4. Reconciliation's union arm is reachable               | **Incorrect.** The union contains keys from the two immutable maps queried by the match. `(None, None)` is impossible under this construction.                                                                                                                                                                                                                                                                                                            |
| 5. Generations are checked only in debug builds          | **Mixed.** The coalescer could regress a pending generation and overwrite a newer loss; now it rejects/counts stale input in every build, without accumulating history. The cited scan follow-up `fence` already compares generations and rejects older/ended/unseeded signals in release. Production per-watch coalescers also receive an immutable generation; this is bounded API hardening, not demonstrated durable corruption.                      |
| 6. Watcher root check/open race                          | **Confirmed, with narrower impact.** The final component could become a junction between path checking and opening. The watch now opens the reparse object and verifies the actual handle before starting a worker. Deterministic directory-to-junction and directory-to-file tests cover the window. Parent-junction and nested notification limitations remain; notifications are hints and enumeration remains authoritative.                          |
| 7. All poison recovery must fail closed                  | **Overbroad.** Database locks already return safe errors/fallback states. Scan lifecycle panic containment stops the host and emits fixed diagnostic codes. Some recovery is teardown, where refusing to take an owned child/worker could prevent cleanup. No particular corrupted invariant or unlogged reachable panic was demonstrated. Any future change needs a lock-by-lock invariant and teardown policy.                                          |
| 8. Root 65 silently loses production events              | **Incorrect production implication; standalone mismatch confirmed.** Production supplies a separate coalescer for each watch. The standalone default now matches its documented 100-root budget. A public rejection counter already existed; platform statistics now expose bounded rejection/stale counters. Capacity remains bounded.                                                                                                                   |
| 9. Cursor meaning is lost in Library pagination          | **Incorrect.** The cited catch handles scan/retry actions. Page loading separately recognizes both cursor errors, resets a non-null cursor to page one, and announces the restart. Arbitrary cursor errors returned for scan actions are contract failures.                                                                                                                                                                                               |
| 10. Tauri collapses `invalid_request`                    | **Incorrect.** `execute` rethrows `PlatformError` unchanged. Envelope errors keep their typed code/correlation ID; local startup-view validation yields `invalid_request`. Invalid response data intentionally becomes `internal`; transport exceptions intentionally become `unavailable`.                                                                                                                                                               |
| 11. Startup router ignores platform replacement          | **Design limitation, no demonstrated product bug.** The desktop constructs one platform/adapter at mount. Router construction happens after preference loading, and the hash check protects explicit deep links before navigation. Runtime platform replacement is not a shipped flow. A future replacement API should own router disposal and route retention explicitly.                                                                                |
| 12. Swallowed diagnostics and sample cancellation        | **Mixed, intentional behavior.** `mount.tsx` is under `src/`, not `src/app/`. Raw React errors are suppressed to avoid persisting paths/tokens; an error boundary provides recovery UI. Sample cancellation immediately fences results and native work remains deadline/admission-bounded if cancellation transport fails. Calling it acknowledged native termination would be misleading; no raw exception logging or new false retry promise was added. |

## Robustness and client suggestions

| Suggestion                                            | Independent conclusion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Backup retry and pending-file GC                      | A single backup step returns `Busy` explicitly rather than hanging. Bounded retry could improve availability, but no unrecoverable failure was demonstrated. Pending backups are deliberately retained for explicit maintenance; automatic deletion would conflict with current recovery/preservation rules. No GC added.                                                                                                                                                                                                         |
| Unbounded integrity results and SQL normalization     | SQLite's default `integrity_check` reports at most 100 errors ([SQLite documentation](https://www.sqlite.org/pragma.html#pragma_integrity_check)); the unbounded-result claim is inaccurate. Streaming would still simplify allocation. Migration ledger SQL equality intentionally checks exact committed migration bytes. Normalizing arbitrary SQL whitespace can change literals/comments and weaken the ledger contract. Keep exact equality. The separate migration-ledger collection could be bounded in future hardening. |
| Storage check/open and missing Windows hardlink check | **Confirmed asymmetry, fixed.** Existing Windows database/lock/backup files are checked through metadata handles; private files are verified after opening and before permission changes or returned writes. Regression tests preserve outside bytes/permissions, including insertion after the initial check. The per-user directory remains the trust boundary; this does not pin all parents or eliminate SQLite's separate pathname race.                                                                                     |
| Poisoned retry cursor                                 | Recovery exists, but only owned cursor cloning/assignment occurs under its lock, and the cursor is an optimization over durable state. Sweeps wrap at the end. No reachable invalid durable operation was demonstrated. Resetting on poison could be defensive, but blanket fail-closed or generic logging is not established as necessary.                                                                                                                                                                                       |
| Shared path/IPC/error helpers                         | Possible maintenance work; these boundaries serve different purposes. Notification validation, locator-key case semantics, and reconciliation paths are not interchangeable. Unify only after preserving their distinct contract tests.                                                                                                                                                                                                                                                                                           |
| Split LibraryPage / add cancellation/data layer       | File size is real maintenance debt. Existing sequence fences, request coalescing, adapter identity, and durable cancellation semantics must survive extraction. This change extracts only the shared focus mechanism. Aborting an IPC promise does not itself cancel native work; a broad new data layer is a separately scoped design task.                                                                                                                                                                                      |
| More ESLint plugins, Zod, script lint                 | These are tool choices, not proven defects. Script syntax checking is limited lint coverage, but Node regression/policy tests already run in the gate. New plugins/schema libraries need a focused dependency/rule rollout, not speculative adoption in a bug fix.                                                                                                                                                                                                                                                                |
| Add react-router-dom / align Vitest and Vite          | **Incorrect.** The pinned react-router package directly exports its dom subpath; react-router-dom is unnecessary for this import. Installed Vitest 5 supports Vite 6.4, 7, and 8, including pinned Vite 8.3.0. Different major numbers do not imply incompatibility.                                                                                                                                                                                                                                                              |
| Re-enable axe contrast; repair skip link              | **Wrong proposed fixes.** jsdom cannot calculate rendered contrast; separate policy tests enforce relevant 4.5:1 text and 3:1 focus token pairs. Native anchor hash navigation would interfere with the hash router; the handler intentionally focuses main content without replacing the application route. Real-browser contrast coverage could supplement existing evidence.                                                                                                                                                   |
| Replace focus timer polling                           | **Confirmed, fixed.** Focus waits for a React commit with a connected enabled control, and later user interaction/new actions/unmount invalidate it. Delayed status/pagination and folder-removal tests cover behavior beyond the old timer window.                                                                                                                                                                                                                                                                               |
| Tokens-only UI package and AppIcon location           | No missing compile/test lane is established for a CSS-only package: root policy tests validate the tokens consumed by client components. Moving components into a shared package is useful only when actual consumers need them.                                                                                                                                                                                                                                                                                                  |
| Review-harness branch in production                   | The desktop entry explicitly supplies `native`; the badge branch adds no filesystem, parser, or command authority. Removing unused review UI through a build flag could reduce bundle debt, but it is not a demonstrated security/correctness bug. Retain truthful harness labeling.                                                                                                                                                                                                                                              |

## Security, CI, documentation, and hygiene

| Claim/suggestion                                    | Independent conclusion and action                                                                                                                                                                                                                                                                                                                                                                                                                   |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Private reporting/support policy missing            | **Confirmed.** GitHub private reporting was actually disabled despite issue forms linking to it. Enabled and GET-verified on 2026-10-09; SECURITY now links the private channel and describes development-only version status. No maintainer email invented and no LICENSE chosen.                                                                                                                                                                  |
| Validity/non-provider scanning disabled             | **Confirmed setting, existing explicit limitation.** GitHub API confirms both disabled while secret scanning/push protection are enabled. Stronger scanning is a policy choice involving coverage/noise and validity-check handling; existing documentation records the settings. A failed/unavailable protection fixture is not a passing test. No risk acceptance fabricated.                                                                     |
| CodeQL/dependency review informational              | **Confirmed, intentionally documented governance.** These do not currently block merge; promotion changes repository protection and remains an explicit owner decision. Rust `build-mode: none` is supported, not evidence of a broken Rust scan ([GitHub documentation](https://docs.github.com/en/code-security/concepts/code-scanning/codeql/codeql-for-compiled-languages)).                                                                    |
| High-only npm audit gap                             | **Confirmed with fresh evidence, fixed.** Audit and Dependabot identify moderate advisories in `js-yaml`, `markdown-it`, and `smol-toml`. Pin patched 5.4.1, 14.3.1, and 1.9.0, respectively; security CI now fails at moderate. One low KaTeX tooling advisory remains; its broader version migration is separate work.                                                                                                                            |
| No Python audit/SBOM despite real parser packaging  | **Overstated gate.** Python is research-only with an empty dependency list; the packaged parser is Rust and covered by Cargo/RustSec. A Python audit would add no current dependency coverage. SBOM/signing belong to the explicit production-release gate, which remains open; an unsigned development smoke is not a production release.                                                                                                          |
| RustSec exceptions and pending Dependabot work      | Seventeen documented exceptions are real and must be revisited on Tauri/platform changes. There are five open Dependabot PRs at inspection, not about twenty open PRs. Retained remote branches are not equivalent to pending dependency work.                                                                                                                                                                                                      |
| Privacy ignores files and misses DB companions      | **Mixed.** Ignored generated/user data is intentionally outside the candidate-commit scan; force-added tracked files are included. The companion suffix gap is real and now regression-tested for case variants, Windows/nested paths, and safe near matches.                                                                                                                                                                                       |
| README/ROADMAP/SECURITY stale                       | **Confirmed in part, fixed.** README packaging and app-visible status were stale; analysis/sample threat boundaries needed updates. At `b27b761`, mixer qualification was delivered but parsing still absent; PR #301 adds parser support, while storage projection/Library display remain open. Refresh current status and archive superseded blocks without pretending the historical failed scanner p95 has been superseded by a qualifying run. |
| Branches, tags, stale ignores, TODO, public history | Retained branches and zero tags are not defects without a release/retention policy. Old parser artifact ignores are harmless. Watcher final-component TODO is resolved by this hardening; narrower parent/nested limitations remain documented. The fixture manifest explicitly records the owner's accepted unsanitized-history exposure; this is real privacy debt, not permission to rewrite/delete history or claim it resolved.                |

## Implementation and validation boundary

Changes are restricted to watcher/storage file guards and their tests, a
shared client focus hook and regression tests, privacy policy checks, three
documentation-tool overrides/lock entries, the audit threshold and CI policy
assertion, and status/security documentation. No schema or parser-result
migration, scan activation, phase acceptance, branch-protection promotion,
license selection, installed qualification, or performance qualification is
part of this change.

Three agents worked in disjoint source boundaries; the primary independently
verified the original claims and reviewed the combined diffs. Rust builds are
serialized by the primary. Raw logs, source/diff provenance, toolchain output,
cache inventory/isolation evidence and disk preflights are retained outside
Git and disposable caches. The raw handoff records the selected existing
cache's absolute path and sole owner, `/root`; no per-agent Rust caches were
created. Existing CI success belongs to the unchanged baseline, not these edits.

Validation on the pinned Windows toolchain passed:

- toolchain verification and embedded SQLite floor (3.53.2 against 3.51.3);
- repository privacy, formatting/Rustfmt, Markdown/script/client lint, and
  TypeScript checks;
- workspace Clippy and enabled `analysis-jobs` Clippy, both with warnings denied;
- all 131 Node policy/regression tests;
- all 392 client tests across 31 files, using Vitest's thread pool with one worker;
- all 535 workspace Rust tests, using one test thread; 12 explicitly unverified
  environment fixtures remain ignored;
- all 157 enabled native-analysis tests; four explicitly unverified/manual
  fixtures remain ignored;
- production frontend and optimized Windows desktop build (`pnpm build`);
- `pnpm audit --audit-level moderate`, with one low tooling advisory remaining;
- workflow syntax using pinned actionlint, and `git diff --check`;
- SHA-256 comparison proving all 19 approved FLP fixtures byte-identical after
  the complete native suites.

The literal local `pnpm check` run stalled in Vitest's default process pool
before tests began. The complete client suite passed with
`node node_modules/vitest/vitest.mjs run --pool=threads --maxWorkers=1` from
`apps/client`. A subsequent default parallel workspace run failed three
unchanged parser-supervisor tests against their two-second deadlines; the full
workspace passed with `cargo test --workspace --locked -- --test-threads=1`.
No deadline, assertion, production behavior or committed runner configuration
was relaxed. Both initial failures and the equivalent passing runs are retained.
The remaining gate stages were completed directly, including the production
build. Thus the evidence is equivalent stage coverage, not a claim that the
literal default local driver passed.

Fresh pull-request CI applies to the new head. The working agreement's ten
check requirements and the owner's manual merge authority remain intact;
baseline CI is not substituted and current server-side enforcement is not
claimed.

## Remaining decisions

Owner decisions remain license selection, production activation/release
support, restoration of the documented `main` protection, stronger optional
secret-scanning settings, promotion of informational CI contexts, and
public-history removal. A future focused task can address
low-severity KaTeX tooling migration, broader client extraction, browser
contrast coverage, bounded migration-ledger streaming, or an explicit
pending-backup maintenance command. None is represented as completed here.
