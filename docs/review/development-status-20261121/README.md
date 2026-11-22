# Phase 3 follow-through

Reviewed 2026-11-21 against merged [28fc508](https://github.com/guilhermebmichelin-create/fruitboard/commit/28fc508).
The scope is the opt-in Windows Library. Phase 2 remains accepted with known
gaps. Phase 3 exit and the proposed resource budgets remain pending.

## Current Windows evidence

The native repeat used PR #430's retained release probe. Its hash and the
recorded source diff matched the merged 28fc508 inputs. Each size had an
initial request, one excluded warm-up, and ten measured warm requests. The
timer includes the native command and outer envelope serialization.

| Root entries | Result                               |  Warm p95 | Background CPU guard |
| ------------ | ------------------------------------ | --------: | -------------------- |
| 100          | Complete                             |  10.31 ms | Passed               |
| 1,000        | Complete                             |  88.93 ms | Passed               |
| 2,000        | Complete                             | 161.47 ms | Failed               |
| 2,001        | Explicit limit, no partial aggregate |   2.88 ms | Passed               |
| 2,000 repeat | Complete                             | 161.96 ms | Passed               |

The first 2,000-entry run exceeded the unchanged 0.5 CPU-second per-process
guard in Windows Explorer and two T3 processes. Its timing is
characterization. The separately retained 2,000-entry repeat met the proposed
200 ms native candidate under the same guard. The result is ready for review
under #259. The budget still requires an owner decision.

The installed repeat used the hash-verified 28fc508 development package and its
retained 100-entry review profile. Timing ended after the Library selected the
expected root and displayed a project heading. A persistent helper was ready
before launch. All 12 launches used one graceful-close request, exited within
ten seconds, and produced valid eight-stage traces. Warm p95 was 686.27 ms
across ten measured launches. Windows Explorer, the text input service, T3 and
an unbound WebView2 process exceeded the unchanged guard. A separate repeat
at the same boundary returned 702.81 ms and also exceeded the guard. Each
repeat produced 12 validated graceful-close traces. Both timings are
characterization. Each review package was uninstalled and its review database
was preserved.

Earlier installed checks with the same package verified two additional
correctness cases. Explorer refused a 2,001-location root without returning
partial entries or counts. A constructed 64 MiB FLP completed automatic and
explicit analysis, produced a new snapshot, and retained matching saved
version and channel-count values on readback. These cases establish installed
behavior. The memory window during real analysis still needs qualification.

The [November 22 installed memory review](../installed-memory-306/README.md)
adds guarded observations for the 64 MiB analysis and 2,000-entry steady
Explorer windows on the same package.

## Qualification order

1. Review the guarded current-source native Explorer evidence under
   [#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259).
   Preserve the earlier failed guard receipts and review the proposed 200 ms
   p95 target for complete native reads. Installed limits are a separate check.
2. For [#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306),
   bind the installed runner to a current package, the expected profile and
   fixture hashes. Acquire the existing installed host lock and prepare outside
   the timing windows. Confirm the startup endpoint includes root selection
   as specified in the [budget proposal](../phase3-qualification-306/budget-proposal.md).
   The [October 19 attempt](../resource-remainder-20261019/README.md) selected
   the root after timing, so its startup value describes a different boundary.
   Collect guarded startup at 100, 1,000 and 2,000 entries, installed Explorer
   latency, steady and 64 MiB analysis memory, and graceful close. Preserve the
   existing installed correctness evidence for the 2,001-entry refusal. Keep
   the historical scanner targets and evidence separate.
3. Resume [cross-build work](../../research/cross-build-candidates-411.md)
   when the exact 26.1.1.5547 and 25.2.4.5242 installers are available. Record
   GUI expectations before using the research decoder on genuine saves. Review
   each field before proposing any production support expansion.

Quiet runs require a reserved Windows host window with other workloads idle,
output-aware storage preflight and run evidence outside the reusable cache.
The [runner preparation](../resource-qualification-runner-306/README.md) records
the unchanged all-process noise guard, source and artifact verification, and
the distinction between qualification and characterization.

## Tracker reconciliation

| Open issues                                                                                                                                            | Merged evidence                                                                                                                                                                                      | Disposition to review                                                                                                                      |
| ------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| [#347](https://github.com/guilhermebmichelin-create/fruitboard/issues/347)                                                                             | [PR #348](https://github.com/guilhermebmichelin-create/fruitboard/pull/348) and [installed review](../project-relative-installed-20261028/README.md)                                                 | Same-root check delivered. Track broader dependency resolution separately.                                                                 |
| [#324](https://github.com/guilhermebmichelin-create/fruitboard/issues/324)                                                                             | [Parser and native validation](https://github.com/guilhermebmichelin-create/fruitboard/pull/427), [Library and installed evidence](https://github.com/guilhermebmichelin-create/fruitboard/pull/429) | Qualified saved totals delivered. Targets, curves, placement and MIDI need separate scope.                                                 |
| [#366](https://github.com/guilhermebmichelin-create/fruitboard/issues/366)                                                                             | [Note pilot delivery](../note-pilot-395/README.md) and [installed review](../installed-notes-397/README.md)                                                                                          | Qualified ordinary notes delivered. Preserve the exact-build, layout and value boundaries.                                                 |
| [#220–#223](https://github.com/guilhermebmichelin-create/fruitboard/pull/225)                                                                          | [Fix and test mapping](../application-fixes-20260929.md)                                                                                                                                             | Compare each issue's acceptance criteria with its merged fix. Keep P2-08 and scanner qualification separate.                               |
| [#349](https://github.com/guilhermebmichelin-create/fruitboard/issues/349)                                                                             | [PR #350](https://github.com/guilhermebmichelin-create/fruitboard/pull/350) and the current lockfile                                                                                                 | The requested markdown-it 14.3.3 pin is present and required security CI passed on the reviewed baseline.                                  |
| [#259](https://github.com/guilhermebmichelin-create/fruitboard/issues/259)                                                                             | Current guarded native matrix above                                                                                                                                                                  | Review the measured limit-sized read and proposed native target.                                                                           |
| [#399](https://github.com/guilhermebmichelin-create/fruitboard/issues/399), [#306](https://github.com/guilhermebmichelin-create/fruitboard/issues/306) | Prepared evidence and procedures linked above                                                                                                                                                        | Continue until genuine additional-build evidence and quiet installed resource results are collected, or record an explicit scope decision. |

## Checkpoint decisions

Review guarded measurements before accepting resource targets. Decide whether
broader dependency resolution, arrangements and duration, MIDI, or mixer
effects are required for Phase 3 exit, or explicitly defer each with its scope,
reason and follow-up. The proposed first supported workflow is the local NTFS
Library with explicit per-field and saved-build limits. The historical scanner
performance, DriveFS, and
FAT32/cross-volume dispositions are separate. Phase 4 starts only after the
owner accepts the [Phase 3 checkpoint](../phase3-qualification-306/checkpoint.md).
