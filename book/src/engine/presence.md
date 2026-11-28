# Things that come and go

A `Presence` is something you show and hide. Hiding does not drop it: the presence keeps the content, as it was, until its way out is over, so there is always something to draw the exit with.

| Call | What it does |
| --- | --- |
| `Presence::absent()` | Nothing shown. |
| `presence.show(content, enter, now)` | Show it, or change what is shown. |
| `presence.hide(exit, now)` | Start the way out. |
| `presence.frame(now)` | Each frame: what to draw, with its phase and progress. |
| `Presence::leaving(content, exit, now)` | Something only seen leaving: on its way out from fully here. |
| `presence.wanted()` | The content that should be on screen; `None` from `hide` on, while it still leaves. |

```rust
use std::time::Duration;
use ginga::{Easing, Phase, Presence, Tween};

let ms = Duration::from_millis;
let fade = Tween::new(ms(100), Easing::Linear);
let mut toast = Presence::absent();

toast.show("Saved", fade, ms(0));
assert_eq!(toast.frame(ms(50)).unwrap().progress, 0.5);

toast.hide(fade, ms(1_000));
let leaving = toast.frame(ms(1_050)).unwrap();
assert_eq!((*leaving.content, leaving.phase), ("Saved", Phase::Exiting));

assert!(toast.frame(ms(1_100)).is_none()); // gone, and the content dropped
```

A `PresenceList` does the same for a list with keys. Tell it what the list holds with `sync`: a new key arrives, a missing key leaves and keeps its place until it is gone.

```rust
use std::time::Duration;
use ginga::{Easing, InOut, Phase, PresenceList, Tween};

let ms = Duration::from_millis;
let fade = Tween::new(ms(100), Easing::Linear);
let mut rows = PresenceList::new();

rows.sync([(1, "one"), (2, "two"), (3, "three")], InOut::new(fade, fade), ms(0));
rows.sync([(1, "one"), (3, "three")], InOut::new(fade, fade), ms(1_000));

let drawn: Vec<_> = rows.frame(ms(1_050)).map(|row| (*row.content, row.phase)).collect();
assert_eq!(
    drawn,
    [("one", Phase::Present), ("two", Phase::Exiting), ("three", Phase::Present)]
);
```

A `Slot` is one place whose content is swapped, such as a title: the old content leaves and the new one arrives. With `Swap::Sync` they cross; with `Swap::Wait` the new one arrives, unseen until then, only when the old one is gone. `frame` hands back what leaves (two at most, the oldest first) and then what arrives:

```rust
use std::time::Duration;
use ginga::{Easing, InOut, Slot, Swap, Tween};

let ms = Duration::from_millis;
let fade = Tween::new(ms(100), Easing::Linear);
let mut title = Slot::new(Swap::Wait);

title.show("Inbox", InOut::INSTANT, ms(0));
title.show("Sent", InOut::new(fade, fade), ms(0));
let [leaving, _, arriving] = title.frame(ms(50));
assert_eq!(leaving.unwrap().progress, 0.5); // "Inbox", halfway out
assert!(arriving.is_none()); // "Sent" waits for it, unseen

title.frame(ms(100));
assert_eq!(title.take_gone(), Some("Inbox")); // seen, and its way out is over
```

`take_gone` hands back, once each, what was seen and let go: its way out over, or cut short by a fourth swap while two leave. What waited and was never seen is not gone.

`Look::mix` draws a way in or out as a look: the look a thing arrives from, mixed into its own by its progress. A `Map` moves one value and a `LookMotion` moves a look in time; `mix` is a look at a progress that something else, such as a presence, moves.

```rust
use ginga::Look;

let from = Look::new().offset(0.0, 24.0).opacity(0.0);
assert_eq!(from.mix(Look::new(), 0.5), Look::new().offset(0.0, 12.0).opacity(0.5));
assert_eq!(from.mix(Look::new(), 1.0), Look::new()); // there: no offset, fully opaque
```

An item on its way out is still drawn, so it can still be clicked or focused unless you say otherwise: check for `Phase::Exiting`.

An item whose place changes can travel there. Tell the list where its items are laid out with `place_all`: a function that gets each item, with its index in the order `frame` gives them, and answers its place; the list answers, in each item's `offset`, how far from its place to draw it this frame, a distance that heads for zero. A reorder in the middle of a move turns around from where the item is, with its speed. Places are told apart to a tenth, so give them in pixels or another unit where a tenth is too small to see.

```rust
use std::time::Duration;
use ginga::{InOut, PresenceList, Spring};

let ms = Duration::from_millis;
let mut rows = PresenceList::new();

rows.sync([("a", ()), ("b", ())], InOut::INSTANT, ms(0));
rows.place_all(|index, _| index as f32 * 40.0, Spring::SNAPPY, ms(0));

rows.sync([("b", ()), ("a", ())], InOut::INSTANT, ms(1_000)); // "b" goes first
rows.place_all(|index, _| index as f32 * 40.0, Spring::SNAPPY, ms(1_000));

let b = rows.frame(ms(1_000)).next().unwrap();
assert_eq!((*b.key, b.offset), ("b", 40.0)); // drawn where it was, 40 below its new place
assert!(b.moving()); // ask for another frame
```

In a GPUI list of rows of one height, a row's place is its index times the height, known before layout. Each row is drawn `offset` away from it:

```rust,ignore
const ROW: f32 = 40.;

// `rows` is a `ginga_gpui::PresenceList`: it reads the frame's clock, asks
// for the next frame while a row moves, and applies reduced motion.
self.rows.place_all(|index, _| index as f32 * ROW, Spring::SNAPPY, cx);

let rows: Vec<_> = self
    .rows
    .frame(window, cx)
    .map(|row| {
        div()
            .h(px(ROW))
            .relative()
            .top(px(row.offset))
            .opacity(row.progress)
            .child(row.content.label.clone())
    })
    .collect();
div().flex().flex_col().children(rows)
```

Only the place moves: GPUI cannot scale a `div`, so a row whose size changes takes its new size at once.
