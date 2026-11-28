# In a GPUI view

Keep the motion next to the state it animates:

```rust,ignore
struct Panel {
    open: bool,
    width: ginga_gpui::Motion<Pixels>,
}

impl Render for Panel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let target = if self.open { px(320.) } else { px(0.) };
        let width = self.width.toward(target, Spring::SNAPPY, window, cx);
        div().w(width)
    }
}
```

`toward` reads the frame's clock, asks GPUI for another frame while the value moves, and moves at once under reduced motion (GPUI's `reduce_motion`). A delay is timing, not motion, so it is kept: under reduced motion a delayed value holds still until its delay is over, then is at its target at once (`Transition::without_motion`, and `Transitions::without_motion` for a look). A `ginga_gpui::Motion` moves `f32` and GPUI's `Pixels`, `Point<Pixels>`, `Size<Pixels>`, `Bounds<Pixels>`, `Rgba` and `Hsla`; another GPUI type gets an `Animate` impl. `motion.inspect(cx)` hands back the engine's motion and this frame's time, to write it down with `Trace::of`. `ginga_gpui::Presence` is a `Presence` read on the frame's clock: `show(content, enter, cx)`, `hide(exit, cx)` and `frame(window, cx)`. `shown.cover(element)` (`CoverExt`) draws what `frame` gave covered while it leaves: it takes no clicks, and the keyboard focus is taken from inside it, as `present` does. An event handler with no window reads `wanted()`, the content that should be on screen, and `sample(cx)`, what `frame` would hand back; `Presence::leaving(content, exit, cx)` makes a thing that is only seen leaving. `ginga_gpui::PresenceList` is a `PresenceList` on the same clock: `sync(items, in_out, cx)`, `place_all(place, transition, cx)` and `frame(window, cx)`, which asks for the next frame while any item arrives, leaves or travels, and applies reduced motion to each way and each travel to a new place; `sample(cx)` is what `frame` would hand back, for an event handler with no window.

When the logic of a motion lives in a crate that knows nothing about GPUI, that crate keeps a `ginga::Motion` and takes the time as an argument; the view supplies it with `ginga_gpui::Clock`.
