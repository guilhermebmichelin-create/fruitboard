# Drive virtual scan roots — experimental policy

Status: **EXPERIMENTAL; manual scans only; host validation not yet performed.**

This note describes the safety boundary for the `driveVirtual` scan-root mode.
It is implementation policy, not a claim that Google Drive for Desktop has
passed enumeration, placeholder, offline, or reconciliation testing.

## Behavior

- A `driveVirtual` root may be enumerated only when the user requests a scan.
- The desktop watcher supervisor never creates a native watcher for this mode.
- Watcher hints, watcher shutdown, reconnect gaps, and coverage-loss signals
  must never trigger scans or determine whether a file is missing.
- A manual scan may add or update projects it observes. It never infers that
  unobserved projects were deleted, even when the scan reports success.
  Failed, partial, or interrupted enumeration likewise cannot remove prior
  known projects.
- Switching a root from `driveVirtual` back to `localNtfs` may start a fresh
  watcher generation. Enabling/disabling a virtual root alone does not.
- Native watcher behavior on DriveFS, if probed, is research data only; it
  does not make application watcher support implicit.

The mode is experimental because virtual mounts can disconnect, present
placeholders, or change metadata/enumeration behavior. Manual scans limit the
impact of uncertain event delivery but do not establish that the provider's
enumeration is complete or side-effect free.

## Required Windows host validation

Run against only disposable synthetic leaves on the actual DriveFS mount.
Never scan personal Drive content for a test. Record the Drive client version,
Windows build, mount identity, UI-reported mirror/stream mode, commit, run ID,
seed, and fixture manifest hash. Preserve the raw observed values needed to
reproduce a finding while excluding account identifiers, credentials, private
paths, and raw client diagnostics.

| Case | Procedure | Required record / safe outcome |
| --- | --- | --- |
| Mirrored comparison | Add a `localNtfs` root pointing at a disposable mirrored folder on a fixed NTFS volume. | Compare observed projects to the manifest and record sync behavior separately. This is a comparison run, not `driveVirtual` qualification; the normal local-NTFS missing-file rules apply. |
| Streamed placeholders | Repeat with streamed mode, including known offline placeholders. | Record enumeration and metadata availability; report any hydration or other side effect. Unobserved prior projects remain known. |
| Disconnect during manual scan | Pause/disconnect DriveFS during a scan, then reconnect and explicitly scan again. | Interrupted/failed first pass cannot mark unseen projects missing; the later pass is recorded separately and also cannot infer deletions. |
| Watcher exclusion | Start or restart the desktop supervisor with a virtual root; create, rename, delete, and reconnect. | No application native watch is registered and no watcher-derived scan job or missing decision occurs. A manual scan remains user-triggered. |
| Local NTFS regression | Repeat enable/disable/re-enable and watch restart on a disposable local NTFS root. | Existing watcher registration and fresh-generation recovery behavior remain intact. |

For the lower-level DriveFS filesystem watcher characterization (burst,
overflow, metadata, and disconnect observations), use
[`p2-47-drivefs-host-runbook.md`](p2-47-drivefs-host-runbook.md). That probe
does not substitute for the app-level cases above.

## Evidence and decision rules

No host run has been performed for this implementation. Unit tests establish
only that the supervisor skips virtual roots and resumes native watching when
the configured mode changes back to local NTFS. Do not describe DriveFS manual
scanning as validated until the Windows matrix above is executed and its
environment, inputs, observed results, limitations, and decision are committed
to research notes.

If a completed scan cannot establish authoritative enumeration after a
disconnect, keep the root in manual-only experimental status and ensure that
incomplete results preserve prior known files. Any proposal to enable native
DriveFS watchers or automatic reconciliation is a separate change requiring
host evidence and explicit review.
