# Changelog

Each release of `ginga` and `ginga-gpui`, newest first. The two crates are released together, with the same version.

## Unreleased

- README: "Coming from Motion, SwiftUI or Compose", a table from their everyday tools to ginga, with recipes: steps, several stops, a stagger backward or from the centre, the scroll position, an icon that turns, writing motion down, reduced motion on purpose, and pause, seek, speed and reverse. The gallery's turn page.
- `Opened::height_glides(transition)`: the height an opened element opens to glides to its content's new height, keeping its speed, instead of jumping; under reduced motion it is taken at once.
- A `Sequence` step can be a `LookMotion` as well as a `Motion`: `then`, `with` and `at` take any `Retarget`. `Sequence::without_motion`, and in the adapter `ginga_gpui::sequence(now, cx)`, a sequence that under reduced motion puts each step at its target after its own delay.
- `Variant::set_in_order(name, order, ..)`, in the engine and the adapter: the order of the parent and its children for one change, so a menu opens parent first and closes children first. `set` keeps the order given to `order`.
- `Present::on_gone(|content, window, cx| ..)`: called with each content once it is gone, after the frame that finds it gone, instant ways out and reduced motion included. `Slot::take_gone`, in the engine: what was seen and let go, once each; content a wait dropped unseen is not gone. Content that waited while what was leaving came back is now dropped at once, unseen, instead of leaving.
- `LookState::inspect(cx)`: a declared look's engine motion and this frame's time, to write it down with `Trace::of`.
- `.glides(id, transition)` (`GlideExt`, `Glides`): an element travels to where layout puts it, painted, with layout untouched; scrolls, painted offsets and a glide around it are not moves.
- `.opened(id, progress)` (`OpenExt`, `Opened`): an element that takes `progress` of its own height, measured as it is laid out, as an accordion opens; what is past the open part is cut off and takes no clicks.
- `Spring::physical(stiffness, damping, mass)`: a spring given as Motion, react-spring and SwiftUI's `interpolatingSpring` give it, and `SpringError::NotASpring` for values that make none.
- Variants: a parent's name that its children follow one after another, before or after the parent (`Variant`, `Order`, `ginga_gpui::Variant`, `.leads(..)`, `.follows(..)`). `Transitions::delay` and `LookMotion::settles_at`.
- `Keyframes::behind(lag)`, in the engine and the adapter: keyframes that play behind others of the same loop, partway through a round from their start; a count of rounds waits on the first stop until the lag is over. `ginga_gpui::Keyframes::starting_at`, kept by `restart`.
- `LookExt::with_look`: a look applied as it is, with no motion and no element state, such as one mixed by a presence's progress.
- `shown.cover(element)` (`CoverExt`): what a presence draws takes no clicks and loses the keyboard focus while it leaves.
- A look's offset, and a dragged element's, is painted instead of laid out: it adds to the element's own place, `absolute` ones included, and the pointer finds the element where it is painted. Before, it replaced the element's `left` and `top`: an `absolute` element that gave its place as the offset, as 0.1.0's docs said, now gives it with `left` and `top`.
- `Presence::wanted` and `Presence::leaving`, in the engine and the adapter, and `ginga_gpui::Presence::sample(cx)`, read with no window.
- Reduced motion keeps a transition's delay: the value holds still until the delay is over, then is at its target at once. `Transition::without_motion` and `Transitions::without_motion`. Before, the delay was dropped with the motion. A variant's schedule and a shared element's trip are still dropped.
- `ginga_gpui::PresenceList`: a list presence on the view's clock. `frame(window, cx)` asks for the next frame while any item moves, and `sync`, `place_all` and `sample` take `cx`; under reduced motion every way, and every travel to a new place, is over at once after its delay. Breaking: `ginga_gpui::PresenceList` was the engine's, re-exported; that one is now `ginga_gpui::ginga::PresenceList`.
- A list presence's items at rest no longer glide when an item before them is let go and the layout moves them up by the room it took: they take their new place at once. Before, they were drawn a row too low and glided back up. An item on its way somewhere goes on from where it is drawn, and a place that stays put, as in a list held at its bottom, is no move.

## 0.1.0

The first release. The engine, `ginga`: springs, tweens, keyframes, coasting, colors, looks, presences and slots, derived values, sequences, trails and the trace. The GPUI adapter, `ginga-gpui`, for `gpui-pre =0.3.12`: the frame's clock and frame requests, reduced motion, a look declared on an element, ways in and out, shared elements, dragging, loops and islands.
