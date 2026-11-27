# A shared element

How `.shared(key, transition)` makes two elements one thing that travels between their destinations, and the traps found on the way (#33).

## Where it is kept

- Each element keeps, under its own id (`ElementId::Name(key)` inside whatever ids are around it), a `Kept`: its motion of bounds, its last destination, and a token `Rc<()>`.
- An app global, `Places`, keeps for each window and key the last element that drew it: its global id, a `Weak` of its token, and its motion. It is keyed by `(WindowId, key)`: an app global is shared by every window, and two windows of one view would otherwise fly marks between them. GPUI drops element state one frame after the element is no longer drawn, so the `Weak` tells whether the last owner was drawn in the frame before. A key whose owner is long gone is no place to come from.
- When a new key is written, entries whose owner is gone are dropped, so a key per list row does not grow the map for the app's life. At rest an element writes nothing: only a new destination, a new motion or a new owner is written.
- `Places` also holds the one clock every shared element reads. Each element reads it once per frame, in `request_layout`, and carries the time to `prepaint` in its `Frame`.

## The frames of a trip

1. The new element's state is made in `request_layout`. It takes the motion of the key's last element when that is another element, still alive, in the same window, and motion is not reduced. It has a motion but no destination yet: it is laid out normally and not painted (`Mode::Measuring`). This is the one frame a trip spends drawing nothing. It cannot draw the traveller then: the element is laid out once per frame, in its destination, and an element is not `Clone`.
2. In `prepaint` it learns its bounds and sets the motion's target there, from wherever the motion is. That keeps a change of destination in the middle of a trip continuous.
3. Each frame after, until the motion settles, it is a holder `div` in the layout with the element itself `absolute` at the motion's bounds, relative to the holder, inside `deferred(..).with_priority(1)`, so it is drawn over the rest (`Mode::Travelling`). The holder takes the element's own layout style: its position and inset, margins, flex grow, shrink and basis, `align_self`, min and max size, and its width and height where the element names one (else the size it had). So an absolute mark stays out of the flow, a margin keeps its room, and a destination whose size changes mid-trip (a `w_full` card whose parent resizes) is followed, not snapped to. Tests cover all three.
4. Settled, it is drawn in place and asks for no frame. Later moves of its destination, from layout, are jumps: only the first destination, or one that changes while travelling, is a target.

## Traps

- GPUI's global id holds only the ids of ancestors that *have* ids. Tabs or rows with no id give every shared element of a key the same global id, so GPUI sees one element that moved: no new state, and no trip. The tests give each tab and the row and card an id, and the doc says so.
- `Mode` and `Frame` are `pub` in a private module, because they appear in the `Element` impl's associated type (E0446 otherwise).
- Bounds move through `impl Animate for Bounds<Pixels>`, whose stand-in is `InPixels<4>`: x, y, width, height.
- Under reduced motion a new element takes no motion from the key, so it is in its destination in its first frame and asks for none.

Related: [[codebase/ways-in-and-out-on-an-element]], [[codebase/testing-the-adapter]], [[glossary]] (shared element, traveller).
