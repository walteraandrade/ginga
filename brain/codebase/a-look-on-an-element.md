# A look on an element

`LookExt::look` wraps any `Styled` element in `Looked<E>`, the way GPUI's own `with_animation` does: in `request_layout` it reads its motion, styles the inner element with this frame's look, turns it into an `AnyElement` and lays that out. Prepaint moves it by the offset; paint only hands on.

## Where the motion lives

- **With the id**: `window.with_element_state(global_id, ..)`. GPUI keeps that state only for elements drawn in the frame that just ended. Skip the element for one frame (a hidden branch, a page switched away) and its motion is gone: it comes back taking its look at once. That is the right default for a thing that stays on screen.
- **In a `LookState`**: an `Rc<RefCell<..>>` the view owns. Use it when the element must keep its motion while it is not drawn, such as a way out (#28) or a tab that is switched away and back.

A `Looked` with a `LookState` reports no id, so it adds no level to the element's global id.

## How each value is applied

- Offset: painted, not laid out (#61). `prepaint` runs the element inside `window.with_element_offset`, so GPUI adds it to every bound laid out below, the children's and their hitboxes included; the element's own hitbox (for hover and press) is inserted at `bounds + offset`. Its own `left`, `top` and `absolute` place stay its own. Before #61 the offset was written into `left` and `top`, which replaced an absolute element's place and lagged a card behind any move of the text it sits on. A non-finite offset paints nowhere else (`offset_of`). `present` and the drag move the same way: `present` draws each content with `.with_look(look)`, and `Dragged` carries the frame's offset in its layout state.
- `with_look` (`WithLook`) is the same mapping with no motion: no id, no element state, no hitbox of its own, and so no level in the global id and no frame requests. It styles in `request_layout` and paints the offset with `prepaint_at`.
- Opacity is clamped to `0..=1`, width, height and radius to `0` and up: a bouncy spring passes its target, and GPUI should not get a negative size.
- Colors (`bg`, `text_color`, `border_color`) go to GPUI as they are, with no clamp here: `Color::new` holds a color to its range when it is built, and `Color::from_channels` holds each channel there while it moves, dividing by the alpha before clamping it so a spring past full alpha keeps the hue.

## Hover, pressed and focused

- The pointer's state (hovered, pressed) lives in an `Rc<Cell<Gestures>>` inside the element's motion state, because the window's mouse listeners live past the frame that registered them. `request_layout` reads it to choose the look, and hands the cell to `paint` through the layout state.
- A hitbox is inserted in `prepaint` only when a hover or pressed look is given, so a plain look costs nothing more. It is inserted before the children prepaint, and the listeners are registered before the children paint, as the drag's are: a child that stops the press keeps it from the element. A test covers it. The press does not check `default_prevented`, as GPUI's `.active()` does: a `div` that tracks a focus handle calls `prevent_default` on every press when it takes the focus, and its listener runs before ours, so the check would turn the pressed look off on every focusable button.
- The listeners are window-wide, like the drag's. The move listener is in the capture phase, so nothing under the pointer can stop it. The press is in the bubble phase and checks the hitbox. The release is in the capture phase and is heard anywhere. None of them stops propagation, so the element's own `on_click` still runs. After a change they notify the view that drew the element (`window.current_view()`), and no other.
- Hover goes stale three ways, and each is closed: the pointer leaves the window without a last move (a `MouseExitEvent` clears it); a release is lost outside the window (a move with the left button up clears pressed, as in the drag); the element moves under a still pointer, as in a scroll (`paint` compares this frame's hit test with the stored hover and asks for a frame when they differ). After an exit, the stored hover stays off until the pointer moves in the window again, because GPUI keeps the last pointer position and its hit test would still say "over". Tests cover all three.
- It keeps track only of the moments it has a look for (`Hears`): a hover-only element registers no press listeners, and a press-only one never marks itself hovered, so a moment without a look redraws nothing. A moment whose look is no longer given is forgotten in `frame`, so it cannot come back stale.
- The first frame starts the look motion at the wanted look (focused included), not at the declared look: an element that appears with the focus takes that look at once. A test covers it.
- A value moves in a moment only when the element's own look names it too: a value a look starts naming jumps there (see `LookMotion`). The book's focus ring (A look for a moment) gives the rest look a transparent border for that reason.
- A size in a hover or pressed look moves the hit area. GPUI counts a click only when the release is over the element, so a pressed look that shrinks it can lose a click near an edge; the gallery sinks its buttons with an offset instead.
- The focused look reads `FocusHandle::is_focused` while laying out. A change of focus redraws the window, so no listener is needed. The element itself has to `track_focus` the same handle.
- `Gestures` is `pub` in a private module: it appears in `Looked`'s `Element` impl, which is public, but it cannot be named outside the crate.

## Settled

The look is settled when every value it names is; a value it stopped naming may still be moving in the look motion, and is neither drawn nor waited for.

Related: [[glossary]] (look, look motion, declared look), [[principles/settled-is-exact]].
