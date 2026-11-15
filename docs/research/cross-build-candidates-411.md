# Exact Windows candidates and save recipes

Issue: [#411](https://github.com/guilhermebmichelin-create/fruitboard/issues/411).
Parent: [cross-build pilot #399](cross-build-pilot-399.md).
Registered after merged PR410, source
`ce7d08d0691dfc18699cd68ae312ceea3e614477`, on 2026-11-11.
Status: **Official candidates identified; no additional build tested.**

## Candidate selection and availability

Image-Line's [installer archive](https://forum.image-line.com/viewtopic.php?f=200&t=194258)
lists these Windows packages. Its [support instructions](https://support.image-line.com/action/knowledgebase?ans=300)
require owners to sign in for older installers. The shared browser showed the
listings but hid attachments while signed out; no package was acquired.

| Role                                           | Exact Windows candidate | Official archive filename        | Experiment state                                   |
| ---------------------------------------------- | ----------------------- | -------------------------------- | -------------------------------------------------- |
| Neighbor of represented 26.1.0.5530            | 26.1.1.5547             | `flstudio_win64_26.1.1.5547.zip` | Not tested; program and About confirmation pending |
| Investigate the reported 25.2.4 framing change | 25.2.4.5242             | `flstudio_win64_25.2.4.5242.zip` | Not tested; program and About confirmation pending |

These targets are neither the survey's macOS 25.2.4.4960 nor the earlier
[Windows 25.2.4.5234 release candidate](https://forum.image-line.com/viewtopic.php?t=339034).
The transition role is a hypothesis, not proof this Windows build introduces the
change. The adjacent test does not qualify all 26.1.x builds. An archive filename
is not executable identity or proof of the version saved in a project.

Read-only local inspection found only the already represented runnable builds;
the 20/21 directories have no matching program. No matching installer appeared
among top-level Downloads filenames. Other uninspected storage is not ruled out.
The owner has been asked for archive sign-in or an existing local installer path.

## Before authoring

1. Obtain each candidate lawfully from the archive or an owner-held package with
   verifiable origin. Record acquisition source/date, length, SHA-256 and the
   installer's signature result. Keep account/session data outside Git.
2. Budget download, extraction, installation and evidence output; restore at
   least 30 GiB free after expected output on every used volume before fixture
   work. Inspect existing caches and run the four-path storage preflight. Use a
   separate installation folder; preserve existing builds and record any shared
   settings changes. No installation is started by this registration.
3. Record executable path privately, length, SHA-256, version resource and signer.
   Capture the exact matching About build. Stop on a different version and amend
   the registration before authoring or decoder inspection.
4. Complete the private observation sheet below before content/event inspection.
   Retain unexpected values and failed attempts; do not revise expectations to
   fit parser output. Each candidate starts from its own build's Empty template,
   rather than resaving an older-build fixture as the baseline.

## Equivalent recipes

Use private role prefixes `CB26` and `CB25`, not new corpus F-slots. Author each
build's baseline at 130 BPM with one built-in empty Sampler explicitly named
`Compatibility Sampler`, File `(none)`, no notes, Playlist clips, recording,
effects or external plugins. Register eight cases: CB26-B/T/N/S and CB25-B/T/N/S.

| Suffix | Intended GUI change from this build's baseline | Intended tempo | Intended channel name   | Intended active sample reference             |
| ------ | ---------------------------------------------- | -------------- | ----------------------- | -------------------------------------------- |
| B      | Baseline, no change                            | 130 BPM        | `Compatibility Sampler` | File `(none)`                                |
| T      | Base tempo only                                | 131 BPM        | `Compatibility Sampler` | File `(none)`                                |
| N      | Explicit channel name only                     | 130 BPM        | `Compatibility Renamed` | File `(none)`                                |
| S      | Load generated silence only                    | 130 BPM        | `Compatibility Sampler` | Independently confirmed synthetic path below |

Every case must show exactly one channel. Start each variant from an independent
copy of its own build's baseline; do not accumulate T/N/S edits. Loading audio
may rename the channel: an unexpected rename is a failed one-property control
until separately resolved, not a clean sample-only comparison.

For S, generate one second of zero-valued mono 16-bit PCM at 44.1 kHz. Independently
verify encoding, frame count and zero samples and retain its hash privately. Use
the same intended inert path in both builds:
`C:\Users\Public\Documents\FruitboardFixtures\CrossBuild\compatibility-silence.wav`.
Confirm the full reference text through the matching GUI; do not infer serialized
text/casing from the intended location or the parser. Audio stays outside Git.

These are intended recipes, not observations. File `(none)` alone does not prove
the absence of every saved path. Record stale/unexpected references as
discrepancies. A one-property GUI control also does not guarantee a one-byte
file difference; retain unrelated changes and distinguish serialization metadata
from semantic changes.

## Independent observation sheet

Copy this sheet into private evidence for each case. Every actual entry starts
as **not collected**; no candidate hash or GUI observation is invented here.

| Entry                 | Required independent record                                                       |
| --------------------- | --------------------------------------------------------------------------------- |
| Identity              | Case ID, registered build, executable hash, About capture and timestamp           |
| Starting source       | Empty-template provenance or this build's baseline-copy hash; prompts/actions     |
| GUI facts             | Actual tempo, channel count/name, Sampler identity and File state/full inert path |
| Scope guard           | Actual notes/Playlist/effects/recording/external-plugin state; unexpected deltas  |
| Raw save              | Private path, length, SHA-256, save timestamp; original bytes preserved           |
| Sanitized candidate   | Private path, length, SHA-256, independent sanitation diff and privacy findings   |
| Matching-build reopen | Actual values/prompts, inspection without saving, before/after candidate hashes   |
| Expectations freeze   | Five pilot field expectations/provenance; sheet hash/date before extraction       |
| Publication decision  | Exact bytes/expectations approval reference, or not approved; private by default  |

Apply [existing fixture rules](../../DEVELOPMENT.md#parser-fixture-rules).
Inspect registration/license identifiers, paths and opaque state anew. Do not
reuse older event offsets or assume the same sanitation grammar. Keep raw and
sanitized bytes separately, document every changed event/length adjustment and
reopen the sanitized candidate in the exact build without saving. Stop if an
acceptable derivative cannot be established. Existing approvals do not cover
these eight uncreated cases; exact-byte public inclusion remains a later decision.

## Decoder and promotion boundary

The starting source is merged PR410 above. The independently retained research
binary from `7d987c3` has matching compiled parser inputs and the provenance in
[the supervised comparison](cross-build-probe-409.md). It admits only the existing
controls: do not pass new paths to it or patch a saved version. A later reviewed
candidate harness must bind approved hashes, independent expectations,
exact-build admission and explicit 1/3/4-byte event172 hypotheses, preserving the
existing authority, input, process and output limits. Changed native code needs
its own build and updated-head checks.

Record a production refusal as policy-blocked. Compare research version, tempo,
channel count/name and sample-reference text separately, preserving extracted,
inferred and unavailable states. Successful decoding or matching basic fields
does not select a framing width. Patterns, notes, Playlist, mixer and plugins
remain outside this pilot. Support expansion, storage/display and family policies
require separate evidence and implementation. Missing prerequisites stay not tested.

## Validation and storage

Owned primary-checkout scope is this registration and the pilot pointer/table.
This documentation changes no runtime, corpus, tools, dependencies or CI commands.
The registration remains pinned to PR410; later base merges need their own
updated-head checks and do not change the uncollected experiment observations.
Required checks: scoped Markdown, privacy, whitespace and official-source/link
verification, followed by normal updated-head CI. No new tests, local build/cache
writes, fixture generation or installed/qualification window are needed for this
documentation. Existing CI applies only to unchanged code.

Owner `/root` preserves the primary `target` and reusable `desktop-feature-tests`
validation cache; absolute paths and single ownership are recorded in the private
`cross-build-candidates-20261111` handoff. Initial C: capacity is about 27.3 GiB,
below the 30 GiB reserve before expected output. Keep private observations,
capacity/access receipts and owner-draft hashes outside compiler caches in that
evidence directory. No deletion, new compiler cache or cleanup-related rebuild
cost. Fixture/build work needs archive access, restored capacity and a fresh
output-aware preflight.
