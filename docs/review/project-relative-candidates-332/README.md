# Project-relative sample candidates: native review

Status: **Conditional support for proposed ADR-008; owner acceptance and
implementation remain pending.** Date: 2026-10-18. Issue:
[#332](https://github.com/guilhermebmichelin-create/fruitboard/issues/332).
Source reviewed: `d90b660ff7cb3a65d74447ce84c1cc338134447b`.

## Owner decision

The current Windows checker can support
[ADR-008's proposed rule](../../adr/008-project-relative-sample-candidates.md)
without another folder grant, OS right, IPC command or renderer-supplied path.
The review found no native-authority blocker to that precise rule. The owner
still needs to accept its meaning: Fruitboard checks a candidate from the folder
containing the current project file, inside the already selected root. This
does not establish where FL Studio would find the sample.

This review is conditional design support, not an implementation/test receipt.
Relative references remain unchecked. The
[accepted absolute-only design](../../adr/007-sample-presence-boundary.md), G5,
resource decisions and Phase 3 exit retain their current status. A useful
distinction is that handling `..` in memory can keep a candidate inside a folder
without asking Windows to traverse a parent path.

## Source findings and proposed delta

- **Keep the absolute parser strict.**
  [AbsolutePath](../../../crates/sample-presence/src/path.rs#L3) represents a
  drive and ordinary components, and rejects relative paths and dot components.
  [CapturedInput](../../../crates/sample-presence/src/execution.rs#L39) stores
  absolute root/source paths and validates the reference projection.
  [Execution](../../../crates/sample-presence/src/execution.rs#L307) currently
  parses each reference as absolute. Add a distinct relative parser and bounded
  candidate representation; do not make the absolute parser a normalizer.
- **Bind the base to the actual source.**
  [Host capture](../../../apps/desktop/src-tauri/src/foundation/sample_check/host.rs#L59)
  obtains references and source identity from the current authorized snapshot.
  [Windows qualification](../../../crates/sample-presence/src/native.rs#L215)
  opens and retains the actual root-to-source-parent chain and source handle.
  Its lexical prefix check alone is not authority. Derive the component base
  only after qualification succeeds and preserve those held-object bindings.
- **Reuse exact-child access.**
  [The policy walk](../../../crates/sample-presence/src/execution.rs#L245)
  already probes from the held root. Windows
  [open_child](../../../crates/sample-presence/src/native.rs#L46) validates a
  single ordinary component before the
  [metadata-only FFI](../../../crates/sample-presence/src/native/ffi.rs#L149)
  query/open. Keep that defense strict; no dot/parent handle operation, joined
  path reopen, tree enumeration or new filesystem primitive is needed.
- **Preserve physical checks on surviving candidates.**
  [Case comparison](../../../crates/sample-presence/src/native.rs#L299),
  reparse/offline rejection, identity and ordinary-file qualification remain
  required. [Final revalidation](../../../crates/sample-presence/src/native.rs#L348)
  checks held ancestry and the current source parent/name binding. A lexical
  component cancelled by `..` is validated as text but never queried; its
  physical existence or reparse/offline state is intentionally unknown.
- **Keep path authority out of the renderer.**
  [The request/response contract](../../../apps/client/src/library/samplePresence.ts#L38)
  remains opaque IDs, fingerprint, fixed outcomes/reasons and ordered channel
  slots. No candidate base or resolved path crosses IPC. Existing host/session,
  snapshot, cancellation, delivery and renderer-generation fences still apply.
- **Change the explanation and absence copy together.**
  [The controls](../../../apps/client/src/library/SamplePresenceControls.tsx#L57)
  currently say relative paths remain unchecked;
  [the report](../../../apps/client/src/library/ProjectSamples.tsx#L83) says
  absence is at the saved path. Future relative behavior needs ADR-008's
  project-file-folder explanation and fixed candidate-safe copy, including
  unchecked reasons that currently refer to a literal saved path. No FL Studio
  resolver claim, clickable/open/play action or new persistent report is added.

## Reconciliation with the accepted design

ADR-007 and the
[original authority review](../sample-presence-278/authority-review.md#evidence-boundaries)
leave relative mapping behind a separate grant/GUI gate. Merely selecting a
scan root does not approve a relative base. ADR-008 explicitly proposes
superseding that gate only for its owner-approved, app-defined same-root rule;
the existing explicit action would authorize the amended eligibility. It adds
no persisted folder grant. Placeholder, external-folder, cloud and search
capabilities keep their separate gates.

Genuine-save and independent GUI evidence remain necessary for any FL Studio
compatibility claim, limited to the exact build tested. A controlled synthetic
path/native/UI case can demonstrate Fruitboard's interpretation; it cannot
demonstrate FL Studio's behavior or genuine end-to-end parser compatibility.
Do not rewrite approved FLPs to manufacture that evidence. New public fixture
bytes still require exact owner approval under
[the fixture rules](../../../DEVELOPMENT.md#parser-fixture-rules).

## Acceptance matrix after design approval

| Boundary                 | Required evidence                                                                                                                                                                                                                                                                                                                                                    |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Existing behavior        | Existing absolute, malformed, placeholder, outside-root and no-reference outcomes stay stable. Rejected absolute/rooted forms never fall back to relative interpretation.                                                                                                                                                                                            |
| Raw grammar              | Validate every raw component before elimination; cover invalid names, separators, controls, `%` and per-component `$` placeholders followed by `..`. Only exact `.`/`..` receive special treatment.                                                                                                                                                                  |
| Lexical containment      | Child, sibling and parent-relative candidates stay inside the qualified root. Any intermediate escape is `outside_root` with zero candidate queries, even after later re-entry. Normalization to root is `not_regular_file`; cancelled ordinary components are never queried.                                                                                        |
| Work and depth           | Keep capture's empty/NUL/byte/UTF-16/aggregate failures as whole-request `InvalidInput` before qualification. Check raw relative count and every resulting full depth at 64, including root components; these depth failures are item-level `limit_reached`.                                                                                                         |
| Metadata authority       | Recording-port tests assert exact component queries and duplicate channel slots. Real NTFS tests cover case-sensitive/source-parent binding, denied versus authoritative absence, reparses, replacement/save races, and unchanged source/target bytes. Injected offline/recall flags prove refusal before candidate open; they do not qualify a real cloud provider. |
| Freshness and retirement | Host tests retain disabled/removed root, replaced source, superseding snapshot, cancel/deadline/shutdown, blocked-worker admission and final durable fences. Client tests reject stale row/fingerprint/request-generation replies.                                                                                                                                   |
| UI and privacy           | Change scope, absence and applicable unchecked copy together. Verify strict IPC/no paths or raw errors, ephemeral results, keyboard/cancel/focus/status behavior, narrow width and enlarged text; inspect the installed app.                                                                                                                                         |
| Honest evidence          | Label synthetic native/UI evidence as such. Keep genuine parser/FL Studio evidence separate, approved and exact-build scoped. No G5 closure, resource/phase acceptance or production activation follows from a design or implementation PR.                                                                                                                          |

Zero **candidate** queries does not mean zero root/source qualification work.
The current request qualifies its authority before iterating references. Keep
the existing 256-channel, reference/payload, 2,048-operation, 132-handle,
two-second, single-worker/no-queue and 64 KiB bounds from ADR-007; no budget is
approved here. Memoization remains limited to identical literal references
within one authorized request.

## Suggested implementation sequence

1. Record explicit owner acceptance of ADR-008 and this scoped review first.
2. Implement portable grammar/candidate execution in `path.rs`, `execution.rs`
   and, if needed, `lib.rs`; extend the recording-port tests in
   `crates/sample-presence/tests/policy.rs`. Keep the absolute parser strict.
3. Independently review and test the resulting native walk in `native.rs` and
   `native/tests.rs`, including source-parent binding and total depth/handle
   accounting. Change no syscall or grant merely for convenience.
4. Review host/client fences, update controls/report/reason copy together and
   add the corresponding host/component/installed evidence. Update delivered
   scope only after those reviewed implementation slices.

If implementation requires broader authority or different semantics, return to
design review. This sequence does not authorize runtime work while ADR-008 is
Proposed.

## Review and validation boundary

A GPT-6 Luna agent at maximum effort performed the source audit and reviewed
the proposed ADR; the orchestrator reconciled its findings in this packet.
This documentation-only change needs Markdown, privacy, local-link and diff
checks plus normal updated-head CI. Test results belong in the PR receipts;
this packet claims no relative-path test pass, build or installed observation.

The 15 successful checks on PR331 head
`783bc48b593698be6c6d7ef5f996cb6228878ac6` cover the identical merged source tree
at `d90b660`; they apply to unchanged application code, not to this new proposal
or future relative behavior. Required checks on this PR's new head remain
required. No duplicate native build is needed for this documentation scope.

No local build/cache or installed window is needed here. The single future
validation cache remains under `/root` ownership; its absolute path and retained
private review are recorded outside Git and compiler caches. Before future
build/fixture work, run the four-path storage preflight with honest output
estimates and at least 30 GiB free afterward on every involved volume. Keep
heavy builds sequential; installed tests hold their host lock and qualification
runs alone with all agents/build/test workloads quiescent. Preserve source,
toolchains/dependencies, approved fixtures, profiles/databases and historical
evidence. This design work creates no compiler output or cleanup/rebuild cost.
