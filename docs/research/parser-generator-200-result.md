# FL Studio 2026 generator-name slice (issue #200)

Status: implementation validated locally; the owner approved the exact
sanitized F14 bytes for public repository inclusion on 2026-09-29. The file
is included in the corpus and exact-head CI remains a merge gate.

## Registered case and GUI result

Before generation or parsing, the private case specified FL Studio 2026 build
26.1.0.5530, 130 BPM, the original built-in Sampler with no sample, and one
built-in `3x Osc` generator renamed `Fixture Synth A`. It started from an
independent copy of the approved F11 project. No external plugin, audio,
recording, note, or playlist placement was added. This is a production
follow-up outside the completed F01-F13 research allocation.

The project was saved through the matching FL Studio GUI. Its sanitized
candidate was reopened in that build without saving. The GUI showed 130 BPM,
two channels, and the 3x Osc instrument under `Fixture Synth A`. A missing
synthetic Public Documents project-data folder was handled with `Do nothing`.
The candidate SHA-256 stayed unchanged after closing. Screenshots stay private
outside Git because application chrome displays account information.

## Event evidence and privacy boundary

The controlled save has two channel-kind events. The Sampler is kind 0 with an
empty UTF-16 event 201. The 3x Osc channel is kind 1 with event 201 containing
`3x Osc`; a separate event 203 contains its editable label
`Fixture Synth A`. That separation supports a generator-name field distinct
from channel names. It does not establish the meaning of event 201 for other
plugins or saved builds.

The sanitized candidate is 47,881 bytes with SHA-256
`d4acf044b447ff578a59070c2020aeacf5a473de2ce97821e8d2dc25a5904ec5`.
It differs from the private GUI save only by removal of one 16-byte
registration payload and its two framing bytes, followed by correction of the
FLdt length. Bounded event and byte scans found no remaining registration
event, non-Public Windows home path, email, known local account name, or
embedded RIFF audio. These checks do not decode the opaque native plugin
state. The approved sanitized file is pinned in the
[corpus manifest](../../fixtures/parser-corpus/manifest.md); the raw GUI save
and screenshots remain outside Git. The owner approved only the exact
47,881-byte candidate under [the fixture rules](../../DEVELOPMENT.md#parser-fixture-rules).

## Parser behavior and local validation

`channelGeneratorNames` is available for the exact verified 2026 build. It
labels the kind-0 Sampler as an inference and extracts `3x Osc` from event 201.
An editable channel label from event 203 cannot replace the generator class.
The aggregate array retains each item's provenance; mixing the inferred
Sampler and extracted synth yields medium confidence. Other generator classes,
malformed/multiple class events, zero-channel projects, and the 2024/2025
builds receive field-level `unsupported` results. This is a narrow first
generator slice, with no mixer effects, external VST identities, vendor data,
or plugin-state interpretation.

Focused parser tests, warning-denied Clippy, Rustfmt, changed Markdown lint,
and repository privacy verification pass. A direct JSON-lines request against
the private sanitized candidate returned two channels, channel names
`Sampler` and `Fixture Synth A`, and generator names `Sampler` and `3x Osc`,
without echoing the input path or changing candidate bytes. The ordinary
committed corpus test also reads exact approved F14 bytes and checks the
separation between generator and channel label. Exact-head CI remains a gate
before merge.
