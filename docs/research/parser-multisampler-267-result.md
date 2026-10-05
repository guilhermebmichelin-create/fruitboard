# Default Sampler labels: multi-channel qualification (issue #267)

Status: the owner approved the exact F15 bytes and registered expectations on
2026-10-05. PR268 adds the fixture, pinned privacy entry and read-only regression
test. This qualifies one clone path in the existing exact 2026 build; no new
supported build or broader Phase 3 acceptance is claimed.

## Independent GUI case

The case was registered before generation/parsing: start from an independent
copy of approved F11 in FL Studio 26.1.0.5530, keep 130 BPM, clone the original
empty Sampler twice, and record the actual automatic labels before saving.
No personal song, note, sample, playlist placement or external plugin was added
in the clean attempt. The pre-save GUI showed three channels named `Sampler`,
`Sampler`, `Sampler`, each built-in Sampler with File `(none)`.

An initial automation attempt lost focus while entering the Save As filename
and added unwanted Edison recording state. That save was excluded, retained
privately as a failure, and never parsed or offered for publication. The clean
case restarted from the unchanged F11 copy and repeated the observations.

The clean GUI save is 48,281 bytes. Sanitation removed exactly one registration
event (ID 200, 16-byte payload plus two framing bytes) and corrected the FLdt
length; every other byte is unchanged. Candidate
`FIX-FL2026-MULTISAMPLER.flp` is 48,263 bytes, SHA-256
`d9f09c8f61293ce1ea95ee925af0bdb67b4978668cc273966e553cc6f2e5a71e`.
It reopened in the same GUI with identical labels, tempo and empty sample fields;
closing without saving preserved its hash. Private screenshots retain the
independent observations but display account information and stay outside Git.

Bounded event/byte checks found no registration event, non-Public Windows home
path, known local account name, email or RIFF payload in the candidate. Its
project-data folder is the existing synthetic Public Documents fixture path.
These scans do not fully decode opaque state. The raw save and failed attempt
remain private; the owner approved only the exact sanitized bytes and registered
expectations ("Approve this exact fixture and expectations") under
[the fixture rules](../../DEVELOPMENT.md#parser-fixture-rules).

## Demonstrated mismatch and correction

The retained parser compiled from `e1e00c6dc824dcbd31ead5a8ef7518b1536c5539`
(parser/Cargo inputs identical to merged PR266,
`fc5fdb636f6908ad931485b7654d77aacdd5c7b4`) read the unchanged candidate.
It returned the correct build, 130 BPM, three channels and Sampler instruments,
but invented `Sampler 2` and `Sampler 3` for the last two editable labels.
Neither the file nor the GUI supplies those suffixes: all three class events
are empty, and no editable-label event is stored.

New results infer the literal `Sampler` for each unstored verified Sampler
label. A separately stored label such as `Sampler 2` stays extracted verbatim.
First-default confidence remains high; later defaults and the aggregate retain
medium confidence. The observation does not qualify every creation path or
multi-channel FL Studio 2025 projects. Instruments/plugin references keep their
existing separate provenance and semantics.

The new name method is `sampler-label-for-known-build`; mixed stored/default
labels use `mixed-extracted-and-sampler-label`. Fresh reply validation requires
these methods, exact literal defaults, per-item confidence and array agreement;
the old numbered method cannot publish a new result. Storage records the typed
method without retaining arbitrary extensions.

## Immutable older results

The private projection keeps its version-1 shape; no database migration or
snapshot rewrite is needed. Native display recognizes both the new method and
the previous bounded `sampler-default-for-known-build` method. An older numbered
value stays readable and explicitly explains that an earlier Fruitboard inferred
it, with an instruction to analyze the project again. Merely refreshing reads
saved results; it does not rewrite them or start analysis.

Mixed old/new per-item methods, mismatched aggregate methods, arbitrary suffixes,
unverified builds and inconsistent confidence fail closed. The renderer accepts
the corrected duplicate defaults and the bounded legacy values emitted by the
native projection. Existing source/root/fingerprint fences, retry limits and
read-only parser authority are unchanged. Reverting to an older app does not
teach its old validator the new method; corrected snapshots require this app
version for display, while their immutable data remains retained.

## Validation and remaining work

Focused regression tests cover repeated defaults, sparse/reordered channels,
stored numbered labels, fresh-reply method/name/confidence rejection, mixed
provenance and legacy display without mutation. The owner-approved F15 regression
uses the genuine-save bytes and independently recorded GUI values, pins the
exact SHA-256 and verifies unchanged bytes after file parsing. Constructed tests
remain defenses, not additional GUI or supported-build coverage. The previous
twelve approved fixtures retain their bytes; the corpus now has thirteen files.

Pinned Windows full/default checks, changed native-feature checks, a direct
corrected-parser comparison and updated-head CI are required before handoff.
Their final results/source hashes are recorded with the private handoff.
No performance budget, installed workflow or broader Phase 3 acceptance is
promoted by these checks.

This resolves the discovered naming defect and adds its approved fixture. G3
still needs the other independent fixture cases in the
[readiness ledger](../review/flp-intelligence-readiness/gap-ledger.md): additional
exact builds, absent tempo/zero channels, richer arrangements, external wrapper
metadata and independent embedded date/time-counter ground truth.

## Build and evidence handoff

Codex `/root` owns the reused review checkout and the existing single
`bounded-probes` validation cache. Its absolute paths and exclusive write windows
are recorded in the private task plan. Preflight checks source/build/cache/
evidence separately, with at least 30 GiB free after expected output (6 GiB for
checks). GUI preparation ran alone; no additional agents/caches were created.
Qualification measurements were not run.

Private preregistration, screenshots, raw/sanitized saves, sanitation receipts,
baseline/corrected parser replies, source/binary hashes, check logs and handoff
stay outside compiler caches. Retained executables are independent copies.
Reuse the existing cache sequentially; retain these records and the earlier
failed attempt. No source, profile, fixture, database or evidence cleanup is
part of this task.
