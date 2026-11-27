# Ways in and out on an element

How `ginga_gpui::present` draws a thing until its way out is over, and the traps it avoids (#28).

## Where it is kept

- Under the element's id, with `window.with_element_state`: a `Slot` of numbered content, its clock, and the next number. The view calls `present` every frame, with `None` once the thing should go. The element itself is always drawn, so GPUI keeps its state.
- Each content is numbered when it is first shown, and drawn in a `div` whose id is its number. So when it swaps from arriving to leaving, what it holds under its own ids (a look motion, a focus) stays with it. Two contents are the same when the content is, whatever their numbers: `Numbered`'s `PartialEq` compares the content only.
- `Slot::show` with the same content does not replace it, so the number is kept. That is why `Presence` has `come_back`, which shows again without a new content. A new number is spent only when no content drawn equals the new one.
- Content that turns around on its way out (`came_back`) comes back from its *leaving* look, not its arriving one: the progress is continuous, and switching looks at the turn would jump. A test covers it.

## How it is drawn

- The look is `arrive_from.mix(Look::new(), progress)` while entering, and `leave_to.mix(..)` while exiting. Mixed into an empty look, only an offset and an opacity move (to none and to opaque). Other values in an arriving look are left out (see `Look::mix`).
- What a swap sends away is drawn `absolute` at the top left of a `relative` box, under the arriving one, and takes no room. The look's offset is painted from there (see [[codebase/a-look-on-an-element]]), so it adds to the top left instead of replacing it.
- While leaving, an `occlude()` div covers the content, so it takes no clicks. The wrapper of each leaving content tracks a focus handle of its own, and while the focus is inside it (`contains_focused`) the window is blurred, deferred out of the draw. It is checked on the rendered frame, so the focus goes one frame after the way out starts, and a Tab into leaving content is undone a frame later. The handle is tracked only while leaving: tracked always, a click on plain content would focus the wrapper.
- Under reduced motion both ways are `Instant`. Without `arrive_from` the way in is `Instant`, and without `leave_to` the way out is.

## Swap and Wait

`Swap::Wait` delays the new content's way in by what is left of the ways out (`Presence::gone_at`, from `settles_at`), and `Slot` hides it until then (`waits_until`): an arriving look without an opacity would otherwise show it over the old one. Content shown during the wait takes the waiting one's place, unseen, so nothing leaves twice. Up to two contents leave at once; a fourth swap within one way out lets the oldest go. `Present` sets the swap each frame (`set_swap`), so a change applies to the next swap.

Related: [[codebase/a-look-on-an-element]], [[glossary]] (slot, swap, mix, arriving look).
