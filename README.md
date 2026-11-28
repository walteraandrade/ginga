# ginga

[![CI](https://github.com/walteraandrade/ginga/actions/workflows/ci.yml/badge.svg)](https://github.com/walteraandrade/ginga/actions/workflows/ci.yml)

Motion for Rust UIs, as a pure function of time.

A `Motion` is a value on its way to a target. You own it, you hand it the time, and it answers with the value at that time. Nothing ticks, nothing allocates while it runs, and the same times always give the same values. That makes motion something a test, a log or a coding agent can read as text, without a screen.

## What it is for

ginga has two goals, in this order.

**1. Performance.** Motion is drawn every frame, so a frame of it must cost next to nothing, and nothing at all once it is over.

- A value at a time is computed in closed form from where the motion started, never stepped from the last frame: a skipped or late frame changes nothing. A frame of a moving motion costs tens of nanoseconds, and a few once it is settled ([What it costs](#what-it-costs)).
- Running a motion never allocates; a test fails if it does.
- A motion says when it is settled, and a settled motion is exactly on its target, so whoever draws stops asking for frames.
- In GPUI, what a frame costs is what GPUI renders for it. An `Island` keeps a motion in a small view of its own, so the heavy views beside it, when they are cached, are not rendered again ([Motion that redraws little](#motion-that-redraws-little)).

**2. Easy to use for AI agents.** A coding agent cannot watch an animation. ginga is built so that it does not have to.

- Time is an argument, and the engine never reads a clock: an agent can ask for the value at any time and get the same answer every run.
- `Trace` writes a motion down as a table of times and values, in a stable format: a test asserts on it, and a change in motion shows up as a diff ([Motion as text](#motion-as-text)).
- The API is small and named in plain words (`toward`, `jump_to`, `settled`). This README shows it, and its engine examples run as tests; the GPUI ones are sketches, checked by the adapter's own tests.
- An invalid value does not build, and the error says what to use instead.
- The rules for a change are in [AGENTS.md](https://github.com/walteraandrade/ginga/blob/main/AGENTS.md), and the reasons and decisions behind them in [`brain/`](https://github.com/walteraandrade/ginga/blob/main/brain/index.md), so an agent that starts cold knows what was already decided.

When the two pull apart, performance wins.

## Status

Early work: until 1.0, a new minor version can change the API. Two crates, on crates.io:

| Crate | What it is | Depends on |
| --- | --- | --- |
| [`ginga`](https://github.com/walteraandrade/ginga/tree/main/crates/ginga) | The engine: springs, tweens, keyframes, coasting, colors, looks, presences and slots, derived values, sequences, trails and the trace. | nothing |
| [`ginga-gpui`](https://github.com/walteraandrade/ginga/tree/main/crates/ginga-gpui) | GPUI views: the frame's clock and frame requests, reduced motion, a look declared on an element, variants, ways in and out, shared elements, dragging, loops and islands. | `gpui-pre =0.3.12`, the GPUI that `gpui-kit` 0.9.1 builds on |

```toml
[dependencies]
ginga = "0.1"
ginga-gpui = "0.1" # for GPUI views
```

To follow `main` instead, depend on them from GitHub, both from the same revision:

```toml
[dependencies]
ginga = { git = "https://github.com/walteraandrade/ginga" }
ginga-gpui = { git = "https://github.com/walteraandrade/ginga" }
```

`ginga-gpui` pins one GPUI, `gpui-pre =0.3.12`: an app must use the same one, or the types do not match. It also re-exports the engine as `ginga_gpui::ginga`.

## A value that moves

| Call | What it does |
| --- | --- |
| `Motion::at(value)` | A value at rest. |
| `motion.toward(target, transition, now)` | Each frame: head for a target, get the value now. |
| `motion.set_target(target, transition, now)` | Change course at a moment you choose. |
| `motion.jump_to(value)` | Move with no motion. |
| `motion.sample(now)` | Read the value at any time, without changing anything. |

A transition is `Spring::SNAPPY`, `Spring::SMOOTH`, `Spring::BOUNCY`, a `Spring::new(response)`, a `Tween::new(duration, easing)` or `Transition::Instant`. A spring given the way Motion, react-spring or a design spec gives it, as a stiffness, a damping and a mass, is `Spring::physical(stiffness, damping, mass)?`: the same spring, at the same cost.

```rust
use std::time::Duration;
use ginga::{Motion, Spring};

let ms = Duration::from_millis;
// Motion's default spring: stiffness 100, damping 10, mass 1.
let spring = Spring::physical(100.0, 10.0, 1.0)?;
let mut x = Motion::at(0.0);
x.set_target(100.0, spring, ms(0));
assert!(x.sample(ms(400)).value > 100.0); // it passes its target, as a damping ratio of 0.5 does
assert!(Spring::physical(100.0, 0.0, 1.0).is_err()); // with no damping it would never stop
# Ok::<(), ginga::SpringError>(())
```

Anything `Animatable` can move: `f32`, tuples of up to four `f32`, `[f32; N]`, a `Color`, and your own types.

```rust
use std::time::Duration;
use ginga::{Motion, Spring};

let ms = Duration::from_millis;
let mut width = Motion::at(180.0);

// Frame at 0 ms: the panel should be 320 wide.
width.toward(320.0, Spring::SNAPPY, ms(0));

// Frame at 100 ms: on its way.
let sample = width.toward(320.0, Spring::SNAPPY, ms(100));
assert!(sample.value > 180.0 && sample.value < 320.0);
assert!(!sample.settled); // draw another frame

// Frame at 100 ms, the panel closes: the spring keeps its speed and turns around.
width.toward(180.0, Spring::SNAPPY, ms(100));
```

`now` is a `Duration` since any fixed moment you choose. The engine never reads a clock.

### One after another

Any transition can wait before it starts: `.delay(d)` on a spring, a tween or a `Transition`. Until then the value holds still where it is. To let a group arrive in turn, give each item a delay one step longer than the one before, up to a cap:

```rust
use std::time::Duration;
use ginga::{Motion, Spring};

let ms = Duration::from_millis;
let mut bars = [Motion::at(0.0); 5];

for (n, bar) in (0..).zip(&mut bars) {
    let delay = (ms(40) * n).min(ms(120)); // 40 ms apart, never more than 120 ms late
    bar.set_target(1.0, Spring::SMOOTH.delay(delay), ms(0));
}

let started: Vec<bool> = bars.iter().map(|bar| bar.sample(ms(60)).value > 0.0).collect();
assert_eq!(started, [true, true, false, false, false]);
```

The same works for things that come and go: `rows[n].show(row, fade.delay(step * n), now)`.

### Through stops, and again

`Keyframes` go through several stops, each at its time with its own easing, and can repeat: `.times(n)` or `.forever()`, `.mirrored()` to go back and forth, `.pause(d)` between rounds, `.starting_at(t)` to start later, `.behind(lag)` to play a lag behind copies of the same loop, partway through a round from the start. Like a motion, they are read at any time with `sample(now)`; `rest()` is where they rest once played.

```rust
use std::time::Duration;
use ginga::{Easing, Keyframes, Stop};

let ms = Duration::from_millis;
let breath = Keyframes::new([
    Stop::new(ms(0), 0.4, Easing::Linear),
    Stop::new(ms(450), 1.0, Easing::InOutSine), // in
    Stop::new(ms(600), 1.0, Easing::Linear),    // and hold
])?
.times(4)
.mirrored()
.pause(ms(100));

assert_eq!(breath.sample(ms(600)).value, 1.0); // the first round's end
assert_eq!(breath.sample(ms(1_300)).value, 0.4); // back out, the second round
assert_eq!(breath.settles_at(), Some(ms(2_700)));
# Ok::<(), ginga::KeyframesError>(())
```

Keyframes that repeat forever are never settled, and their `settles_at` is `None`: whoever draws them asks for frames as long as they are drawn. Stops that are not in order, or a first stop not at zero, do not build. The stops are an array, so keyframes are plain data and reading them never allocates.

### Let go with a speed

`motion.coast(velocity, coast, now)` lets a value go with a speed, in units per second: it keeps that speed at first and slows to a stop by the same share every instant, so it glides and never stops with a jolt. A `Coast` is `Coast::QUICK` (a time constant of 0.1 s), `Coast::SLOW` (0.5 s) or `Coast::new(time_constant)`, and the value rests a time constant's worth of its first speed on: `coast.rest(from, velocity)`. `coast_within(velocity, coast, min, max, now)` comes to rest inside limits, and a value that would pass one, or is past it already, moves with the coast's spring to it instead; `coast_to_nearest(velocity, coast, &points, now)` comes to rest on the nearest snap point the same way. Each keeps the speed it was let go with, and is settled exactly where it rests. `.spring(Spring::BOUNCY)` on a coast makes a wall bouncier. A `Trail` gives the speed a value is let go with: `trail.note(now, value)` as it is moved by hand, then `trail.speed(now)`, over the last 100 ms, and zero if it was held still for 50 ms.

```rust
use std::time::Duration;
use ginga::{Coast, Motion};

let ms = Duration::from_millis;
let mut card = Motion::at(0.0);
card.coast(1_000.0, Coast::QUICK, ms(0)); // thrown at 1,000 px/s
assert_eq!(card.target(), 100.0);         // rests 100 px on
assert!((card.sample(ms(100)).value - 63.2).abs() < 0.1); // 1 - 1/e of the way after 0.1 s

let mut knob = Motion::at(0.0);
knob.coast_within(3_000.0, Coast::QUICK, 0.0, 200.0, ms(0)); // it would pass 200
assert_eq!(knob.sample(knob.settles_at().unwrap()).value, 200.0);
```

### Values from values, and steps in order

A `Map` derives a value from a number: `Map::new(from_start, from_end, to_start, to_end)?` maps one range onto a range of anything that moves, such as a progress into a width, or a width into a color. Past an end it goes on in the same line, as a spring's overshoot should; `.clamped()` holds it to the ends. `map.at(number)` maps one number, and `map.of(&motion)` reads it from a motion each frame: it samples and writes down as a `Trace` like a motion, and reading it allocates nothing. A value that *follows* another with a spring needs nothing new: hand the leader's value to `follower.toward(..)` on every frame.

A `Sequence` starts several motions as one: `then` starts a step when the one before ends, `with` together with it, and `at(time, ..)` at a set time, such as 120 ms before the one before ends. Each step sets its motion's target once, with a delay; nothing runs between frames. Each call returns the `Step`'s start and end, and `settles_at()` says when the last one ends. Add every step when the sequence starts, and give each motion one step. `SettledOnce` says `true` once, on the first frame at or after a motion's or a sequence's end: for code that runs once at the end. A motion's end moves with a new target, so the old end never comes and the new one is reported instead; a target taken at once, as under reduced motion, ends when it is set, or when its delay ends. A sequence's end is the time it was given and does not move: to wait for a new end, start another sequence. In a GPUI view, a sequence moves engine motions (`ginga::Motion`, also reachable as `ginga_gpui::ginga::Motion`) read with the view's `ginga_gpui::Clock`, as any motion the view's own logic keeps. Use one `SettledOnce` for each thing you wait for: two ends at the same time are one end to it.

```rust
use std::time::Duration;
use ginga::{Color, Easing, Map, Motion, Sequence, SettledOnce, Tween};

let ms = Duration::from_millis;
let fade = Tween::new(ms(100), Easing::Linear);

// A width from a progress, and a color from the width, held to its ends.
let mut progress = Motion::at(0.0);
progress.set_target(1.0, fade, ms(0));
let width = Map::new(0.0, 1.0, 40.0, 240.0)?;
let fill = Map::new(40.0, 240.0, Color::hex(0xe5484d), Color::hex(0x30a46c))?.clamped();
let now = width.of(&progress).sample(ms(50)).value;
assert_eq!(now, 140.0);
assert_eq!(fill.at(300.0), Color::hex(0x30a46c));

// Three steps: one after another, the last overlapping by 40 ms.
let (mut title, mut body, mut button) = (Motion::at(0.0), Motion::at(0.0), Motion::at(0.0));
let mut entrance = Sequence::starting_at(ms(0));
entrance.then(&mut title, 1.0, fade);
let second = entrance.then(&mut body, 1.0, fade);
let third = entrance.at(second.ends.saturating_sub(ms(40)), &mut button, 1.0, fade);
assert_eq!(third.to_string(), "160 ms to 260 ms");

let mut done = SettledOnce::default();
assert!(!done.check(Some(entrance.settles_at()), ms(250)));
assert!(done.check(Some(entrance.settles_at()), ms(266)));
# Ok::<(), ginga::MapError>(())
```

## Things that come and go

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

## Motion as text

`Trace::of(&motion, from, step)` reads a motion at regular times until it settles. Printing it gives one row per step:

```text
ms           value
0            0.000
50          25.000
100         50.000
150         75.000
200        100.000  settled
```

The format is stable: put it in a test and a change in motion shows up as a diff.

A `LookMotion` (below) is written down the same way, with a column for each value its look names. `Trace::of` takes either: both are `Traceable`, and nothing else is.

```text
ms               x           y     opacity
0            0.000       0.000       0.000
50          20.000      10.000       0.500
100         40.000      20.000       1.000  settled
```

## In a GPUI view

Keep the motion next to the state it animates:

```rust,ignore
struct Panel {
    open: bool,
    width: ginga_gpui::Motion<Pixels>,
}

impl Render for Panel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let target = if self.open { px(320.) } else { px(0.) };
        let width = self.width.toward(target, Spring::SNAPPY, window, cx);
        div().w(width)
    }
}
```

`toward` reads the frame's clock, asks GPUI for another frame while the value moves, and moves at once when the desktop asks for reduced motion. A delay is timing, not motion, so it is kept: under reduced motion a delayed value holds still until its delay is over, then is at its target at once (`Transition::without_motion`, and `Transitions::without_motion` for a look). A `ginga_gpui::Motion` moves `f32` and GPUI's `Pixels`, `Point<Pixels>`, `Size<Pixels>`, `Bounds<Pixels>`, `Rgba` and `Hsla`; another GPUI type gets an `Animate` impl. `motion.inspect(cx)` hands back the engine's motion and this frame's time, to write it down with `Trace::of`. `ginga_gpui::Presence` is a `Presence` read on the frame's clock: `show(content, enter, cx)`, `hide(exit, cx)` and `frame(window, cx)`. `shown.cover(element)` (`CoverExt`) draws what `frame` gave covered while it leaves: it takes no clicks, and the keyboard focus is taken from inside it, as `present` does. An event handler with no window reads `wanted()`, the content that should be on screen, and `sample(cx)`, what `frame` would hand back; `Presence::leaving(content, exit, cx)` makes a thing that is only seen leaving. `ginga_gpui::PresenceList` is a `PresenceList` on the same clock: `sync(items, in_out, cx)`, `place_all(place, transition, cx)` and `frame(window, cx)`, which asks for the next frame while any item arrives, leaves or travels, and applies reduced motion to each way and each travel to a new place; `sample(cx)` is what `frame` would hand back, for an event handler with no window.

When the logic of a motion lives in a crate that knows nothing about GPUI, that crate keeps a `ginga::Motion` and takes the time as an argument; the view supplies it with `ginga_gpui::Clock`.

## A look on an element

Or declare on the element how it should look, and it gets there by itself. A `Look` names any of an offset from the element's own place, an opacity, a width and height, a corner radius and colors; a value it does not name stays the element's own style. The offset moves only what is painted and where the pointer finds the element: the layout does not see it, so an `absolute` element keeps its own place and slides from there.

```rust,ignore
let look = if self.open {
    Look::new().size(320., 200.).opacity(1.)
} else {
    Look::new().size(160., 100.).opacity(0.6)
};
div().bg(rgb(0x2a2f3a)).look("card", look, Spring::SNAPPY)
```

A look can name colors too: `.background(..)`, `.text_color(..)` and `.border_color(..)` take a `Color`: `Color::hex(0x3b6ef5)`, `Color::new(r, g, b, alpha)`, `Color::TRANSPARENT`, any of them `.with_alpha(a)`, or one taken from GPUI with `ginga_gpui::color(rgb(0x3b6ef5))`. Each value is held to `0.0..=1.0`. GPUI's `Rgba` and `Hsla` can also move in a `ginga_gpui::Motion`. Colors mix in linear light with the alpha premultiplied: red on its way to blue passes through purple, never green, and a color fading in from transparent never darkens.

```rust
use std::time::Duration;
use ginga::{Color, Easing, Look, LookMotion, Tween};

let ms = Duration::from_millis;
let mut button = LookMotion::at(Look::new().background(Color::hex(0xff0000)));
button.set_target(Look::new().background(Color::hex(0x0000ff)), Tween::new(ms(100), Easing::Linear), ms(0));

let middle = button.sample(ms(50)).value.background.unwrap();
assert_eq!(middle.green, 0.0);       // never green
assert!(middle.red > 0.7);           // a bright purple, as light mixes

// A fill that fades in from nothing keeps its own color, only less opaque.
let red = Color::hex(0xe5484d);
let mut fill = LookMotion::at(Look::new().background(Color::TRANSPARENT));
fill.set_target(Look::new().background(red), Tween::new(ms(100), Easing::Linear), ms(0));
let half = fill.sample(ms(50)).value.background.unwrap();
assert!((half.red - red.red).abs() < 1e-4 && (half.alpha - 0.5).abs() < 1e-6);
```

When the look changes, the element moves from where it is, and a spring keeps its speed. The motion is kept with the element's id, so the view needs no field for it. On its first frame the element takes its look at once; `.from(look)` makes it arrive from another one. An element that must keep its motion while it is not drawn takes a `LookState` the view owns: `.look_with(&self.card, look, transitions)`. Like `toward`, it asks for frames only while it moves and, under reduced motion, takes each value at once after that value's delay. To write a declared look down, keep it in a `LookState`: `self.card.inspect(cx)` hands back its engine motion and this frame's time for `Trace::of`, as `motion.inspect(cx)` does for a `ginga_gpui::Motion`, and is `None` until an element has drawn with it. A look the view already has, such as one mixed by a presence's progress (`from.mix(Look::new(), shown.progress)`), is applied as it is with `.with_look(look)`: the same mapping, with no motion and no element state.

In the engine the same thing is a `LookMotion`, which takes the time as an argument like a `Motion`.

Each value of a look can move on its own clock. A single `Transition`, `Spring` or `Tween` moves the whole look; a `Transitions` gives values their own, and what it leaves out comes from the defaults under it:

| Value | With nothing given |
| --- | --- |
| offset, width, height, radius | `Spring::SNAPPY` |
| opacity, background, text and border color | a 150 ms tween, `Easing::Out` |

A value's own transition wins over the whole look's (`.all(..)`), which wins over the defaults it is laid over with `.or(defaults)`, which win over the table. The layers hold whatever order the calls come in. A view keeps its defaults in a field or a function and lays each look's over them. An app sets its own once for every declared look with `ginga_gpui::set_default_look_transitions(transitions, cx)`; a `ginga_gpui::Motion` takes the transition it is given and nothing else.

```rust
use std::time::Duration;
use ginga::{Easing, Look, LookMotion, Spring, Transitions, Tween};

let ms = Duration::from_millis;
let mut card = LookMotion::at(Look::new().offset(0.0, 120.0).opacity(0.0));
let shown = Look::new().offset(0.0, 0.0).opacity(1.0);
// It springs to its place while it fades in a fixed 100 ms.
let transitions = Transitions::new()
    .offset(Spring::BOUNCY)
    .opacity(Tween::new(ms(100), Easing::Linear));
card.set_target(shown, transitions, ms(0));
let at_100 = card.sample(ms(100)).value;
assert_eq!(at_100.opacity, Some(1.0));        // the fade is over
assert!(at_100.offset.unwrap().1 > 0.0);       // the spring is still on its way
```

### A look for a moment

An element can take another look while the pointer is over it, while it is pressed, and while it has the keyboard focus. Each one names only what changes, and is laid over the element's own look with `Look::or`. Pressed wins over hover, and hover wins over focused. When the moment ends, the element goes back the same way, from where it is: a pointer that crosses quickly turns the motion around without a jump. The element's own handlers still run.

```rust,ignore
div()
    .id("save")
    .track_focus(&self.focus)
    .on_click(cx.listener(|this, _, _, cx| this.save(cx)))
    // A value moves in a moment only when the element's own look names it too:
    // without the transparent border, the ring would snap on and off.
    .look("save", Look::new().size(120., 40.).background(fill).border_color(Color::TRANSPARENT), Spring::SNAPPY)
    .hover(Look::new().size(128., 44.))
    .pressed(Look::new().offset(0., 2.))
    .focused(&self.focus, Look::new().border_color(ring))
```

`Look::or` lays any two looks over each other:

```rust
use ginga::Look;

let rest = Look::new().size(120.0, 40.0).opacity(0.8);
assert_eq!(Look::new().opacity(1.0).or(rest), Look::new().size(120.0, 40.0).opacity(1.0));
```

### Ways in and out

`present` draws a thing while it is there and keeps drawing it on its way out, with no field in the view: say each frame whether it is there, and how it comes and goes. It arrives from `arrive_from` and leaves to `leave_to`, each an offset and an opacity; wanted again on its way out, it turns around from where it is. While it leaves it takes no clicks, and the focus leaves it. Other content in its place swaps, crossing (`Swap::Sync`) or one after the other (`Swap::Wait`).

```rust,ignore
let ways = InOut::new(Spring::SNAPPY, Tween::new(ms(200), Easing::In));
present("toast", self.saved.then_some("Saved"), ways, |text| {
    div().px_4().py_2().rounded_lg().bg(green).child(*text)
})
.arrive_from(Look::new().offset(0., 24.).opacity(0.))
.leave_to(Look::new().opacity(0.))
```

Content that is there in the first frame arrives too. Without `arrive_from` it appears at once, and without `leave_to` it is gone at once. `.on_gone(|content, window, cx| ..)` is called with each content once it is gone, after the frame that finds it gone: to drop what the view kept for it, or to move the focus. An instant way out, or one that reduced motion makes instant, is over on the frame that starts it; content wanted again on its way out is not gone.

### Gliding to a new place

`.glides(id, transition)` makes an element travel to where layout puts it, instead of jumping there: a card in a wrapping row when one before it goes, a panel's content when a sidebar opens, a row when the one above it grows. It is drawn where it was in the frame of the move and heads for its new place, keeping its speed if the place changes again on the way.

```rust,ignore
div().flex().flex_wrap().children(self.cards.iter().map(|card| {
    div().w(px(120.)).h(px(72.)).child(card.title.clone()).glides(("card", card.id), Spring::SNAPPY)
}))
```

Only what is painted moves, and where the pointer finds it: layout is not touched, so nothing is laid out again while it glides and the elements around it take their places at once. A frame of a glide costs one sample of its motion and a frame request. A move is a change of its place in the window's layout: a scroll, an offset that a look or a drag paints it with, and a glide of an element around it are not moves, so an element that glides inside another that glides moves with it and glides only by how its place inside changes. A change of the window's size is not a move either. Its size is not moved: it takes its new size at once. Its first frame is in place, and under reduced motion it is in its new place at once, after its delay. The rows of GPUI's `list` and `uniform_list` are each laid out on their own: use `PresenceList::place_all` there. A cached view, a `deferred` element and an `anchored` one are laid out or drawn on their own too, so a glide inside one does not see it move: put the glide on the cached view, or around it. `.glides(..)` goes on any element, a look or another wrapper of ginga's included.

### Opening to its own height

`.opened(id, progress)` gives an element `progress` of its own height, as an accordion opens: at `0.0` it takes no room, at `1.0` it is the element as it is, and part of the way open the rest is cut off and takes no clicks. Its own height is measured as it is laid out, at the width it has, so wrapped text opens to the height it wraps to. The progress comes from whatever moves it, such as a presence:

```rust,ignore
let answer = self.answer.frame(window, cx).map(|shown| {
    div().p_2().child(*shown.content).opened("answer", shown.progress)
});
div().child(question).children(answer)
```

It is held in a box that takes its place in the parent: its margins, its width, its share of a flex row and its position are the box's, so it is laid out the same at every progress, and its margins stay around it, closed too. A progress past `0.0..=1.0`, such as a bouncy spring's, is held to it. It keeps the height it measured last under its id: the first frame drawn part of the way open, and a frame where the content's height changes part of the way open, draw the height before and ask for the next frame. A height given as a share of the parent's, such as `h_full`, has no height of its own to open to. Part of the way open, only what is above or below the box is cut, so a shadow at its sides stays; closed, nothing of it is drawn, so it takes no clicks, no focus and no keys, and it asks for no frame.

What it costs is layout and rendering: while it opens, its parent is laid out again on each frame, and cached views in it or moved by it render again on each frame, since a cached view is reused only at the same place, size and cut. A cached view beside its parent does not. Fully open and still, it asks for nothing. [The measurement](https://github.com/walteraandrade/ginga/blob/main/docs/measurements/2026-11-28-accordion/README.md).

### One thing in two places

`.shared(key, transition)` says that elements in different places are one thing. When one of a key appears as another of it goes, it does not appear in its place: it travels there from where the other was, its place and its size changing together. A tab's mark slides to the next tab; a row opens into a card.

```rust,ignore
// In each tab: only the selected one draws the mark.
div().id(tab).children((tab == selected).then(|| {
    div().h(px(2.)).w_full().bg(accent).shared("tab-mark", Spring::SNAPPY)
}))
```

While it travels it is drawn above the rest, and its destination stays empty at its size, with its margins and its position, so nothing around it moves. A key names one thing in a window, so two tab bars need a key each. It comes only from an element of its key drawn in the frame before, in the same window; the frame it appears in measures where it goes and draws nothing, so a trip starts with one blank frame. Under reduced motion it is in its destination at once. Its content is laid out again at each size it passes, since GPUI cannot scale an element. Two elements of one key must have different ids around them, such as tabs that each have an id: to GPUI, two with the same ids around them are one element, which takes its new place at once.

### A parent and its children

A `Variant` is a name a parent shows, such as `Closed` or `Open`, that its children follow one after another. The view keeps it and hands it by hand to the parent, with `.leads(&menu)`, and to each child, with `.follows(&menu, n)`. Each element picks its own look for the name. When the name changes, child `n` starts one step after child `n - 1`, and the parent goes before them (`Order::ParentFirst`), after them (`Order::ChildrenFirst`) or with the first one (`Order::Together`, the default).

```rust,ignore
// In the view's state:
let menu = Variant::new(Menu::Closed).stagger(ms(40)).order(Order::ParentFirst);

// While rendering:
let open = self.menu.name() == Menu::Open;
div()
    .look("menu", if open { Look::new().height(176.) } else { Look::new().height(32.) }, Spring::SNAPPY)
    .leads(&self.menu)
    .children((0..4).map(|n| {
        let row = if open { Look::new().offset(0., 0.).opacity(1.) } else { Look::new().offset(-12., 0.).opacity(0.) };
        div().look(("row", n), row, Spring::SNAPPY).follows(&self.menu, n)
    }))

// On a click: every view that drew the menu renders again, cached ones too.
self.menu.set(Menu::Open, cx);
```

Only a change of look that comes with a new name follows the schedule: a hover, or a change of the app's own, goes at once. The schedule's delay is added to any a transition has of its own. A parent that goes after its children waits for the ones laid out inside it, cached views of their own included; children drawn beside it do not hold it. Times count from the change of name, so a child drawn a frame late keeps its place. Under reduced motion the schedule is dropped: every element changes at once, after only its own transition's delay.

In the engine, a `Variant` is the schedule alone. Each element asks for its delay and says when it will be settled, so the side that waits knows how long:

```rust
use std::time::Duration;
use ginga::{Easing, Look, LookMotion, Order, Transitions, Tween, Variant};

let ms = Duration::from_millis;
let fade = Transitions::from(Tween::new(ms(100), Easing::Linear));
let mut menu = Variant::new(false).stagger(ms(50)).order(Order::ParentFirst);
let (mut panel, mut rows) = (LookMotion::at(Look::new().opacity(0.0)), [LookMotion::at(Look::new().opacity(0.0)); 3]);

menu.set(true, ms(0));
panel.set_target(Look::new().opacity(1.0), fade.delay(menu.parent_delay(ms(0))), ms(0));
menu.parent_ends(panel.settles_at().unwrap());
for (n, row) in (0..).zip(&mut rows) {
    row.set_target(Look::new().opacity(1.0), fade.delay(menu.child_delay(n, ms(0))), ms(0));
}
// The panel fades in over 100 ms; then the rows, 50 ms apart.
assert_eq!(rows.map(|row| row.sample(ms(175)).value.opacity.unwrap()), [0.75, 0.25, 0.0]);
```

## An element to drag

`.draggable(&state, drag)` lets the pointer drag any styled element. While it is held it is exactly under the pointer, with no spring between them; let go, it keeps the pointer's speed and coasts. A `Drag` says how: `.along(Axis::Horizontal)`, `.within(min, max)` to spring back inside limits, `.snap_to(points)` to come to rest on the nearest one, and `.coast(Coast::SLOW)`. The view owns a `DragState`, `DragState::default()` or `DragState::at(offset)`: `offset()` is where the element was last drawn, `rest()` where it comes to rest, `is_held()` whether the pointer holds it, and `throw(velocity, &drag, cx)` lets it go as a pointer would, for a key or a button.

```rust,ignore
// The view owns `knob: DragState`. A slider's knob with five stops:
let stops = (0..5).map(|stop| point(px(93. * stop as f32), px(0.)));
div().size(px(28.)).draggable(&self.knob, Drag::new().along(Axis::Horizontal).snap_to(stops))
```

Like a look's, the offset moves what is painted and where the pointer finds the element, from wherever it is laid out. With snap points, the limits are left out: put the points inside them. Only the topmost element under the pointer is taken, and a move with the button up lets it go, as when a release was lost outside the window. Under reduced motion a thrown element is at once where it comes to rest.

## A loop in a view

`ginga_gpui::Keyframes` are keyframes a view owns. They start on the first frame that draws them, or `starting_at(d)` after it, `restart(cx)` plays them again, `behind(lag)` plays them `lag` behind copies without it, partway through a round from the first frame, so staggered dots share one loop, and they rest where they end under reduced motion (`rest()`: a loop on its first stop). A slow loop need not be drawn at the display's rate: `.at_most_every(d)` asks for the next frame no sooner than `d` after this one. It holds back only its own requests, so draw the loop in a view of its own, such as an `Island`, or another motion in the view draws it on every frame anyway. The cap is per set of keyframes, not per view, as GPUI's `Animation::with_max_fps` is per element: two capped sets in one view that started apart can draw the view up to once each per `d`, so values that should share a rate share one set.

```rust,ignore
// Built once:
let breath = Keyframes::new([
    Stop::new(ms(0), 0.4, Easing::Linear),
    Stop::new(ms(600), 1.0, Easing::InOutSine),
])?
.forever()
.mirrored()
.at_most_every(ms(30)); // about 30 frames a second at 60 Hz

// While rendering:
let opacity = self.breath.frame(window, cx);
mark().opacity(opacity)
```

## Motion that redraws little

GPUI renders again the view that asks for an animation frame, and every view above it, on each frame of a motion. A view beside them is reused only when it is embedded with `.cached(style)`. So a motion is cheap when what moves is drawn by a small view of its own, the heavy views sit beside it and are cached, and the views above it are cheap.

`ginga_gpui::Island` is that small view: a state and a function that draws it.

```rust,ignore
let dot = cx.new(|_| {
    Island::new(Dot::default(), |dot, window, cx| {
        let x = dot.x.toward(dot.target, Spring::SNAPPY, window, cx);
        div().absolute().left(x).size_2().into_any_element()
    })
});

// In the parent: the heavy view is cached, the island sits beside it.
div()
    .relative()
    .child(self.document.clone().cached(StyleRefinement::default().size_full()))
    .child(dot.clone())
```

Measured on its own, with a heavy view of 2,400 boxes and a dot that never stops, a frame cost the main thread about 19 to 22 ms with the island and 48 to 50 ms without it, and the heavy view did not render again while the dot moved: [the measurement](https://github.com/walteraandrade/ginga/blob/main/docs/measurements/2026-11-27-island-alone/README.md) (software rendering, no GPU). In an app it depends on what else asks for frames: in Desafinado a frame that skipped the heavy view cost about 40 % less, but only one frame in four skipped it: [that measurement](https://github.com/walteraandrade/ginga/blob/main/docs/measurements/2026-11-26-ai-caret-island/README.md). With the requests tray's marks in an island too, most frames of a Review skip it, and the Review draws about 17 % less: [the next one](https://github.com/walteraandrade/ginga/blob/main/docs/measurements/2026-11-27-tray-marks/README.md). Two loops in two views draw on their own beats, so the window can draw more frames than either asks for.

## Watch it move

The gallery is an example app with a page for each thing ginga does: a value toward a target with a spring and a tween, a presence, a list whose items come and go, an accordion, a look, colors, a transition for each value, keyframes, derived values and sequences, a look while hovered, pressed or focused, ways in and out, a shared element, cards that glide to their new places, a drag, and an island. Each page is driven by a few lines that can be copied. It runs on Linux.

```sh
cargo run -p ginga-gpui --example gallery                  # open it
crates/ginga-gpui/examples/gallery/record presence         # record a page
```

Recording runs the page in a virtual display and leaves its frames and a contact sheet in `target/gallery/<page>/`, so a pull request can show a motion beside its trace. It runs on Linux and needs `xvfb-run`, `ffmpeg`, a Vulkan driver (Mesa's, without a GPU) and ImageMagick.

## What it costs

`cargo run --release --example bench`, best of 31 interleaved rounds, on a shared cloud Xeon at 2.1 GHz (2026-11-27). A frame costs one thing while the value moves and another once it is settled, so the two are timed apart:

| One frame of | Moving | Settled |
| --- | --- | --- |
| a value at rest | | 4 ns |
| a spring, 1 channel | 11 ns | 4 ns |
| a spring, 4 channels | 13 ns | 6 ns |
| a bouncy spring, 1 channel | 32 ns | 4 ns |
| a bouncy spring, 4 channels | 22 ns | 6 ns |
| a tween, 2 channels | 14 to 39 ns | 5 ns |
| a spring with 4 channels and a new target every frame | 279 ns | |

Each number includes about 1 ns that the timing loop costs on its own; the bench prints it as its first row.

The bouncy spring with one channel reports slower than with four although it runs fewer instructions: valgrind counts 237 per frame for one channel and 261 for four. It is an effect of code layout, not of the work, and which case gets the slow layout changes with unrelated edits. [Issue #6](https://github.com/walteraandrade/ginga/issues/6) is closed on that explanation, and its measure is now instructions per frame: a few nanoseconds between bench cases are not evidence by themselves. See [the note](https://github.com/walteraandrade/ginga/blob/main/brain/codebase/bouncy-spring-bench-and-code-layout.md).

Reading and retargeting a motion never allocates; [`tests/no_alloc.rs`](https://github.com/walteraandrade/ginga/blob/main/crates/ginga/tests/no_alloc.rs) fails if it does.

The engine is not where a frame's time goes. Measured in a real app, a frame of motion cost the main thread about 1.2 ms, and two engines cost the same: [the measurement](https://github.com/walteraandrade/ginga/blob/main/docs/measurements/2026-11-26-ai-caret/README.md). What a motion costs is what GPUI renders for it.

## What it does not do yet

- Animate a change of size that layout makes: an item whose content grows takes its new size at once. A size that a look names does move.
- Gestures beyond a drag with one pointer: pinch, rotate, a drag on a touchpad's scroll.

Limits that come from GPUI and not from this library: a `div` has no transform, so moving a box means changing its position or size and laying out again; and there is no way to redraw one element without rendering its view.

## Inspirations

ginga takes ideas that work elsewhere and fits them to its two goals ([What it is for](#what-it-is-for)):

| Inspiration | The idea taken | Where it is in ginga |
| --- | --- | --- |
| [Motion](https://motion.dev) (formerly Framer Motion) | You say the target and the library takes care of the way there. The value and its speed live outside the drawing (`MotionValue`). What leaves stays on screen until its way out is over (`AnimatePresence`). | `Motion::toward`, `Presence`, `PresenceList` |
| Motion's `animate`, `whileHover`, `layoutId`, variants and drag | A look declared on the element; a look for a moment; one thing in two places; a parent's name that its children follow, one after another; a drag that is thrown and springs back. | `.look(..)`, `.hover(..)`, `.pressed(..)`, `.focused(..)`, `.shared(..)`, `.leads(..)` and `.follows(..)`, `.draggable(..)` |
| Motion's `layout`, gpui-motion's `with_flip` and gpui-ce-motion's `LayoutGroup` | An element whose place layout changes travels there, painted, with layout untouched. | `.glides(..)` |
| [lilt](https://github.com/ejjonny/lilt), for iced | Time is an argument. There is no tick. | `toward(target, transition, now)`, `sample(now)` |
| [SwiftUI](https://developer.apple.com/xcode/swiftui/) | A spring described first by its duration and its bounce, rather than by mass and stiffness. A spring that is interrupted keeps its speed. | `Spring::new(response)`, `.bounce(..)` |
| Motion, react-spring and SwiftUI's `interpolatingSpring` | A spring given as a stiffness, a damping and a mass, as specs and other code give it. | `Spring::physical(..)` |
| SwiftUI and [Jetpack Compose](https://developer.android.com/compose) | A type moves if it can become a row of numbers and come back. Each number has its own speed. | `Animatable`, `to_channels`, `from_channels` |
| [Neovide](https://neovide.dev) and gpui-motion | A spring in closed form: the value comes straight from the formula, never by stepping frame to frame. | `crates/ginga/src/spring.rs` |
| `gpui-base::motion` | At rest costs nothing. An invalid value stops at the constructor. The library gives a phase and a progress; the app decides what they look like. | `Sample::settled`, `SpringError`, `Phase::{Entering, Present, Exiting}` |
| gpui-ce-motion | The state has an owner in plain sight: it lives in your view. Its `Target`, a closed list of properties, is the model for a look. | a `Motion` is a field of your view, and the engine keeps no state of its own; `Look` |
| [react-spring](https://www.react-spring.dev) and [Svelte](https://svelte.dev) | A rule for rest (a precision) to stop asking for frames. | `Animatable::PRECISION` |
| [GSAP](https://gsap.com) | Time as an object: the timeline. | `Sequence` |
| [FLIP](https://aerotwist.com/blog/flip-your-animations/), by Paul Lewis | Measure before, measure after, and animate the difference. | `PresenceList::place_all`, `.shared(..)`, `.glides(..)` |
| Flutter's `Hero` and Compose's `SharedTransitionLayout` | An element flies between two places. | `.shared(..)` |

## Contributing

How to contribute is in [CONTRIBUTING.md](https://github.com/walteraandrade/ginga/blob/main/CONTRIBUTING.md), and the rules for a change are in [AGENTS.md](https://github.com/walteraandrade/ginga/blob/main/AGENTS.md), for people and coding agents alike. The checks to run before pushing:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings                 # the engine
cargo test                                                # the engine, in seconds
cargo clippy -p ginga-gpui --all-targets -- -D warnings   # the GPUI adapter; the first build takes minutes
cargo test -p ginga-gpui                                  # the adapter on GPUI's test platform, no display
```

The engine needs Rust 1.87 or newer and the adapter Rust 1.95, the oldest that builds its GPUI. CI checks each on that version, on every pull request.

Everyone taking part follows the [code of conduct](https://github.com/walteraandrade/ginga/blob/main/CODE_OF_CONDUCT.md); report a security problem as [SECURITY.md](https://github.com/walteraandrade/ginga/blob/main/SECURITY.md) says.

## License

Either of [Apache-2.0](https://github.com/walteraandrade/ginga/blob/main/LICENSE-APACHE) or [MIT](https://github.com/walteraandrade/ginga/blob/main/LICENSE-MIT), at your option. Unless you say otherwise, a contribution you submit for inclusion, as defined in the Apache-2.0 license, is offered under the same two, with no other terms.
