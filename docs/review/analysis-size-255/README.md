# Ordinary-sized FLP analysis

Issue #255 follows the owner's report of the size-limit message on every tested
project. The supplied screenshot shows 4,601,596 bytes, exceeding the original
4 MiB ceiling. The owner confirmed their largest projects are under 25 MB.

The shared parser/native scheduling limit is now 64 MiB (67,108,864 bytes).
Opaque plugin-state event 213 can use that budget, while interpreted events
retain their 2 MiB ceiling. Wrapper state/path subrecords are skipped through
checked slices; name/vendor text and subrecord counts remain bounded. The
descriptor advertises the actual maximum event size, and parser adapter 0.1.1
distinguishes this policy from 0.1.0 for descriptor checks and durable analysis
eligibility. No database migration or source-file mutation is required.

## Validation boundary

Constructed inputs cover the reported 4,601,596-byte size, 25,000,000 bytes,
the exact 64 MiB ceiling and one byte over it. Parser tests verify useful saved
metadata and omission of large opaque state. Malformed/truncated lengths and
oversized interpreted events remain rejected. Storage tests cover automatic
discovery and explicit requests at the ordinary sizes and exact ceiling, and
prove larger inputs create no queued work.

The Windows native integration uses independently constructed large copies of
the approved sample fixture, the actual supervised parser, native same-handle
observations and SQLite publication. It verifies saved facts, source SHA-256,
held-source write/rename rejection and byte-identical input after analysis.
These are correctness tests, not performance qualification or a claim about
the owner's private projects, which were not read.

## Limitations

Saved-build support and all other projection/count/output limits remain as
documented in [the parser contract](../../../crates/flp-parser/README.md).
Increasing the input ceiling does not imply that every FL Studio format is
supported. The Windows app and matching parser must both be rebuilt; an older
running app retains its old limit until closed and reopened with the new build.
Plugin Explorer remains the next feature in #254 after this blocker is fixed.
