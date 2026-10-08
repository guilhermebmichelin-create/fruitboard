# Saved pattern note records in Library

Status: issue [#294](https://github.com/guilhermebmichelin-create/fruitboard/issues/294),
accepted note-count design slice 3, merged as PR295 on 2026-10-08.
Merge: `2bb2c187eef0d4faee56e96f5774af6b9b7b8251`, tree
`19cb8ba31317f82a18d763693377370d1e8234f2` (identical to tested PR head).
Baseline: PR293 merge `ff48da24fc15151cc81d82edb168126811825706`, tree
`acb63317fe62b4c111c6ffb0b5cbd71823f7cb1e`.

## Behavior and boundary

The existing **Saved patterns** list adds **Saved note records** beneath each
saved name. Counts include stored slide/step records and entries that may not
sound; reusing a pattern in the Playlist does not increase its stored count.
There is no project musical total or per-channel count. Only exact build
26.1.0.5530 and the previously qualified initial content series are supported.
Explicit-zero payloads remain unqualified. Missing payloads stay unknown.

New analysis saves selected numeric counts and fixed states in the additive
`patternNoteCounts` field of immutable private projection version 1. There is
no migration, new table or rewrite. Older snapshots without the field remain
readable and explain that another analysis is needed. Unadvertised, ambiguous,
unsupported-layout and limit states use fixed explanations. Malformed present
data fails the complete details read; it cannot silently become an older result.

The authorized native read revalidates the exact build, coverage, ordered IDs
against saved patterns, positive integer counts, 65,536 per-pattern and 262,144
total ceilings, aggregate/state relationships and exact shapes. The renderer
independently checks the selected IDs, counts, states and ceilings. Raw notes,
pitches, velocities, positions, lengths and flags are never persisted or sent
to the renderer. Names remain inert escaped text.

Existing root/location/row-fingerprint/current-snapshot, publication/session/
lease fences remain authoritative. Disabled, changed or missing sources expose
no current counts. Loading clears previous results; late replies after closing
are discarded. Refresh reads results without analysis. Explicit analysis keeps
the existing attempt policy. The existing 20-pattern disclosure retains keyboard
focus. Default desktop and PWA analysis behavior remains unchanged.

## Validation record

Full pinned Windows `pnpm check` passes; scan-console has 118 passing tests,
analysis-jobs 157 (four existing opt-in review/host tests ignored), with
all-target warning-denied Clippy for both compositions. Three explicitly enabled
real-parser worker tests pass separately. Approved F16/F17 cross real parsing,
immutable publication, authorized reads, refresh, restart and root revocation.
Storage tests retain last-good results after malformed replacement attempts and
preserve earlier snapshots. Native/renderer tests reject foreign/reordered IDs,
zero/fractional/oversized counts, invalid states and total-ceiling violations.
All 15 approved FLPs and recorded note expectations remain byte-identical.

Installed qualification used a fresh, separately identified unsigned development
package/profile under the exclusive host lock, built from
`e63898668de169d51bb1e22f4495dc91e1928a4c`, tree
`5f062d6827686ecc2f891ce8525cb22f827e98c5`. Later changes add only this review
record/screenshots. Installed real scans/analysis yield 3/2/4 and a named empty
pattern with unknown note count. Opening/refreshing preserves snapshot and
attempt count; explicit analysis creates a new snapshot and uses the existing
attempt policy. Counts survive restart and uninstall/reinstall. Disabled roots
hide counts; changed-source scans fence an older displayed row. Source bytes,
personal/FoundationSmoke profiles and all approved fixtures remain unchanged.
The review app was closed/uninstalled; its private database/evidence remains.

Desktop, 390px and 200%-text layouts were visually inspected; no horizontal
overflow was measured. Native axe WCAG checks, including contrast, report zero
violations at all three sizes and on constructed pagination. Keyboard Enter
reveals 20/40/45 records and collapses to 20 while retaining button focus.
Closing during a deliberately held successful native reply clears counts,
retains disclosure focus and discards the late result. This is a constructed
transport race; existing native freshness tests cover authority changes.

Constructed older/layout/limit/paging/malformed states replace only the renderer
reply after a real native read. They do not modify saved data or qualify a new
FL Studio encoding. Initial capture/focus-driver defects were corrected and the
complete journey passed on resume in the same fresh review profile. Final
viewport captures supersede an initial blank 200%-text capture.

Independently copied installer SHA-256:
`145b9cb58b41314e03bad7f13481c4495b6e18aa66beece8831dbdfbe3a0854c`.
Desktop executable:
`9e50a0ac378477a93e4d4e34120374f859e21a697fce273d315cfb9ec885c8d8`.
Parser executable:
`b01fac6314a59736960b12b38895856a18ddd59d6cb9e3498857b21102666fd7`.
The private handoff retains final head/tree, source equality, exact commands,
host-lock ownership, artifact/profile/corpus hashes and final CI receipts.

## Sanitized installed screenshots

Only approved fixture names and synthetic review values appear here. Cropped
views come from the actual installed native renderer; viewport captures show
the app around the tested region. At 390px the last pattern is also shown after
scrolling, so its count is visible above the fixed navigation.

- [Desktop: real 3/2/4 counts](screenshots/notes-desktop.png)
- [390px list](screenshots/notes-390.png) and
  [last pattern after scrolling](screenshots/notes-390-last-pattern.png)
- [200% text: real first-pattern count](screenshots/notes-text-200-viewport.png)
- [Real named empty pattern: unknown count](screenshots/notes-missing-payload.png)
- [Constructed older result](screenshots/notes-older-constructed.png)
- [Constructed unverified layout](screenshots/notes-layout-constructed.png)
- [Constructed note-record limit](screenshots/notes-limit-constructed.png)
- [Constructed 45-pattern paging and keyboard focus](screenshots/notes-paging-constructed-viewport.png)
- [Loading during a constructed held reply](screenshots/notes-loading.png)
- [Safe details error for constructed malformed data](screenshots/notes-malformed.png)

## Remaining work and recovery

This completes the three narrow saved note-count slices when accepted. Broader
G6 automation/MIDI, mixer/nested plugins, tempo changes and richer arrangements
remain open. It does not accept Phase 3, production activation/distribution,
resource budgets or performance qualification. No new command, permission,
filesystem resolver, audio/process capability or dependency is added.

Rollback removes selection/display, retains immutable history and ignores the
additive field. Owner review decides whether to accept this slice; fixture
publication approval already covers the exact unchanged F16/F17 files.
