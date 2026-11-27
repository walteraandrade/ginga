# The engine knows no framework

**The rule.** `crates/ginga` has no dependencies and no word about any UI framework. `crates/ginga-gpui` adds only what needs GPUI: the frame's clock, the frame request, reduced motion, and stand-ins for GPUI's own types. Motion logic goes in the engine.

**Why.** The engine is the part worth trusting and testing: plain math over plain data, which builds in seconds and can be checked with a trace. Keeping GPUI out of it means its tests run without building GPUI, which takes minutes, and that another framework could get its own thin adapter. Logic that lands in the adapter can only be tested with GPUI's test platform or a window, so it is harder to check and easy to duplicate.

**How to apply it in a review.**

- A new dependency in `crates/ginga/Cargo.toml` is a finding, dev-dependencies included unless a person agreed to it.
- A doc comment or name in the engine that mentions GPUI, a window, an element or a view is a finding. The engine speaks of values, targets and times.
- A change in the adapter that computes motion (an easing, a schedule, a stagger, a mix of colors) is a finding: it belongs in the engine, with the adapter calling it.
- A new GPUI type that moves gets an `Animate` impl and a stand-in, not new motion code.

Related: [[codebase/gpui-pin-and-the-orphan-rule]].
