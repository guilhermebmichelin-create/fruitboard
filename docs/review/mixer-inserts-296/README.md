# Saved mixer insert count and name proposal

Status: **F18-F21 fixture approval recorded; narrow positional mapping for review.**
No extraction or app support is delivered. The
[genuine qualification result #298](../../research/parser-mixer-inserts-298-result.md)
records exact approved bytes, observations and restrictions after PR297.
Issue [#296](https://github.com/guilhermebmichelin-create/fruitboard/issues/296).
Planning source: PR295 merge `2bb2c187eef0d4faee56e96f5774af6b9b7b8251`,
tree `19cb8ba31317f82a18d763693377370d1e8234f2`.
This is a narrow [G6](../flp-intelligence-readiness/gap-ledger.md) step inside
Phase 3. Resource budgets, the phase exit and production activation stay open.

## Owner outcome and meaning

After a new analysis, the Library could show how many ordinary mixer insert
records the file explicitly saves, and their saved names where present.
A default track visible in FL Studio is not automatically a stored record.
An unnamed saved record still counts; a name alone does not prove a record.
The count says nothing about whether tracks are used, routed or audible.

Image-Line's [Mixer manual](https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/mixer.htm)
distinguishes insert tracks, Master and Current, and describes editable track
labels. This GUI documentation motivates the distinction; it does not specify
FLP bytes, saved IDs, record boundaries or the target build's default tracks.
Do not use its current GUI capacity as a compatibility expectation.

Count only independently qualified ordinary insert records. Exclude Master and
Current only through a qualified identity distinction; do not assume special
numeric IDs, zero/one-based GUI numbering or order. If that distinction or the
complete insert section cannot be established, publish an unsupported field,
not a guessed count. No special-track names or roles are exposed in this slice.
Duplicate names remain separate entries, and absent names remain unavailable.
Sparse layouts remain unsupported until genuine evidence qualifies their
identity representation. Never synthesize `Insert N` or `Master`.

F18-F21 now have independently registered mixer expectations and exact owner
approval. F16/F17 qualify note records, and F15 qualifies Sampler labels; their
earlier approvals were not used to authorize the new files. The
[fixture gate](fixture-plan.md) records the completed evidence boundary before code.
No unofficial decoder code is copied, translated or made a dependency.

## Proposed parser contract

Qualified build/layout: **26.1.0.5530, complete eighteen-record layout only**.
Sixteen ordinary records occupy section positions 1 through 16; independently
qualified Master/Current occupy positions 0/17. Other totals/orderings remain
unsupported. The positional mapping below is proposed for review before
implementation; public fixture approval alone does not implement it.
Keep protocol 1/schema 2. Advertise `mixerInsertCount` and `mixerInsertNames`
together with positive `maxMixerInserts` and `maxMixerInsertCandidates` limits,
each at or below **512**, with the returned-record limit no larger than the
candidate limit. Change the parser package/adapter version with extraction,
not this design. An unadvertised pair is unsupported; it is not an empty list.

Every non-failed field uses coverage `explicit-saved-mixer-insert-records`.
The count is extracted only when the qualified complete section gives a total.
The names field contains exactly one entry per counted saved identity, strictly
ascending. The proposed ID envelope is an unsigned integer from 0 through
65,535, conditional on qualifying the actual saved identity representation.
This is a validation envelope, not a claim about on-disk width or GUI numbering.
The genuine representation is positional: no separate numeric ID payload was
observed. `savedInsertId` means the record's zero-based position in the
complete qualified saved section, scoped to that save; it is not a persistent
ID across edits. In this layout the exact ordinary ID set is 1-16, count is 16,
and both names collections contain all sixteen entries. This clarifies the
original conditional representation before coding.

| Approved genuine case | Proposed count and names mapping |
| --- | --- |
| F18 / F21 | Extracted count 16; incomplete names with sixteen unavailable entries |
| F19 | Extracted count 16; incomplete names, IDs 2/7 extracted as `Fixture Mixer A` / `Fixture Mixer B`, other fourteen unavailable |
| F20 | Extracted count 16; incomplete names, IDs 2/7 separately extracted as `Fixture Mixer Same`, other fourteen unavailable |

Every unavailable entry uses `MIXER_INSERT_NAME_NOT_STORED`; the incomplete
aggregate uses `MIXER_INSERT_NAMES_INCOMPLETE`. These are proposed field
results from independent evidence, not output from the current parser.

All extracted names use aggregate `extracted` with `value`, no `items`/reason.
Any absent name uses aggregate `unavailable / MIXER_INSERT_NAMES_INCOMPLETE`
with `items`, while the complete record count can remain extracted.
First-slice names have only extracted or unavailable entry states. Unqualified
name encoding makes the entire pair unsupported; do not publish a partial list
or silently treat invalid text as absent. Explicit empty text, if independently
qualified, remains an extracted empty value and receives an empty-name UI label.

| Condition | Pair state / fixed reason |
| --- | --- |
| Complete qualified ordinary-insert section | Extracted count; names as above, with matching IDs/list length |
| Qualified explicit zero-record encoding | Extracted zero and empty extracted names; only if separately qualified |
| Section omitted | Both `unavailable / MIXER_INSERT_DATA_NOT_STORED`, no values/items |
| Other saved build | Both `unsupported / MIXER_INSERT_UNVERIFIED_BUILD` |
| Unknown record framing or text encoding | Both `unsupported / MIXER_INSERT_LAYOUT_UNVERIFIED` |
| Unqualified identity, special-track distinction, section ordering or termination | Both `unsupported / MIXER_INSERT_BINDING_UNVERIFIED` |
| Duplicate saved IDs, conflicting records or repeated section | Both `unsupported / MIXER_INSERT_RECORDS_AMBIGUOUS`; never choose/merge duplicates |
| Candidate, text or selected-output ceiling exceeded | Both `unsupported / MIXER_INSERT_LIMIT_EXCEEDED`; no truncation |
| Whole-file parse failure | Existing fixed failed outcome; both fields `failed` with its fixed reason, no coverage/value/items |

Absent data is never zero. Unknown boundaries revoke active mixer identity;
channel, pattern, Playlist and plugin contexts cannot authorize mixer labels.
Names must bind to qualified saved insert identities within the verified section.
Do not reuse existing event IDs/context switches as mixer evidence. Qualification
must record the actual entry, identity, name, special-track and termination rules.

## Bounds and full typed validation

The **512 candidate** ceiling is a proposed safety envelope, not a claim about
available tracks or a performance acceptance target. Charge every candidate
before filtering, including excluded special tracks, duplicates and malformed
candidates. Bound repeated name candidates by the existing event cap and reject
duplicate name assignments as ambiguous. If one aggregate event contains many
records, check its length, offsets and candidate count before allocation/walking.
Use checked borrowed slices and a bounded selected ID/name map. No raw state,
routing, effect slots or per-plugin copies are retained.

Keep 64 MiB input, 2 MiB interpreted-event, 100,000 events, 256 channels,
1,024 patterns, 65,536 notes per pattern, 262,144 total notes, 256 KiB reply and
256 KiB stored projection ceilings. Opaque plugin state retains its existing
checked skip policy. Names obey existing 4,095 UTF-16-unit and 12,285 UTF-8-byte
limits; the aggregate reply/projection cap also applies. Existing supervisor
deadline, cancellation/replacement and unchanged-file attempt limits remain.

The full native validator requires the advertised field pair and limits
together, qualified saved build, exact coverage/status/reason shapes, integer
count within limits, ordered unique bounded IDs, list/count equality and bounded
names. Extracted aggregates contain only `value`; incomplete names contain only
`items` plus the fixed reason and at least one unavailable entry. Non-entry
states carry neither collection; count and names global reasons agree.
IDs must satisfy the exact qualified ordinary position set 1-16, exclude
Master/Current positions 0/17, and correspond to count 16 and all sixteen names
entries. Envelope bounds alone do not authorize an ID or another count.
Reapply those identity rules when reading stored data and decoding the
selected renderer DTO; a parser's category claim cannot override them.
Reject missing required fields, extra selected keys, invented inference/method/
confidence, fractions, negatives, overflows, bad text, duplicate/missing IDs,
wrong coverage, invalid state relationships and value/items/reason contradictions.
Discard unrelated extensions. Invalid advertised data fails the complete reply;
it cannot become older valid metadata. Keep the initial-only validator usable.

## Authority, persistence and rollback

| Boundary | Required review |
| --- | --- |
| Input/process | Existing held read-only source, enabled-root grant and native-selected isolated parser; no new opens, sample reads, plugin loads or renderer process permissions |
| Validation/publication | Existing descriptor/envelope correlation, fingerprint and full typed validation, then root/location/source/publication/session/lease rechecks; preserve last-good results on rejected work |
| Storage | Optional selected `mixerInserts` in immutable private projection version 1; bounded count/IDs/name states only; no raw section, migration, new table or snapshot rewrite |
| Details read | Revalidate selected build, coverage, shapes, IDs/count/text limits under existing current-result authorization before DTO projection |
| Older/malformed data | Missing additive field becomes `not_saved`; malformed present data fails the complete authorized read, never an older-state fallback |
| Renderer/Explorer | Strict selected DTO decoding; names remain escaped inert text. No filesystem/audio/MIDI/plugin capability. Inserts do not become plugin/effect references or verified Explorer slots |
| Diagnostics | Fixed allowlisted codes only; no names, source paths or raw state in logs/sync |

This stays within [ADR-002](../../adr/002-flp-parser-process.md) authority.
Refresh reads saved results without parsing or rewriting. A new analysis is
explicit and obeys existing attempt rules. Rollback removes the new extraction,
selected projection and UI, preserves immutable results, and leaves additive
fields unselected by older readers. Tests must confirm that behavior with old
and new snapshots; no data migration is proposed.

## Future Library display and states

Add **Saved mixer inserts** after saved patterns. Explain: **These are explicit
saved records. The count does not show which inserts are used or audible.**
Each entry shows **Saved insert ID: N** and its saved name or fixed missing-name
explanation. The ID label explicitly avoids promising FL Studio's track number.
Expose 20 entries initially, with keyboard-accessible show-more/show-less and
stable focus; preserve IDs as keys, including sparse IDs and duplicate names.

| State | Future behavior |
| --- | --- |
| Current extracted | Complete saved-record count and bounded list; unknown names explained per entry |
| Explicit zero, if qualified | `0 saved mixer inserts`; only with extracted count/empty matching list |
| No saved section / unadvertised / unverified / ambiguous / over-limit | Fixed unavailable/unsupported explanation; no numeric total or partial list |
| Older snapshot | Explain mixer details were not saved and require a new analysis |
| Loading / read error | Clear prior entries; accessible pending/error message; no old count presented as current |
| Changed/missing source or disabled root | Existing no-current-details state; no stale mixer data |
| Close/reopen, row/root changes or late response | Existing request/snapshot/fingerprint fences; discard stale responses |
| Default desktop / PWA | Existing unavailable-analysis behavior; no new capability or activation |

Use text-safe rendering, wrapping, accessible list/section labels, announced
states, keyboard focus retention, reduced-motion defaults, 390px layout and
200% text checks including axe contrast. No sorting/grouping by name that hides
distinct records. A future read-only installed journey must use approved genuine
values plus clearly labeled constructed malformed/limit/older cases.

## Review and next work

Review this meaning, field contract, ceilings and gate before implementation.
Then follow the [three-slice plan](implementation-plan.md): genuine qualification
and exact fixture approval, parser/full validation, immutable Library integration
and installed validation. Approving this design does not approve future fixture
bytes. No fixture publication decision is requested before actual sanitized
files and their expectations exist.

Effects/slots, nested plugin state, routing/sends, channel destinations, Master/
Current display, automation/MIDI, playback/editing, tempo maps, richer arrangements,
broader builds, dependency resolution and resource-budget acceptance remain
outside this slice. This proposal does not complete broader G6 or open Phase 4.
