# Parser fixture owner handoff — 2026-09-20

Issue #138 is still gated: the corpus has no committed `.flp` bytes and no
digest. This note makes the remaining owner-authored work concrete without
inventing a file, parser result, version string, byte offset, or expected value.

## What the owner needs to provide

For each fixture slot, use a disposable copy of the named installed FL Studio
build and record the values before any parser or spike is run:

| Slot family | Owner action still missing | Required pre-registration |
| ----------- | -------------------------- | -------------------------- |
| F01 baseline | Save a minimal project in the oldest lawfully available build | Exact FL version/build, tempo, ordered channel names, and raw sample references, including declared absence where applicable |
| F02 FL 20 | Save the approved minimal absent-field case | Confirm that the chosen FL 20 build can genuinely save the intended absent base-tempo field; if the normal UI always writes a default, stop and ask for an approved replacement case rather than editing bytes |
| F03 FL 21 | Save the approved minimal absent-field case | Confirm whether a normal new project can contain no channels and no sample references; if not, record the achievable case and obtain owner approval before changing the manifest row |
| F04/F05 FL 2024 pair | Save two projects differing in exactly one declared property | Name the property and pre-register both exact values; keep all other visible inputs identical and record the build used for both |
| F06 FL 2025/current | Save a plain version-coverage project | Exact installed FL version/build and the four staged expected values |
| F07–F10 robustness | Derive truncation, unknown-event, malformed-length, and resource-limit bytes from approved F01 only | Approve and record the derivation operation and expected typed outcome before creating each derivative; do not choose offsets or claims after seeing parser output |

The existing manifest remains the authoritative slot list and schema. If an
“absent” case cannot be produced by the selected FL build’s normal UI, the
owner must approve a replacement semantic case or leave that row `not covered`.
There is no safe agent-only way to resolve that ambiguity from the empty corpus.

## Per-fixture handoff record

Before a file is offered for a manifest PR, the owner should supply this small
record for review (values are intentionally blank here):

```text
fixture_id:
slot_and_version_row:
fl_studio_version_and_build:
authoring_method: owner-produced minimal save
source_project: disposable synthetic project; no personal content
expected_saved_version:
expected_base_tempo: extracted/unavailable/unsupported/failed + reason
expected_channel_names:
expected_sample_references:
derivation_from_base: none, or approved byte operation for F07-F10
license: owner-produced, no third-party content
privacy_attestation: synthetic-only; no personal names, paths, samples, or tokens
sha256: computed only after the exact bytes are final
owner_signoff:
```

The owner should attach the exact build provenance and the pre-registration
record to the fixture PR, then force-add only the reviewed `.flp` bytes. Compute
the SHA-256 after finalization, verify it before and after the spike run, and
keep the source project outside the repository if it contains anything not
covered by the synthetic-only attestation.

## Gate disposition

- Current manifest status: all ten assigned slots `not covered`; two reserved.
- No parser implementation or FLP parser fixture is authorized by this note.
- The spike starts only after the fixture PR is reviewed and merged, as already
  approved on #138.
- If the owner cannot lawfully produce a row, retain `not covered`; do not
  substitute a public GPL corpus, Image-Line demo/template content, or a hand-
  invented byte stream.
