# A shared element

How `.shared(key, transition)` makes two elements one thing that travels between their places, and the traps found on the way (#33).

## Where it is kept

- Each element keeps, under its own id (`ElementId::Name(key)` inside whatever ids are around it), a `Kept`: its motion of bounds, its last place, and a token `Rc<()>`.
- A window-wide global, `Places`, keeps for each key the last element that drew it: its global id, a `Weak` of its token, and its motion. GPUI drops element state one frame after the element is no longer drawn, so the `Weak` tells whether the last owner was drawn in the frame before. A key whose owner is long gone is no place to come from.
- `Places` also holds the one clock every shared element reads, so a motion handed from one element to another reads the same time.
- `Places.last` is never pruned: it holds one entry per key, and keys are few.

## The frames of a trip

1. The new element's state is made in `request_layout`. It takes the motion of the key's place when the owner is another element and still alive. It has a motion but no place yet: it is laid out normally and not painted (`Mode::Measuring`). This is the one frame a trip spends drawing nothing; a frame is cheaper than guessing the layout.
2. In `prepaint` it learns its bounds and sets the motion's target there, from wherever the motion is. That keeps a change of destination in the middle of a trip continuous, which a test covers.
3. Each frame after, until the motion settles, it is a placeholder `div` at its place's size, with the element itself `absolute` at the motion's bounds, relative to the place, inside `deferred(..).with_priority(1)`, so it is drawn over the rest (`Mode::Travelling`).
4. Settled, it is drawn in place and asks for no frame. Later moves of its place, from layout, are jumps: only the first place, or a place that changes while travelling, is a target.

## Traps

- GPUI's global id holds only the ids of ancestors that *have* ids. Tabs or rows with no id give every shared element of a key the same global id, so GPUI sees one element that moved: no new state, and no trip. The tests give each tab and the row and card an id, and the doc says so.
- `Mode` is `pub` in a private module, because it appears in the `Element` impl's associated type (E0446 otherwise).
- Bounds move through `impl Animate for Bounds<Pixels>`, whose stand-in is `InPixels<4>`: x, y, width, height.
- Reduced motion makes the transition `Instant`, so the element still measures one frame and then is in place.

Related: [[codebase/ways-in-and-out-on-an-element]], [[codebase/testing-the-adapter]], [[glossary]] (shared element, traveller).
