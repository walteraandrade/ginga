# A frame cap is per set of keyframes

**Chosen.** `Keyframes::at_most_every` caps the frames one set of keyframes asks for, with a timer of its own. The docs say so; there is no cap shared by a view. Issue #65.

**Options.**

- One pending capped frame per view, shared by every capped set in it.
- A line in the docs saying the cap is per set.

**Why.** GPUI's own `Animation::with_max_fps` keeps one pending timer per animated element (`delayed_frame_pending` in its `AnimationState`), so a cap per set is what a GPUI user already expects. A cap per view would need state shared across every set in a view, kept by view id, for a case with a simple answer: things that should share a rate share one set and draw from its value. Keeping it per set keeps the API as it is ([[principles/a-small-api]]). Waltinho was told the docs option was picked and did not object.

**What follows from it.**

- Two capped sets started on the same frame with the same `every` stay in step: their timers end together, and their two notifications are one redraw.
- Started apart (a `restart`, a set first drawn later), they can draw the view up to once per set in each `every`.
- Desafinado's requests tray shares one breath across its marks for this reason.
