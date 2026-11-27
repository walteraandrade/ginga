# A sequence is a schedule of delays

**Chosen.** A `Sequence` sets each step's target once, at the time the sequence starts, with a delay that puts the step at its place: `then` after the step before ends (its `settles_at`), `with` together with it, `at` a given time. It keeps only the times (where it starts, the last step, the latest end), so it is `Copy` and never allocates. `SettledOnce` reports an end once, from the time the app already has. Derived values are a `Map` from a number onto a range, read from a `Motion<f32>` with `of`. Issue #34, which allowed two pull requests; both halves landed in one, as they share a gallery page.

**Options.**

- A schedule of delays on the motions the app already owns.
- A sequence that owns its motions, steps through them, and is sampled itself.
- Callbacks run by the engine when a step ends.

**Why.** The engine reads no clock and runs nothing between frames, so a step cannot "start when the one before ends" except by knowing that time ahead: `settles_at` gives it, and the existing delay on a transition does the rest ([[decisions/2026-11-27-stagger-is-a-delay-on-the-transition]]). A sequence that owned its motions would need one type for motions of different values, or a box. Callbacks would make the engine run code at a time, which [[principles/time-is-an-argument]] rules out; `SettledOnce` lets the app run its own code on the frame where it sees the end.

**What follows from it.**

- A step after a spring starts at the spring's `settles_at`, a safe bound: the spring looks arrived a little before the next step starts.
- A motion given another target in the middle leaves the sequence's times behind; the docs say so.
- A follower needs no type: `toward` on every frame with the leader's value retargets a spring that keeps its speed. A test checks it trails without jumps and settles exactly on the leader's rest.
- `Map::at` gives the ends back exactly as given: a color rebuilt from its channels would not compare equal.
