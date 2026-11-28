# A loop in a view

`ginga_gpui::Keyframes` are keyframes a view owns. They start on the first frame that draws them, or `starting_at(d)` after it, `restart(cx)` plays them again, `behind(lag)` plays them `lag` behind copies without it, partway through a round from the first frame, so staggered dots share one loop, and they rest where they end under reduced motion (`rest()`: a loop on its first stop). A slow loop need not be drawn at the display's rate: `.at_most_every(d)` asks for the next frame no sooner than `d` after this one. It holds back only its own requests, so draw the loop in a view of its own, such as an `Island`, or another motion in the view draws it on every frame anyway. The cap is per set of keyframes, not per view, as GPUI's `Animation::with_max_fps` is per element: two capped sets in one view that started apart can draw the view up to once each per `d`, so values that should share a rate share one set.

```rust,ignore
// Built once:
let breath = Keyframes::new([
    Stop::new(ms(0), 0.4, Easing::Linear),
    Stop::new(ms(600), 1.0, Easing::InOutSine),
])?
.forever()
.mirrored()
.at_most_every(ms(30)); // about 30 frames a second at 60 Hz

// While rendering:
let opacity = self.breath.frame(window, cx);
mark().opacity(opacity)
```
