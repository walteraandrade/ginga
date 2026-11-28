# A step is anything retargeted

**Chosen.** A `Sequence` step is anything that is `Retarget`: `Motion<T>` with a `Transition`, or `LookMotion` with `Transitions`. The trait is sealed, and it has two items. `retarget(target, transition, delay, still, now)` returns when the step starts; `settles_at` says when it ends. Under reduced motion (`Sequence::without_motion`, which the adapter's `sequence(now, cx)` gives), a sequence drops its schedule as well as its motion. Issue #84, PR #93.

**Options.**

- A trait over the engine's two motions. This is what was chosen.
- A second set of calls for looks (`then_look`, ...). It doubles the API.
- Adapter motions as steps, which the issue asked for, through their engine motion and `unless_reduced`.

**Why.**

- One trait keeps three calls. The first version made the trait open, with five items of transition arithmetic, and the roundtable found a bug in it: a look's start counted the values its look did not name. The sealed two-item shape lets each motion work out its own start from its own target.
- Adapter motions are left out. Each `ginga_gpui::Motion` has a `Clock` of its own, with its own epoch, and `toward` gives it a target on every render. A sequence needs one start time for all its steps and sets each target once, so a sequence in a view moves engine motions and looks read with the view's `Clock`. The README said so before this change.
- Reduced motion drops the schedule, as it does for a variant ([[decisions/2026-11-27-reduced-motion-keeps-delays]]). Keeping the places would make the reduced version wait out the whole animation it replaces. Each step's own delay is kept.

**What follows from it.**

- A look step starts when the first value its target names starts.
- A look step moves only the values its look named before; a value named for the first time is at its target at once, as `LookMotion::set_target` does.
