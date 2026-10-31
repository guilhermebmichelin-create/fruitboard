# Saved-note differential research tooling

Issue [#366](https://github.com/guilhermebmichelin-create/fruitboard/issues/366).
Source baseline: main `83a04aa8dc65decaf32452e4c18cd4e32a734140`.
Date: 2026-10-31. **Research tooling only; musical fields remain unqualified.**

## Purpose and use

PR367 made numeric observations reproducible on approved F16/F17. The next
genuine-save experiment needs to compare a baseline with each independently
observed pitch-, position-, duration- or velocity-only variant. A list of plausible
values cannot show whether another part of the record changed as well.

The new command reads two explicit, hash-pinned inputs and reports every changed
byte within their corresponding 24-byte note records:

```text
node scripts/compare-note-records.mjs BASELINE_PATH BASELINE_SHA256 CANDIDATE_PATH CANDIDATE_SHA256
```

Use only private purpose-built synthetic saves from the
[registered experiment](parser-note-sequence-366.md#genuine-save-experiment).
Register actual GUI properties before inspecting bytes; save, reopen without
playback and retain that independent evidence. Pin both exact saved-file digests
before comparison. This command does not create, sanitize or approve fixtures.
Its explicit-path support is separate from the existing fixed F16/F17 observer,
which still accepts no arguments and never reads arbitrary project roots.

No command writes either input or enumerates a directory. Each input must be a
regular file between 22 bytes and 64 MiB, match its supplied lowercase SHA-256,
and pass the existing exact-build/layout observer. Where supported, files are
opened nonblocking so a named pipe is rejected without waiting for a writer.
Both inputs are reopened and
hash-checked again before any report is printed. Failure emits one fixed message,
no report, no path and no arbitrary exception text. Input ownership, quiescence
and privacy remain the experiment runner's responsibility; this offline tool
does not establish scanner or filesystem security authority.

## What the report means

The report contains fixed labels, digests and numeric values only. It includes
all 24 bytes per observed record, so changes outside the candidate words cannot
disappear. Numeric candidate position/flags/channel/length/key words and byte 21
are also compared for convenience. Their names remain hypotheses; in particular,
byte 21 does not become qualified velocity because it changes in a test.

Pairing requires identical header PPQ, pattern IDs in source order and record
cardinality. An absent note payload is refused, not treated as an extracted
empty sequence. Added/removed patterns, reordered IDs or changed record counts
require separate structural research instead of guessed correspondence.
The observer retains its 32-record total cap and malformed-input refusals.
Identical and simultaneous records keep separate source ordinals; no sorting or
deduplication occurs. Alignment by ordinal is positional only: matching shape
cannot establish musical identity or detect every semantic reordering.

The comparison covers **note payload bytes only**. Hashes bind the full inputs,
but unchanged note records do not imply unchanged project metadata, plugin state
or other FLP sections. Changed absolute event offsets are reported independently
for each input. A successful comparison sets `musicalFieldsQualified: false`;
it never asserts that a GUI experiment passed or a field has a musical meaning.

No parser descriptor, application protocol, storage, Library view, output budget
or supported build changes. The [proposed sequence contract](note-sequence-contract-366.md)
and separate parser/validation, storage and Library implementation steps remain
pending genuine-save qualification. MIDI export and full-song reconstruction
remain outside this change.

## Validation and host boundary

Tests compare approved F16 with itself and make explicitly constructed changes
to in-memory copies only. They cover candidate-word deltas, uninterpreted bytes,
duplicate retention, the 32/33-record boundary, structural mismatch, missing or
malformed payloads, digest failures and path-free CLI errors. A Linux-only test
verifies that a FIFO in either input position fails privately without a writer.
Constructed changes
prove the observer's defenses, not independent musical ground truth. No new FLP
files are generated or registered, and approved input bytes remain unchanged.

Required local checks are the pinned Node policy/script tests, script syntax,
scoped Prettier/Markdown lint, repository privacy and whitespace checks. Normal
updated-head CI remains required. PR367's 15 successful checks apply to its
unchanged research source; merged PR357's 15 passes apply to that tested
application/dependency integration, not to this new script or performance.
No local Rust/app build, dependency install, installed test or measurement is
needed for this scripts/tests/docs-only source boundary.

C: had approximately 28.45 GiB free and G: 12.38 GiB, below the 30 GiB reserve.
Only lightweight source, tests and text evidence proceed. No fixture generation,
GUI authoring session, build or installed validation starts under this capacity.
The prior locked-session result remains historical; this task does not claim a
new GUI attempt. Restore capacity with an appropriate independently verified
cleanup or suitable volume before the next genuine-save experiment, then run
the four-path preflight with an honest output allowance. An unlocked matching
FL Studio session and existing ownership guards remain prerequisites.

The primary checkout is the owned source worktree. `/root` owns this change and
its private evidence under the dated `note-record-differentials-20261031`
directory outside source/compiler caches. No local compiler cache is needed or
written. The primary development cache and reusable `desktop-feature-tests`
cache remain preserved under their existing `/root` coordination with write
windows closed. There is no heavyweight-build or installed-test window, new
per-PR cache, obsolete compiler output or cleanup/rebuild cost from this task.
Preserve earlier fixtures, databases, reports and binaries. New public fixture
bytes still require a completed exact-byte owner approval packet under the
[fixture rules](../../DEVELOPMENT.md#parser-fixture-rules).
