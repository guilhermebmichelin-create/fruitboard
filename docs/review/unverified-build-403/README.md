# Detected version for unverified FLP analysis

Date: 2026-11-05. Issue: #403. Baseline: PR #402,
`eaa0f978bf5e12ab6deaa7bd743df2739e9538e9`.

## Delivered behavior

Library Project details now shows the saved FL Studio version recorded by a
matching unsupported-version analysis. The explanation distinguishes an
unverified build from proof of damage or format incompatibility and explains
that another attempt with the same parser does not add build support.

The normal parser still accepts only the previously verified builds. This
change adds no decoding, fixtures, format claims, migration, scheduling,
dependency or production feature flag. The isolated shared-decoder research
from PR #402 remains separate and disabled by default.

## Data and freshness boundary

The existing native response gains optional `analysis.savedVersion`. It comes
from the unsupported snapshot header, after the existing current-source,
root/location/file revision, fingerprint, adapter, schema and terminal-state
checks. Failed, successful, stale, missing and unauthorized results do not
gain the field. Older responses without it remain readable.

Native and client boundaries accept at most 32 characters containing exactly
four nonempty ASCII-numeric components separated by dots. The client also
requires the unsupported state and unsupported-version reason. Invalid values
fail closed; arbitrary parser text, paths and raw diagnostic codes are not
exposed. There is no new logging or network request.

Refresh clears the displayed version while reading. Revoking the row's source
authority removes it immediately. These reads do not queue analysis.

## Evidence scope

The native integration test changes an existing approved fixture's version to
`99.1.0.5530` **in memory**, publishes the resulting unsupported parser reply,
and reads the actual command response. It verifies the version is present only
for that unsupported result and absent when the displayed fingerprint is stale.
All 36 approved FLP files remain byte-identical.

Browser evidence uses the real Project details component, styles and strict
response decoder with a **constructed** `25.2.4.4960` response. This is renderer
evidence, not an installed-app test or evidence that this build's files can be
parsed. No additional genuine FL Studio build was tested.

- [Desktop, 1280 × 1000](desktop.png)
- [Narrow, 390 × 1400](narrow.png)
- [Narrow with 200% text, scrolled to the diagnostic](enlarged.png)
- [Older response without the version](legacy.png)
- [Machine-readable renderer checks](results.json)

Desktop, narrow and enlarged text have no horizontal overflow. Enlarged text
uses a 32 px root font instead of 16 px; it is text-size emulation, not browser
zoom. Axe reports no violations in these states or in the older-response,
failed and source-revoked states. The latter states omit the version; every
renderer check records zero analysis requests. Existing native buttons and
focus behavior are retained; no new control or animation was added.

## Validation

Native validation and renderer review use implementation commit
`f03ec730d612915f5f0b17ce7cb70db14956c5ac`. Later commits add only review
documentation/evidence. Full PR-head CI remains required before the owner's
manual merge. No required check is waived.

The complete client suite passed 455 tests in 37 files. The subsequent client
edit only removes a redundant TypeScript assertion and applies formatting;
runtime behavior is unchanged. Client production build and lint pass.
Repository policy tests pass 205 tests with one platform-specific skip.

Enabled native tests pass 172 tests; four opt-in fixture, installed-data and
performance probes remain ignored by the ordinary suite. Enabled all-target
Clippy passes with warnings denied. Commands:

```text
pnpm --filter @fruitboard/client test
pnpm --filter @fruitboard/client build
pnpm --filter @fruitboard/client lint
node --test "tests/*.test.mjs"
cargo test -p fruitboard-desktop --lib --release --features analysis-jobs --locked -- --test-threads=2
cargo clippy -p fruitboard-desktop --release --features analysis-jobs --locked --all-targets -- -D warnings
```

The monolithic local `pnpm check` and installed app were not rerun for this
display-only projection. The changed client/native surfaces receive the checks
above; unchanged workspace, packaging and platform behavior still require the
normal CI jobs on the final PR head. Privacy, scoped formatting, Markdown lint,
Rust formatting and whitespace checks accompany this packet.

Private logs retain the initial lint failure, a cold-target build stopped for
storage, and the first native regression failure. The native failure exposed an
incorrect assumption that unsupported snapshots have a parser error code; they
store the version in its own column. The implementation now reads that column
from the matching unsupported snapshot.

## Storage and handoff

The existing source checkout and reusable temporary validation cache are reused,
with Codex `/root` as the sole writer. The primary development cache is preserved.
The private handoff records absolute paths and final capacity. Logs, inventories,
hashes and renderer evidence remain outside disposable compiler caches.

Exact-path retirement removed only verified obsolete compiler libraries from
retired probe profiles and the unused cold-target attempt. Protected source,
fixtures, older evidence, executables and debug symbols were retained and
verified. Reusing those retired profiles will require recompiling their removed
libraries. Cargo-created internal hardlink aliases were replaced by independent
copies with hashes preserved.

The cold-target guard stopped compilation, but a final write briefly reduced
free capacity to 29.99 GiB. That run was not accepted as validation. Cleanup
restored the reserve; corrected validation uses the populated host release
profile, a new preflight and an exclusive write window. This work makes no
performance qualification claim.

Owner review and manual merge remain required. Broader build support needs
genuine fixture evidence and its own compatibility review.
