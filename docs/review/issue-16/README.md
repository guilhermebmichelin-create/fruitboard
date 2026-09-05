# Issue #16 startup preference review

This evidence covers the one startup-view preference vertical slice. It uses
the same shared React component with the stateful fake platform port so the
visual review does not grant a browser Tauri authority. Rust integration tests
separately exercise the real command service, migration, repository, and
database reopen.

## Interaction evidence

### Desktop — 1280 × 800 viewport

[Keyboard selection and save](desktop-preference-keyboard.mp4)

### Narrow touch layout — 390 × 844 viewport

[Keyboard selection and save](narrow-preference-keyboard.mp4)

Both recordings show the native select, Save preference action, saved-value
hint, and polite success announcement. The narrow view retains 48-pixel controls
and the existing bottom navigation.

## Behavioral evidence

- Fresh databases default to Home.
- The native command test saves Library, closes the database, reopens it, and
  reads Library through the command envelope.
- Startup routing restores a saved view only when no explicit hash route was
  supplied. Deep links remain authoritative.
- Invalid, missing, extra, and path-like values fail before mutation on the
  Rust boundary. The TypeScript adapter rejects invalid input and malformed or
  inconsistent output.
- Loading, load-error/retry, safe Home fallback, save-success, and save-error
  behavior have component tests. Error text never includes rejected diagnostic
  details.
- Axe covers the settled preference plus startup loading and error states;
  keyboard tests cover selection and submission.

## Deliberate limits

No project record, filesystem path, scanner root, FLP metadata, parser, Kanban,
sync, PWA entry, or installer behavior is added. SQLite remains Rust-owned with
DELETE journaling and one native connection.
