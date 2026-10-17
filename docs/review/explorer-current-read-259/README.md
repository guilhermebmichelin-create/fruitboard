# Current metadata read consolidation

Issue [#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259).
Reviewed merged baseline: PR318, `fa0dbdd2fc9b7f2952fc6386fcb8fd444c2e180b`.
Implementation source: `834350c`; application changes are limited to
[`metadata.rs`](../../../crates/storage-sqlite/src/metadata.rs).

## Current development status

Fruitboard remains in Phase 3. Its bounded native analysis and Library workflow
includes saved facts, channel/sample/plugin references, Plugin Explorer, pattern
names/note-record counts and qualified ordinary mixer records. Phase 2 was
accepted with known gaps; Phase 3 exit, resource budgets, production activation
and distribution remain separate owner decisions. Project grouping, Kanban,
Google Drive sync and the PWA remain later roadmap work.

Merged PR318 defines the next G3/G5/G6 scopes in the
[October 14 packet](../phase3-next-slices-20261014/README.md).
[PR320](https://github.com/guilhermebmichelin-create/fruitboard/pull/320) is the
prepared G3 fixture addition: all fifteen checks passed at
`9e3979c39aaa58828e68a881e84bea953e83abe1`, but the owner has not merged it.
The current merged corpus therefore remains nineteen approved files. Relative
sample mapping still needs its explicit authority/semantics decision, and the
automation case needs independent GUI evidence and exact fixture approval.

The [resource follow-up](../resource-repeat-20261014/README.md) retains guarded
worker observations, failed Explorer noise guards and the unresolved installed
shutdown timeout. A successful separate debugging repeat did not establish a
product fix. Existing scanner failures and all wider compatibility gaps remain
in the [gap ledger](../flp-intelligence-readiness/gap-ledger.md).

## Implementation and invariants

Previously `current_metadata_snapshot` selected the current snapshot ID through
the live-source joins, then issued another statement to load its header and
bounded payload. The new implementation selects those values under the same
joins in one statement. The ID-only and payload APIs share the exact freshness
predicates; historical and current payload reads share the same decoder and
validation. SQL preparation is cached, while results are read afresh each time.

The selected file, snapshot/file/location/root bindings, enabled local-NTFS root,
presence, configuration/metadata revisions, byte size and nanosecond timestamp
checks remain intact. The 256 KiB payload cap, projection version, JSON/digest
checks and negative-outcome payload rules remain intact. Explorer still uses
one transaction, at most 2,000 entries and 8 MiB input; over-limit reads return
no partial aggregate. No migration, parser, filesystem permission, IPC, renderer,
dependency, fixture or production-activation change is included.

New regressions compare current and historical rejection of malformed JSON,
wrong projection versions/digests, missing or oversized payloads and an invalid
payload on a rejected result. Disposable corruption fixtures explicitly bypass
schema write guards; production protections are unchanged. Tests also confirm
that publication does not select a rejected result, preserve historical
negative-result reading, and cover unknown files and disabled-source exclusion
before payload validation. Existing
source-change/restore, root, cross-file binding, history, alias and Explorer
budget tests remain required.

## Validation and evidence

The pinned Windows `pnpm check` passed, including all 136 repository policy
tests, 408 client tests, the locked Rust workspace (151 active storage tests),
warning-denied Clippy and the production build. The analysis-enabled desktop
suite passed 159 active tests and warning-denied Clippy. All four real-parser
native analysis integration tests passed with a freshly built parser, including
pattern/mixer publication and ordinary/limit-sized inputs. All nineteen approved
fixture hashes matched before and after. Source comparison independently
confirmed unchanged freshness predicates and payload-validation logic.

The first full run caught two incorrect new test setups: schema immutability
prevented direct mutation, and a real rejection correctly had no current
pointer. The corrected disposable corruption setups passed their focused tests
and the complete rerun. Original logs remain private. No product fix or relaxed
production constraint was needed for those failures.

Existing PR318/PR320 checks cover their own source only; this changed head needs
its normal required CI and owner review before manual merge.

Both matrices use the same unchanged resource probe and parser. Each count
(100/1,000/2,000/2,001) retains the first request, one excluded warm-up and ten
measured requests, plus a separate subsequent component profile. Setup is
outside command timers. Complete counts or bounded refusal are asserted by the
actual native command. The shared noise guard counts T3 and every observed
unrelated process; failed guards remain characterization. Ten warm samples
have nearest-rank p95 equal to their maximum. No budget or qualification claim
follows from a faster observation.

## Comparison observations

All eight native cases passed complete-result/refusal assertions. Full samples,
first requests, excluded warm-ups, subsequent component profiles, memory
counters, artifact hashes and guard summaries are retained in
[`results.json`](results.json). Both windows failed the unchanged background-CPU
guard, so these are characterization only. No measured speedup is established.

| Entries | Before command/serialization p95 | After command/serialization p95 | Before storage median | After storage median | Result                        |
| ------- | -------------------------------: | ------------------------------: | --------------------: | -------------------: | ----------------------------- |
| 100     |                          8.17 ms |                         8.34 ms |               3.77 ms |              3.68 ms | Complete                      |
| 1,000   |                         86.96 ms |                        94.13 ms |              43.93 ms |             43.54 ms | Complete                      |
| 2,000   |                        177.55 ms |                       175.85 ms |              87.93 ms |             91.62 ms | Complete                      |
| 2,001   |                          2.94 ms |                         3.13 ms |               2.67 ms |              2.64 ms | Limited; no partial aggregate |

Initial CPU load was 7%/8%, below the proposed 20% precondition. Three T3
processes exceeded the unchanged 0.5 CPU-second guard in each window:
3.328125/1.796875/0.765625 seconds before and
6.453125/4.140625/1.65625 seconds after. T3 received no exemption; only the
collector's exact PID/start identity was excluded. Other build/test/agent work
was paused during measurement. Different host activity and the small sample
prevent a causal before/after latency or regression claim. Response byte counts
were unchanged at each size.

The baseline probe was independently retained from build `0e86b8e`; compiled
application/probe inputs were verified equivalent through merged PR318. The
after probe was independently copied from the optimized `analysis-jobs` build
of `834350c`, with link count one and recorded build provenance. Both
used the same unchanged retained parser, same fixture and unchanged probe.

Issue #259 remains open for valid quiet-host qualification and the explicit
budget decision. The remaining installed windows and unexplained shutdown
timeout remain #306 work. Earlier worker/resource observations stay tied to
their original sources; this changed application head needs its own future
qualification. The next G3/G5/G6 steps remain the separately gated scopes above.

## Cache and retention handoff

The existing reusable validation cache has one owner, Codex/root. All builds,
tests and matrices run sequentially. Preflight passed with 36.19 GiB free and
4 GiB estimated build output, leaving 32.19 GiB against the 30 GiB reserve.
Compiler files with hardlink aliases were inventoried and replaced with
independent hash-identical copies before cache writes and subsequent fixture
preflights; source and retained evidence stayed intact. The after matrix
preflight reserved 0.25 GiB additional output and passed; completion had
34.70 GiB free. No new compiler cache or obsolete-output deletion was introduced.
Private handoff records the absolute cache/evidence locations,
binary provenance and hashes, failed attempts, remaining capacity and retention
duties. Reports, databases and independently copied executables stay outside
disposable compiler caches and Git.
