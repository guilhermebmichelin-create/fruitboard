# Phase 2 owner-ready disposition - 2026-09-13

Status: **qualification executed once; Phase 2 is not accepted.** The dated
current-only run is non-qualifying on the unchanged warm-reconciliation target.
This handoff does not authorize issue comments or closures, a budget/scope/
fixture change, production scanning, or Phase 3 work.

## Independently verified measurement-boundary state

Only GitHub-verified merge metadata is recorded as merged here. The current
publication SHA and the measured-source SHA are intentionally separate:

| Item   | Current GitHub state                                                                              | Boundary and evidence                                                                                                                                                                     |
| ------ | ------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `main` | `origin/main` `83da093672b5e2154097c897af533821b04f2352`                                          | Current publication boundary: PR #116 merge, and the exact clean starting point for this documentation refresh                                                                            |
| #110   | Head `e209b8ad46adfc2d66d2c2b18288d0ef254eeb8a`; merge `e2948f1bcc03af162ff95ba06ddd6033f2547da6` | Merged; publishes queued/running/terminal, queued disable/remove, and genuine same-job restart evidence; no acceptance decision                                                           |
| #111   | Head `606635c06b6eabf345c3e91fa95409eee654afea`; merge `914d7bd2475a11a5e7286086f15bce1d4bd148a5` | Merged mixed NTFS/product hardening; installed observations remain tied to tested source `05fb35c153dd3f177b900292d39998da3774b5e4` and report `ef085229471301043a50f4b668901d9c956fb407` |
| #112   | Head `386b4c9808bca0853dbe71757f121d4106b6d82e`; merge `00884ba87c47a24f3ad75aa31f34ef7efe4a5bb8` | Merged Agent 1-owned diagnostic/performance investigation; no performance qualification or Phase 2 acceptance                                                                             |
| #114   | Head `0aa9020c5f524de7f7d0f78226f500f5e021baf3`; merge `71848732216d4ab4e13e73c820b2b8e4d17bddbe` | Merged Agent 1-owned qualification runbook/validator; the dated current-only execution is recorded separately and is non-qualifying, not Phase 2 acceptance                               |
| #115   | Head `c868a91a7fd74a969a9910dd89be212679f05707`; merge `f811cf3cf6c2e0bc4e3cd161bdcd9e063d53eb35` | Merged documentation-only build/disk-space rules; no measurement or acceptance change                                                                                                     |
| #116   | Head `484c5eb2ab3bcf5bca1d2ad419009de5ec61046a`; merge `83da093672b5e2154097c897af533821b04f2352` | Merged documentation-only qualification publication; records the current-only run measured from `69f27f6`, not owner acceptance                                                           |
| #108   | Open draft; head `a44653bd27baaa5d2878c5a3bd118470cb6909dd`                                       | Diagnostic record only: quiet-host gate failed closed at `2561d3f`; later normal-config measurements are not qualification                                                                |
| #113   | Open ready; historical candidate `e90b03cc0bddd1a449e817ea2d89708a85c82ef1`                       | Synthetic combined candidate; no GitHub approval or merge; retain its evidence                                                                                                            |
| #109   | Head `659189528d4cb30568fa7ebdd12c91b454052074`; merge `69f27f64f26aa657182a9260cc8e78f28a5838fb` | Merged documentation-only factual refresh; this is the measured-source boundary for the dated qualification, not the current publication SHA                                              |

The current source, installed observations, CI results, and owner acceptance
are separate facts. A green check proves only that the checked revision passed
that check. It does not make an installed observation current-head evidence or
accept a Phase 2 criterion.

## Subsequent qualification record

The owner-approved fixture/profile decision is now recorded and executed in
[the sanitized 2026-09-13 qualification evidence](qualification-evidence-20260913.md):
`custom-9995`, seed `0`, exactly 10,000 scanner observations,
`repository-minimum`, a dedicated idle window, current merged `main` only, and
unchanged targets. The run retained 10/10 authoritative measured attempts and
3/3 terminal cancellations. The corrected validator returned
`non-qualifying` because warm nearest-rank p95 was 10,173 ms versus the
10,000 ms target. No historical A/B was repeated, and the decision is separate
from the exact measured source pin `69f27f64f26aa657182a9260cc8e78f28a5838fb`.

## Preserved evidence boundaries

- Canonical S5 test/documentation source remains
  `19585dae7bef9ffdec06b71e0627df1b3f7ceb2f`.
- #111's recovered `de396e7` is historical duplicate-S5 provenance with the
  same parent, tree, and stable patch ID as `19585da`; it must not be replayed
  or deleted from the historical record.
- The #111 installed NTFS report remains tested-source evidence from `05fb35c`
  at reviewed report revision `ef085229471301043a50f4b668901d9c956fb407`.
  The corresponding product changes are merged in `914d7bd`, but the NTFS run
  was not silently relabeled as a run on that merged baseline.
- #112 preserves the earlier diagnostic revisions, including
  `eace2e643f462e2cf2b8074ecd66007e2a21d074`, and its merged head
  `386b4c9`; it did not reproduce or explain the original #108 Partial.
- #113's historical candidates, including `5cc8fb549966ebd53c135540b2f4c68339269680`,
  `37cd6c6ec1bef00af03456ddd6155cd0c704c8b8`, and
  `879010d8f3ae91c6f62409cb04ee1f1c8066ab15`, remain audit evidence. The
  frozen `e90b03c` candidate is not a second merge path.

## Recommended dispositions

- **#108:** retain as unexplained historical diagnostics and recommend closing
  without merge only after the owner links its classification to the #112
  record. Do not call the original `Failed`/`Partial` result explained or
  performance-qualified.
- **#113:** retain the synthetic replay and checks as historical evidence and
  recommend closing without merge as superseded by the owner-maintained
  #110/#111/#112/#114/#115/#116 publication stack. Do not retarget or merge it
  as a second integration candidate.

Neither recommendation has been posted or applied.

## Short owner decision list

The fixture/profile choice is already recorded and used. F1/F2 are not pending
selection decisions: do not reselect `custom-9995` or `repository-minimum`, and
do not reinstate historical strict-profile/A/B requirements. The following are
the remaining scope/acceptance decisions; none is accepted by this handoff.

| Decision                     | Recommended scope decision                                                                                         | Technical alternative                                                                                                                                            | Information/approval still needed                                                                                                |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| F3 and 10,000-entry contract | Defer 100,000-entry qualification and retain the existing 10,000-entry contract.                                   | Authorize coordinated end-to-end scaling, then measure memory/disk, latency, cancellation, crash recovery, and publication at 100,000.                           | Dated deferral or explicit scale-project authorization; #96 is proposal history only.                                            |
| #47 DriveFS scope            | Exclude or explicitly leave DriveFS Mirror/Stream unverified for this phase.                                       | Authorize the exact DriveFS mode matrix with mode capture, hydration/placeholder, burst/rename/disconnect/watcher, side-effect, and cloud/pause-resume evidence. | Scope choice and, if included, a qualified synced test environment/consent.                                                      |
| #48 FAT32/cross-volume scope | Exclude or explicitly leave FAT32/cross-volume identity unverified for this phase.                                 | Authorize genuine writable FAT32 USB/VHD and cross-volume runs with drive letter, serial, filesystem, allocation-unit, identity, and cleanup evidence.           | Scope choice and, if included, qualified media/consent; DriveFS, the system partition, and no-media devices are not substitutes. |
| P2-09 watcher evidence       | Accept deterministic/local-NTFS synthetic evidence only within an explicit boundary.                               | Authorize OS-buffer overflow/timing and in-scope DriveFS evidence; code changes follow only a discovered defect.                                                 | Evidence boundary decision.                                                                                                      |
| P2-10 content-read gate      | Accept static guards and fixture source-byte equality for the filesystem-only MVP.                                 | Add a runtime content-read spy test/CI gate with retained logs.                                                                                                  | Static-versus-spy decision; no runtime spy is claimed.                                                                           |
| Onboarding and #107/S5       | Accept inline Preferences onboarding; close #107 as no defect after owner review; do not rerun S5.                 | Build a named first-run route or request defect-specific code/evidence only if review finds a concrete issue.                                                    | Scope confirmation and no-defect disposition.                                                                                    |
| P2 acceptance                | Review the P2-01 through P2-12 ledger; keep P2-08 Partial until its listed requirements are evidenced or excluded. | Complete only owner-selected evidence/defect work.                                                                                                               | Criterion-by-criterion acceptance and any explicit exclusions.                                                                   |

The #114 runbook validates report aggregation from individual measurements; it
does not supply a benchmark, a qualification result, or an acceptance change.
No production scanning or Phase 3 activity is authorized.

## Publication handoff

The #109/#114 documentation-only merge history includes merge commit
`71848732216d4ab4e13e73c820b2b8e4d17bddbe`; the resulting source tree measured
by this qualification is `69f27f64f26aa657182a9260cc8e78f28a5838fb`. Current
publication is `origin/main` `83da093672b5e2154097c897af533821b04f2352`,
from which this branch starts. This handoff preserves that source/evidence
distinction. Run documentation/privacy/formatting checks and required CI on
the new PR head, review the evidence diff, and verify any resulting GitHub
merge and final `main` SHA. Agent 1's merged source work remains attributed to
Agent 1; the owner retains Phase 2 acceptance authority.

## Handoff controls and remaining information

- No local build, fixture generation, installed run, S5/NTFS rerun, or compiler
  cache was used. No cleanup is required; the primary workspace's dirty and
  untracked files were preserved.
- The only working volume used for this documentation task is `C:`. Final
  capacity check: **64.33 GiB free of 475.45 GiB**; the 30 GiB reserve is
  maintained. No `G:` build or evidence output was used.
- Evidence links: [current qualification evidence](qualification-evidence-20260913.md),
  [live review status](README.md#live-review-status-2026-09-13), the merged
  [PR #116](https://github.com/guilhermebmichelin-create/fruitboard/pull/116),
  historical diagnostics [PR #108](https://github.com/guilhermebmichelin-create/fruitboard/pull/108),
  and superseded candidate [PR #113](https://github.com/guilhermebmichelin-create/fruitboard/pull/113).
- Genuinely needed information is limited to owner dispositions: F3 scope,
  #47/#48 inclusion or exclusion, watcher evidence boundary, P2-10 static or
  runtime-spy gate, inline onboarding, #107/S5, and criterion-by-criterion
  acceptance. If P2-08 stays in scope, a current-head installed proof of the
  explicit Retry paths and the merged typed diagnostic presentation is still
  needed; no defect is asserted from the older installed records.
