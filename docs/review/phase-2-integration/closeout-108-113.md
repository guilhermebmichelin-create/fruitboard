# Phase 2 close-out rationale for PR #108 and PR #113

Status: **draft recommendation only; neither pull request was merged, closed,
retargeted, or edited by this task.** This record applies the acceptance
packet's [Proposed issue updates](acceptance-packet-2026-09-12.md#proposed-issue-updates-not-posted)
to the two open draft candidate PRs. It does not accept Phase 2, does not
qualify scanner performance, and does not mutate any GitHub issue or PR.

## Basis

The packet's owner/maintainer section classifies #108 as performance
**diagnostics only** and #113 as an **older, superseded synthetic candidate**.
It recommends closing each without merge once linked to its classification and
successor evidence. The related merged stack is #110, #111, #112, #114, #115,
and #116. PR #108's classification is linked to merged #112; PR #113 is
superseded by the merged #110/#111/#112/#114/#115/#116 stack.

## PR #108 - performance measurement diagnostics

- Repository link: [#108](https://github.com/guilhermebmichelin-create/fruitboard/pull/108)
- Title: `docs(#41): record performance measurement evidence`
- Head SHA: `a44653bd27baaa5d2878c5a3bd118470cb6909dd`
- State at review time: open draft; no merge commit.

Packet disposition (verbatim):

> **#108:** Preserve the quiet-host failure and later normal-config results as
> unexplained historical diagnostics. Recommend closing without merge after
> linking its classification to #112; do not claim performance qualification
> or silently convert its historical strict/A/B recommendation into a current
> requirement.

Rationale: the coordinated strict quiet-host gate failed closed, the retained
current-main `Failed`/`Partial` iteration is unexplained, and merged #112
records that the original Partial was not reproduced while adding bounded
diagnostic coverage. The individual check runs on the #108 head remain per-PR
provenance, not integrated or performance-qualification evidence. There is no
merge vehicle remaining for the historical strict/A/B recommendation.

Recommended action: close without merge after linking the classification to
PR #112 (`386b4c9808bca0853dbe71757f121d4106b6d82e` / merge
`00884ba87c47a24f3ad75aa31f34ef7efe4a5bb8`). Not executed here.

## PR #113 - superseded synthetic combined candidate

- Repository link: [#113](https://github.com/guilhermebmichelin-create/fruitboard/pull/113)
- Title: `chore(#41): publish Phase 2 combined review candidate`
- Head SHA: `e90b03cc0bddd1a449e817ea2d89708a85c82ef1`
- State at review time: open and ready; no merge commit.

Packet disposition (verbatim):

> **#113:** Preserve its replay, frozen candidate, and checks as historical
> evidence. Recommend closing without merge as superseded by the merged
> #110/#111/#112/#114/#115/#116 stack; do not retarget or merge it as a second
> integration candidate.

Rationale: #113 replays the #110-#112 source stack onto an older `main`
(`3ebac5f7a76c3425620ceba59e6078b32fb6cd85`) and is not the current source
boundary. Its ten green contexts are historical candidate evidence; the merged
stack (`#110` merge `e2948f1`, `#111` merge `914d7bd`, `#112` merge `00884ba`,
`#114` merge `7184873`, `#115` merge `f811cf3`, `#116` merge `83da093`) is the
publication baseline. Re-merging #113 would duplicate already-merged work and
must not be used as a second integration candidate.

Recommended action: close without merge; do not retarget. Not executed here.

## Explicit non-actions

- No merge, close, reopen, retarget, label, review, comment, or other GitHub
  mutation was performed on #108, #113, or any issue.
- No performance qualification, Phase 2 acceptance, or production activation
  is claimed.
- This document is coordination/history only and does not change the packet's
  decision list.
