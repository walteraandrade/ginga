# Coming from Motion, SwiftUI or Compose

What you reach for elsewhere, and where it is here. Where the answer is a few lines of code, the table links to them below. Where it says GPUI cannot, that limit is GPUI's, not ginga's.

| You know | In ginga |
| --- | --- |
| Motion's `animate`, SwiftUI's `.animation`, Compose's `animate*AsState` | `ginga_gpui::Motion::toward(target, transition, ..)` in a view, `.look(id, look, transition)` on an element |
| A spring, a tween, `duration`/`bounce`, `stiffness`/`damping`/`mass` | `Spring::new(response).bounce(..)`, `Spring::physical(..)`, `Tween::new(duration, easing)`, `Transition::Instant` |
| `keyframes`, `repeat`, `repeatType: "mirror"`, `steps()` | `Keyframes` with `.times(n)`, `.forever()`, `.mirrored()`, `.pause(..)`; steps: [below](#steps) |
| `useTransform`, `interpolate` | `Map::new(..)?.of(&motion)`, or `Map::new(..)?.clamped().of(&motion)`; through several stops: [below](#through-several-stops) |
| `animateColor`, `animateColorAsState` | a color moves as any value: `ginga_gpui::Motion<Rgba>` or `<Hsla>`, or a look's `background`, `text_color`, `border_color` |
| Variants, `staggerChildren`, `when: "beforeChildren"`, `staggerDirection: -1` | `Variant` with `.stagger(..)`, `Order`, `set_in_order`; backward or from the centre: [below](#a-stagger-backward-or-from-the-centre) |
| `AnimatePresence`, `exit`, `onExitComplete`, `mode="wait"` | `Presence`, `present(..)` with `.leave_to(..)` and `.on_gone(..)`, `Swap::Wait`, `PresenceList` |
| `layout`, `layoutId`, `animateContentSize` | `.glides(..)`, `.shared(key, ..)`, `.opened(id, 1.).height_glides(..)` |
| `whileHover`, `whileTap`, `whileFocus` | `.hover(..)`, `.pressed(..)`, `.focused(..)` |
| `drag`, `dragConstraints`, a fling | `.draggable(..)` with `.within(..)`, and `Coast`. While held it stays under the pointer, past a limit too, and springs back when let go; there is no `dragElastic` |
| `useScroll`, a scroll that animates | a `Map` of the scroll position, or a `Motion` written into the `ScrollHandle`: [below](#from-the-scroll-position) |
| `useReducedMotion`, `MotionConfig reducedMotion` | GPUI's `reduce_motion` flag, read for you (`unless_reduced`). The pinned GPUI does not fill it from the desktop, so the app sets it: [below](#reduced-motion-on-purpose) |
| A test of an animation | `sample(now)` and `Trace::of(..)`, read at given times: [below](#writing-it-down) |
| `onAnimationComplete`, `.then` | `Sample::settled`, `settles_at()`, `SettledOnce` |
| GSAP's timeline, Motion's `animate([...])` sequences | `Sequence` with `then`, `with`, `at`; pause, seek, speed, reverse: [below](#pause-seek-speed-reverse) |
| `scale`, `rotate`, `transform` on a `div` | GPUI cannot: a `div` has no transform. Change its size or its offset; an icon turns with its own transformation: [below](#an-icon-that-turns) |

## Steps

Two stops at the same time are a jump. A caret that blinks:

```rust
use std::time::Duration;
use ginga::{Easing, Keyframes, Stop};

let ms = Duration::from_millis;
let caret = Keyframes::new([
    Stop::new(ms(0), 1.0, Easing::Linear),
    Stop::new(ms(500), 1.0, Easing::Linear),
    Stop::new(ms(500), 0.0, Easing::Linear), // the jump
    Stop::new(ms(1_000), 0.0, Easing::Linear),
])?
.forever();
assert_eq!(caret.sample(ms(499)).value, 1.0);
assert_eq!(caret.sample(ms(500)).value, 1.0); // at the jump: still the earlier stop
assert_eq!(caret.sample(ms(501)).value, 0.0); // then the later one
assert_eq!(caret.sample(ms(1_499)).value, 1.0); // and again, forever
# Ok::<(), ginga::KeyframesError>(())
```

## Through several stops

A value through several stops, as `useTransform` with arrays: keyframes read at a number, as a time.

```rust
use std::time::Duration;
use ginga::{Easing, Keyframes, Stop};

// The number becomes a time, which cannot be negative or NaN: hold it to 0..=1
// first, since a bouncy spring passes both ends a little.
let at = |x: f32| Duration::from_secs_f32(x.max(0.0).min(1.0));
// Scroll progress 0 → 0.5 → 1 gives opacity 0 → 1 → 0.
let fade = Keyframes::new([
    Stop::new(at(0.0), 0.0, Easing::Linear),
    Stop::new(at(0.5), 1.0, Easing::Linear),
    Stop::new(at(1.0), 0.0, Easing::Linear),
])?;
assert_eq!(fade.sample(at(0.75)).value, 0.5);
assert_eq!(fade.sample(at(-0.04)).value, 0.0); // past the start: held there
# Ok::<(), ginga::KeyframesError>(())
```

## A stagger backward or from the centre

A child's place is a number you give, so the order is yours. Backward on the way back:

```rust,ignore
row.follows(&self.menu, if open { n } else { last - n })
```

Or out from the middle:

```rust,ignore
row.follows(&self.menu, n.abs_diff(mid))
```

## From the scroll position

A value from the scroll position is a `Map` of where the scroll is. A scroll that glides is a `Motion` written into the `ScrollHandle` until it settles, and given up when the user scrolls.

```rust,ignore
let (offset, max) = (self.scroll.offset(), self.scroll.max_offset());
let progress = (-offset.y / max.y.max(px(1.))).clamp(0., 1.);
// `self.header` is a `Map::new(0., 1., 64., 40.)?.clamped()`, built once in `new`.
let header = self.header.at(progress);

// A scroll that glides to `target`: `self.glide` is an `Option<Motion<f32>>`.
// It starts from where the scroll is, not where an old glide left it:
//     self.glide = Some(Motion::at(-self.scroll.offset().y.as_f32()));
if let Some(glide) = &mut self.glide {
    let y = glide.toward(target, Spring::SMOOTH, window, cx);
    self.scroll.set_offset(point(px(0.), px(-y)));
    if y == target {
        self.glide = None; // settled, so exactly there: the scroll is the user's again
    }
}
// and on the scroll area, the user's own scroll gives it up:
.on_scroll_wheel(cx.listener(|this, _, _, _| this.glide = None))
```

## An icon that turns

A `div` cannot rotate, but an `svg` can: give its transformation a motion. The gallery's turn page does this.

```rust,ignore
let turned = self.turned.toward(if open { 0.25 } else { 0. }, Spring::SNAPPY, window, cx);
svg()
    .path("icons/chevron.svg") // from the app's `AssetSource`
    .size(px(24.))
    .text_color(rgb(0xe0e2e6)) // an svg draws only in its own text color
    .with_transformation(Transformation::rotate(radians(turned * TAU)))
```

## Writing it down

A presence, a list or a slot is read with `sample(now)`, so a test writes down what it would draw:

```rust
use std::time::Duration;
use ginga::{Easing, InOut, Presence, PresenceList, Tween};

let ms = Duration::from_millis;
let mut toast = Presence::absent();
toast.show("Saved", Tween::new(ms(100), Easing::Linear), ms(0));
let line = |t| toast.sample(ms(t)).map(|s| format!("{} {:?} {:.2}", s.content, s.phase, s.progress));
assert_eq!(line(50).as_deref(), Some("Saved Entering 0.50"));

// A list gives its items in order.
let mut rows = PresenceList::new();
let fade = Tween::new(ms(100), Easing::Linear);
rows.sync([(1, "one"), (2, "two")], InOut::new(fade, fade), ms(0));
rows.sync([(1, "one")], InOut::new(fade, fade), ms(200));
let lines: Vec<_> = rows.sample(ms(250)).map(|s| format!("{} {:?}", s.content, s.phase)).collect();
assert_eq!(lines, ["one Present", "two Exiting"]);
```

## Reduced motion on purpose

The adapter reads GPUI's own flag, `cx.reduce_motion()`. GPUI 0.3.12 starts it off and does not read the desktop's preference, so an app sets it, from its own setting or from the desktop's, and a test sets it the same way: `cx.set_reduce_motion(true)`. Every motion, look and presence in the adapter then keeps its delay and drops its motion; a variant's or a sequence's schedule and a shared element's trip are dropped whole.

## Pause, seek, speed, reverse

They are the time you pass. Nothing runs between frames, so a motion read at another time is there. This holds for the engine's motions and `Sequence`; the adapter's `Motion`, `Presence` and `PresenceList` read the frame's clock themselves. To pause or scrub in a view, keep engine motions, hand `sample` your own time made from `Clock::now`, and ask for frames while it plays.

```rust
use std::time::Duration;
use ginga::{Easing, Motion, Tween};

let ms = Duration::from_millis;
let mut bar = Motion::at(0.0);
bar.set_target(100.0, Tween::new(ms(100), Easing::Linear), ms(0));
let paused_at = ms(25);
assert_eq!(bar.sample(paused_at).value, 25.0); // paused: keep reading one time
assert_eq!(bar.sample(ms(75)).value, 75.0); // seek: read another
assert_eq!(bar.sample(ms(25) * 2).value, 50.0); // twice as fast: 25 ms in
assert_eq!(bar.sample(ms(100) - ms(25)).value, 75.0); // backward: 25 ms in
```
