# Behind is a lag in time

**Chosen.** `Keyframes::behind(lag)`: keyframes that play `lag` behind the same keyframes without it. A loop (`forever`) is partway through a round from its start; a count of rounds rests on the first stop until the lag is over, then plays only its rounds. `settles_at` counts from `start + lag`; `times(0)` stays settled at `start`. Issue #64, pull request #72.

**Options.**

- A lag in time, `behind(d)`.
- An advance in time, `offset(d)` or `ahead(d)`.
- A phase, a fraction of the round, `at_phase(0.13)`.
- Hand-made waits inside a round of the same length, as the gallery's dots had.

**Why.** Staggered copies are told in time everywhere else in ginga (`delay`, `starting_at`), and a lag reads the way the issue's acceptance criterion does: the copy at `t` is where the first was at `t − lag`. A phase would change meaning when the round's length changes, and needs a float checked for range. An advance reads backward for a row of dots. Hand-made waits break once the rise and fall fill the round.

**What follows from it.**

- A count of rounds plays no extra round before its first: `times(n)` plays `n` rounds in all, as its doc says, and `times(n).behind(d)` waits as `starting_at` does. The roundtable on #72 asked for this.
- *Not `starting_at`*: `starting_at` waits on the first stop, `behind` is partway through the loop. Used together, a loop waits on the first stop until its start, then is partway through a round.
- `ginga_gpui::Keyframes::restart` keeps both the lag and the delay of `starting_at`, so restarted copies keep their places in the loop.
- `Keyframes` grows by one `Duration`; `Keyframes<f32, 3>` passes clippy's limit for passing by value.
