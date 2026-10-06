# Pattern note fixture qualification plan

Status: **Preregistered candidate cases; no fixture generated or approved.**
Parent [design](README.md), issue #288. Baseline is PR285 merge
`d19587f52132f31065bbaab97a572d3c5487cf00`.
The existing manifest currently ends at F15; F16/F17 below are proposed slots,
not a claim that files or hashes already exist. Recheck allocation before use.

## Controlled genuine-save cases

Before creation or byte inspection, retain this recipe plus the actual matching
FL Studio executable/build provenance in private evidence. Use only an
independent copy of approved F11, build **26.1.0.5530**, 130 BPM, and empty
Samplers with File `(none)`. Name the two channels `Fixture Notes One` and
`Fixture Notes Two`; no personal songs, audio, samples, presets, external
plugins, embedded art or comments. Use two copies for separate cases.

| Proposed slot | Case registered before creation                                                                                 | Independent expected observations                                                                                                                                                                                                                          |
| ------------- | --------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| F16           | Two empty Samplers; three named patterns: `Fixture Notes A`, `Fixture Notes B`, `Fixture Steps C`               | A: two ordinary Piano roll entries on channel One plus one ordinary entry on Two, total 3. B: one ordinary entry and one slide entry on One, total 2 visible entries, no entries on Two. C: exactly four enabled Step Sequencer steps on One, none on Two. |
| F16 Playlist  | Place A twice, B once and C once as four separate pattern clips                                                 | Three unique patterns; repeats do not change the registered content counts. Placement/timing is inspected only to confirm repeat independence, not to qualify new duration formulas.                                                                       |
| F17           | One empty Sampler; name and retain a pattern `Fixture Named Empty`, with no Piano roll entries or enabled steps | GUI-empty contents. Register whether it survives reopen as a saved pattern; do not preregister a zero-length payload or claim its absent bytes prove zero.                                                                                                 |

For F16, draw ordinary notes at distinct grid positions and pitches so each
can be counted unambiguously; overlap the slide with its ordinary note and
disable ghost-note display while documenting counts. Record each channel's
Piano roll and the Step Sequencer, and inspect both channels for every pattern.
Do not count ghost notes or Playlist clips as contents. Do not play the project.
If a channel retains steps/notes after switching patterns, correct the GUI
recipe before the first registered save and record that change. Recipe changes
after byte inspection require a new case, not edited expected values.

## Evidence and approval sequence

1. Save only to fresh private review paths, with compiler/test workloads
   quiescent. Record matching-build GUI captures, pattern names and the counts
   above before running a parser or examining record lengths.
2. Hash the original saves. Independently sanitize narrowly identified private
   registration/project-data fields; retain source/candidate hashes, exact changed
   offsets and a byte-difference inventory privately. Do not change note records,
   pattern IDs/names or channel content to make a decoder pass.
3. Reopen the sanitized candidate in the matching GUI without saving or playback.
   Count the registered entries again, verify File `(none)`, two/one channels,
   names, placements and named-empty survival. Close without saving and require
   unchanged hashes. A changed/lost case is not qualified.
4. After independent GUI evidence, inspect the sanitized bytes read-only. Record
   event framing, qualified content/property section transitions, pattern-ID
   binding, record lengths, ordinary/slide/step representation and empty-case
   encoding. Derive format evidence independently; disagreement is a finding,
   not authority to revise the registered GUI counts. A zero-length payload
   cannot gain support unless a genuine GUI-empty case demonstrates it.
5. Prepare `FIXTURE_APPROVAL.md` with exact filenames, lengths, SHA-256,
   provenance/license, registered and independently observed expectations,
   sanitation inventory and bounded privacy review for paths, registration,
   email/comments, embedded audio/art and other sensitive data. Keep raw GUI
   captures and original saves private. Ask the owner to approve the exact
   candidate bytes and expectations; earlier F13/F15 approvals do not extend.
6. Only after exact approval, add the named files, manifest rows and the privacy
   allowlist in their own fixture PR. A design or parser PR is not privacy
   approval. Preserve all 13 existing hashes and require the fixture PR's normal
   gates. No new public FLP enters this design PR.

If the installed matching build is unavailable, continue independent design
or constructed defensive tests; keep genuine-build qualification pending.
If F16's steps/slides or F17's zero encoding differ from the candidate layout,
narrow support explicitly before implementation. Never infer new build support
from a neighboring build or agreement between two parsers.

## Constructed defense cases

In-memory byte arrays may cover sparse/reordered IDs, repeated property markers,
zero-length candidates, duplicate/contradictory payloads, missing context,
channel/arrangement transitions, unknown section boundaries, lengths 1/23/25,
truncation/overflow, counts at/above both ceilings, older builds and malformed
protocol relationships. They are not GUI compatibility or empty-payload proof.
Use no committed binary derivatives without their own fixture approval.

The shared [storage/cache/evidence rules](implementation-plan.md#resource-and-handoff-boundary)
apply before fixture creation. Capture original/candidate hashes independently
of parser output. Never inspect the owner's real project roots.
