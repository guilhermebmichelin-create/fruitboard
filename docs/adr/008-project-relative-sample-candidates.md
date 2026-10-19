# ADR-008: Project-relative sample candidates

- Status: **Proposed; owner acceptance and implementation are pending.**
- Date: 2026-10-18
- Decision owner: product owner
- Related issue: [#332](https://github.com/guilhermebmichelin-create/fruitboard/issues/332)
- Reviewed source: `d90b660ff7cb3a65d74447ce84c1cc338134447b`
- Conditional successor to: [ADR-007](007-sample-presence-boundary.md), only for
  relative eligibility, its GUI-evidence prerequisite and corresponding UI copy

## Decision requested

Let the existing explicit sample check interpret an eligible relative saved
reference from the directory containing the current authorized FLP. Describe
this as **Fruitboard's candidate rule**, constrained to that project's existing
selected, enabled, qualified local NTFS root. A result describes the checked
candidate, not proof that FL Studio would load that file.

This is a proposed change to ADR-007, whose accepted first slice permits only
literal absolute paths and leaves relative references unchecked. Its alternative
requiring independent GUI evidence before relative mapping would be superseded
only for this explicitly app-defined interpretation. Any FL Studio compatibility
claim still requires an owner-approved genuine save and independent GUI evidence
for the exact build tested. Synthetic strings and policy tests cannot supply it.

For this same-root, app-defined slice only, acceptance would also supersede the
blanket future-relative grant/GUI gate in the
[original authority review](../review/sample-presence-278/authority-review.md#evidence-boundaries).
The existing explicit action would authorize the amended eligibility inside the
same folder grant; it would not create another persisted grant. Placeholder,
external-folder and cloud support retain their separate design/grant gates.

The [native review](../review/project-relative-candidates-332/README.md) maps this
proposal to the current implementation and its required acceptance evidence.
The broader [G5 proposal](../review/phase3-next-slices-20261014/reference-resolution.md)
remains context, not an additional grant or approval.

## Proposed path rule

1. Keep existing absolute-path parsing and observations unchanged. A rejected
   absolute, drive-relative, rooted, UNC, device or URL form must not fall back
   to interpretation as a project-relative path.
2. Obtain the current source path and saved references natively through the
   existing root/location/snapshot/fingerprint authorization. Qualify the held
   root, source and their namespace bindings before deriving the source-parent
   component stack. A textual prefix alone cannot establish that base.
3. Accept only backslash-separated relative components. Validate **every raw
   component before eliminating any of them**. Reuse the conservative ordinary
   component restrictions: no empty or trailing component, alternate separator,
   stream, wildcard, reserved name, short-name spelling, ambiguous trailing
   dot/space, or control/bidi formatting. Only the exact `.` and `..` tokens are
   exempt from the ordinary trailing-dot restriction. Any `%` or raw component
   beginning with `$` yields `unresolved_placeholder`; no expansion occurs.
   An invalid or placeholder component followed by `..` is still rejected.
4. Starting from that qualified source parent, process components in memory.
   Ordinary components append, `.` stays in place, and `..` removes one
   component. If any step would climb above the selected root, return
   `not_checked/outside_root` before a candidate query, even if later components
   would re-enter it. Never pass `.` or `..` to a filesystem operation.
5. Preserve the existing capture/shape checks: an empty saved value, NUL,
   oversized reference or aggregate payload fails the whole request before
   qualification. An absent channel value remains `no_saved_reference`. For
   otherwise valid inputs, cap the raw relative sequence at 64 components and
   every resulting full component stack at the
   existing 64-component limit, including root components. This bounds lexical
   work as well as the final path. Exceeding either new relative-depth cap yields
   the item-level `not_checked/limit_reached` result.
6. Walk only the resulting exact child chain from the held root through the
   existing metadata port. Preserve spelling, Unicode and actual directory case
   semantics. Do not use `canonicalize`, reopen a joined path, enumerate, search
   other roots, or consult process working-directory or FL Studio folders.
7. A candidate equal to the selected root is `not_checked/not_regular_file`.
   Only authoritative exact-name absence from a trusted parent is `not_found`;
   access errors, reparses, offline/recall indications and uncertain metadata
   remain `not_checked`. Source/root/snapshot/session changes discard the report.

Normalization is an app interpretation, not Windows path traversal. For
example, `Missing\..\Kick.wav` maps to the candidate `Kick.wav` beside the FLP;
it does not query `Missing` or establish that such a directory exists. The UI
must explain this interpretation before the user starts a check. Lexically
cancelled ordinary components are validated as text but never queried; their
physical reparse/offline state is intentionally unknown. The surviving chain
still requires all native no-follow/no-recall checks.

## User-facing meaning

Keep the explicit **Check saved samples**, check-again and cancel actions. The
scope explanation must say that absolute paths are checked as saved and relative
candidates are checked from **the folder containing this project file**, only
inside the selected folder. This base is the currently authorized FLP's parent,
which may differ from the selected scan root. It must not call that rule
FL Studio's resolver.

Use fixed report copy that remains true for both forms, such as **Present when
checked** and **Not found at checked candidate**. Do not present a
relative observation as absence at the literal saved string. Show the existing
saved reference and check time; do not return resolved paths or add open/play
actions. Preserve keyboard use, focus restoration, status announcements,
non-color meaning, narrow layouts and enlarged text.

The protocol retains its fixed outcome/reason vocabulary and opaque correlation
IDs. The renderer supplies no base, path, reference, folder grant or budget.
Reports remain ephemeral; no database schema, immutable snapshot, sync payload,
telemetry or raw-error/path logging is added.

## Authority and limits retained

All other ADR-007 requirements remain binding: local NTFS qualification,
exact-child metadata-only operations, no reparse or recall traversal, actual
case/identity/ancestry and parent/name checks, ordinary concurrent file use,
final durable freshness fences, cancellation and late-publication rejection.
The source's identity/size/time fence still cannot detect content edits that
preserve all those observations.

Retain one worker without a queue, 256 channels, existing reference and aggregate
payload bounds, 2,048 operations including qualification/revalidation, 132 held
handles, the two-second cooperative deadline and the 64 KiB response cap.
Keep memoization limited to identical literal references in one authorized
request. These stops are not approved performance budgets; a blocked kernel
call remains subject to the existing single-worker retirement behavior.

There is no new folder grant, external-library lookup, environment/placeholder
expansion, cloud/network access, fallback search, audio read, playback or write.
Default desktop/PWA capability and production activation do not change.

## Acceptance and implementation gate

The owner must explicitly accept this precise app-defined rule and its scoped
native review before runtime implementation. Record the decision and date here;
until then this ADR is Proposed and relative references remain unchecked.
Merging a proposal document alone does not record that acceptance.

After acceptance, use separate reviewed policy, Windows-authority and app/UI
slices with the tests and installed evidence in the native review. If a slice
needs broader authority, new grants or different semantics, return to design
review before adding them. New public FLPs/audio require their own exact fixture
approval; this proposal grants none.

Design acceptance does not prove implementation, FL Studio compatibility, G5
completion, resource budgets, Phase 3 exit, production activation or release.
