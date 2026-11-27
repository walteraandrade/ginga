# Changelog

Each release of `ginga` and `ginga-gpui`, newest first. The two crates are released together, with the same version.

## Unreleased

- Variants: a parent's name that its children follow one after another, before or after the parent (`Variant`, `Order`, `ginga_gpui::Variant`, `.leads(..)`, `.follows(..)`). `Transitions::delay` and `LookMotion::settles_at`.
- A look's offset, and a dragged element's, is painted instead of laid out: it adds to the element's own place, `absolute` ones included, and the pointer finds the element where it is painted. Before, it replaced the element's `left` and `top`: an `absolute` element that gave its place as the offset, as 0.1.0's docs said, now gives it with `left` and `top`.
- `Presence::wanted` and `Presence::leaving`, in the engine and the adapter, and `ginga_gpui::Presence::sample(cx)`, read with no window.

## 0.1.0

The first release. The engine, `ginga`: springs, tweens, keyframes, coasting, colors, looks, presences and slots, derived values, sequences, trails and the trace. The GPUI adapter, `ginga-gpui`, for `gpui-pre =0.3.12`: the frame's clock and frame requests, reduced motion, a look declared on an element, ways in and out, shared elements, dragging, loops and islands.
