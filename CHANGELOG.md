# Changelog

Each release of `ginga` and `ginga-gpui`, newest first. The two crates are released together, with the same version.

## Unreleased

- Variants: a parent's name that its children follow one after another, before or after the parent (`Variant`, `Order`, `ginga_gpui::Variant`, `.leads(..)`, `.follows(..)`). `Transitions::delay` and `LookMotion::settles_at`.
- `LookExt::with_look`: a look applied as it is, with no motion and no element state, such as one mixed by a presence's progress.
- A look's offset, and a dragged element's, is painted instead of laid out: it adds to the element's own place, `absolute` ones included, and the pointer finds the element where it is painted. Before, it replaced the element's `left` and `top`: an `absolute` element that gave its place as the offset, as 0.1.0's docs said, now gives it with `left` and `top`.
- `Presence::wanted` and `Presence::leaving`, in the engine and the adapter, and `ginga_gpui::Presence::sample(cx)`, read with no window.
- Reduced motion keeps a transition's delay: the value holds still until the delay is over, then is at its target at once. `Transition::without_motion` and `Transitions::without_motion`. Before, the delay was dropped with the motion. A variant's schedule and a shared element's trip are still dropped.
- A list presence's items at rest no longer glide when an item before them is let go and the layout moves them up by the room it took: they take their new place at once. Before, they were drawn a row too low and glided back up. An item on its way somewhere goes on from where it is drawn, and a place that stays put, as in a list held at its bottom, is no move.

## 0.1.0

The first release. The engine, `ginga`: springs, tweens, keyframes, coasting, colors, looks, presences and slots, derived values, sequences, trails and the trace. The GPUI adapter, `ginga-gpui`, for `gpui-pre =0.3.12`: the frame's clock and frame requests, reduced motion, a look declared on an element, ways in and out, shared elements, dragging, loops and islands.
