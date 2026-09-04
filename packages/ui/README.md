# Fruitboard UI foundation

`@fruitboard/ui` owns shared interface primitives only when the product has a
real consumer for them. Issue #13 introduces its first asset:
`src/tokens.css`.

The token vocabulary covers typography, spacing, light surfaces, borders,
shape, elevation, motion, focus indicators, icons, controls, and shell geometry.
Application styles consume these semantic custom properties and do not reach
into native or platform behavior.

Dark mode is deferred, not rejected. A future theme must preserve the semantic
token names and pass the same contrast and state checks before it ships.
