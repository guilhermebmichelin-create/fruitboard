# Remaining installed resource characterization, 2026-10-19

**Result: the attempt04 100-entry startup timing was below its proposed 1,500 ms candidate, but the unchanged all-process CPU guard failed. This is characterization only. The run stopped after that first noisy window; resource budgets, complete qualification and the Phase 3 exit remain unapproved.** Issues [#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306) and [#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259) remain open.

The run reused a retained populated review profile and the already-built analysis-jobs package. It did not repeat the four guarded latency windows from the [October 18 report](../resource-current-20261018/README.md). It recorded one 100-entry startup series and 14 valid graceful-close traces. Startup-1,000/2,000, whole-app steady-state memory, a parser job on the retained constructed 64 MiB FLP and the 2,001-entry refusal remain uncollected. See [results.json](results.json) for sanitized values and hash receipts.

## Source and reused package

The current baseline is merged `ce4331e` (tree `4ee79f5`). The reused package was built at `839e5fb` (tree `8e8d292`); no rebuild occurred. The commits differ only in the gap ledger and two October 18 report files, with 22 compiled-graph pins matched. Source-pin and static-manifest hashes are in results. The 15 hosted checks on `839e5fb` cover unchanged application code, not performance; normal CI remains required on the documentation PR updated head.

Installed desktop (`05f518…`) differs from post-Tauri (`133553…`); parser and smoke sidecars match retained bytes. The bare prebundle Cargo executable was not captured. Full hashes and sizes are in the JSON.

## Reused correctness profile

The root gate reused 3,121 entries across four roots: the 21-file cohort (18 complete/current, three failed/stale) and 100/1,000/2,000-entry scale roots with expected current counts. This checked the retained profile/root gate, not a new corpus scan or broader compatibility. All 21 source/staged pairs and 671 protected profile hashes stayed unchanged. No fixture or 2,001-entry root was created.

The database was 19,324,928 bytes (`e0edaa…`) at reuse and 19,333,120 bytes (`ed0784…`) after startup; it was independently retained and unchanged by uninstall. Contents remain private.

## Startup and incidental memory

The startup series preserved the first request, one excluded warm-up and ten measured warm samples. Its boundary runs from process creation through CDP attachment and the observed selected Library heading on the populated profile; root selection happens after startup because it is not persistent. OS caches were retained, so this is not a cold-start or display-paint measurement. The warm median was 734.7518 ms and nearest-rank p95/max was 766.4243 ms against the proposed 1,500 ms candidate. Initial processor load was 2%, below the unchanged 20% precondition after the five-second settle. The all-process guard nevertheless failed: Explorer used 0.640625 CPU-seconds and `nvcontainer` used 0.671875, each above the unchanged 0.5 CPU-second allowance. T3 was observed and counted; all observed unrelated processes were included, with no process-name exemptions. The harness closed no unrelated application; FL Studio presence during the window is not established. No retry or forced pass was used.

The 49 recorded memory observations are incidental samples during this startup window, not memory qualification. The report retains each elapsed time, summed working-set bytes and summed private-commit bytes, plus all 48 measured intervals. Although the requested sample interval was 100 ms and the process-tree refresh was 1,000 ms, actual intervals included 1,159.9386 and 2,800.1797 ms. The observed peaks were 352.3672 MiB working set and 219.3242 MiB private commit. Short peaks and inaccessible or terminated processes can be missed; summed working sets can double-count shared pages, and private commit is not physical RAM.

## Closes, stopped stages and retained attempts

All 14 attempt04 closes met the one-close contract: one graceful request, a 10,000 ms deadline, exit code 0 and a strict eight-stage trace; no retries. Their elapsed p95/max was 373.9609 ms; native shutdown p95/max was 7 ms. The attempt schedule contained 42 planned closes: 14 were executed and 28 planned closes were not executed. Because startup100 failed the noise guard, its timing does not satisfy qualification; it needs a fresh guard-passing run before the remaining stages. The two valid closes from attempt03 are separate and are not combined with these values.

Attempt03 had a separate pre-readiness sampler failure from stale run-folder/lock-owner binding. It produced no performance values; its failure log remains private. Attempt04 also had a separate pre-readiness receipt-initialization error: the first identity-approval request stopped before app launch or sampling because a mutable receipt field was missing. After the receipt was initialized, the actual identity approval succeeded. The pre-initialization static artifact manifest, pre-readiness initialization receipt and post-approval runtime artifact manifest are identified separately by hash. The placeholder appears only as a hash in the initialization receipt, not as a separately retained file. Neither issue changed application code or package bytes, and neither is treated as a product result.

The owned install registration was removed, its host lock released, and no owned app processes remained. The final database, profiles, fixtures, binaries, raw traces, logs and failure evidence remain retained outside the repository and disposable build caches. Raw inventories, process identities, personal paths and database contents are not published.

## Decisions and next evidence

The next qualification run needs a fresh startup100 window to pass its noise guard before startup-1,000/2,000 and the 2,000-entry whole-app memory window, a parser job on the retained constructed 64 MiB FLP and the new 2,001-entry refusal. The constructed FLP is a workload input, not GUI-derived compatibility evidence. Keep the existing CPU limits and sampling boundaries. The four previously guarded latency windows remain in the October 18 report; they are not repeated here. The [budget proposal](../phase3-qualification-306/budget-proposal.md) and [checkpoint](../phase3-qualification-306/checkpoint.md) remain proposals. ADR-008 is still proposed and unanswered; merging or publishing this packet does not accept it, close #259/#306, accept Phase 3 or authorize Phase 4. The separate [October 14 repeat](../resource-repeat-20261014/README.md) retains the earlier native Explorer observation and unexplained timeout.

This documentation-only packet needed no build or cache write. C: had 33.57 GiB free before the 0.05 GiB output estimate (33.52 GiB after); G: had 13.32 GiB unused. The reusable cache is owned by `/root` with its write window closed; the primary build output is preserved. No compiler-cache cleanup occurred. The owned installation and registration were removed.
