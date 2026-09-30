# ADR-002: Versioned FLP parser sidecar boundary

- Status: Accepted; Rust selected for the next production parser implementation
  on 2026-09-28. Compatibility and packaging gates remain open.
- Date: 2026-09-04
- Approved: 2026-09-04 by the product owner

## Context

PyFLP exposes useful FL Studio metadata but is Python-based, unofficial,
GPL-3.0, marked Alpha on PyPI, and its stable release predates current FL Studio
formats. Parser failures must not crash the scanner or couple application data
to PyFLP classes. The product must never edit FLP contents.

## Decision

### 2026-09-28 owner selection

The product owner selected the independent Rust parser approach for the next
production implementation after reviewing the approved ten-fixture research
corpus. The [F12 result](../research/parser-spike-138-f12-result-20260928.md)
shows extraction of a saved sample reference; the
[inferred-name result](../research/parser-spike-138-inferred-default-result-20260928.md)
passes the ten-file current-policy comparison while labeling the displayed
default `Sampler` as an inference. Stable PyFLP 2.2.1 supplied no complete parse
on this corpus. The selection is for an isolated, bounded, read-only Rust
sidecar behind the replaceable `FlpParser` port. It does not authorize enabling
FLP content reads in the filesystem-only scanner or distributing an unqualified
parser binary.

Implementation proceeds in reviewable slices: establish the versioned process
protocol and parser core, validate the maintained approved corpus, then add
supervision and packaging. Compatibility work remains open for FL Studio 20/21,
absent tempo, zero channels, and other untested features. Windows
package/startup/crash/signing/update, provenance, and hostile-input limits must
be reviewed before distribution. The existing PyFLP GPL-3.0 gate remains
closed; no PyFLP package enters the product.

### Original boundary decision

Define a replaceable `FlpParser` port and versioned, read-only JSON-lines
protocol. After the filesystem-only Scanner MVP, evaluate a bounded independent
Rust parser before selecting the production adapter. Python/PyFLP remains a
candidate if the Rust spike does not meet the initial metadata requirements.
The selected parser remains an isolated process: Rust launches it, validates
paths/results, enforces limits/timeouts, and persists immutable snapshots.

This planning amendment authorizes research, not a complete Rust rewrite or
production parser adoption. Follow the field matrix, safety checks, and decision
criteria in [the roadmap](../../ROADMAP.md#parser-selection-before-phase-3).

The selected Rust sidecar may be adopted for distribution only after:

1. a representative FL-version/feature compatibility matrix;
2. Windows package/startup/crash/signing/update tests and provenance review.

Any future PyFLP adoption would additionally require an explicit decision that
the product's distribution complies with PyFLP's GPL-3.0 obligations.

Acceptance covers the replaceable parser boundary and proof-of-concept work,
not permission to add, bundle, or distribute PyFLP. The product owner intends
public open-source distribution, but has not selected the exact license.
See [LICENSE_INTENT.md](../../LICENSE_INTENT.md).

## Alternatives

- Embed CPython: tighter crash and packaging coupling with little user benefit.
- Require system Python: unacceptable installation/reproducibility burden.
- Local HTTP service: unnecessary port/authentication surface compared with
  stdio.
- Remote parser: violates offline/privacy goals by uploading FLPs.
- Immediate Rust rewrite: high reverse-engineering cost before the required
  metadata/reliability boundary is measured. A bounded Rust research spike is
  now the first evaluation step; a full rewrite remains unapproved.

## Consequences

- Parser upgrades and replacements do not change core repository interfaces.
- Process startup, artifact size, antivirus behavior, and cross-platform builds
  become explicit engineering work.
- Typed partial/unsupported/failed outcomes preserve metadata honesty.
- A sidecar crash loses one in-flight parse and is recoverable.
- Process separation does not itself resolve licensing obligations.
- If any gate fails, Scanner MVP can still discover filesystem metadata while a
  different parser is evaluated.

## Phase 1 implementation note

Issue #11 pins an isolated Python 3.11.16 research environment and its empty
dependency lock. This is toolchain preparation only: PyFLP is absent from the
manifest and lockfile, and the parser adoption/distribution gates remain
unchanged.

Issue #18 packages a zero-dependency Rust executable solely to prove Tauri's
external-binary lifecycle from paths and arguments containing spaces/Unicode.
It responds to one fixed ping, exits with one controlled failure, or waits to be
timed out and terminated. It has no parser protocol, Python runtime, PyFLP,
filesystem input, or product authority. P0-G is satisfied, while P0-B/P0-C and
the three adoption conditions above remain closed.

## Rust supervision implementation note

Issue #226 adds the first native process-transport slice inside the selected
Rust parser crate: bounded request/reply pipes, protocol envelope validation,
deadlines, cancellation, process retirement/restart, request-count recycling,
and shutdown. It discards subprocess stderr and returns fixed transport-error
categories. Real-process and approved-corpus tests validate this component.

Extended application validation and publication freshness checks, scanner
integration, packaged binary resolution, resource-limit qualification, and
distribution gates remain open. This component adds no renderer command,
parser packaging, or automatic FLP content reads.

Issue #230 adds the initial native result-validation component for five core
metadata fields. It validates the selected descriptor, field/provenance and
fingerprint forms, and caller-supplied current file/root observations, returning
typed data without unvalidated extensions or raw diagnostics. This does not
wire the parser into the scanner or storage. Extended metadata validation and
transactional publication freshness checks remain open with application
integration, packaging and qualification.

Issue #232 subsequently adds project dates, the saved FL time counter, bounded
top-level plugin references and a verified pattern-clip bar span. Its separate
full typed projection checks these facts and exposes authoritative request
size/modified time without activating parsing in the app. Embedded/local and
filesystem dates stay separate, and saved FL time never becomes tracked work.
Constructed metadata tests do not expand independent GUI-qualified compatibility;
see [the format/evidence record](../research/parser-project-facts-232.md).

## References

- [Detailed parser design](../../FLP_PARSER.md)
- [Tauri sidecars](https://v2.tauri.app/develop/sidecar/)
- [PyFLP package/license](https://pypi.org/project/pyflp/)
