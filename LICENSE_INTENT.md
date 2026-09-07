# License and distribution intent

Status: **Phase 0 decision; final product license not yet selected**

The owner intends to release Fruitboard publicly as open source after completion.
That intent is now partially realized: the repository has been public on GitHub
since 2026-09-07. This is repository visibility only — the exact license and a
public application release have not yet been approved, and the repository does
not currently grant an open-source license. A final `LICENSE` file must not be
added by assumption.

## Product-owner direction

The public open-source intention supersedes any assumption that Fruitboard must
remain proprietary. It does not yet constitute selection of GPLv3 or another
specific license. PyFLP is GPL-3.0 and remains blocked as an application
dependency, bundled sidecar, installer artifact, or distributed component.
Phase 1 may define and test the replaceable `FlpParser` interface, but it must
not add PyFLP or package a production FLP parser. The current plan evaluates a
bounded independent Rust parser after the filesystem-only Scanner MVP, with
PyFLP retained as a candidate; see ROADMAP.md and ADR-002.

Research P0-A and P0-B/P0-G may evaluate PyFLP only under the documented
research-fixture, privacy, and isolation rules. Research success does not grant
permission to distribute it.

## Decision paths

Before any parser-backed distribution, the product owner must choose and record
one of these paths in an amended ADR-002 after appropriate legal review:

1. adopt a GPL-compatible Fruitboard distribution model and satisfy all source,
   notice, attribution, and corresponding-license obligations; or
2. retain a non-GPL distribution direction and use a suitably licensed parser,
   obtain alternative PyFLP licensing, or commission a documented clean-room
   parser implementation that does not copy protected PyFLP code.

Until that decision, the conservative rule is simple: no PyFLP dependency, no
PyFLP binary in installers or releases, and no representation that the process
boundary removes license obligations.

This document records engineering and distribution intent, not legal advice.
