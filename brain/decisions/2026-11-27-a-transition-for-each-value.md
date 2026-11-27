# A transition for each value of a look

**Chosen.** `Transitions` gives each value of a `Look` its own transition, with `.all(t)` for the whole look and `.or(defaults)` to lay it over a view's or an app's defaults. A value's own wins over the whole look's, which wins over the defaults, which win over ginga's own: `Spring::SNAPPY` for the offset, the size and the radius, a 150 ms `Easing::Out` tween for the opacity and the colors. A `Transition`, `Spring` or `Tween` converts into `Transitions` as the transition for the whole look, so every call that took one still compiles. In the adapter, `set_default_look_transitions(t, cx)` keeps an app's defaults in a GPUI global. Issue #27.

**Options.**

- One struct with an optional transition per value, layered with `or`.
- A closure from the value's name to its transition.
- A trait for "a transition source" with impls for each layer.

**Why.** A struct is `Copy`, allocates nothing, and reads in one line at the call. It is not built as a constant: its setters take `impl Into<Transition>` so that `.offset(Spring::BOUNCY)` reads plainly, and a `const fn` cannot call `into`. A view keeps its defaults in a field or a function. The layers are three and fixed (value, look, defaults), so `or` covers them without a trait. A closure would not be `Copy`.

**What follows from it.**

- A `Transitions` keeps its layers apart: the values' own, the whole look's, and the defaults under it, resolved value by value when `or` is called. So an `.all(t)` after `.or(..)` still wins over the defaults; folding `all` into the values in `or` made the result depend on the order of the calls. One private function, `named`, says the order, and `set_target` and `or` both use it.
- A `Transitions` is about a kilobyte, because each `Transition` holds an `Easing` that can be a Bézier. It allocates nothing. The adapter builds the layered one only on a frame whose look changed; clippy's `large_types_passed_by_value` needs an `#[expect]` on a private function that keeps it.
- Under reduced motion the adapter passes `Transition::Instant` for the whole look, ignoring every layer: a value's own transition would otherwise win over it.
- `LookMotion` and `Transitions` are written from one table, the `values!` macro in `look.rs`: a value added there is added to the motion, the transitions, `at`, `set_target` and `sample` together. `Look::finite` and the trace columns are still written by hand.
