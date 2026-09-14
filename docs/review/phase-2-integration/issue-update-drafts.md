# Proposed issue update drafts (owner/maintainer)

Status: **DRAFT - not posted. No GitHub issue was commented on, edited,
labeled, closed, or otherwise mutated by this task.**

These are paste-ready comment bodies for the owner/maintainer. They restate the
acceptance-packet [Proposed issue updates](acceptance-packet-2026-09-12.md#proposed-issue-updates-not-posted)
for issues #33, #36-#41, #47, #48, and #107. Posting them is an owner action
and is out of scope for this documentation change. No owner acceptance is
implied.

## #33 - Phase 2 epic

**DRAFT - not posted.** Paste-ready comment body:

```markdown
Keep this epic open. Link the merged acceptance packet
(docs/review/phase-2-integration/acceptance-packet-2026-09-12.md) and state
that implementation and evidence are present but the owner has not accepted the
P2 criteria. Record that `custom-9995` and `repository-minimum` were approved
and used; do not reopen F1/F2 selection. Keep F3, platform scope, P2-10,
onboarding, S5 disposition, and criterion acceptance as explicit decisions.
Note that merged #111 is mixed product-and-evidence work, not evidence-only.
Phase 2 is not accepted.
```

## #36, #37, #38, and #40 - feature acceptance pending

**DRAFT - not posted.** Paste-ready comment body for each of #36, #37, #38,
and #40:

```markdown
Replace "implementation pending" wording with "implementation merged;
acceptance evidence is classified in the 2026-09-12 packet." Keep this issue
open until the owner accepts its criterion and any named installed/platform gap
is resolved or explicitly scoped out.
```

## #39 - filesystem-only direction

**DRAFT - not posted.** Paste-ready comment body:

```markdown
Record that the filesystem-only/no-parser direction is implemented, then
record the owner's static-boundary versus runtime-spy choice. Do not claim a
runtime spy before it exists.
```

## #41 - Phase 2 evidence aggregator

**DRAFT - not posted.** Paste-ready comment body:

```markdown
Keep this aggregator open and link the merged acceptance packet plus the dated
live status table (docs/review/phase-2-integration/README.md#live-review-status-2026-09-13).
State that the current publication boundary is `origin/main` `83da093` after
#116, while the measured source is `69f27f6` and #113 is an older combined
candidate; its checks are not acceptance. Preserve the canonical S5 source
`19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`, the tested #111 product source
`05fb35c153dd3f177b900292d39998da3774b5e4`, and the report revision
`ef085229471301043a50f4b668901d9c956fb407`. State that the original Partial
remains unexplained and not reproduced, #112 adds bounded diagnostics without
establishing cause or performance acceptance, and #111 is mixed
product/evidence work. Include the duplicate-S5 mapping and independent Agent 2
review boundary. This is not a Phase 2 acceptance statement.
```

## #47 - DriveFS scope

**DRAFT - not posted.** Paste-ready comment body:

```markdown
Request the missing DriveFS UI mode capture, disposable synced leaves, and
cloud/pause-resume consent, or record an explicit owner scope exclusion. Do not
report another blocked inventory as a run.
```

## #48 - FAT32/cross-volume identity

**DRAFT - not posted.** Paste-ready comment body:

```markdown
Request a genuine disposable FAT32 USB/VHD with recorded identity metadata and
cross-volume consent, or record an explicit scope exclusion. Do not use DriveFS,
the system partition, or the no-media device as a proxy.
```

## #107 - S5 disposition

**DRAFT - not posted.** Paste-ready comment body:

```markdown
Record that #106 demonstrated successor convergence after a terminal
`follow_up_requested` transition, while the September 12 evidence separately
demonstrates same-job recovery from a genuinely running lease. Agent 2 found no
defect and published the isolated test/report artifacts in merged #110. #111's
`de396e7` is tree- and patch-identical S5 recovery provenance, not a second run.
No additional S5 or NTFS run is requested unless a specific new evidence defect
is identified and separately authorized. Require a product/contract change only
if review establishes a genuine defect.
```

## Non-actions

No issue was closed, and no owner acceptance is inferred by these drafts. This
file is a convenience copy of the packet proposals, not a posted decision.
