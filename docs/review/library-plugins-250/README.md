# Saved plugin references in Library details

Date: 2026-10-03. Related issue: #250. Baseline: merged PR #249
(`9eeea843a1a48d6f997e8c3b9d3a092d0724a8cf`).

## Delivered behavior

Project details shows the top-level plugin references already saved in the
project's immutable metadata. Each name, saved class and vendor retains its
extracted, inferred, unavailable or unsupported status. Reference order and
duplicates are preserved. The count describes references, not unique plugins;
positions are neither channel numbers nor stable plugin identifiers. Plugin
references are independent of the channel and sample lists.

The view explicitly distinguishes an older unreported result from a valid
empty list. Neither establishes that the project uses no plugins. Lists show
20 entries initially, with one keyboard button to reveal subsequent batches
or return to 20 while keeping focus. Untrusted Unicode text wraps; control and
bidirectional characters appear as escape codes. Markup and URLs remain text.

A saved reference does not prove installation or availability. Fruitboard
does not search for, load, open or execute plugins in this slice. Top-level
references may omit nested plugins and do not establish comprehensive mixer,
Patcher or VST coverage. Displaying a wrapper name/vendor adds no parser
compatibility claim.

## Native and privacy boundary

The existing authorized local `get_project_details` read gains an optional
allowlisted projection. No new command, capability, dependency, migration,
parser/storage production code, scheduling or worker behavior is introduced.
Opening and refreshing read saved data without parsing or rewriting it.
Metadata retains its existing development-only, default-disabled setting.
The existing root/location authorization, row fingerprint, current-snapshot
and scan-generation fences protect the full details view. Disabled, stale or
missing sources yield no plugin list; closed or replaced panels discard late
replies.

Native projection and client validation check coverage, field provenance,
allowed inference builds/method/confidence, fixed reasons and relationships
between class/name/vendor. Only the established default Sampler inference is
accepted, for saved builds 25.1.3.4922 and 26.1.0.5530. Built-in names must match
their saved class; wrapper missing/unsupported combinations follow the parser
contract. Contradictory records fail the read safely. Unknown extensions are
discarded instead of forwarding arbitrary saved JSON.

Bounds retain 1,024 references, nonempty/NUL-free text within 4,095 UTF-16 units
and the existing byte limits, plus a 256 KiB serialized plugin-list budget.
The immutable input remains under the existing whole-metadata 256 KiB limit.
Native output adds dense one-based reference positions and explicit nulls for
missing/unsupported values; the client validates these again.

Saved names/classes/vendors intentionally reach this explicit local view.
They stay out of logs, events, sync, request arguments and error diagnostics.
No paths, state blobs, private extensions or new filesystem authority are
projected. Rollback removes the display while preserving snapshots/history.

## Rendered evidence

The actual client and native response validator use a synthetic read-only
transport at desktop 1280 × 1800 and narrow 390 × 844. Checks cover populated,
partial, empty, older, long, progressive, no-current, disabled, error and loading
states; keyboard focus; 200% text; reduced motion; horizontal overflow; and
rendered accessibility/contrast. Checks use the actual styles. Independent
component crops hide navigation/skip-link overlays only for screenshots.
These are browser checks, not installed-app or performance qualification.
Wrapper names/vendors in these examples are constructed display records.

- [Saved references, desktop](mixed-desktop.png)
- [Saved references, narrow](mixed-narrow.png)
- [Missing and unsupported fields, narrow](partial-narrow.png)
- [Older response, narrow](older-narrow.png)
- [Empty saved list, narrow](empty-narrow.png)
- [Long Unicode/control/markup text, narrow](long-narrow.png)
- [Progressive list, narrow](many-narrow.png)

## Validation and handoff

Required checks: full pinned Windows `pnpm check`; analysis-jobs enabled native
tests and all-target Clippy with warnings denied; client contract/renderer
tests; the approved F14 parser → validated storage → authorized details read;
unchanged approved fixture hashes; an independent older database read with
before/after hashes; rendered checks above; and all final-head CI before owner
review and manual merge.

Local native tests pass 136 tests, with the separately selected copied-database
probe passing its one normally ignored test. Client tests pass 314 tests.
The real F14 integration verifies inferred Sampler and extracted 3x Osc, unchanged
saved payload, absence from logs, and omission for disabled/stale sources.
Constructed wrapper tests cover fields and contradictions without claiming real
wrapper compatibility. The 262,144-byte database copy stays byte-identical.
All 12 approved FLP fixtures remain unchanged; no new fixture is generated.

The private handoff records the final full-check and CI results, source/binary
hashes, absolute cache ownership and capacity. Codex `/root` owns the existing
reusable temporary cache and its sequential heavyweight window. Maintain at
least 30 GiB free with a 4 GiB output allowance. Logs, copied databases, reports
and independently copied binaries remain outside disposable compiler caches
under `fruitboard-review-evidence/library-plugins-next`. Preserve primary
source/cache and earlier evidence. No obsolete intermediates are selected for
deletion; reuse the same cache for subsequent work.

Installed-app workflow/compatibility review, performance qualification,
production activation, Plugin Explorer, installed-plugin discovery and broader
parser support remain separate work.
