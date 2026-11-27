# A drag on an element

How `ginga_gpui::Dragged` follows the pointer, and the traps it avoids (#32).

## Where the pointer is heard

- A `div`'s `on_mouse_move` and `on_mouse_up` fire only while the pointer is over the element. A quick drag leaves the element within one frame, and a release outside it would be lost.
- So `Dragged` registers window-wide listeners in `paint` with `window.on_mouse_event`: the press checks its own hitbox (inserted in `prepaint`), the move and the release are heard anywhere in the window while the element is held, in the capture phase so that nothing under the pointer can stop them.
- After each move or release it calls `window.refresh()`: the listener does not know which view draws the element.

## Where the element is

- `DragState` is an `Rc<RefCell<..>>` the view owns, because the listeners live past the frame that registered them and must reach the same state. There is no keyed form like `.look(id, ..)`: element state is reachable only while the element is laid out.
- While held, the offset is `from + (pointer − pointer at the press)`, exact, with no motion running; the engine's motion is set with `jump_to` at the press and again at the release.
- A press while it is already held (a release lost outside the window) goes on from where it is held, not from the motion, which still holds where the first press set it. A test covers it.

## The pointer's speed

- The last eight places of the pointer and their times are kept in an array: no allocation while dragging.
- At the release, the speed is measured from the oldest place in the last 100 ms to the newest. If the pointer was still for more than 50 ms before it let go, the speed is zero: the element is set down, not thrown.
- Times come from the state's `Clock`, which on the test platform is the clock the test moves, so a test drags at an exact speed.

## Under reduced motion

The engine call is made as usual, and then the motion jumps to its target: limits and snap points still decide where it rests.
