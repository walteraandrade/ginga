# Reduced motion keeps fades

**Chosen.** Under reduced motion a fade is kept and a movement is not. A look's opacity and its three colors keep their transitions, and its offset, size and radius hold for their delay and are then at their target at once: `Transitions::for_reduced_motion`, which a declared look uses. A `present` way whose look names an opacity other than 1 keeps its transition, and its offset is left out when it is drawn, so the content fades where it is laid out. A way that only moves is taken at once after its delay, as before. A bare `Motion`, a `Presence`, a `PresenceList` and a `Keyframes` stay at once: nothing says their value is a fade, and there is no flag to say so. Issue #85.

**Options.**

- Keep fades, only where the value is known to be an opacity or a color.
- Keep dropping every motion, as [[decisions/2026-11-27-reduced-motion-keeps-delays]] settled.

**Why.** A fade is not the motion that causes vestibular trouble; sliding, scaling and zooming are. Motion's `MotionConfig reducedMotion="user"` keeps opacity and color and drops transforms and layout, and Apple advises replacing slides with cross-fades. With every motion dropped, a toast popped in and out. Waltinho chose this on 2026-11-28, on the decision card for #85.

**What follows from it.**

- A `present` way that fades and moves, under reduced motion, does not jump to its offset: it is drawn in its place for the whole fade, arriving and leaving.
- An opacity of 1 is where content is when it is there, so a way that names only that is no fade: it moves, and goes at once. Otherwise it would sit still, covered, for the whole way.
- Content that turns around on its way out comes back from its leaving look, so that look says whether the way back fades.
- Which values of a look are fades is marked in the `values!` table, so a new value has to say.
- A `Sequence` under reduced motion still drops every motion of its steps, a look's fades included (`Sequence::without_motion`). Keeping a look step's fades there is a follow-up, #99.
- `settles_at` of a look under reduced motion is when its last fade ends, not when its last delay ends.
- A variant's schedule, a sequence's schedule and a shared element's trip are still dropped whole.
- A view that wants a bare motion to fade under reduced motion passes it the transition itself instead of `unless_reduced`.
