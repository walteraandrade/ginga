# ginga

Motion for Rust UIs, as a pure function of time.

A `Motion` is a value on its way to a target. You own it, you hand it the time, and it answers with the value at that time. Nothing ticks, nothing allocates while it runs, and the same times always give the same values. That makes motion something a test, a log or a coding agent can read as text, without a screen.

## What it is for

ginga has two goals, in this order.

**1. Performance.** Motion is drawn every frame, so a frame of it must cost next to nothing, and nothing at all once it is over.

- A value at a time is computed in closed form from where the motion started, never stepped from the last frame: a skipped or late frame changes nothing. A frame of a moving motion costs tens of nanoseconds, and a few once it is settled ([What it costs](cost.md)).
- Running a motion never allocates; a test fails if it does.
- A motion says when it is settled, and a settled motion is exactly on its target, so whoever draws stops asking for frames.
- In GPUI, what a frame costs is what GPUI renders for it. An `Island` keeps a motion in a small view of its own, so the heavy views beside it, when they are cached, are not rendered again ([Motion that redraws little](gpui/island.md)).

**2. Easy to use for AI agents.** A coding agent cannot watch an animation. ginga is built so that it does not have to.

- Time is an argument, and the engine never reads a clock: an agent can ask for the value at any time and get the same answer every run.
- `Trace` writes a motion down as a table of times and values, in a stable format: a test asserts on it, and a change in motion shows up as a diff ([Motion as text](engine/trace.md)).
- The API is small and named in plain words (`toward`, `jump_to`, `settled`). This book shows it, and its engine examples run as tests; the GPUI ones are sketches, checked by the adapter's own tests.
- An invalid value does not build, and the error says what to use instead.
- The rules for a change are in [AGENTS.md](https://github.com/walteraandrade/ginga/blob/main/AGENTS.md), and the reasons and decisions behind them in [`brain/`](https://github.com/walteraandrade/ginga/blob/main/brain/index.md), so an agent that starts cold knows what was already decided.

When the two pull apart, performance wins.
