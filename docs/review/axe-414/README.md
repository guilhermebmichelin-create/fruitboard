# axe-core 4.14 accessibility follow-up

The dependency update enables Label in Name checks by default. The previous
jsdom tests lacked a canvas renderer, producing incomplete checks when axe
tried to distinguish ordinary text from icon-font ligatures.

The test setup now uses exact `@napi-rs/canvas` 1.0.13 for real glyph rendering.
It is a development dependency imported only by the Vitest setup. The rule is
not disabled, and incomplete results still fail the existing accessibility tests.
A negative control confirms that the old expanded-details label fails while
the corrected label passes without incomplete results.

The details toggle now announces "Hide project details" when expanded and
"Project details" when collapsed, each followed by its synthetic or actual file
name. "Refresh details" is contiguous in the refresh button's accessible name.
The removal confirmation says "Confirm removal", matching its existing
accessible name. Actions, source authority, storage and layouts are unchanged.

## Validation

- All 413 client tests passed across 34 files, including the three new rule
  controls and the existing loading/error, keyboard/focus and accessibility tests.
- Client ESLint, strict TypeScript and production web build passed. The web
  build retains existing non-Windows Safari-target BigInt warnings; this change
  does not alter the supported Windows target or claim Safari qualification.
- All 164 Node script/policy tests, privacy, scoped formatting, Markdown lint
  and whitespace checks passed.
- A real headless Chromium run checked synthetic Preferences and Library
  states at 1280 x 800 and 390 x 844. Both states had zero Label in Name
  violations or incomplete results and no horizontal overflow. Keyboard
  Enter closed and reopened details without losing focus.

The shared preview snapshot timed out twice, so browser evidence used the
repository's existing Playwright Core capture tooling and installed Chromium.
Raw browser observations are retained privately outside compiler caches.
No real project or filesystem scan was used. These are browser/client checks,
not installed-app, parser-field or performance qualification. Normal CI must
pass again on the updated PR head before merge.

## Screenshots

- [Desktop removal confirmation](desktop-confirm-removal.png)
- [Narrow removal confirmation](narrow-confirm-removal.png)
- [Desktop expanded project details](desktop-project-details.png)
- [Narrow expanded project details](narrow-project-details.png)
