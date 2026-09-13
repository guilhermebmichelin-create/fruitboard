# Phase 2 owner-ready disposition - 2026-09-13

Status: **qualification executed once; Phase 2 is not accepted.** The dated
current-only run is non-qualifying on the unchanged warm-reconciliation target.
This handoff does not authorize issue comments or closures, a budget/scope/
fixture change, production scanning, or Phase 3 work.

## Independently verified measurement-boundary state

Only GitHub-verified merge metadata is recorded as merged here:

| Item | Current GitHub state | Boundary and evidence |
| --- | --- | --- |
| `main` | `69f27f64f26aa657182a9260cc8e78f28a5838fb` | Exact current-main source used by the dated scanner qualification after the verified #109, #110, #111, #112, and #114 merges |
| #110 | Head `e209b8ad46adfc2d66d2c2b18288d0ef254eeb8a`; merge `e2948f1bcc03af162ff95ba06ddd6033f2547da6` | Merged; publishes queued/running/terminal, queued disable/remove, and genuine same-job restart evidence; no acceptance decision |
| #111 | Head `606635c06b6eabf345c3e91fa95409eee654afea`; merge `914d7bd2475a11a5e7286086f15bce1d4bd148a5` | Merged mixed NTFS/product hardening; installed observations remain tied to tested source `05fb35c153dd3f177b900292d39998da3774b5e4` and report `ef085229471301043a50f4b668901d9c956fb407` |
| #112 | Head `386b4c9808bca0853dbe71757f121d4106b6d82e`; merge `00884ba87c47a24f3ad75aa31f34ef7efe4a5bb8` | Merged Agent 1-owned diagnostic/performance investigation; no performance qualification or Phase 2 acceptance |
| #114 | Head `0aa9020c5f524de7f7d0f78226f500f5e021baf3`; merge `71848732216d4ab4e13e73c820b2b8e4d17bddbe` | Merged Agent 1-owned qualification runbook/validator; the dated current-only execution is recorded separately and is non-qualifying, not Phase 2 acceptance |
| #108 | Open draft; head `a44653bd27baaa5d2878c5a3bd118470cb6909dd` | Diagnostic record only: quiet-host gate failed closed at `2561d3f`; later normal-config measurements are not qualification |
| #113 | Open ready; historical candidate `e90b03cc0bddd1a449e817ea2d89708a85c82ef1` | Synthetic combined candidate; no GitHub approval or merge; retain its evidence |
| #109 | Head `659189528d4cb30568fa7ebdd12c91b454052074`; merge `69f27f64f26aa657182a9260cc8e78f28a5838fb` | Merged documentation-only factual refresh; its resulting tree is the measured-source boundary for the dated qualification |

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

- **#108:** retain as diagnostic evidence and recommend closing without merge
  only after the owner links its classification to the #112 record. Do not
  call the original `Failed`/`Partial` result explained or performance-
  qualified.
- **#113:** retain the synthetic replay and checks as historical evidence and
  recommend closing without merge as superseded by the owner-maintained
  #110/#111/#112/#114 source stack. Do not retarget or merge it as a second
  integration candidate.

Neither recommendation has been posted or applied.

## Remaining acceptance and scope decisions

The fixture/profile choice and one current-only measurement are recorded above.
The following acceptance and scope decisions remain; none is accepted by this
handoff, and no recommendation changes the existing budget or scope:

- F1 fixture alignment is recorded for this run as exactly 10,000 observations
  using `custom-9995`, seed `0`, and manifest
  `a4760a282395adf43ee0433499c0a178f3d9e5e2faa0b1237256f26c1196d08a`.
  Preserve the accepted 10,005-observation baseline hash
  `8c3d85ec01299995208abfa450378b1b37c704e423593d7a4370ec465254afba` as
  historical evidence unless the owner explicitly changes the contract.
- F2 has one current-only non-qualifying result: the unchanged warm-p95 target
  was missed by 173 ms. Any future measurement needs a separately authorized
  new run; this record does not authorize A/B testing, target changes, or a
  rerun to seek a pass. F3 scale evidence remains unmeasured.
- Decide F3 scale treatment and the current 10,000-entry contract; do not
  adopt a new limit or budget from a recommendation alone.
- Decide Phase 2 scope for #47 DriveFS and #48 FAT32/cross-volume identity, and
  obtain the exact qualified runs if they remain in scope.
- Decide the P2-09 overflow/timing evidence boundary and P2-10 static versus
  runtime content-read-spy gate.
- Review inline onboarding, #107/S5 disposition, and each P2-01 through P2-12
  criterion for explicit owner acceptance. P2-08 remains Partial; no criterion
  is promoted by this documentation merge.

The #114 runbook validates report aggregation from individual measurements; it
does not supply a benchmark, a qualification result, or an acceptance change.
No production scanning or Phase 3 activity is authorized.

## Publication handoff

The #109/#114 documentation-only merge history includes merge commit
`71848732216d4ab4e13e73c820b2b8e4d17bddbe`; the resulting `main` tree measured
by this qualification is `69f27f64f26aa657182a9260cc8e78f28a5838fb`. This
handoff preserves that source/evidence distinction.
Publication is based on post-#115 main
`f811cf3cf6c2e0bc4e3cd161bdcd9e063d53eb35`. Preserve the measured SHA
above when the documentation baseline advances. Run documentation/privacy/formatting checks and the
required CI, review the evidence diff, and verify the resulting GitHub merge
and final `main` SHA. Agent 1's merged source work remains attributed to Agent
1; the owner retains Phase 2 acceptance authority.
