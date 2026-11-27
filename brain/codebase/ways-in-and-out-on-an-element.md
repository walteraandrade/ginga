# Ways in and out on an element

How `ginga_gpui::present` draws a thing until its way out is over, and the traps it avoids (#28).

## Where it is kept

- Under the element's id, with `window.with_element_state`: a `Slot` of numbered content, its clock, and the next number. The view calls `present` every frame, with `None` once the thing should go. The element itself is always drawn, so GPUI keeps its state.
- Each content is numbered when it is first shown, and drawn in a `div` whose id is its number. So when it swaps from arriving to leaving, what it holds under its own ids (a look motion, a focus) stays with it. Two contents are the same when the content is, whatever their numbers: `Numbered`'s `PartialEq` compares the content only.
- `Slot::show` with the same content does not replace it, so the number is kept. That is why `Presence` has `come_back`, which shows again without a new content.

## How it is drawn

- The look is `arrive_from.mix(Look::new(), progress)` while entering, and `leave_to.mix(..)` while exiting. Mixed into an empty look, only an offset and an opacity move (to none and to opaque). Other values in an arriving look are left out (see `Look::mix`).
- What a swap sends away is drawn `absolute` at the top left of a `relative` box, under the arriving one, and takes no room. It is made absolute *before* the look is applied: `styled` keeps an absolute element absolute and puts the offset in `left` and `top`, and a later `top_0()` would undo it. A test caught that.
- While leaving, an `occlude()` div covers the content, so it takes no clicks and gets no focus from the pointer. The keyboard can still reach a focusable child on its way out; GPUI gives no way to take tab stops out of a subtree.
- Under reduced motion both ways are `Instant`. Without `arrive_from` the way in is `Instant`, and without `leave_to` the way out is.

## Swap and Wait

`Swap::Wait` delays the new content's way in by what is left of the old one's way out (`Presence::gone_at`, from `settles_at`). It is one delay, set at the swap: the schedule is fixed then, as every motion's is.

Related: [[codebase/a-look-on-an-element]], [[glossary]] (slot, swap, mix, arriving look).
