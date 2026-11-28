# What it does not do yet

- Animate a change of width that layout makes: an item whose content grows wider takes its new width at once. A height glides with `.opened(id, 1.).height_glides(..)`, and a size that a look names moves.
- Gestures beyond a drag with one pointer: pinch, rotate, a drag on a touchpad's scroll.

Limits that come from GPUI and not from this library: a `div` has no transform, so moving a box means changing its position or size and laying out again; and there is no way to redraw one element without rendering its view.
