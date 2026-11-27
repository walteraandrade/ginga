# ginga

[![CI](https://github.com/walteraandrade/ginga/actions/workflows/ci.yml/badge.svg)](https://github.com/walteraandrade/ginga/actions/workflows/ci.yml)

Motion for Rust UIs, as a pure function of time.

A `Motion` is a value on its way to a target. You own it, you hand it the time, and it answers with the value at that time. Nothing ticks, nothing allocates while it runs, and the same times always give the same values. That makes motion something a test, a log or a coding agent can read as text, without a screen.

Early work. Two crates:

| Crate | What it is | Depends on |
| --- | --- | --- |
| [`ginga`](crates/ginga) | The engine: springs, tweens, the trace. | nothing |
| [`ginga-gpui`](crates/ginga-gpui) | The frame's clock, frame requests and reduced motion for GPUI views. | `gpui-pre =0.3.12`, the GPUI that `gpui-kit` 0.9.1 builds on |

## A value that moves

| Call | What it does |
| --- | --- |
| `Motion::at(value)` | A value at rest. |
| `motion.toward(target, transition, now)` | Each frame: head for a target, get the value now. |
| `motion.set_target(target, transition, now)` | Change course at a moment you choose. |
| `motion.jump_to(value)` | Move with no motion. |
| `motion.sample(now)` | Read the value at any time, without changing anything. |

A transition is `Spring::SNAPPY`, `Spring::SMOOTH`, `Spring::BOUNCY`, a `Spring::new(response)`, a `Tween::new(duration, easing)` or `Transition::Instant`. Anything `Animatable` can move: `f32`, tuples of up to four `f32`, `[f32; N]`, and your own types.

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

`Keyframes` go through several stops, each at its time with its own easing, and can repeat: `.times(n)` or `.forever()`, `.mirrored()` to go back and forth, `.pause(d)` between rounds, `.starting_at(t)` to start later. Like a motion, they are read at any time with `sample(now)`; `rest()` is where they rest once played.

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

## Things that come and go

A `Presence` is something you show and hide. Hiding does not drop it: the presence keeps the content, as it was, until its way out is over, so there is always something to draw the exit with.

| Call | What it does |
| --- | --- |
| `Presence::absent()` | Nothing shown. |
| `presence.show(content, enter, now)` | Show it, or change what is shown. |
| `presence.hide(exit, now)` | Start the way out. |
| `presence.frame(now)` | Each frame: what to draw, with its phase and progress. |

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

let transition = ginga_gpui::unless_reduced(Spring::SNAPPY, cx);
self.rows.place_all(|index, _| index as f32 * ROW, transition, now);

let mut moving = false;
let rows: Vec<_> = self
    .rows
    .frame(now)
    .map(|row| {
        moving |= row.moving();
        div()
            .h(px(ROW))
            .relative()
            .top(px(row.offset))
            .opacity(row.progress)
            .child(row.content.label.clone())
    })
    .collect();
if moving {
    window.request_animation_frame();
}
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

`toward` reads the frame's clock, asks GPUI for another frame while the value moves, and moves at once when the desktop asks for reduced motion.

When the logic of a motion lives in a crate that knows nothing about GPUI, that crate keeps a `ginga::Motion` and takes the time as an argument; the view supplies it with `ginga_gpui::Clock`.

## A look on an element

Or declare on the element how it should look, and it gets there by itself. A `Look` names any of an offset from the element's own place, an opacity, a width and height, a corner radius and colors; a value it does not name stays the element's own style.

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

When the look changes, the element moves from where it is, and a spring keeps its speed. The motion is kept with the element's id, so the view needs no field for it. On its first frame the element takes its look at once; `.from(look)` makes it arrive from another one. An element that must keep its motion while it is not drawn takes a `LookState` the view owns: `.look_with(&self.card, look, transitions)`. Like `toward`, it asks for frames only while it moves and moves at once under reduced motion.

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
    .look("save", Look::new().size(120., 40.).background(fill), Spring::SNAPPY)
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

## An element to drag

`.draggable(&state, drag)` lets the pointer drag any styled element. While it is held it is exactly under the pointer, with no spring between them; let go, it keeps the pointer's speed and coasts. A `Drag` says how: `.along(Axis::Horizontal)`, `.within(min, max)` to spring back inside limits, `.snap_to(points)` to come to rest on the nearest one, and `.coast(Coast::SLOW)`. The view owns a `DragState`, `DragState::default()` or `DragState::at(offset)`: `offset()` is where the element was last drawn, `rest()` where it comes to rest, `is_held()` whether the pointer holds it, and `throw(velocity, &drag, cx)` lets it go as a pointer would, for a key or a button.

```rust,ignore
// The view owns `knob: DragState`. A slider's knob with five stops:
let stops = (0..5).map(|stop| point(px(93. * stop as f32), px(0.)));
div().size(px(28.)).draggable(&self.knob, Drag::new().along(Axis::Horizontal).snap_to(stops))
```

Like a look, the offset moves the element by `left` and `top` from its place in the layout. With snap points, the limits are left out: put the points inside them. Only the topmost element under the pointer is taken, and a move with the button up lets it go, as when a release was lost outside the window. Under reduced motion a thrown element is at once where it comes to rest.

## A loop in a view

`ginga_gpui::Keyframes` are keyframes a view owns. They start on the first frame that draws them, `restart(cx)` plays them again, and they rest where they end under reduced motion (`rest()`: a loop on its first stop). A slow loop need not be drawn at the display's rate: `.at_most_every(d)` asks for the next frame no sooner than `d` after this one. It holds back only its own requests, so draw the loop in a view of its own, such as an `Island`, or another motion in the view draws it on every frame anyway.

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

Measured on its own, with a heavy view of 2,400 boxes and a dot that never stops, a frame cost the main thread about 19 to 22 ms with the island and 48 to 50 ms without it, and the heavy view did not render again while the dot moved: [the measurement](docs/measurements/2026-11-27-island-alone/README.md) (software rendering, no GPU). In an app it depends on what else asks for frames: in Desafinado a frame that skipped the heavy view cost about 40 % less, but only one frame in four skipped it: [that measurement](docs/measurements/2026-11-26-ai-caret-island/README.md).

## Watch it move

The gallery is an example app with a page for each thing ginga does: a value toward a target with a spring and a tween, a presence, a list presence, a look, colors, keyframes and an island. Each page is driven by a few lines that can be copied.

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

Each number includes about 1 ns that the timing loop costs on its own; the bench prints it as its first row. The bouncy spring with one channel reports slower than with four although it runs fewer instructions (valgrind): which case gets the slow code layout changes with unrelated edits, see [the note](brain/codebase/bouncy-spring-bench-and-code-layout.md).

Reading and retargeting a motion never allocates; [`tests/no_alloc.rs`](crates/ginga/tests/no_alloc.rs) fails if it does.

The engine is not where a frame's time goes. Measured in a real app, a frame of motion cost the main thread about 1.2 ms, and two engines cost the same: [the measurement](docs/measurements/2026-11-26-ai-caret/README.md). What a motion costs is what GPUI renders for it.

## What it does not do yet

- Animate a change of size that layout makes: an item whose content grows takes its new size at once. A size that a look names does move.
- Shared elements.
- Sequences.
- Gestures beyond a drag with one pointer: pinch, rotate, a drag on a touchpad's scroll.

Limits that come from GPUI and not from this library: a `div` has no transform, so moving a box means changing its position or size and laying out again; and there is no way to redraw one element without rendering its view.

## Development

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings                 # the engine
cargo test                                                # the engine, in seconds
cargo clippy -p ginga-gpui --all-targets -- -D warnings   # the GPUI adapter; the first build takes minutes
cargo test -p ginga-gpui                                  # the adapter on GPUI's test platform, no display
```

The engine needs Rust 1.87 or newer and the adapter Rust 1.95, the oldest that builds its GPUI. CI checks each on that version, on every pull request.

Rules for changes are in [AGENTS.md](AGENTS.md).

## License

Either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option. Unless you say otherwise, a contribution you submit for inclusion, as defined in the Apache-2.0 license, is offered under the same two, with no other terms.
