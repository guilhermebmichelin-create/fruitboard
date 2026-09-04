# License and distribution intent

Status: **Phase 0 decision; final product license not yet selected**

Fruitboard is currently a private architecture-stage repository. No public
distribution is approved, and the repository does not currently grant an open
source license. A final `LICENSE` file must not be added by assumption.

## Product-owner direction

As of 2026-09-04, the product owner has **not** accepted GPL distribution for
Fruitboard. PyFLP is GPL-3.0 and therefore remains blocked as an application
dependency, bundled sidecar, installer artifact, or distributed component.
Phase 1 may define and test the replaceable `FlpParser` interface, but it must
not add PyFLP or package a production FLP parser.

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
