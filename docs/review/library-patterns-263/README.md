# Library saved patterns

Related issue: [#263](https://github.com/guilhermebmichelin-create/fruitboard/issues/263).
Source boundary: this focused change on merged PR262,
`7b6a7d1e0b0fe1e2e3fc8ebe5b5275e3d167db48`. This delivers readiness gap G2;
owner review and final-head CI are required. Budgets and Phase 3 exit remain pending.

## Owner summary

After a new analysis, Library details can show the saved pattern count and names.
The approved project has three patterns and four playlist placements; the count
is three. Missing names remain unknown, and older saved results explain that a
new analysis is needed to add pattern details. Refresh details only rereads results.

## Data and display

The parser already extracted these fields; extraction, supported builds and file
limits are unchanged. Only exact saved build `26.1.0.5530` has verified pattern
details. Other supported builds retain unsupported states. Absent markers mean
the count is unknown: FL Studio may display a default empty pattern without
storing its ID. No zero count, default name or implicit pattern is invented.

The full typed validator requires both advertised fields, a count from 1 to the
descriptor's bounded maximum (at most 1024), strictly ascending unique positive
16-bit saved IDs, matching count/list length and aggregate/per-name states.
Verified playlist placements must refer to listed IDs. IDs can have gaps; they
are not dense display positions. Empty stored names remain extracted. Names have
the existing 4095 UTF-16-unit / 12285 UTF-8-byte bounds; NUL is rejected.

Only the selected `patterns` container enters the existing private version-1
immutable projection. The 256 KiB total projection limit remains in force.
Unknown parser/response extensions are discarded. Older snapshots without the
field remain readable as `not_saved`; malformed present containers fail safely.
No schema migration, snapshot rewriting, filesystem resolver or parser authority
comes from reading saved JSON.

The authorized details command retains the same root, location, presence,
fingerprint and current-snapshot gates. Disabled/missing/changed sources expose
no pattern data. The renderer validates the selected response again and shows
20 patterns at a time with keyboard controls that retain focus. Saved names are
inert text; control/bidi characters are visibly escaped. The list resets for a
new saved result and clears while refreshing or losing source eligibility.

## Validation and visuals

Focused tests cover approved F13 through parsing, immutable publication and the
actual authorized read command; repeated reads create no analysis job. They also
cover corrupted counts/IDs/names, unadvertised fields, older builds/snapshots,
missing/empty names, sparse IDs, Unicode/text bounds, last-good preservation,
root/fingerprint denial, refresh clearing, keyboard paging and axe accessibility.
Constructed protocol cases are defense tests, not new compatibility ground truth.

Each detail section has its own snapshot-based React key. The Library accessibility
test queues React updates during axe's asynchronous snapshot, verifies restored
focus and audits the final ready view after a follow-up read. This fixes an
observed test race where a refresh removed headings mid-audit; violation and
incomplete-result assertions stay intact.

Required checks use the pinned Windows toolchain:

```text
pnpm check
cargo test --workspace --features fruitboard-desktop/analysis-jobs --locked
cargo clippy --workspace --all-targets --features fruitboard-desktop/analysis-jobs --locked -- -D warnings
cargo test -p fruitboard-analysis-execution --locked native_analysis -- --ignored
```

The final command requires a freshly built matching parser selected explicitly
with `FRUITBOARD_ANALYSIS_TEST_PARSER`; Windows feature CI runs it too. All twelve
approved corpus files must retain their before/after hashes. The explicit fresh
native review-profile test refuses an existing directory; it seeds approved F13
through the actual parser/publication/read path. This is native development UI
evidence, not an installed journey or performance qualification. The owner's
personal FoundationSmoke profile must not be seeded, swapped or opened for review.

Private logs, exact source/build hashes, independent executables and screenshots
are retained outside compiler caches. Native desktop/narrow positive-flow and
clearly labeled constructed older/missing-name/default/paging renderer evidence
must accompany review. The handoff records their locations and observations.

Native development UI, using the matching binary and a fresh Local review
identity seeded only with approved F13: [1280 px](screenshots/native-desktop.png)
and [390 px](screenshots/native-narrow.png). Both show the three extracted names,
no horizontal overflow and zero reported WCAG A/AA axe violations including
contrast. Refresh retains the same pattern display. The three real-process
integration tests separately cover F13 publication, the original sample and
ordinary project sizes. No personal profile or local installer was used.

Constructed renderer states (actual decoder/components; not genuine-save
compatibility evidence): [older saved result](screenshots/older-desktop.png),
[missing/empty names and inert escaped text at 390 px](screenshots/missing-narrow.png),
and [unknown default pattern count at 390 px](screenshots/absent-narrow.png).
Both 1280 px and 390 px reviews covered eleven states, keyboard paging through
45 IDs, 200% text with a maximum Unicode name, no stale displays/native actions,
no horizontal overflow, and zero reported WCAG A/AA axe violations including
contrast. Captures use the app's unmodified styles.

The single reusable cache remains owned by Codex `/root`; heavyweight work runs
sequentially after the explicit-path disk/cache audit. Reserve at least 30 GiB
after expected output. No new permanent compiler cache or cleanup is introduced.

## Limits and next work

Pattern notes, MIDI, automation, editing/playback, richer arrangement support,
new saved builds and sample availability remain outside this change. No plugin
is loaded and no source FLP is modified. This does not approve native budgets,
whole-app resources, Phase 3 exit, production activation or public distribution.
New analysis still follows the existing three-attempt limit for an unchanged file.

Next prioritize [G4's fresh-identity installed combined workflow](../flp-intelligence-readiness/gap-ledger.md#qualification-sequence).
Compatibility fixtures and the remaining phase decisions need their own scope.
