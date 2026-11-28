# What it costs

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
