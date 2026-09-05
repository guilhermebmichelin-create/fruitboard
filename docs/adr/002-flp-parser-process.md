# ADR-002: Versioned FLP parser sidecar boundary

- Status: Accepted (conditional; PyFLP adoption and distribution remain blocked)
- Date: 2026-09-04
- Approved: 2026-09-04 by the product owner

## Context

PyFLP exposes useful FL Studio metadata but is Python-based, unofficial,
GPL-3.0, marked Alpha on PyPI, and its stable release predates current FL Studio
formats. Parser failures must not crash the scanner or couple application data
to PyFLP classes. The product must never edit FLP contents.

## Decision

Define a replaceable `FlpParser` port and versioned, read-only JSON-lines
protocol. The leading adapter is a supervised Python/PyFLP sidecar packaged as
an architecture-specific Tauri external binary. Rust launches it, validates
paths/results, enforces limits/timeouts, and persists immutable snapshots.

The sidecar may be adopted for distribution only after:

1. a representative FL-version/feature compatibility matrix;
2. Windows package/startup/crash/signing/update tests;
3. an explicit decision that the product's distribution complies with PyFLP's
   GPL-3.0 obligations.

Acceptance covers the replaceable parser boundary and proof-of-concept work,
not permission to add, bundle, or distribute PyFLP. The product owner has not
accepted GPL distribution. See [LICENSE_INTENT.md](../../LICENSE_INTENT.md).

## Alternatives

- Embed CPython: tighter crash and packaging coupling with little user benefit.
- Require system Python: unacceptable installation/reproducibility burden.
- Local HTTP service: unnecessary port/authentication surface compared with
  stdio.
- Remote parser: violates offline/privacy goals by uploading FLPs.
- Immediate Rust rewrite: high reverse-engineering cost before the required
  metadata/reliability boundary is measured.

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

## References

- [Detailed parser design](../../FLP_PARSER.md)
- [Tauri sidecars](https://v2.tauri.app/develop/sidecar/)
- [PyFLP package/license](https://pypi.org/project/pyflp/)
