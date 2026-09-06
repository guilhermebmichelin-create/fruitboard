# Issue #35 root settings evidence

Captured 2026-09-06 with local Chromium 1234, the shared React application and
its stateful fake platform adapter, using two synthetic paths named Projects.
No personal folders were read. This is browser/fake-adapter evidence: the
Connected label comes from the fake health response, not a native connection.

## Rendered and keyboard evidence

| Viewport           | Same-name controls                   | Explicit Cancel                    | Removal confirmation                       |
| ------------------ | ------------------------------------ | ---------------------------------- | ------------------------------------------ |
| Desktop 1280 x 800 | [Rename focus](desktop-keyboard.png) | [Cancel focus](desktop-cancel.png) | [Confirmation](desktop-confirmation.png)   |
| Narrow 390 x 844   | [Rename focus](narrow-keyboard.png)  | [Cancel focus](narrow-cancel.png)  | [Wrapped actions](narrow-confirmation.png) |

Images are full-page captures at those CSS viewport widths; narrow images are
longer than the viewport and fixed navigation appears at its captured position.
[Keyboard trace](keyboard-trace.json) records real Tab/Enter/Escape events,
focus assertions and horizontal overflow checks. The sequence reaches Rename
by Tab, edits a draft, tabs to Cancel, verifies restored Rename focus, repeats
with Escape, saves Released, verifies restored focus, opens confirmation with
Keep focused, cancels back to the matching Remove control, and removes that
same-name root through the keyboard before asserting Add folder focus.

Rendered testing initially caught horizontal overflow in the narrow removal
confirmation. Wrapping the action row fixes it; both sequences now pass without
horizontal overflow. Same-name control labels include the canonical path only
when needed to distinguish roots. Component tests additionally assert removal
of the selected same-name root and that explicit Cancel never saves its draft.

Reproduction: serve the client with Vite, mount `mountFruitboard` with
`createFakePlatform`, add Projects at `C:\Synthetic\Music\Projects` and
`D:\Synthetic\Archive\Projects`, navigate to Preferences, and follow the
keyboard sequence above at both viewport sizes. The capture harness is checked
in alongside the evidence; it uses an externally installed Playwright Core and
Chromium, not a production application dependency.

## Installed-app evidence and limits

The [#34 installed picker run](../issue-34/README.md) covers native selection,
cancellation and persistence at its recorded revision. It does not validate
these new rendered settings changes. No installed-app run was repeated here;
no screenshot here proves native picker, SQLite, restart or Windows packaging.

Inline Preferences onboarding remains the implemented proposal. Explicit owner
acceptance of replacing the separate first-run route is tracked separately and
is still pending. These captures make the current settings UI reviewable;
documenting it is not consent.
