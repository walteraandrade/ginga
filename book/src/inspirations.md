# Inspirations

ginga takes ideas that work elsewhere and fits them to its two goals ([What it is for](introduction.md#what-it-is-for)):

| Inspiration | The idea taken | Where it is in ginga |
| --- | --- | --- |
| [Motion](https://motion.dev) (formerly Framer Motion) | You say the target and the library takes care of the way there. The value and its speed live outside the drawing (`MotionValue`). What leaves stays on screen until its way out is over (`AnimatePresence`). | `Motion::toward`, `Presence`, `PresenceList` |
| Motion's `animate`, `whileHover`, `layoutId`, variants and drag | A look declared on the element; a look for a moment; one thing in two places; a parent's name that its children follow, one after another; a drag that is thrown and springs back. | `.look(..)`, `.hover(..)`, `.pressed(..)`, `.focused(..)`, `.shared(..)`, `.leads(..)` and `.follows(..)`, `.draggable(..)` |
| Motion's `layout`, gpui-motion's `with_flip` and gpui-ce-motion's `LayoutGroup` | An element whose place layout changes travels there, painted, with layout untouched. | `.glides(..)` |
| [lilt](https://github.com/ejjonny/lilt), for iced | Time is an argument. There is no tick. | `toward(target, transition, now)`, `sample(now)` |
| [SwiftUI](https://developer.apple.com/xcode/swiftui/) | A spring described first by its duration and its bounce, rather than by mass and stiffness. A spring that is interrupted keeps its speed. | `Spring::new(response)`, `.bounce(..)` |
| Motion, react-spring and SwiftUI's `interpolatingSpring` | A spring given as a stiffness, a damping and a mass, as specs and other code give it. | `Spring::physical(..)` |
| SwiftUI and [Jetpack Compose](https://developer.android.com/compose) | A type moves if it can become a row of numbers and come back. Each number has its own speed. | `Animatable`, `to_channels`, `from_channels` |
| [Neovide](https://neovide.dev) and gpui-motion | A spring in closed form: the value comes straight from the formula, never by stepping frame to frame. | `crates/ginga/src/spring.rs` |
| `gpui-base::motion` | At rest costs nothing. An invalid value stops at the constructor. The library gives a phase and a progress; the app decides what they look like. | `Sample::settled`, `SpringError`, `Phase::{Entering, Present, Exiting}` |
| gpui-ce-motion | The state has an owner in plain sight: it lives in your view. Its `Target`, a closed list of properties, is the model for a look. | a `Motion` is a field of your view, and the engine keeps no state of its own; `Look` |
| [react-spring](https://www.react-spring.dev) and [Svelte](https://svelte.dev) | A rule for rest (a precision) to stop asking for frames. | `Animatable::PRECISION` |
| [GSAP](https://gsap.com) | Time as an object: the timeline. | `Sequence` |
| [FLIP](https://aerotwist.com/blog/flip-your-animations/), by Paul Lewis | Measure before, measure after, and animate the difference. | `PresenceList::place_all`, `.shared(..)`, `.glides(..)` |
| Flutter's `Hero` and Compose's `SharedTransitionLayout` | An element flies between two places. | `.shared(..)` |
