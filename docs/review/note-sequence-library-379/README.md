# Saved ordinary-note sequences in Library

Issue: [#379](https://github.com/guilhermebmichelin-create/fruitboard/issues/379).
Dependency: [storage PR378](https://github.com/guilhermebmichelin-create/fruitboard/pull/378),
corrected head `67dc21300f342151199489ed55a4810f9f76d463`.
Status: **Library implementation; locked CI and installed validation pending.**

## Behavior and limits

The existing authorized details command now selects a typed note-sequence
summary. It reuses the saved-field grammar and checks the same build, pattern
IDs, note counts, dense channel bindings and bounds before creating the DTO.
The strict client decoder checks shapes, reasons, relationships, ordinals,
observed raw values, PPQ and per-pattern/total/JSON caps again. No raw saved JSON
or new command enters the renderer.

Each pattern has a keyboard-accessible sequence disclosure. Qualified ordinary
records are grouped by existing channel labels, preserving source ordinals and
duplicates. Numeric saved key, pattern-local start/duration ticks and raw
velocity byte are shown without pitch naming, MIDI, seconds or velocity
conversion. Playlist reuse does not expand sequences. The first 64 records are
shown, with explicit reveal/reduce controls retaining keyboard focus. Every
saved record remains reachable.

Old results explain that details were not saved and need a new analysis.
Missing payloads, unverified records/layouts/PPQ/channels/builds and limits retain
explicit explanations, never shortened or invented empty sequences. Counts stay
separate. Loading/error/no-current states remove sequence content. The panel
also hides loaded details synchronously when the row fingerprint, presence or
root eligibility changes, before a replacement read resolves. Existing disposal
and snapshot keys reject late replies and reset disclosure.

Qualification remains exact FL Studio 26.1.0.5530, 96 PPQ, observed raw-value
sets and one/two-channel Sampler subset from F24-F30. Ceilings remain 512 records
per pattern, 1,024 candidates and 128 KiB detail JSON, with the existing 256 KiB
reply/persistence limits. No grammar/build support, fixtures, schema,
dependencies, locks, commands, filesystem grants, sync/logging, playback/export,
default activation or resource/phase decisions change.

## Validation boundary

Two feature-enabled native regressions cover all seven approved cases through
parse, full validation, immutable storage, reopen and the authorized details
response. They also cover older/malformed data, root/row mismatch, revocation,
no implicit analysis and unchanged fixture bytes. They run in the existing
`analysis-jobs` desktop suite. Native compilation/execution must pass on the
changed head; formatting is not execution evidence.

Client tests cover strict decoding and mixed/older/refused states, duplicate
ordering, caps, channel labels, paging/keyboard focus, escaped hostile text and
authority-loss/refresh/late responses. The pre-existing dependency set passed a
66-test related compatibility run in seven files. A final additive mixed-state
case passed in a subsequent six-test decoder run. Scoped local ESLint passes.
These results are compatibility evidence, not the locked gate.

Reused local tools include Vitest 5.0.3 and TypeScript 6.0.3; several dependencies
differ from the branch's lockfile. Local typechecking reaches one unchanged
environment error: those dependencies lack `@napi-rs/canvas` required by the
current test setup. The compatibility smoke uses the previous cleanup-only setup,
with filesystem module/result caches disabled. The committed canvas-backed CI
setup and all required checks remain unchanged. Updated-head client lint,
typecheck, tests and build; native default/analysis-jobs tests and Clippy;
parser/storage checks; and the complete Windows/packaging gates remain required.
Prior CI applies only to unchanged source.

## Renderer review

Screenshots show the actual changed components/styles against **constructed**
values in an in-memory loopback renderer harness using existing dependencies.
The harness passes responses through the strict client decoder. This is neither
an installed native workflow nor additional FL Studio compatibility evidence.

- [Ordinary records, desktop](screenshots/ordinary-desktop.png).
- [390 px width](screenshots/ordinary-390.png).
- [390 px with 32 px default text](screenshots/ordinary-390-text-200.png).
- [All 130 constructed records reachable](screenshots/paging-130.png).
- [Older result](screenshots/older.png), [missing payload](screenshots/missing.png)
  and [unverified tick resolution](screenshots/unsupported.png).

Browser checks verified 64 → 128 → 130 records with Enter and retained focus,
immediate clearing on source revocation, fixed older/missing/unsupported copy,
and loading/error states. Desktop and 390 px layouts, including 32 px default
text, had no horizontal document overflow. This is not installed or performance
qualification. Raw screenshots, interaction receipts, logs and the bounded
harness remain private outside compiler caches.

## Build and handoff

Owned source: dedicated `fruitboard-note-sequence-library` worktree, branch
`feat/379-note-sequence-library`, owner `/root`. Validate native details modules/
tests, client decoding/components/tests/styles and matching docs above the
declared storage base. No delegated agents are used.

Four-path preflight fails at 27.50 GiB free, with 1 GiB expected native output
leaving 26.50 GiB against the 30 GiB reserve. The reusable `desktop-feature-tests`
cache also reports hardlink aliases. Its sole owner is `/root`; its write window
stays closed. No local native compilation, installer run, FLP fixture generation
or dependency installation starts. The small renderer review transforms JS in
memory without writing a bundle/compiler cache; it is not an installed substitute.

Absolute source/cache/evidence paths and receipts are in the private
`note-sequence-library-20261101` handoff outside compiler caches. Primary cache,
dirty primary documents, corpus, profiles and historical evidence are preserved.
No cleanup or new permanent compiler cache is introduced. Restore reserve and
isolation before heavyweight work; coordinate one writer and exclusive installed
host window. Qualification runs alone.

PR378 must merge first; the owner performs every merge. Changed-head checks and
remaining installed evidence need explicit review. No broader note coverage,
Phase 3 acceptance, resource budget, activation or distribution follows.
