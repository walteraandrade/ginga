# Stagger is a delay on the transition

**Chosen.** `Tween::delay`, `Spring::delay` and `Transition::delay` give the same transition, starting a set time after the target is set. Stagger needs no type of its own: item `n` uses `delay((step * n).min(cap))`. Issue #8, pull request #20.

**Options.**

- A delay on the transition.
- A delay on the way in of a presence.
- A stagger type that schedules a group.

**Why.** The issue names two uses, and only one is a presence: the rows of Desafinado's Ask bubble arrive, but the bars of its charts grow, and they are plain motions. A delay on the transition serves both with one call, and `Presence::show` and `PresenceList::sync` take transitions already. A delay on the way in would have left the bars to hand-written delays. A stagger type would add a public type for what one line of arithmetic does ([[principles/a-small-api]]).

**What follows from it.**

- Before the delay ends, the value holds still where it was, and the motion is not settled. `settles_at` counts the delay.
- A delayed spring starts from rest: carrying the old speed across the wait would make it jump into motion at the end.
- A delayed `Instant` jumps when the delay ends.
- Not covered yet: a `PresenceList` gives every item that arrives in one `sync` the same way in, so its rows cannot be staggered from a single `sync`. A `Presence` per row can.
