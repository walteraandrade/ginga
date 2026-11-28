# A glide on an element

How `.glides(id, transition)` makes an element travel to where layout puts it (#80), and the traps it avoids.

## Where its place comes from

- In `prepaint`, the bounds GPUI hands an element already include `window.element_offset()`: every offset painted around it, a scroll container's included, a look's or a drag's offset, a glide around it. A glide takes its place as `bounds.origin - window.element_offset()`, so none of those is a move. Comparing raw bounds, as a first sketch did, glides on every scroll tick. Both critics of the 2026-11-28 review found this, and a test covers a scroll and a look's offset.
- A glide inside another glide measures its place from the outer one's place in the window (an app global stack, `Around`, pushed and popped around the child's prepaint). Otherwise a box that glides would carry its child, and the child would also glide by the same move: twice. A test covers it.
- A move shorter than half a pixel is not a move: GPUI snaps what it adds to the pixel.
- A frame where the window's size changed jumps: a resize is not something to animate.

## How it moves

- Its state, under its id, keeps a `Motion` of its place (`Point<Pixels>` through `InPixels<2>`), the last place, its own `Clock`, and the viewport size. On a move it sets the motion's target to the new place, so a second move on the way keeps the speed; it paints the child at `value - place` with `prepaint_at`, the same paint offset a look uses (#61). Settled, the offset is exactly zero.
- Layout is not touched: no relayout per frame. The frame of the move is drawn where the element was, since its bounds are known in prepaint before the child prepaints: no blank frame, unlike `.shared`.
- Its first frame is in place; under reduced motion the transition goes through `unless_reduced`, so a move is at once after its delay.

## Limits

- Rows of GPUI's `list` and `uniform_list` are laid out as roots, each on its own: their place is not a place in the window. `PresenceList::place_all` is the tool there.
- Size is not animated: see the accordion (#81).

Related: [[codebase/a-look-on-an-element]], [[codebase/a-shared-element]], [[glossary]] (glide).
