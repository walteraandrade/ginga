# An opened element

`crates/ginga-gpui/src/opened.rs`: `.opened(id, progress)`, an element that takes `progress` of its own height (#81).

**A box at every progress.** The element is always laid out inside a box of its own (`window.request_layout` with one child), and what places it in its parent moves to the box: margins, position and inset, flex grow, shrink and basis, `align_self`, width and its min and max (`place_in_parent`). A first version put the element straight in its parent at `1.0` and in a box below it, so a margin, a `flex_1` or a `w_full` resolved against two different parents: the height jumped at the end of every opening, and in a row the content was measured at its max-content width. All three review seats found it (PR #90). Tests cover a margin at 0.999 and a `flex_1` in a row.

**Where the height comes from.** It is not known before layout, so prepaint reads the content's laid-out height (`window.layout_bounds` of the child) and keeps it under the element's id. Each frame lays the box out with the height from the frame before. So the first frame drawn part of the way open, before anything was measured, takes no room, and a frame where the content's height changes draws the height before. Either way prepaint sees the new height and asks for one more frame.

**The content is not squeezed.** The box is a column and the content gets `flex_shrink: 0`. Without it, content whose automatic min-height is zero (one with `overflow_hidden`, as a rounded card has) shrinks to the box, measures that back, and closes a little more every frame. A test holds such content at 0.5.

**What is cut takes no clicks.** Part of the way open, prepaint and paint run under a content mask of the box's height, wide past its sides so a shadow or a focus ring stays, and GPUI clips a hitbox to the content mask it is inserted under. A content mask does not stop a tab stop, a key handler or a `deferred` child, so closed (`0.0`) the content is laid out, to be measured, and never prepainted or painted. The Stranger found both in review (PR #90).

**Frames.** It asks for a frame only when the new height moves the box half a pixel or more (`change × progress`): closed it never asks, and its kept height is updated on every frame.

**What it costs.** The box's height changes every frame while it opens, so its parent is laid out again. A cached view inside renders again on every frame, though its place and size stay, because the cut around it changes and GPUI reuses a cached view only under the same content mask ([[codebase/what-a-cached-view-saves]]). A cached view that the opening moves renders again too; one beside the parent does not. `tests/opened_cost.rs` counts it; the numbers are in `docs/measurements/2026-11-28-accordion`.

**What it does not do.** A height given as a share of the parent's (`h_full`) has no height of its own to measure. A content height that changes while fully open is taken at once: gliding to it is #86.
