# Independent review of the final #111/#112 candidate

Date: 2026-09-12

## Reviewed state

The exact combined candidate supplied by Agent 1 was
`5cc8fb549966ebd53c135540b2f4c68339269680`. It was clean at review time and
is a documentation-only handoff commit on top of the code candidate
`879010d8f3ae91c6f62409cb04ee1f1c8066ab15`, whose direct parent is the #111
durable-diagnostics commit `f7ebbf4e61ffef9501c4804763f1c6e469e6c486`.
This review covers the combined tree at `5cc8fb...`; the focused #112
corrections recorded here are separate and do not edit Agent 1's
acceptance packet.

## Findings by severity

### Blocker / high: none

The combined code has no identified blocker in the reviewed scan lifecycle.
`finish_scan_run_with_error` accepts only the closed diagnostic vocabulary and
validates the session, lease, job, root generation and configuration revision
inside its transaction. Cancellation, follow-up requests, stale leases and
publication fences retain precedence over worker-supplied diagnostics.

Direct and shared worker resolution carry the bounded `partial_class` only
for a terminal `Partial` enumeration, while non-authoritative runs discard
staging and leave committed rows and the last-success marker unchanged.

### Medium: corrected in the focused #112 patch

The benchmark aggregation path filtered timing values only for `null` and
could therefore admit a missing or status-inconsistent `scan_finished` record
as a successful timing sample. The correction now requires an explicit
authoritative `Published` / `Complete` result with a valid non-negative
`scan_ms`, keeps failed records in the report, and reports the successful
sample count. Cancellation latency uses the same bounded duration check.

### Low / informational: documentation corrected

The investigation README now records the verified checked-out raw-file
SHA-256 in uppercase, distinguishes it from the Git blob ID, preserves the
historical raw artifact, and states that the nine other measured iterations
were five before iteration 6 and four after it.

## Interaction and evidence review

| Area                              | Result and evidence                                                                                                                                                                                                                                                    |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Durable diagnostics               | `access_denied` and `resource_limit` remain durable, fixed codes. Arbitrary/native/path-bearing overrides are rejected. Existing direct denial/resource-limit and shared resource-limit regressions cover both capabilities.                                           |
| Partial reporting                 | The closed enum and omitted-diagnostic fallback produce only bounded `partial_*` values. The direct disappearance regression and the focused shared-worker regression verify class propagation, discarded staging, unchanged rows/marker/snapshot, and later recovery. |
| Cancellation and follow-up        | Existing regressions verify cooperative and durable cancellation, follow-up coalescing, and cancellation/follow-up precedence during resolution.                                                                                                                       |
| Stale/revision/publication fences | Existing regressions cover expired/replaced leases, root generation/config revision invalidation, between-batch cancellation, and pre-publication invalidation. Failed work cannot publish staged records.                                                             |
| Output safety                     | Driver parsing whitelists fixed statuses, outcomes, identifiers, codes and partial classes. Unparseable output, native stderr, paths and arbitrary messages are not retained. Storage error display is fixed vocabulary.                                               |
| Benchmark disposition             | Failed iterations remain visible. Successful statistics use only valid authoritative samples and state `sampleCount`; harness completion/exit status is not a scan disposition.                                                                                        |

## Evidence commands and disposition

The raw-file bytes were independently verified as SHA-256
`0174F9081DEA7D46C7CAF984F31BC17BECB87A3F4D51B9234BDCD461D1F2C6FD`.
The corresponding Git blob ID is
`2f104fe8d37fb27655aa8d36b8a52752c3affbf9`, a Git SHA-1 object identifier,
not a file-byte SHA-256. No historical raw artifact was modified.

Focused checks passed:

- exact combined candidate: benchmark scaffold Node tests, 8 passed;
- exact combined candidate: storage diagnostic vocabulary test, 1 passed;
- exact combined candidate: storage cancellation/follow-up/stale-fence test,
  1 passed;
- exact combined candidate: scan-execution crate tests, 57 passed and 1
  documented scale test ignored;
- owned #112 branch: benchmark scaffold Node tests, 9 passed;
- owned #112 branch: scan-execution crate tests, 58 passed and 1 documented
  scale test ignored;
- owned #112 branch: pinned `cargo fmt --all -- --check`, Node syntax check,
  and Prettier checks for the changed scripts/report passed.

Full workspace/desktop CI was left to Agent 1 because the host had only about
3.1 GB free; no benchmark rerun was used as a substitute.

No repeated benchmark was run to obtain passing samples. No S5 or NTFS rerun
was required. The original trigger remains unknown and was not reproduced;
this review does not declare that failure fixed or invent a cause.

## Readiness assessment

Code-merge readiness: ready for owner/CI confirmation after the focused #112
corrections and local checks listed in the handoff. No code-merge blocker was
identified in the exact combined candidate.

Phase 2 acceptance: separate and not granted by this review. The original
performance evidence remains failed-closed because the host was not an
exclusive quiet performance host, and the iteration-6 trigger remains
unexplained and unreproduced.
