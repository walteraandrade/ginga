# A drag on an element

How `ginga_gpui::Dragged` follows the pointer, and the traps it avoids (#32).

## Where the pointer is heard

- A `div`'s `on_mouse_move` and `on_mouse_up` fire only while the pointer is over the element. A quick drag leaves the element within one frame, and a release outside it would be lost.
- So `Dragged` registers window-wide listeners in `paint` with `window.on_mouse_event`: the press checks its own hitbox (inserted in `prepaint`), the move and the release are heard anywhere in the window while the element is held, in the capture phase so that nothing under the pointer can stop them.
- After a press, a move or a release it notifies the view that drew the element (`window.current_view()` in `paint`), not `window.refresh()`: a refresh draws every view in the window again, cached ones included, which would undo what an island saves.
- The press stops propagation, so only the topmost draggable under the pointer is taken: `HitboxBehavior::Normal` does not hide the hitboxes beneath.
- The press listener is registered before the element's children paint. GPUI runs bubble listeners last-registered first, so a child's listener runs before it: a button inside a card that stops the press keeps the card from being taken. Registered after, the card would take every press first. A test covers it.
- A move with the left button up, while held, is a release that was lost (outside the window): the element is let go there.

## Where the element is

- `DragState` is an `Rc<RefCell<..>>` the view owns, because the listeners live past the frame that registered them and must reach the same state. There is no keyed form like `.look(id, ..)`: element state is reachable only while the element is laid out.
- While held, the offset is `from + (pointer − pointer at the press)`, exact, with no motion running; the engine's motion is set with `jump_to` at the press and again at the release.
- A press while it is already held (a release lost outside the window) goes on from where it is held, not from the motion, which still holds where the first press set it. A test covers it.

## The pointer's speed

- The engine's `Trail` measures it: the adapter notes the pointer at each move and asks for the speed at the release. The rules (100 ms, still for 50 ms) live and are tested there.
- A trail keeps eight places at least 15 ms apart, overwriting the newest until the next is far enough: a 1 kHz mouse would otherwise fill eight places in 8 ms and measure only the very end of the throw.
- Times come from the state's `Clock`, which on the test platform is the clock the test moves, so a test drags at an exact speed.

## Under reduced motion

The engine call is made as usual, and then the motion jumps to its target: limits and snap points still decide where it rests.
