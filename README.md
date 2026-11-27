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

In the one app measured so far, a frame that skipped the heavy view cost about 40 % less, and one frame in four skipped it: [the measurement](docs/measurements/2026-11-26-ai-caret-island/README.md).

## What it costs

`cargo run --release --example bench`, best of nine rounds on the machine this was written on (2026-11-26):

| One frame of | Time |
| --- | --- |
| a value at rest | 7 ns |
| a spring, 1 to 4 channels | 12 to 25 ns |
| a spring with 4 channels and a new target every frame | 165 ns |
| a tween, 2 channels | 11 to 15 ns |

Reading and retargeting a motion never allocates; [`tests/no_alloc.rs`](crates/ginga/tests/no_alloc.rs) fails if it does.

The engine is not where a frame's time goes. Measured in a real app, a frame of motion cost the main thread about 1.2 ms, and two engines cost the same: [the measurement](docs/measurements/2026-11-26-ai-caret/README.md). What a motion costs is what GPUI renders for it.

## What it does not do yet

- Animate layout: an item of a `PresenceList` that moves to a new place jumps there.
- Shared elements.
- Sequences, stagger, keyframes.
- Gestures and drag.

Limits that come from GPUI and not from this library: a `div` has no transform, so moving a box means changing its position or size and laying out again; and there is no way to redraw one element without rendering its view.

## Development

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings   # the engine
cargo test                                  # the engine, in seconds
cargo clippy -p ginga-gpui -- -D warnings   # the GPUI adapter; the first build takes minutes
```

The engine needs Rust 1.87 or newer and the adapter Rust 1.95, the oldest that builds its GPUI. CI checks each on that version, on every pull request.

Rules for changes are in [AGENTS.md](AGENTS.md). No license has been chosen yet.
