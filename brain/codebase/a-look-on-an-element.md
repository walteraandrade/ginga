# A look on an element

`LookExt::look` wraps any `Styled` element in `Looked<E>`, the way GPUI's own `with_animation` does: in `request_layout` it reads its motion, styles the inner element with this frame's look, turns it into an `AnyElement` and lays that out. Prepaint and paint only hand on.

## Where the motion lives

- **With the id**: `window.with_element_state(global_id, ..)`. GPUI keeps that state only for elements drawn in the frame that just ended. Skip the element for one frame (a hidden branch, a page switched away) and its motion is gone: it comes back taking its look at once. That is the right default for a thing that stays on screen.
- **In a `LookState`**: an `Rc<RefCell<..>>` the view owns. Use it when the element must keep its motion while it is not drawn, such as a way out (#28) or a tab that is switched away and back.

A `Looked` with a `LookState` reports no id, so it adds no level to the element's global id.

## How each value is applied

- Offset: `left` and `top`, after `relative()` unless the element is `absolute`. An absolute element keeps its position, and the offset replaces its own `left` and `top`, so it should give its place as the offset.
- Opacity is clamped to `0..=1`, width, height and radius to `0` and up: a bouncy spring passes its target, and GPUI should not get a negative size.
- Colors (`bg`, `text_color`, `border_color`) go to GPUI as they are, with no clamp here: `Color::new` holds a color to its range when it is built, and `Color::from_channels` holds each channel there while it moves, dividing by the alpha before clamping it so a spring past full alpha keeps the hue.

## Settled

The look is settled when every value it names is; a value it stopped naming may still be moving in the look motion, and is neither drawn nor waited for.

Related: [[glossary]] (look, look motion, declared look), [[principles/settled-is-exact]].
