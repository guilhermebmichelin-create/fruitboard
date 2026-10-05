# Sample presence native authority review

Status: design audit accepted by PR279; Windows slice #282 implemented for review;
application integration and installed evidence pending.
Baseline: merged PR268, `91261525383ce79adb090550329167ef347f7253`.
The [accepted ADR](../../adr/007-sample-presence-boundary.md) is authoritative for
scope. This review identifies what a native implementation must demonstrate.
The [policy review](../sample-presence-policy-280/README.md) maps recording-port
tests to their limited proof. The [Windows review](../sample-presence-windows-282/README.md)
maps actual native tests and their limitations. Durable host/IPC/renderer,
installed and real blocked-kernel/provider evidence remain open.

## Existing boundaries inspected

The [sample projection](../../../apps/desktop/src-tauri/src/foundation/project_details/samples.rs)
retains up to 256 ordered channel entries, fixed unavailable reasons and bounded
stored strings. It does not open their targets. The
[details command](../../../apps/desktop/src-tauri/src/foundation/project_details.rs)
resolves root/location IDs under the database guard and rejects stale displayed
fingerprints. These are useful authorization inputs, not a sample probe.

The [analysis source authority](../../../crates/analysis-execution/src/native.rs)
qualifies held local-NTFS source identity and attributes. Its FLP helper grants
content-reading authority; it must not become a generic sample opener. The
[enumeration port](../../../crates/filesystem-enumeration/src/lib.rs) uses owned
directory/entry capabilities and requires handle-derived locator spelling.
Its whole-directory cursor must not be repurposed into an unbounded sample
search. Exact-child metadata queries need their own review and tests.

## Trust and operations

Trusted inputs are native root/source objects and a current validated snapshot
selected under durable authorization. Stored sample strings are untrusted input
even though their shape is validated. The renderer may identify a displayed
project and request/cancel a check; it cannot provide a path or broaden a grant.
Root/source/session/snapshot authority must remain current until report return.

Allowed observations are attributes, object/volume identity, case mode and
qualified exact-name absence. Root/source metadata can be compared to captured
fingerprints; sample contents and FLP bytes are not read by this feature. Probe
paths derive only from permitted stored absolute references. No process/plugin
execution, audio decoding, recursive walk, network fallback or write operation
is part of the port.

An explicit check grants metadata inspection within the selected source root,
not other configured roots. Qualification/source failure produces a fixed
request-level unavailable/stale result and no sample probes. Per-candidate
failures preserve the distinction between absence and inability to inspect.

## Threats and proof obligations

| Threat or race | Required response/proof |
| --- | --- |
| Renderer sends a different path, folder or hidden extension field | Strict request rejects it; recording port observes zero probes |
| Valid-looking reference points outside selected root or into another enabled root | `not_checked/outside_root`, zero candidate operations |
| String prefix matches a sibling folder, drive alias or ambiguous component | Component/held-object containment required; no prefix-derived authority |
| Relative name or FL/environment placeholder | No guessed base/expansion; `not_checked`, no target lookup |
| UNC/device/stream/URL/drive-relative/wildcard syntax | Reject before filesystem calls |
| Junction/symlink/cloud tag introduced at root, parent or leaf | No follow/hydration; refuse or invalidate before further operations |
| Known offline/recall attributes or incomplete qualification | No hydration/network fallback; `not_checked` |
| Source/root disabled, replaced, revised, superseded or runtime restarted | Cancel/no further operations; final fence rejects all provisional results |
| Source changes before watcher publication | Held source identity/size/high-resolution modification comparison invalidates request |
| Old source handle survives a same-name replacement | Revalidate namespace binding from trusted parents, not only held-handle identity |
| Access denied, unexpected filesystem error or directory at target | Never `not_found`; fixed `not_checked` reason |
| Qualified exact child/intermediate component is absent | `not_found` permitted only with trusted parent and authoritative absence |
| Target created/removed after its probe | Report is explicitly a time-of-check observation, not a frozen dependency set |
| Case-sensitive/Unicode spelling ambiguity | Port-owned filesystem semantics; no renderer or second Unicode/case normalization |
| Many/deep/duplicate references | Fixed budgets, ordered full output, same-request exact-literal reuse only |
| Kernel call blocks past cancellation/deadline | Fence UI timeout/late results; retain single-worker gate until retirement |
| Navigation or newer details response wins a race | Renderer request/source/snapshot/generation correlation discards late report |
| Private path appears in logs, events, errors or report | Allowlisted codes only; report excludes paths, raw OS data and extra strings |

The implementation must subscribe its cancellation token to root/source/session
revocation and check it before every operation. A previously started kernel call
may still finish; it cannot publish or authorize another operation afterward.
Timer/cancellation paths must not destroy guards still used by a blocked call.
Shutdown closes admission and fences publication before retiring the worker.

## Concrete acceptance cases

Use recording/injected ports for deterministic policy/race/time tests. Windows
tests create only private disposable roots explicitly owned by the test; do not
enumerate the owner's project folders, access a real network or hydrate a cloud
file to demonstrate rejection.

1. Within-root full saved path to an ordinary file yields `present`; a qualified
   absent leaf and absent intermediate directory yield `not_found`.
2. Denied metadata access, directory target, unknown volume identity, changed
   ancestor and incomplete metadata all yield `not_checked` or request invalidation.
3. Outside/sibling/other-root references, relative/drive-relative references,
   placeholders, UNC/device namespaces, alternate streams, controls, dot/parent
   components and ambiguous spellings cause zero forbidden candidate calls.
4. Replace each ancestor/leaf with a reparse between inspection/open; prove no
   target traversal and no missing claim. Simulated offline/recall/cloud states
   cause zero hydration/content/network operations. Native opened rights must
   request attributes only; a recording content-read counter stays zero.
5. Capture a valid snapshot, then disable/remove the root, replace the source,
   change its metadata, publish a newer snapshot or rotate the host session.
   Inject each change before admission, between probes and before response;
    all detectable stale responses are discarded. Include same-name replacement
    while an old handle survives and normal FLP saves during the check; no source
    write lock is added. Database guard is released during I/O. Preserved
    identity/size/timestamp content edits are an explicit metadata-only limitation.
6. A pending denied/blocked call exceeds the injected monotonic deadline.
   UI completes with bounded timeout state; a second request is busy and does
   not start a second worker. Retired old work cannot publish or access freed guards.
7. Exact duplicate references retain separate channel positions but share an
   observation within one request. Differently spelled references do not acquire
   authority from a guessed case-fold or global cache.
8. Check 256 entries, all-unavailable/zero-entry input, old snapshot without a
   reference field, overlong text, depth 64/65, operation/handle boundary and
   response overflow. Budget exhaustion marks remaining entries unchecked;
   it does not omit them or guess they are missing.
9. Strict client decode rejects wrong IDs/snapshot/order/count, extra statuses,
   raw path/error fields, invalid timestamps and stale request generations.
10. Opening/refreshing details performs no sample I/O. Only the explicit action
    does. PWA/default-disabled composition exposes no functional probe.
11. Keyboard/cancel/check-again, progressive lists, announcements, narrow view,
    200% text, reduced motion and stale/busy/error/unchecked states are covered.
12. Keep project/sample files and an independent database copy byte-identical
    before/after. Known sentinel references stay out of logs/events/telemetry.

## Evidence boundaries

Constructed stored reference strings and private test files prove host/policy
behavior, not FL Studio path mapping or broader parser compatibility. Unchanged
approved F12 can demonstrate an outside-root unchecked result; F15's empty
Samplers demonstrate no-reference semantics. Do not rewrite either file to
manufacture evidence. Every approved fixture must retain its hash.

Future relative/placeholder/external-folder support needs its own native grant
and independently registered GUI cases, exact sanitized-byte approval and path
mapping review. Native operational limits are separate from the unaccepted G1
performance targets. This original design audit is not a test receipt; the
separate implementation review records native evidence without claiming a
benchmark or complete application delivery.
