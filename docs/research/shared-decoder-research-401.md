# Shared-decoder compatibility research

Issue: [#401](https://github.com/guilhermebmichelin-create/fruitboard/issues/401).
Parent: the [baseline and experiment protocol](cross-build-pilot-399.md), PR400.
Status: **Opt-in research tooling; no additional production-supported build.**

## Development outcome

The [public-source survey](flp-version-literature-399.md) identifies different
reported event172 widths and field-specific format changes. The new probe
executes Fruitboard's actual bounded Rust decoder with an explicit exact-build
and event172-width hypothesis. This replaces neither the independent GUI oracle
nor genuine saves; it makes the planned shared-decoder experiment executable.

The existing production entry points keep their original three-build admission
and framing rules. The native protocol/capabilities, default packaging, adapter
version, validation, immutable storage and Library behavior are unchanged. The
ordinary application does not gain a research option or a broader support claim.

## Isolation and limits

`compatibility-research` is a Cargo feature disabled by default. Its library API
is `research::inspect(bytes, profile)`; it performs no file IO. A profile requires
a canonical four-component exact build and a choice of one, three or four bytes
for event172. The decoder admits only that exact build for the experiment. It
never chooses a profile because parsing succeeds.

Both modes instantiate the same decoder implementation. Production supplies its
existing version check, width function and a no-op event observer; research
supplies exact-build admission, the registered width and a bounded observer.
The existing 64 MiB file, 100,000 event, channel/text/payload limits and malformed
checks remain. Other per-field build restrictions and default-label inference
rules still apply. Research can therefore fail on a semantic guard as well as
framing; a failure alone does not prove format incompatibility.

The research result is explicitly `researchOnly: true`, `qualified: false`.
It contains the profile, decoder outcome/code, five pilot field observations
(version, base tempo, channel count/names and sample-reference text), and at most
32 event-boundary records. The prefix records IDs/offsets/lengths, never opaque
payload bytes. Truncation is explicit. It has no native metadata/protocol
envelope; it cannot be substituted for a validated application result. Calling
the normal parser with the feature enabled still rejects unlisted builds.

## Fixed approved-control probe

```text
cargo run -p fruitboard-flp-parser --features compatibility-research --example compatibility_probe --locked
```

This executable accepts no arguments or arbitrary candidate paths. It uses the
same frozen F01/F06/F11/F12 specification as the baseline runner and compares all
three explicitly registered width hypotheses. It uses native root authorization
and held read-only files, checks exact size/hash, bounds each read to the fixed
control size plus one, then independently rereads the held source and verifies
unchanged bytes/size/time before producing any report. Controls are at most
128 KiB and the final report is capped at 256 KiB. No input is written.

For externally supervised use, bound the process lifetime as in the local
validation runner; the Rust example itself is a finite fixed-corpus command,
not a general process supervisor. Do not publish arbitrary future API observations:
candidate field text needs its own privacy review. The supplied example reads
only the already approved, hash-pinned public controls.

New-build files require a later reviewed exact-case registration with independent
expectations. This example deliberately does not accept such files yet. The
library API changes research admission, not production admission; it does not
patch saved versions or convert constructed tests into genuine-save evidence.

## Correctness checks

New regressions cover:

- Production rejection of an unlisted build before and after a research call,
  with the research feature enabled.
- Exact-build mismatch, canonical profile validation and absence of product
  metadata envelopes/extra field groups in observations.
- A constructed ambiguity: one- and three-byte interpretations both complete
  with identical basic fields but different event boundaries. Both remain
  unqualified; no automatic winner is selected.
- Bounded prefix output and unchanged malformed/oversized-input rejection.
- The approved controls matching the unchanged production decoder's five fields
  under their existing production width rules.
- Product protocol rejection of a research method in both build modes.

The required `rust-portable` CI job adds feature-enabled tests, warning-denied
all-target Clippy and the fixed example alongside the existing default tests.
All existing CI contexts remain intact. Windows local validation also covers
both modes and the authorized fixed-control example; no full desktop or
installed/performance run is claimed for this tooling change.

## Executed results

[Sanitized results](shared-decoder-results-401.json) pin the implementation and
independently copied binaries. Default mode passes **133 tests** at `794231f`;
research mode passes **140 tests**, repeated at `7d987c3` after moving the profile
length check ahead of allocation and adding an oversized-profile regression.
That follow-up changes only the default-disabled research module and its gated
tests; the normal parser's compiled inputs are identical. Both all-target Clippy
modes pass with warnings denied. The current normal executable also passes the
four-control/twenty-field baseline over the real authorized protocol.

The four-control research example passes bounded execution and argument refusal.
For approved F11/F12, one- and three-byte hypotheses both complete with identical
five-field observations, but report 1,423 versus 1,421 events and different
framing prefixes. Four-byte framing fails with `EVENT_LENGTH_OUT_OF_BOUNDS`.
This independently reproduces a concrete ambiguity on existing approved bytes;
it does not settle the correct general width or qualify another saved build.
F01/F06 complete under all three hypotheses, with no event172 in the captured
prefix. Absence from that bounded prefix is not a whole-file absence claim.

All **36 original FLP hashes** and both owner-draft hashes remain unchanged.
No new fixture, GUI operation, external package or production activation is
included. The minimum observed build capacity is about **30.61 GiB free**;
future work needs a fresh preflight. All raw receipts and earlier source results
remain private, alongside independently retained default/probe binaries.

## Source and storage boundary

Owned primary-checkout changes are the parser feature manifest, shared-decoder
policy seam, isolated research module/example, defensive tests, focused CI
commands and this documentation. Parent source is PR400's `a314116`; its
baseline runner and literature findings remain separately reviewed. The new
parser source requires its own tests and normal updated-head CI; earlier native
results are not promoted to it.

Owner `/root` sequentially reuses the existing `desktop-feature-tests` cache
with an explicit target directory and release profile, preserving the primary
`target`. The four-path preflight allows 0.65 GiB additional output while
maintaining the 30 GiB reserve. No parallel heavy build or installed/qualification
window is used. Private receipts, exact paths, hashes, current capacity and
independent copied evidence remain outside compiler caches under
`shared-decoder-research-20261104`. No new permanent cache or blanket cleanup is
introduced. Retain the evidence and owner drafts; mutable compiler outputs are
reusable, not historical binary evidence.
