# Strict native Automation Clip count study

Issue [#324](https://github.com/guilhermebmichelin-create/fruitboard/issues/324).
Fixture baseline: PR [#418](https://github.com/guilhermebmichelin-create/fruitboard/pull/418),
merged as `a4f3aa818b463f9006b5424a42037c4d6fb7f9a2` with approved F46–F48.
Status: **Executable research for the demonstrated layout.**

## Evidence and proposed boundary

The [fixed-input study](../../tests/automation-differential-fixtures.test.mjs)
matches the independent GUI counts and order for F11, F23 and F39–F48.
F11 and F48 match the complete observed zero-object layout. F42 has an empty
Playlist and still has one clip definition and three editable points. Reordering
in F46 retains two objects and six points in the new order. Removing the
two-point object in F47 leaves one object and four points. F45 still has one
editable point. These cases distinguish clip definitions, editable points and
Playlist placements.

The candidate rule deliberately recognizes only the demonstrated layout:

1. Require an exact `26.1.0.5530` build marker before declarations, the observed
   format/96 PPQ header and a complete bounded event walk. Reject duplicate
   version markers. Use the verified three-byte width for event 172 only within
   this research profile.
2. Require one to three unique declarations, in a single contiguous channel
   section before its Playlist boundary, matching the header count. Recognize
   the observed ordered record shapes for a type-0 Sampler followed by zero to
   two type-5 clip declarations. Each declaration must carry the observed
   empty generator name (event 201 with one UTF-16LE NUL). Each clip owns one
   scoped event 234. Clear scope at the observed section boundaries and reject
   generator names outside a declaration. Never use a saved channel ID as a
   persistent identity, or use the name to infer the target.
3. Check the reviewed 17-byte prefix and 112-byte suffix of a point payload,
   the little-endian count at offset 17, and exact length
   `21 + 24 × count + 112`. The study accepts one to four points per present
   object and up to six points total. These are observed research limits.
   The interior point records remain opaque. Report the object and point totals
   only after every declaration passes.
4. Recognize an empty set in the study only when the complete single-Sampler
   layout and explicit empty Playlist agree, with no type-5 declaration or
   stray point payload. Preserve unavailable results for layouts outside that
   boundary. Never turn absent records, an empty Playlist alone, ambiguity, or
   bounds exhaustion into zero. A present clip containing a zero-length point
   list remains unqualified.

Constructed variants exercise missing, duplicate and orphan records, scope
transitions, version and header contradictions, contradictory generator names,
unfamiliar state signatures, zero on a present curve, count/length inconsistency
and aggregate limits. The study mutates only in-memory input. No authored
fixture is changed or generated.

## Review decision before native implementation

This rule demonstrates a deliberately small recognition surface. Exact channel
record ordering may reject ordinary projects saved with different settings,
even in the same build. The observed suffix is a signature, not a decoded LFO
or target-state check. The Playlist only fences the channel section and
corroborates the controlled empty case. Nonempty Playlist references, point
coordinates and curve semantics are outside this hypothesis.

Review whether the proposed recognizer can safely advertise counts for genuine
projects outside these exact controls. In particular, qualify any additional
channel/layout shapes or opaque-state variations before widening the rule.
For the native slice, use the existing bounded event reader and typed validation.
Require checked arithmetic, a complete declaration set, and explicit
unavailable state whenever any input scope is unqualified. Test that basic
metadata remains available when the optional count is refused. Update parser
capabilities and both direct and supervised validation together. Keep the
authorized immutable Library projection in a subsequent reviewable slice,
with explicit handling for older results that have no count.

Production counts, protocol, storage and Library behavior are unchanged by this
research. Issue #324 remains open.
