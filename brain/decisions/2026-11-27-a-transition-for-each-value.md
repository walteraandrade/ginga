# A transition for each value of a look

**Chosen.** `Transitions` gives each value of a `Look` its own transition, with `.all(t)` for the whole look and `.or(defaults)` to lay it over a view's or an app's defaults. A value's own wins over the whole look's, which wins over the defaults, which win over ginga's own: `Spring::SNAPPY` for the offset, the size and the radius, a 150 ms `Easing::Out` tween for the opacity and the colors. A `Transition`, `Spring` or `Tween` converts into `Transitions` as the transition for the whole look, so every call that took one still compiles. In the adapter, `set_default_transitions(t, cx)` keeps an app's defaults in a GPUI global. Issue #27.

**Options.**

- One struct with an optional transition per value, layered with `or`.
- A closure from the value's name to its transition.
- A trait for "a transition source" with impls for each layer.

**Why.** A struct is `Copy`, allocates nothing, can be built as a constant, and reads in one line at the call. The layers are three and fixed (value, look, defaults), so `or` covers them without a trait. A closure would be neither `Copy` nor comparable.

**What follows from it.**

- A `Transitions` is about 500 bytes, because each `Transition` holds an `Easing` that can be a Bézier. Passing it by value is fine for a call per frame; clippy's `large_types_passed_by_value` needs an `#[expect]` on a private function that keeps it.
- Under reduced motion the adapter passes `Transition::Instant` for the whole look, ignoring every layer: a value's own transition would otherwise win over it.
- `LookMotion` and `Transitions` are written from one table, the `values!` macro in `look.rs`: a value added there is added to the motion, the transitions, `at`, `set_target` and `sample` together. `Look::finite` and the trace columns are still written by hand.
