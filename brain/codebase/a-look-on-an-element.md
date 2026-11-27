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

## Hover, pressed and focused

- The pointer's state (hovered, pressed) lives in an `Rc<Cell<Gestures>>` inside the element's motion state, because the window's mouse listeners live past the frame that registered them. `request_layout` reads it to choose the look, and hands the cell to `paint` through the layout state.
- A hitbox is inserted in `prepaint` only when a hover or pressed look is given, so a plain look costs nothing more.
- The listeners are window-wide, like the drag's. The move listener is in the capture phase, so nothing under the pointer can stop it. The press is in the bubble phase and checks the hitbox. The release is in the capture phase and is heard anywhere. None of them stops propagation, so the element's own `on_click` still runs. After a change they notify the view that drew the element (`window.current_view()`), and no other.
- The focused look reads `FocusHandle::is_focused` while laying out. A change of focus redraws the window, so no listener is needed. The element itself has to `track_focus` the same handle.
- `Gestures` is `pub` in a private module: it appears in `Looked`'s `Element` impl, which is public, but it cannot be named outside the crate.

## Settled

The look is settled when every value it names is; a value it stopped naming may still be moving in the look motion, and is neither drawn nor waited for.

Related: [[glossary]] (look, look motion, declared look), [[principles/settled-is-exact]].
