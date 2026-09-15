# Non-NTFS scanner scope decision - 2026-09-15

- Status: **Owner decision recorded — local NTFS is the supported scanner
  scope; DriveFS, FAT32, and cross-volume identity are excluded. #47 and #48
  stay open as unverified-territory markers.**
- Date: 2026-09-15
- Decision owner: repository owner
- Baseline: `origin/main` `23547c3`, the merge of PR #136; this record is
  published on the `docs/47-48-scope-exclusion` branch.
- Related: #47, #48, #33 (Phase 2 epic), #36, #37. Evidence:
  [P0-D](p0-d-drivefs-watcher.md), [P0-E](p0-e-file-identity.md),
  [P0-E follow-up](p0-e-followup-20260908.md), the
  [P2-47 runbook](p2-47-drivefs-host-runbook.md), the
  [P2-48 runbook](p2-48-fat32-identity-runbook.md), and the
  [Phase 2 acceptance record](../review/phase-2-integration/acceptance-2026-09-14.md).

## Question

Which filesystem environments does the filesystem-only scanner support while
the #47/#48 environments have no qualifying host in this environment?

## Basis

- P0-D found no DriveFS driver, service process, or mount on the survey host;
  every DriveFS case stayed unverified. Local NTFS watcher behavior was
  characterized.
- P0-E characterized local NTFS file identity; FAT32 file-ID semantics and
  cross-volume moves stayed unverified. The 2026-09-08 follow-up confirmed no
  writable FAT32 volume and no second writable real volume on that host.
- The 2026-09-14 Phase 2 acceptance record left #47/#48 open, stated that no
  scope exclusion had been applied, and the two runbooks allow an explicit
  owner scope decision as the alternative to a host run.

## Decision

Recorded by this documentation task from the owner's 2026-09-15 decision:

1. Supported scope is **local NTFS only**. DriveFS-backed roots (mirrored and
   streamed), FAT32 file-ID semantics, and cross-volume file identity are
   excluded from the supported scanner scope.
2. The exclusion is a no-qualifying-host decision, not a failure finding. The
   excluded environments remain **unverified**, and this record makes no
   support, fallback, or qualification claim about them either way.
3. #47 and #48 stay **open** as unverified-territory markers. They are not
   closed, relabeled, or edited, and no comment on them claims support.
4. Outside the supported scope, identity continuity remains
   path-continuity-only with explicit uncertainty, matching the P0-E
   decisions; this record specifies no new fallback behavior.
5. Host runs may reopen the decision. A run that satisfies the reopening
   evidence below can be proposed for a **new explicit owner approval**, but
   the run alone does not change the supported scope.

## What this record does not change

- No budget, quota, fixture, or performance-target change; performance stays
  unqualified and F1/F2/F3 carry forward exactly as recorded on 2026-09-14.
- No Phase 3 start, no parser work, and no production-scanning activation; the
  bounded Rust-parser spike remains the stated next step.
- No code, workflow, lockfile, or other documentation is touched, and no
  dated record is rewritten.
- Accepted local-NTFS dispositions (P2-02, P2-07, P2-09) are unchanged.

## Reopening evidence

- **#47:** an authenticated Drive for Desktop Windows host run covering
  mirrored vs streamed enumeration, placeholder vs hydrated metadata,
  hydration side effects of reads, burst/rename/disconnect behavior, and
  watcher fidelity on Drive roots.
- **#48:** a Windows host with a genuine writable FAT32 volume plus a second
  writable volume, covering the P0-E operation table on FAT32 and
  NTFS-to-FAT32-to-NTFS moves, with drive letter, serial, filesystem, and
  allocation-unit evidence.

## Checks and provenance

- Decision provenance: this file records the owner's 2026-09-15 decision, as
  authorized by the same owner-approved workflow that produced the Phase 2
  acceptance record; the two issue comments link back to this record and do
  not close, label, or edit #47/#48.
- Documentation-only change: `pnpm privacy:check`, `pnpm lint:docs`,
  `pnpm lint:scripts`, and `git diff --check` must pass. No local build,
  fixture, benchmark, installed run, or Cargo cache is used; merged-main CI
  covers the unchanged code and the PR's required contexts are the
  publication gate.
