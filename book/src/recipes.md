# Recipes

Find what you want to do in the table, then copy the recipe. The engine recipes below are complete programs, imports included, and they run as tests on every change, so they compile against the current API. The GPUI recipes are short sketches, and each one links to a gallery page that does the same thing in full and compiles in CI.

## I want to…

| I want to | Use | Where |
| --- | --- | --- |
| move a value toward a target on every frame | `Motion::toward(target, transition, now)` | [recipe](#move-a-value-toward-a-target-on-every-frame), [A value that moves](engine/motion.md) |
| pick how it moves: springy, eased, at once | `Spring::SNAPPY`, `Spring::new(response).bounce(b)`, `Spring::physical(..)`, `Tween::new(duration, easing)`, `Transition::Instant` | [A value that moves](engine/motion.md) |
| start a motion later, or make a group arrive in turn | `.delay(d)` on any transition | [recipe](#make-a-group-arrive-one-after-another) |
| show and hide something, and draw it while it leaves | `Presence`; in GPUI `present(..)` | [recipe](#show-and-hide-a-toast), [Things that come and go](engine/presence.md), [Ways in and out](gpui/ways-in-and-out.md) |
| animate a list as items arrive, leave and reorder | `PresenceList::sync`, `place_all` | [recipe](#animate-a-list-as-items-arrive-and-leave), [Things that come and go](engine/presence.md) |
| swap one content for another in the same place | `Slot` with `Swap::Sync` or `Swap::Wait` | [Things that come and go](engine/presence.md) |
| loop, pulse, blink or go through several values | `Keyframes` with `.forever()`, `.times(n)`, `.mirrored()` | [recipe](#pulse-forever), [A value that moves](engine/motion.md#through-stops-and-again) |
| let a thrown value glide to a stop, inside limits or on a snap point | `Motion::coast`, `coast_within`, `coast_to_nearest`, `Trail` for the speed | [recipe](#throw-a-card-and-let-it-glide), [A value that moves](engine/motion.md#let-go-with-a-speed) |
| run several motions one after another, or overlapping | `Sequence` with `then`, `with`, `at` | [recipe](#chain-three-motions) |
| compute one value from another (a progress into a width or a color) | `Map::new(..)?.of(&motion)` | [A value that moves](engine/motion.md#values-from-values-and-steps-in-order) |
| run code once when a motion ends | `SettledOnce`, `settles_at()` | [A value that moves](engine/motion.md#values-from-values-and-steps-in-order) |
| move a type of my own | `impl Animatable` | [recipe](#move-a-type-of-your-own) |
| check a motion in a test, without a screen | `sample(now)`, `Trace::of(..)` | [recipe](#check-a-motion-in-a-test), [Motion as text](engine/trace.md) |
| animate a field of a GPUI view | `ginga_gpui::Motion::toward(target, transition, window, cx)` | [recipe](#gpui-a-panel-that-opens), [In a GPUI view](gpui/view.md) |
| declare on a GPUI element how it looks, and let it get there | `.look(id, look, transition)` | [recipe](#gpui-a-card-that-changes-its-look), [A look on an element](gpui/look.md) |
| change a GPUI element's look while hovered, pressed or focused | `.hover(..)`, `.pressed(..)`, `.focused(..)` | [A look for a moment](gpui/look-moment.md) |
| make a GPUI element come in and go out | `present(..)` with `.arrive_from(..)`, `.leave_to(..)` | [recipe](#gpui-a-toast-that-rises-in-and-fades-out), [Ways in and out](gpui/ways-in-and-out.md) |
| let a GPUI element travel to where layout puts it | `.glides(id, transition)` | [Gliding to a new place](gpui/glide.md) |
| open a GPUI element to its own height, as an accordion | `.opened(id, progress)` | [Opening to its own height](gpui/opened.md) |
| fly one thing between two places (a tab's mark, a row into a card) | `.shared(key, transition)` | [One thing in two places](gpui/shared.md) |
| make a menu's rows follow it, one after another | `Variant`, `.leads(..)`, `.follows(..)` | [A parent and its children](gpui/variants.md) |
| drag an element, throw it, and keep it inside limits | `.draggable(&state, drag)` | [An element to drag](gpui/drag.md) |
| keep a GPUI animation from redrawing heavy views | an `Island` | [Motion that redraws little](gpui/island.md) |
| follow reduced motion | automatic in `ginga-gpui`; `Transition::without_motion` in the engine | [In a GPUI view](gpui/view.md), [Coming from…](coming-from.md#reduced-motion-on-purpose) |
| scale or rotate a GPUI `div` | not possible: GPUI has no transform. Change its size or offset | [When to use ginga](when-to-use.md#do-not-use-it-for) |

## Engine recipes

Each one needs only `ginga = "0.1"`. Time is a `Duration` from any moment you choose; in an app it is the frame's time, and here it is written out so the numbers can be checked.

### Move a value toward a target on every frame

The whole loop for any framework: read the time, call `toward`, draw the value, and ask for another frame while it is not settled.

```rust
use std::time::Duration;
use ginga::{Motion, Spring};

struct Panel {
    open: bool,
    width: Motion<f32>,
}

impl Panel {
    /// Called once per frame with the frame's time. Returns the width to draw,
    /// and whether to ask the framework for another frame.
    fn frame(&mut self, now: Duration) -> (f32, bool) {
        let target = if self.open { 320.0 } else { 0.0 };
        let sample = self.width.toward(target, Spring::SNAPPY, now);
        (sample.value, !sample.settled)
    }
}

let ms = Duration::from_millis;
let mut panel = Panel { open: true, width: Motion::at(0.0) };

let (width, again) = panel.frame(ms(0));
assert_eq!(width, 0.0);
assert!(again); // on its way: ask for the next frame

let (width, again) = panel.frame(ms(100));
assert!(width > 0.0 && width < 320.0 && again);

let (width, again) = panel.frame(ms(2_000));
assert_eq!(width, 320.0); // settled exactly on its target
assert!(!again);          // stop asking for frames
```

### Show and hide a toast

The toast keeps its content while it leaves, so there is always something to draw the way out with.

```rust
use std::time::Duration;
use ginga::{Easing, Phase, Presence, Tween};

let ms = Duration::from_millis;
let fade = Tween::new(ms(200), Easing::Linear);
let mut toast = Presence::absent();

toast.show("Saved", fade, ms(0));
toast.hide(fade, ms(1_000));

// Each frame: draw what `frame` gives, with its progress as the opacity.
let shown = toast.frame(ms(1_100)).unwrap();
assert_eq!(*shown.content, "Saved");
assert_eq!(shown.phase, Phase::Exiting);
assert_eq!(shown.progress, 0.5);

assert!(toast.frame(ms(1_200)).is_none()); // gone: draw nothing
```

### Animate a list as items arrive and leave

Tell the list what it holds; it says what to draw, with each item's phase and progress, and where to draw it while it moves.

```rust
use std::time::Duration;
use ginga::{Easing, InOut, Phase, PresenceList, Spring, Tween};

let ms = Duration::from_millis;
let ways = InOut::new(Tween::new(ms(150), Easing::Out), Tween::new(ms(150), Easing::In));
const ROW: f32 = 40.0;

let mut rows = PresenceList::new();
rows.sync([(1, "Inbox"), (2, "Sent"), (3, "Drafts")], ways, ms(0));
rows.place_all(|index, _| index as f32 * ROW, Spring::SNAPPY, ms(0));

// "Sent" is removed: it leaves, and keeps its place until it is gone.
rows.sync([(1, "Inbox"), (3, "Drafts")], ways, ms(1_000));
rows.place_all(|index, _| index as f32 * ROW, Spring::SNAPPY, ms(1_000));

let drawn: Vec<_> = rows.frame(ms(1_050)).map(|row| (*row.content, row.phase)).collect();
assert_eq!(drawn, [("Inbox", Phase::Present), ("Sent", Phase::Exiting), ("Drafts", Phase::Present)]);

// Once it is gone, "Drafts" travels up to its new place.
let after: Vec<_> = rows.frame(ms(1_200)).map(|row| *row.content).collect();
assert_eq!(after, ["Inbox", "Drafts"]);
```

### Make a group arrive one after another

A stagger is a delay one step longer for each item, up to a cap.

```rust
use std::time::Duration;
use ginga::{Motion, Spring};

let ms = Duration::from_millis;
let mut cards = [Motion::at(0.0); 6];
for (n, card) in (0..).zip(&mut cards) {
    let delay = (ms(50) * n).min(ms(200));
    card.set_target(1.0, Spring::SMOOTH.delay(delay), ms(0));
}

let moving = cards.iter().filter(|card| card.sample(ms(120)).value > 0.0).count();
assert_eq!(moving, 3); // the ones delayed 0, 50 and 100 ms
```

### Pulse forever

Keyframes go through stops and can repeat. A loop that repeats forever is never settled, so whoever draws it keeps asking for frames.

```rust
use std::time::Duration;
use ginga::{Easing, Keyframes, Stop};

let ms = Duration::from_millis;
let pulse = Keyframes::new([
    Stop::new(ms(0), 1.0, Easing::Linear),
    Stop::new(ms(600), 0.4, Easing::InOutSine),
])?
.mirrored()
.forever();

assert_eq!(pulse.sample(ms(600)).value, 0.4);   // the end of the first round
assert_eq!(pulse.sample(ms(1_200)).value, 1.0); // mirrored: back again
assert_eq!(pulse.settles_at(), None);           // it never ends
# Ok::<(), ginga::KeyframesError>(())
```

### Throw a card and let it glide

A `Trail` measures the speed the card is let go with; `coast_within` glides it to a stop inside limits, and springs it back if the throw would pass one.

```rust
use std::time::Duration;
use ginga::{Coast, Motion, Trail};

let ms = Duration::from_millis;
let mut trail = Trail::default();
trail.note(ms(0), 0.0);
trail.note(ms(50), 40.0); // dragged 40 px in 50 ms
let speed = trail.speed(ms(50));
assert!(speed > 0.0); // px per second, toward the right

let mut card = Motion::at(40.0);
card.coast_within(speed, Coast::QUICK, 0.0, 300.0, ms(50));
let rest = card.settles_at().unwrap();
let end = card.sample(rest).value;
assert!(end > 40.0 && end <= 300.0); // it glides on, and stays inside
```

### Chain three motions

A `Sequence` sets each motion's target once, with a delay; nothing runs between frames.

```rust
use std::time::Duration;
use ginga::{Easing, Motion, Sequence, Tween};

let ms = Duration::from_millis;
let fade = Tween::new(ms(100), Easing::Linear);
let (mut title, mut body, mut button) = (Motion::at(0.0), Motion::at(0.0), Motion::at(0.0));

let mut entrance = Sequence::starting_at(ms(0));
entrance.then(&mut title, 1.0, fade);  // 0 to 100 ms
entrance.then(&mut body, 1.0, fade);   // 100 to 200 ms
entrance.with(&mut button, 1.0, fade); // with the body

assert_eq!(body.sample(ms(150)).value, 0.5);
assert_eq!(button.sample(ms(150)).value, 0.5);
assert_eq!(entrance.settles_at(), ms(200));
```

### Move a type of your own

A type moves if it can become a few numbers and come back. Each number keeps its own speed.

```rust
use std::time::Duration;
use ginga::{Animatable, Easing, Motion, Tween};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Camera {
    zoom: f32,
    angle: f32,
}

impl Animatable for Camera {
    type Channels = [f32; 2];
    fn to_channels(self) -> [f32; 2] {
        [self.zoom, self.angle]
    }
    fn from_channels([zoom, angle]: [f32; 2]) -> Self {
        Camera { zoom, angle }
    }
}

let ms = Duration::from_millis;
let mut camera = Motion::at(Camera { zoom: 1.0, angle: 0.0 });
camera.set_target(Camera { zoom: 2.0, angle: 90.0 }, Tween::new(ms(100), Easing::Linear), ms(0));
assert_eq!(camera.sample(ms(50)).value, Camera { zoom: 1.5, angle: 45.0 });
```

### Check a motion in a test

Read it at given times, or write it down with `Trace` and compare the text. The same times always give the same values.

```rust
use std::time::Duration;
use ginga::{Easing, Motion, Trace, Tween};

let ms = Duration::from_millis;
let mut x = Motion::at(0.0);
x.set_target(100.0, Tween::new(ms(200), Easing::Linear), ms(0));

assert_eq!(x.sample(ms(100)).value, 50.0);

let trace = Trace::of(&x, ms(0), ms(50)).to_string();
assert!(trace.contains("100.000  settled"));
println!("{trace}"); // a table of times and values, one row per 50 ms
```

## GPUI recipes

These need `ginga-gpui = "0.1"` and the GPUI it pins, `gpui-pre =0.3.12`. They are the lines that matter; each links to a gallery page that does the same in full and compiles in CI.

### GPUI: a panel that opens

Keep the motion in the view, next to the state it animates. `toward` reads the frame's clock, asks for frames while the value moves, and goes at once under reduced motion. Full example: [`gallery/toward.rs`](https://github.com/walteraandrade/ginga/blob/main/crates/ginga-gpui/examples/gallery/toward.rs).

```rust,ignore
use ginga_gpui::{Motion, Spring};
use gpui::{Context, IntoElement, Pixels, Render, Styled, Window, div, px};

struct Panel {
    open: bool,
    width: Motion<Pixels>,
}

impl Render for Panel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let target = if self.open { px(320.) } else { px(0.) };
        let width = self.width.toward(target, Spring::SNAPPY, window, cx);
        div().w(width)
    }
}
```

### GPUI: a card that changes its look

Say how the element should look; it gets there by itself, and the motion is kept with the element's id, so the view needs no field for it. Full example: [`gallery/look.rs`](https://github.com/walteraandrade/ginga/blob/main/crates/ginga-gpui/examples/gallery/look.rs).

```rust,ignore
use ginga_gpui::{Look, LookExt, Spring};
use gpui::{Styled, div, rgb};

let look = if self.open {
    Look::new().size(320., 200.).opacity(1.)
} else {
    Look::new().size(160., 100.).opacity(0.6)
};
div().bg(rgb(0x2a2f3a)).look("card", look, Spring::SNAPPY)
```

### GPUI: a toast that rises in and fades out

`present` draws a thing while it is there and keeps drawing it on its way out, with no field in the view. Full example: [`gallery/ways.rs`](https://github.com/walteraandrade/ginga/blob/main/crates/ginga-gpui/examples/gallery/ways.rs).

```rust,ignore
use std::time::Duration;
use ginga_gpui::{Easing, InOut, Look, Spring, Tween, present};
use gpui::{ParentElement, Styled, div};

let ways = InOut::new(Spring::SNAPPY, Tween::new(Duration::from_millis(200), Easing::In));
present("toast", self.saved.then_some("Saved"), ways, |text| {
    div().px_4().py_2().rounded_lg().child(*text)
})
.arrive_from(Look::new().offset(0., 24.).opacity(0.))
.leave_to(Look::new().opacity(0.))
```
