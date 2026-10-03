# Saved sample references in Library details

Date: 2026-10-03. Related issue: #248. Baseline: merged PR #247
(`7da524be49d76b1fc64c7905bfe0426b88a26fd3`).

## Delivered behavior

Project details shows saved per-channel sample references in parser order.
Extracted text, duplicate references and channels without a stored reference
remain distinct. Counts describe channels with saved references, not unique
files. The view explains zero-channel, all-missing and older-response states.
Lists show 20 entries initially, with keyboard controls to show more or fewer.
Control and bidirectional characters appear as visible escape codes; long
Unicode references wrap without creating links or embedded content.

Saved reference text does not prove that its target exists or is missing.
Fruitboard does not locate, open, play or check referenced files in this
slice. Opening and refreshing details remain read-only. A saved reference can
contain a relative path, absolute path or FL Studio placeholder; none becomes
filesystem authority. Parser-order positions are not FL's saved channel IDs.

## Native and privacy boundary

The existing authorized local `get_project_details` read gains one optional
sample-reference projection. The immutable saved metadata already contains
these values; no parser, worker, storage schema, capability or command changes
are required. Current source/root/row fingerprints and existing scan-snapshot
and late-response fences cover the entire view. Disabled or stale sources
return no sample references. Old responses remain readable with an explicit
unreported explanation; the default metadata feature remains disabled.

Native projection validates count/order, extracted/missing item shapes, the
fixed missing reason and aggregate provenance. It rejects inferred paths,
contradictory aggregates, empty/NUL text, mismatched counts, more than 256
entries and strings beyond the existing 4,095 UTF-16-unit / 12,285-byte limits.
The client validates the allowlisted DTO again and rejects sample values in
disabled/no-current responses. Unknown extensions never reach the view.

Raw saved references intentionally reach this explicit local details view.
They are inert text, never logged, emitted in events, synced or sent back as
arguments. Errors use fixed copy. No new file access, audio/plugin execution,
sample resolution or missing-file claim is introduced. Rollback removes the
display while preserving existing snapshots and history.

## Rendered evidence

The actual client and native response validator use a synthetic read-only
transport at desktop 1280 × 1800 and narrow 390 × 844. Browser checks cover
mixed/missing/older/zero-channel/long/progressive states, disabled/error/loading,
keyboard focus, 200% text, reduced motion and rendered accessibility/contrast.
Unmodified styles are used for checks; independent component crops omit
navigation/skip-link overlays. This is not installed-app, compatibility or
performance qualification.

- [Saved references, desktop](mixed-desktop.png)
- [Saved references, narrow](mixed-narrow.png)
- [No references stored, narrow](missing-narrow.png)
- [Older response, narrow](older-narrow.png)
- [No channels, narrow](empty-narrow.png)
- [Long Unicode and control text, narrow](long-narrow.png)
- [Progressive list, narrow](many-narrow.png)

## Validation and handoff

Required: full pinned Windows `pnpm check`; enabled native tests and all-target
warning-denied Clippy; native projection and actual approved F12 parser-to-
storage-to-command integration; client bounds/provenance/inert-text/refresh/
late-response tests; unchanged approved FLP hashes; independent older database
read; rendered checks above; all final-head CI before owner manual merge.

Local native tests pass 132 tests and client tests pass 278 tests. The native
integration displays the exact pre-registered reference from the approved F12
fixture without resolving it and verifies it stays out of logs. No new fixture,
runtime dependency, migration, parser support or production activation is added.
The full pinned Windows `pnpm check` and enabled all-target warning-denied
Clippy pass. The independent older database read also passes; its 262,144-byte
hash and all 12 approved FLP hashes are unchanged. Rendered checks pass.
Sample resolution, audio actions, Plugin Explorer, broader compatibility and
installed/performance qualification remain separate work.

### Required security-check prerequisite

The first CI run failed its unchanged high-severity npm audit on the
[braces stack-exhaustion advisory](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm),
reviewed on 2026-10-02 with no patched release listed. A separate prerequisite
commit replaces the development-only `markdownlint-cli2` 0.23.2 wrapper with
`markdownlint-cli` 0.49.1, removing `braces` and `micromatch` from the lockfile.
The Markdown engine remains 0.41.1, MD013 remains disabled, dot directories are
included and dependency/tool/cache exclusions remain the same. The root
configuration moves to `.markdownlint.json` using the replacement tool's format.
The application dependency resolutions, CI commands, audit threshold and branch
protection remain unchanged. Before/after enumeration covers the identical 123
documents; positive/negative fixtures verify long-line configuration, root,
nested and hidden heading failures and all exclusions. Frozen installation,
the high-severity audit and the full project checks pass after the replacement.

Codex `/root` owns the existing reusable temporary cache and its sequential
heavyweight window. The private handoff records absolute paths, hashes and
final capacity; at least 30 GiB free is maintained with a 4 GiB output allowance.
Private logs, database copies, independent evidence binaries and reports remain
outside disposable caches under the external
`fruitboard-review-evidence/library-samples-20261001` task directory, begun
before the 2026-10-03 continuation. Source, earlier evidence, approved fixtures
and the primary development cache stay protected. No obsolete compiler
intermediates are selected for deletion; the same cache remains reusable.
