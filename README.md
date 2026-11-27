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

- Animate a change of size: an item that grows or shrinks takes its new size at once.
- Shared elements.
- Sequences and keyframes.
- Gestures and drag.

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
