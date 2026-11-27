# Closed-form math

**The rule.** The value at a time is computed from where the motion started (its start time, start value and start velocity) and the time asked for. It is never found by stepping forward from the last frame.

**Why.** A frame can be late, skipped or drawn twice, and a test can ask for any time in any order. Stepping makes the value depend on how often someone asked, so a slow machine would draw a different motion from a fast one, and errors would add up over a long motion. Closed-form math gives the same value at the same time, whoever asks and however often. It is also what makes `sample` cheap enough to take `&self`.

**How to apply it in a review.**

- A motion's state holds where it started, not where it was last drawn. A field such as "last value" or "last time" that a later sample builds on is a finding.
- A new transition, or a new kind of motion, gives a formula for value (and velocity) at time `t` since its start. A loop over small steps of time is a finding.
- A change of course starts a new piece from the value and velocity at that instant, computed in closed form. That keeps it from jumping.
- The test for it samples out of order, or skips frames, and gets the same values.

Related: [[principles/time-is-an-argument]], [[principles/settled-is-exact]].
