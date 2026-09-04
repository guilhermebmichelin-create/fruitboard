# ADR-006: Shared desktop/PWA client architecture

- Status: Accepted
- Date: 2026-09-04
- Approved: 2026-09-04 by the product owner

## Context

Desktop and PWA share project-management concepts and most screens, but only the
desktop scans files, parses FLPs, opens FL Studio, and owns native SQLite. A
shrunk desktop interface would also produce a poor touch experience. Two
unrelated clients would duplicate behavior and drift.

## Decision

Build one React/TypeScript client workspace with shared domain, screen, and UI
packages. All product code consumes explicit platform ports. Desktop adapters
call typed Tauri use cases; PWA adapters use IndexedDB/Drive. Platform
capabilities and routes are feature-declared, so desktop-only actions are absent
or clearly unavailable on PWA.

Use distinct responsive navigation/layout shells where appropriate. Share
concepts and components, not forced pixel-identical layouts. Build separate
desktop and PWA entries so service-worker/browser-auth code does not enter the
privileged desktop bundle unnecessarily.

## Alternatives

- Independent desktop and PWA codebases: maximum layout freedom but duplicate
  domain rules, accessibility work, and conflict bugs.
- One desktop DOM simply scaled down: poor touch/navigation hierarchy and
  confusing desktop-only actions.
- Tauri mobile app instead of PWA: does not match the requested installable web
  companion and adds native distribution complexity now.
- Server-rendered full-stack framework: no server requirement and unnecessary
  runtime coupling for the local-first client.

## Consequences

- Platform interfaces must remain clean and fakeable in tests.
- Shared merge/query/domain rules reduce drift.
- Desktop and PWA persistence implementations require common conformance tests.
- Conditional code splitting and capability-aware navigation add some structure.
- Responsive UX can diverge deliberately while design tokens and vocabulary
  remain consistent.

## References

- [Repository and client architecture](../../ARCHITECTURE.md)
- [PWA offline/sync design](../../SYNC.md#pwa-offline-architecture)
