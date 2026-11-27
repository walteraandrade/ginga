# Changelog

Each release of `ginga` and `ginga-gpui`, newest first. The two crates are released together, with the same version.

## Unreleased

- Variants: a parent's name that its children follow one after another, before or after the parent (`Variant`, `Order`, `ginga_gpui::Variant`, `.leads(..)`, `.follows(..)`). `Transitions::delay` and `LookMotion::settles_at`.

## 0.1.0

The first release. The engine, `ginga`: springs, tweens, keyframes, coasting, colors, looks, presences and slots, derived values, sequences, trails and the trace. The GPUI adapter, `ginga-gpui`, for `gpui-pre =0.3.12`: the frame's clock and frame requests, reduced motion, a look declared on an element, ways in and out, shared elements, dragging, loops and islands.
