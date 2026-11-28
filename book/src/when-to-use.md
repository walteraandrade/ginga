# When to use ginga

ginga moves values in a Rust user interface: a panel that opens, a toast that comes and goes, a list whose rows arrive, leave and reorder, a card that is thrown and springs back, a color that changes. It does not draw anything: you draw the value it gives you.

## Use it when

- **You build a UI in Rust and want things to move.** Widths, heights, positions, opacity, corner radius, colors, or any type of your own that can be turned into a few `f32` (`Animatable`).
- **Your UI is in GPUI.** `ginga-gpui` declares motion on the element (`.look(..)`, `present(..)`, `.glides(..)`, `.shared(..)`, `.draggable(..)`), reads the frame's clock, asks for frames only while something moves, and follows GPUI's reduced-motion flag, which the app sets.
- **Your UI is in another framework, or a game loop.** The `ginga` engine has no dependencies and knows no framework. Keep a `Motion` next to your state, pass it the time on each frame, draw the value, and ask your framework for another frame while the sample is not `settled`. There is no adapter for other frameworks yet. The engine needs only the time, so one is a few lines: [a frame loop](recipes.md#move-a-value-toward-a-target-on-every-frame).
- **You want motion that tests and agents can check.** The same times always give the same values, and `Trace` writes a motion down as text ([Motion as text](engine/trace.md)).
- **You care about the cost of a frame.** A frame of a moving motion costs tens of nanoseconds, a settled one a few, and running a motion never allocates ([What it costs](cost.md)).

## What it does

| You need | ginga has |
| --- | --- |
| A value that heads for a target, and turns around smoothly when the target changes | `Motion` with a `Spring` (keeps its speed) or a `Tween` (a duration and an easing) |
| A loop, a pulse, a blink, a path through several values | `Keyframes`, with `.forever()`, `.times(n)`, `.mirrored()` |
| A thing that comes and goes, and is still drawn while it leaves | `Presence`; in GPUI, `present(..)` |
| A list whose items arrive, leave and move to new places | `PresenceList` with `sync` and `place_all` |
| A throw, a fling, a scroll that glides to a stop | `Motion::coast(..)`, with limits or snap points |
| Steps one after another, or overlapping | a delay on any transition, or a `Sequence` |
| A value computed from another (a progress into a width, a width into a color) | `Map` |
| Motion described on a GPUI element, with hover, press and focus states | `.look(id, look, transition)`, `.hover(..)`, `.pressed(..)`, `.focused(..)` |
| An element that travels to where layout puts it, or between two places | `.glides(..)`, `.shared(..)` |
| A drag | `.draggable(..)` |
| Respect for reduced motion | `ginga-gpui` reads GPUI's `reduce_motion`, which the app sets; the engine has `Transition::without_motion` |

The table from a goal to the exact call, with links and complete examples, is in [Recipes](recipes.md).

## Do not use it for

- **Drawing.** ginga gives values. Vector animation files (Lottie, Rive), sprites and shaders are other tools.
- **A physics world.** A spring here moves one value toward one target, in closed form. Bodies that collide are a physics engine's job.
- **A timeline editor or keyframes made in a design tool.** Keyframes are written in code.
- **Transforms on a GPUI `div`.** GPUI has no transform, so there is no `scale` or `rotate` on a box: change its size or its offset instead. An icon turns with its own transformation ([An icon that turns](coming-from.md#an-icon-that-turns)).
- **A width that layout changes.** An item whose content grows wider takes its new width at once. A height can glide (`.opened(id, 1.).height_glides(..)`), and a size that a look names moves. See [What it does not do yet](not-yet.md).
- **Gestures beyond a drag with one pointer.** Pinch, rotate and a drag on a touchpad's scroll are not there yet.
- **Another GPUI.** `ginga-gpui` pins one version of GPUI ([Getting started](getting-started.md) names it), and an app must use the same one, or the types do not match. The engine works with any.

It is early work: until 1.0, a new minor version can change the API. The minimum Rust versions are in [Contributing](contributing.md).

## Alternatives

- **GPUI's own `with_animation`** plays an animation of a fixed duration on an element. It has no spring, does not turn around from where it is when its target changes, and does not keep a thing drawn while it leaves.
- **gpui-motion**, **gpui-ce-motion** and **`gpui-base::motion`** are motion libraries for GPUI that ginga learned from ([Inspirations](inspirations.md)).
- **[lilt](https://github.com/ejjonny/lilt)** is a motion library for iced, also with time as an argument.
- **Motion (formerly Framer Motion), SwiftUI and Jetpack Compose** are not Rust, but if you know them, [Coming from Motion, SwiftUI or Compose](coming-from.md) maps their tools to ginga's.
