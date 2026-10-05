# Windows sample presence authority review

Status: **Slice 2 implemented for owner review; no application activation.**
Date: 2026-10-05. Issue:
[#282](https://github.com/guilhermebmichelin-create/fruitboard/issues/282).
Baseline: PR281 merge `aecf880e515540c3bce012c0508d81dba2c31e68`, tree
`08d486c295f647bbf88bfd89b69d7b20f802ec57`. Its 15 successful checks cover the
unchanged baseline, not this implementation. Updated-head CI is required.

## Owner summary

The Windows checker can inspect permitted saved sample paths without reading
the project or audio. Tests verify that replacement, permission errors and unsafe
links do not become false missing reports. A related scanner bug is corrected:
checking a disappeared name cannot accidentally recreate an empty file/folder.
The next slice adds the authorized app command and visible check/cancel controls.

## Source and authority boundary

The safe [adapter](../../../crates/sample-presence/src/native.rs) implements the
[policy contract](../../../crates/sample-presence/README.md). Unsafe operations
are confined to [the private FFI module](../../../crates/sample-presence/src/native/ffi.rs).
Windows-only `windows-sys` features reuse the locked workspace version; no external
dependency version changes. Context/fence cloning and a crate-private operations
constructor support captured authority and tests. Windows CI explicitly runs
all-target Clippy and crate tests. Desktop/client/parser/storage, IPC permissions,
snapshots, databases, production composition and approved corpus bytes are unchanged.

Trusted host inputs must identify the current enabled root/location, validated
snapshot, displayed source fingerprint and session. `CurrentAuthorization` is a
required injected seam with no default permit-all production implementation.
The host must release its database guard before filesystem I/O and signal the
request control on revocation, including during a blocked syscall. Native tests
inject this seam; they do not prove real database locking or runtime subscriptions.

Before candidate work, the adapter checks the DOS drive mapping, accepts only a
local harddisk-volume mapping and opens that internal volume directly. Held-handle
filesystem, root identity and source identity/size/high-resolution timestamp must
match the capture. SUBST, remote/redirected/device mappings and uncertain metadata
are refused. Drive binding is rechecked before qualification and report return.
[QueryDosDeviceW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-querydosdevicew)
defines the DOS mapping observation; it is not itself an access grant.

Every exact child query/open uses a held trusted parent and one bounded operation
per syscall. Only exact `STATUS_OBJECT_NAME_NOT_FOUND` followed by successful
parent revalidation authorizes absence. Directory guards retain ancestry;
no recursive listing, search, content read, hash, write or process occurs.
Directory handles allow read/write sharing but deny deletion to pin names.
Source/regular-file handles also allow delete sharing, preserving ordinary saves
and atomic replacement. Final validation checks ancestor attributes/identity/case,
root binding, held source fingerprint and a fresh source parent/name binding.
An old source handle surviving replacement cannot satisfy that last check.

## Native call audit

`NtCreateFile` requests only `FILE_READ_ATTRIBUTES | SYNCHRONIZE`, uses
`FILE_OPEN` (1), `FILE_OPEN_REPARSE_POINT`, `FILE_OPEN_NO_RECALL`, synchronous
non-alert I/O and `OBJ_DONT_REPARSE`. Object names and ABI buffers are owned for
the complete call. Directory kind is attested from the opened handle because the
directory-only open option is incompatible with no-recall. Every valid returned
handle is owned before post-operation cancellation can reject it; RAII closes
the OS handle before releasing its non-cloneable handle-budget token.
[NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile)
documents the rights/disposition contract, and
[OBJECT_ATTRIBUTES](https://learn.microsoft.com/en-us/windows/win32/api/ntdef/ns-ntdef-_object_attributes)
documents rejection of encountered reparses. No-recall instructs file filters
not to recall contents on open; see the
[driver contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntcreatefile).

The existing [enumeration opener](../../../crates/filesystem-enumeration/src/lib.rs)
passed Win32 `OPEN_EXISTING` (3) to NT, where 3 means `FILE_OPEN_IF` and permits
creation. Only its NT call changes to `FILE_OPEN` (1); `CreateFileW` keeps its
valid existing-only value 3. Two real
[Windows regressions](../../../crates/filesystem-enumeration/src/windows_port/existing_only_tests.rs)
first failed with
explicit recreated-file/recreated-directory assertions, then passed after the
correction. The full enumeration suite and warning-denied Clippy pass. No scan
capability, cursor or filesystem qualification is expanded. Fixture creation
stays in a separate test-only module; the existing discovery source policy is
unchanged and continues to reject content reads/writes in production code.

## Case and freshness limits

The saved root prefix must match the trusted captured literal spelling.
Potentially equivalent case variants remain unchecked with unsupported-case-mode;
an OS string comparison alone never grants authority from a broader parent.
Candidate suffixes use each qualified directory's actual kernel case mode, with
no additional Unicode normalization or case folding. Real sensitive-directory
tests keep distinct leaf names distinct.

Metadata detects ordinary source changes and replacements. Deliberate edits
that preserve identity, size and timestamp remain undetectable; an explicit
test records this limitation. Presence is a time-of-check observation, not
matching audio content or complete FL Studio dependency resolution. Relative
references, placeholders, external folders and cloud/DriveFS remain unchecked.

## Evidence map

The [Windows tests](../../../crates/sample-presence/src/native/tests.rs) add 20
cases to the existing 24 portable policy/lifecycle cases; none of the new cases
is ignored. Fixtures are uniquely owned private synthetic bytes. Test setup
and independent before/after reads are separate from adapter I/O authority.
Windows test setup resolves the newly owned temporary directory to its full
ordinary drive spelling; hosted runners may supply an abbreviated TEMP path.
This is fixture preparation only. Production root/sample syntax and authority
never canonicalize or expand saved references.
The same 44-case suite also passes locally with TEMP/TMP deliberately set to an
actual Windows short-name alias and to verbatim drive spelling. Neither variant
requires weakening the production path restrictions.

| Obligation | Evidence and limits |
| --- | --- |
| Present, absent leaf/intermediate, directory target, duplicate slots | Actual NTFS observations, fixed ordered outcomes and byte preservation |
| No content rights or source write lock | Windows rejects `ReadFile` on both source/sample adapter handles; normal writers and data-share locks still work |
| Source/root replacement and surviving old handle | Real save/atomic replacement, wrong identity/size/100 ns timestamp and root-to-junction cases reject stale authority |
| Ancestor binding stays trusted | Real no-delete-sharing pins refuse ancestor rename while descendant writes remain possible |
| Reparse introduced between query/open | Existing junction and insertion race reject before target traversal; owned target sentinels remain unchanged |
| Permission denial is not absence | Test temporarily denies metadata access on its owned target and restores the original DACL before cleanup |
| Known offline/recall/cloud state | Injected flags stop before candidate open; no real provider or hydration experiment is claimed |
| Case semantics and outside-root isolation | Real sensitive/insensitive leaf cases, conservative root variants and zero candidate queries for siblings/network paths |
| Durable revocation and cancellation | Injected current-authorization rejection and cancellation between query/open discard results and prevent the next syscall |
| Budgets and guard lifetime | Maximum source depth plus separate candidate chain fit handle/final-operation bounds; cancelled work keeps admission until lease retirement |
| Existing-only scanner behavior | Both actual disappeared-name regressions fail before the NT disposition correction and pass afterward |

Fake monotonic clocks make native race tests deterministic. They do not qualify
wall-clock performance. Portable blocked-worker tests prove publication/admission
fencing, not forced interruption of a Windows kernel call. Installed UI, real
host database fences, strict IPC decoding, renderer generations, accessibility
and shutdown integration belong to
[slice 3](../sample-presence-278/implementation-plan.md#slice-3-authorized-command-and-library-display).
G5 and Phase 3 acceptance stay open.

## Checks and handoff

Required checks are crate and changed-enumeration Clippy/tests, full pinned Windows
`pnpm check`, Markdown/privacy/link/diff checks, all approved fixture byte/hash
comparisons and normal CI on the final PR head. Actual final counts, exit codes,
SHA/tree, lock hashes and CI state are recorded in the PR and private handoff;
pending checks are never treated as passing evidence.

Codex `/root` exclusively owns the reused `bounded-probes` validation cache;
its absolute path and the review checkout/evidence locations are recorded only
in the private handoff. The pinned runner sets `CARGO_TARGET_DIR`. Preflight
uses four explicit directories, 6 GiB expected output and a 30 GiB reserve on
every used volume. Heavyweight builds/tests run sequentially. No installed or
performance qualification, new permanent cache or broad cleanup is required.
Retain logs, source provenance, preflight/internal-alias inventories and handoff
outside disposable compiler caches. Preserve the primary cache, other worktrees,
toolchains, profiles/databases, approved corpus and historical evidence binaries.
The existing cache remains reusable; no compiler intermediates are designated
obsolete by this slice. Rollback removes the inactive adapter/CI additions;
the independent existing-only scanner correction should be retained.
