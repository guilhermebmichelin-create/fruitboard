# Sample presence design review

Status: **Design accepted in PR279; its three bounded implementation slices
merged as PR281/PR283/PR285. Broader G5 remains open.**

This document retains the design boundary: the design PR itself added no probe.
The [merged app review](../sample-presence-ui-284/README.md) records delivered
behavior and the actual installed/source evidence, without promoting this design
to a qualification report.
Date: 2026-10-05. Issue: #278. Baseline: merged PR268,
`91261525383ce79adb090550329167ef347f7253`.

## Owner summary

Fruitboard currently shows a saved sample path without checking the file.
The proposed first version adds an explicit check of full saved paths inside
the selected project folder. It reports whether a file was present at that
exact path when checked. Paths it cannot safely inspect remain "not checked";
they do not become false missing-sample warnings.

PR279 accepted this scope before introducing file access from saved project strings.
The first version deliberately needs separate qualification before expanding
relative paths, FL Studio placeholders or additional sample folders. Those
references remain visible as saved text.

## Review contents

- [Accepted ADR-007](../../adr/007-sample-presence-boundary.md): explicit
  authority, result vocabulary, lifecycle, bounds, privacy and UI behavior.
- [Native authority review](authority-review.md): threats, proof obligations
  and concrete test cases. This is a design audit, not passed implementation tests.
- [Implementation plan](implementation-plan.md): small policy, Windows port and
  application slices with validation and shared-cache ownership boundaries.
- [Maintained gaps](../flp-intelligence-readiness/gap-ledger.md): G5 stays open.

## Decision recorded through normal PR review

PR279 merged on 2026-10-05 at `53e248ae906bfd0a53cbdf6cbe62442c25238669`,
approving the bounded first-version design for subsequent
implementation PRs. The implementation must still demonstrate exact-child
metadata authority, stale/late-result fences and accurate unchecked states.
Rejecting or revising this design changes that planned scope, not current app
behavior. [Policy slice 1](../sample-presence-policy-280/README.md) is implemented
for review. Production activation, broader path mapping, performance budgets and
the Phase 3 checkpoint remain separate decisions.

## Current contract alignment

Merged PR268 already replaced invented Sampler numbering with literal duplicate
defaults and distinct inference provenance. This documentation slice corrects
the architecture paragraph that still described numbering as FL Studio's
display convention. The genuine approved F15 case and original research notes
remain linked; no fixture or product byte changes are included here.

## Validation and handoff

This is documentation-only. Validate pinned Markdown lint, repository privacy,
local links and diff scope; run the normal updated-head CI. No local Cargo,
desktop build, fixture generation or installed test is required.

The baseline tree equals fully tested PR268 head
`f28daeeb88dde0d555d6d15f4254aedfa1ad1b81`; its all-15 successful checks apply
to unchanged product code and approved corpus. They do not replace checks on
the new PR head or prove an unimplemented filesystem probe.

Codex `/root` owns the reused source checkout and existing reusable validation
cache; absolute paths and final check/source receipts stay in the private
handoff. No writable-cache window is used by this task. Retain existing caches,
fixtures, profiles/databases, raw evidence and independently copied binaries.
Future builds require >=30 GiB free after expected output, explicit preflight,
one cache owner and an exclusive heavyweight window. No cleanup, performance
qualification or later-phase work is part of this slice.
