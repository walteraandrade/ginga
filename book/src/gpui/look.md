# A look on an element

Or declare on the element how it should look, and it gets there by itself. A `Look` names any of an offset from the element's own place, an opacity, a width and height, a corner radius and colors; a value it does not name stays the element's own style. The offset moves only what is painted and where the pointer finds the element: the layout does not see it, so an `absolute` element keeps its own place and slides from there.

```rust,ignore
let look = if self.open {
    Look::new().size(320., 200.).opacity(1.)
} else {
    Look::new().size(160., 100.).opacity(0.6)
};
div().bg(rgb(0x2a2f3a)).look("card", look, Spring::SNAPPY)
```

A look can name colors too: `.background(..)`, `.text_color(..)` and `.border_color(..)` take a `Color`: `Color::hex(0x3b6ef5)`, `Color::new(r, g, b, alpha)`, `Color::TRANSPARENT`, any of them `.with_alpha(a)`, or one taken from GPUI with `ginga_gpui::color(rgb(0x3b6ef5))`. Each value is held to `0.0..=1.0`. GPUI's `Rgba` and `Hsla` can also move in a `ginga_gpui::Motion`. Colors mix in linear light with the alpha premultiplied: red on its way to blue passes through purple, never green, and a color fading in from transparent never darkens.

```rust
use std::time::Duration;
use ginga::{Color, Easing, Look, LookMotion, Tween};

let ms = Duration::from_millis;
let mut button = LookMotion::at(Look::new().background(Color::hex(0xff0000)));
button.set_target(Look::new().background(Color::hex(0x0000ff)), Tween::new(ms(100), Easing::Linear), ms(0));

let middle = button.sample(ms(50)).value.background.unwrap();
assert_eq!(middle.green, 0.0);       // never green
assert!(middle.red > 0.7);           // a bright purple, as light mixes

// A fill that fades in from nothing keeps its own color, only less opaque.
let red = Color::hex(0xe5484d);
let mut fill = LookMotion::at(Look::new().background(Color::TRANSPARENT));
fill.set_target(Look::new().background(red), Tween::new(ms(100), Easing::Linear), ms(0));
let half = fill.sample(ms(50)).value.background.unwrap();
assert!((half.red - red.red).abs() < 1e-4 && (half.alpha - 0.5).abs() < 1e-6);
```

When the look changes, the element moves from where it is, and a spring keeps its speed. The motion is kept with the element's id, so the view needs no field for it. On its first frame the element takes its look at once; `.from(look)` makes it arrive from another one. An element that must keep its motion while it is not drawn takes a `LookState` the view owns: `.look_with(&self.card, look, transitions)`. Like `toward`, it asks for frames only while it moves. Under reduced motion it keeps its fades: the opacity and the colors keep their transitions, and the offset, the size and the radius are taken at once after their delay (`Transitions::for_reduced_motion`). To write a declared look down, keep it in a `LookState`: `self.card.inspect(cx)` hands back its engine motion and this frame's time for `Trace::of`, as `motion.inspect(cx)` does for a `ginga_gpui::Motion`, and is `None` until an element has drawn with it. A look the view already has, such as one mixed by a presence's progress (`from.mix(Look::new(), shown.progress)`), is applied as it is with `.with_look(look)`: the same mapping, with no motion and no element state.

In the engine the same thing is a `LookMotion`, which takes the time as an argument like a `Motion`.

Each value of a look can move on its own clock. A single `Transition`, `Spring` or `Tween` moves the whole look; a `Transitions` gives values their own, and what it leaves out comes from the defaults under it:

| Value | With nothing given |
| --- | --- |
| offset, width, height, radius | `Spring::SNAPPY` |
| opacity, background, text and border color | a 150 ms tween, `Easing::Out` |

A value's own transition wins over the whole look's (`.all(..)`), which wins over the defaults it is laid over with `.or(defaults)`, which win over the table. The layers hold whatever order the calls come in. A view keeps its defaults in a field or a function and lays each look's over them. An app sets its own once for every declared look with `ginga_gpui::set_default_look_transitions(transitions, cx)`; a `ginga_gpui::Motion` takes the transition it is given and nothing else.

```rust
use std::time::Duration;
use ginga::{Easing, Look, LookMotion, Spring, Transitions, Tween};

let ms = Duration::from_millis;
let mut card = LookMotion::at(Look::new().offset(0.0, 120.0).opacity(0.0));
let shown = Look::new().offset(0.0, 0.0).opacity(1.0);
// It springs to its place while it fades in a fixed 100 ms.
let transitions = Transitions::new()
    .offset(Spring::BOUNCY)
    .opacity(Tween::new(ms(100), Easing::Linear));
card.set_target(shown, transitions, ms(0));
let at_100 = card.sample(ms(100)).value;
assert_eq!(at_100.opacity, Some(1.0));        // the fade is over
assert!(at_100.offset.unwrap().1 > 0.0);       // the spring is still on its way
```
