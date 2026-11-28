# An opened element

`crates/ginga-gpui/src/opened.rs`: `.opened(id, progress)`, an element that takes `progress` of its own height (#81).

**Where the height comes from.** Its own height is not known before layout, so it is measured as it is prepainted (`Measured` writes `bounds.size.height` into a cell) and kept under the element's id. Each frame lays out with the height kept from the frame before. So the first frame drawn part of the way open, before anything was measured, takes no room, and a frame where the content's height changes draws the height before. Either way prepaint sees the new height and asks for one more frame. A change under half a pixel is not a change, so rounding does not ask for frames forever.

**Fully open adds nothing.** At `1.0` the element is laid out as it is, inside `Measured`, which adds no layout node. Only part of the way open is it held in a box: `flex_col`, `overflow_hidden`, a height of `progress` × the kept height, and the content set to `flex_shrink_0`. Without `flex_shrink_0` the column would squeeze the content to the box, and the box would measure its own height back.

**What is cut takes no clicks.** `overflow_hidden` sets a content mask, and GPUI clips a hitbox to the content mask it is inserted under. The test clicks a row in the cut part and counts nothing.

**What it costs.** The box's height changes every frame while it opens, so its parent is laid out again. A cached view inside is rendered again, since GPUI reuses a cached view only at the same bounds ([[codebase/what-a-cached-view-saves]]). The README says so.

**What it does not do.** A height given as a share of the parent's (`h_full`) has no height of its own to measure. A content height that changes while fully open is taken at once: gliding to it is #86.
