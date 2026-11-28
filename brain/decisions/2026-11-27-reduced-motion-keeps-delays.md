# Reduced motion keeps a transition's delay

**Chosen.** Under reduced motion a transition keeps its delay and drops only its motion: the value holds still until the delay is over, then is at its target at once. `Transition::without_motion` and `Transitions::without_motion` say it in the engine; `unless_reduced`, a declared look and `present` use them. Issue #62.

**Options.**

- Keep the delay, then go at once.
- Drop the delay with the motion, as before, and say so in the docs.

**Why.** A delay is timing, not motion, and an app gives it meaning: Desafinado waited 160 ms before showing a Suggestion card on hover, as hover intent, and under reduced motion the card popped up as soon as the pointer touched the text. Reduced motion asks for no movement, not for no waiting. Both options were open in the issue; Waltinho was told this one was picked and did not object.

**What follows from it.**

- A stagger built from delays (see [[decisions/2026-11-27-stagger-is-a-delay-on-the-transition]]) still steps under reduced motion, as jumps one after another.
- A variant's schedule is not kept: under reduced motion every element of a variant takes its look at once, as [[decisions/2026-11-27-variants-are-a-handle-passed-by-hand]] says. The schedule is choreography of motion, not a delay the app gave one value.
- A sequence started with `ginga_gpui::sequence(now, cx)` drops its schedule too: each step is at its target after its own delay ([[decisions/2026-11-28-a-step-is-anything-retargeted]]).
- A shared element still does not travel and does not wait: it is in its destination at once.
- `settles_at` of a value under reduced motion is the time its delay ends.
