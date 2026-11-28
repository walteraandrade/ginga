# ginga

[![CI](https://github.com/walteraandrade/ginga/actions/workflows/ci.yml/badge.svg)](https://github.com/walteraandrade/ginga/actions/workflows/ci.yml)
[![Book](https://img.shields.io/badge/book-walteraandrade.github.io%2Fginga-blue)](https://walteraandrade.github.io/ginga/)

Motion for Rust UIs, as a pure function of time.

A `Motion` is a value on its way to a target. You own it, you hand it the time, and it answers with the value at that time. Nothing ticks, nothing allocates while it runs, and the same times always give the same values. That makes motion something a test, a log or a coding agent can read as text, without a screen.

**[Read the book](https://walteraandrade.github.io/ginga/)**: the whole API, with examples that run as tests, a gallery, what it costs, a guide for whoever comes from Motion, SwiftUI or Compose, and the inspirations.

## What it is for

Two goals, in this order: **1. performance**, a frame of motion costs tens of nanoseconds and nothing once it is settled; **2. ease of use for AI agents**, time is an argument and `Trace` writes a motion down as text, so an agent checks motion without watching it. When the two pull apart, performance wins. [More on both](https://walteraandrade.github.io/ginga/introduction.html#what-it-is-for).

## Install

Early work: until 1.0, a new minor version can change the API.

```toml
[dependencies]
ginga = "0.1"      # the engine, with no dependencies
ginga-gpui = "0.1" # GPUI views; pins `gpui-pre =0.3.12`, which an app must use too
```

## A value that moves

```rust
use std::time::Duration;
use ginga::{Motion, Spring, Trace};

let ms = Duration::from_millis;
let mut width = Motion::at(180.0);

// Each frame: head for a target, get the value now.
width.toward(320.0, Spring::SNAPPY, ms(0));
let sample = width.toward(320.0, Spring::SNAPPY, ms(100));
assert!(sample.value > 180.0 && sample.value < 320.0);
assert!(!sample.settled); // draw another frame

// Write it down: one row per 16 ms until it settles.
println!("{}", Trace::of(&width, ms(0), ms(16)));
```

In a GPUI view, `ginga_gpui::Motion` reads the frame's clock and asks for frames while it moves, or `.look(id, look, transition)` declares on an element how it should look: [In a GPUI view](https://walteraandrade.github.io/ginga/gpui/view.html).

## Contributing

[CONTRIBUTING.md](https://github.com/walteraandrade/ginga/blob/main/CONTRIBUTING.md) says how a change gets in, and [AGENTS.md](https://github.com/walteraandrade/ginga/blob/main/AGENTS.md) the rules for it, for people and coding agents alike. The book's pages are in [`book/`](https://github.com/walteraandrade/ginga/tree/main/book).

## License

Either of [Apache-2.0](https://github.com/walteraandrade/ginga/blob/main/LICENSE-APACHE) or [MIT](https://github.com/walteraandrade/ginga/blob/main/LICENSE-MIT), at your option. Unless you say otherwise, a contribution you submit for inclusion, as defined in the Apache-2.0 license, is offered under the same two, with no other terms.
