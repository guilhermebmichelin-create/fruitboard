# Shared dependency security repairs, 2026-10-28

PR350 patches indirect development dependencies without changing audit thresholds,
application capabilities, database schemas, or default feature activation.

- pnpm overrides retain markdown-it 14.3.3, zod 4.6.6 and undici 8.12.2.
- Cargo.lock updates Tokio from 1.53.1 to 1.53.3, a patched version for
  [RUSTSEC-2026-0386](https://rustsec.org/advisories/RUSTSEC-2026-0386.html).
  Cargo also reconciled existing dependency edges while regenerating the lock;
  no other package version changed.
- The advisory concerns Unix AsyncFd ownership. The supported Windows target
  does not justify ignoring it: the unchanged audit policy still rejects it.
- All existing advisory baseline entries and required CI checks remain intact.

## Validation boundary

The frozen installation with Node 24.20.0 and pnpm 11.25.0 succeeds. The live
`pnpm audit --audit-level moderate` succeeds with one disclosed low-severity
finding. All 148 repository Node tests pass. Normal updated-head CI must validate
the complete dependency graph, native builds, client suite and Windows package.
Earlier passing checks cover unchanged source only.

This dependency repair needs no local Rust build, installer, installed-app
test or performance measurement. No compiler cache is written for this slice.
Private command logs remain outside source and compiler caches in the existing
review-evidence area. The primary cache is preserved, and the reusable validation
cache remains reserved to this thread for later feature validation.

This does not accept resource budgets, the Phase 3 exit, a product license or
distribution. PR348 still requires its feature-enabled installed validation.
