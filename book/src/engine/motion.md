# A value that moves

| Call | What it does |
| --- | --- |
| `Motion::at(value)` | A value at rest. |
| `motion.toward(target, transition, now)` | Each frame: head for a target, get the value now. |
| `motion.set_target(target, transition, now)` | Change course at a moment you choose. |
| `motion.jump_to(value)` | Move with no motion. |
| `motion.sample(now)` | Read the value at any time, without changing anything. |

A transition is `Spring::SNAPPY`, `Spring::SMOOTH`, `Spring::BOUNCY`, a `Spring::new(response)`, a `Tween::new(duration, easing)` or `Transition::Instant`. A spring given the way Motion, react-spring or a design spec gives it, as a stiffness, a damping and a mass, is `Spring::physical(stiffness, damping, mass)?`: the same spring, at the same cost.

```rust
use std::time::Duration;
use ginga::{Motion, Spring};

let ms = Duration::from_millis;
// Motion's default spring: stiffness 100, damping 10, mass 1.
let spring = Spring::physical(100.0, 10.0, 1.0)?;
let mut x = Motion::at(0.0);
x.set_target(100.0, spring, ms(0));
assert!(x.sample(ms(400)).value > 100.0); // it passes its target, as a damping ratio of 0.5 does
assert!(Spring::physical(100.0, 0.0, 1.0).is_err()); // with no damping it would never stop
# Ok::<(), ginga::SpringError>(())
```

Anything `Animatable` can move: `f32`, tuples of up to four `f32`, `[f32; N]`, a `Color`, and your own types.

```rust
use std::time::Duration;
use ginga::{Motion, Spring};

let ms = Duration::from_millis;
let mut width = Motion::at(180.0);

// Frame at 0 ms: the panel should be 320 wide.
width.toward(320.0, Spring::SNAPPY, ms(0));

// Frame at 100 ms: on its way.
let sample = width.toward(320.0, Spring::SNAPPY, ms(100));
assert!(sample.value > 180.0 && sample.value < 320.0);
assert!(!sample.settled); // draw another frame

// Frame at 100 ms, the panel closes: the spring keeps its speed and turns around.
width.toward(180.0, Spring::SNAPPY, ms(100));
```

`now` is a `Duration` since any fixed moment you choose. The engine never reads a clock.

## One after another

Any transition can wait before it starts: `.delay(d)` on a spring, a tween or a `Transition`. Until then the value holds still where it is. To let a group arrive in turn, give each item a delay one step longer than the one before, up to a cap:

```rust
use std::time::Duration;
use ginga::{Motion, Spring};

let ms = Duration::from_millis;
let mut bars = [Motion::at(0.0); 5];

for (n, bar) in (0..).zip(&mut bars) {
    let delay = (ms(40) * n).min(ms(120)); // 40 ms apart, never more than 120 ms late
    bar.set_target(1.0, Spring::SMOOTH.delay(delay), ms(0));
}

let started: Vec<bool> = bars.iter().map(|bar| bar.sample(ms(60)).value > 0.0).collect();
assert_eq!(started, [true, true, false, false, false]);
```

The same works for things that come and go: `rows[n].show(row, fade.delay(step * n), now)`.

## Through stops, and again

`Keyframes` go through several stops, each at its time with its own easing, and can repeat: `.times(n)` or `.forever()`, `.mirrored()` to go back and forth, `.pause(d)` between rounds, `.starting_at(t)` to start later, `.behind(lag)` to play a lag behind copies of the same loop, partway through a round from the start. Like a motion, they are read at any time with `sample(now)`; `rest()` is where they rest once played.

```rust
use std::time::Duration;
use ginga::{Easing, Keyframes, Stop};

let ms = Duration::from_millis;
let breath = Keyframes::new([
    Stop::new(ms(0), 0.4, Easing::Linear),
    Stop::new(ms(450), 1.0, Easing::InOutSine), // in
    Stop::new(ms(600), 1.0, Easing::Linear),    // and hold
])?
.times(4)
.mirrored()
.pause(ms(100));

assert_eq!(breath.sample(ms(600)).value, 1.0); // the first round's end
assert_eq!(breath.sample(ms(1_300)).value, 0.4); // back out, the second round
assert_eq!(breath.settles_at(), Some(ms(2_700)));
# Ok::<(), ginga::KeyframesError>(())
```

Keyframes that repeat forever are never settled, and their `settles_at` is `None`: whoever draws them asks for frames as long as they are drawn. Stops that are not in order, or a first stop not at zero, do not build. The stops are an array, so keyframes are plain data and reading them never allocates.

## Let go with a speed

`motion.coast(velocity, coast, now)` lets a value go with a speed, in units per second: it keeps that speed at first and slows to a stop by the same share every instant, so it glides and never stops with a jolt. A `Coast` is `Coast::QUICK` (a time constant of 0.1 s), `Coast::SLOW` (0.5 s) or `Coast::new(time_constant)`, and the value rests a time constant's worth of its first speed on: `coast.rest(from, velocity)`. `coast_within(velocity, coast, min, max, now)` comes to rest inside limits, and a value that would pass one, or is past it already, moves with the coast's spring to it instead; `coast_to_nearest(velocity, coast, &points, now)` comes to rest on the nearest snap point the same way. Each keeps the speed it was let go with, and is settled exactly where it rests. `.spring(Spring::BOUNCY)` on a coast makes a wall bouncier. A `Trail` gives the speed a value is let go with: `trail.note(now, value)` as it is moved by hand, then `trail.speed(now)`, over the last 100 ms, and zero if it was held still for 50 ms.

```rust
use std::time::Duration;
use ginga::{Coast, Motion};

let ms = Duration::from_millis;
let mut card = Motion::at(0.0);
card.coast(1_000.0, Coast::QUICK, ms(0)); // thrown at 1,000 px/s
assert_eq!(card.target(), 100.0);         // rests 100 px on
assert!((card.sample(ms(100)).value - 63.2).abs() < 0.1); // 1 - 1/e of the way after 0.1 s

let mut knob = Motion::at(0.0);
knob.coast_within(3_000.0, Coast::QUICK, 0.0, 200.0, ms(0)); // it would pass 200
assert_eq!(knob.sample(knob.settles_at().unwrap()).value, 200.0);
```

## Values from values, and steps in order

A `Map` derives a value from a number: `Map::new(from_start, from_end, to_start, to_end)?` maps one range onto a range of anything that moves, such as a progress into a width, or a width into a color. Past an end it goes on in the same line, as a spring's overshoot should; `.clamped()` holds it to the ends. `map.at(number)` maps one number, and `map.of(&motion)` reads it from a motion each frame: it samples and writes down as a `Trace` like a motion, and reading it allocates nothing. A value that *follows* another with a spring needs nothing new: hand the leader's value to `follower.toward(..)` on every frame.

A `Sequence` starts several motions as one: `then` starts a step when the one before ends, `with` together with it, and `at(time, ..)` at a set time, such as 120 ms before the one before ends. Each step sets its motion's target once, with a delay; nothing runs between frames. Each call returns the `Step`'s start and end, and `settles_at()` says when the last one ends. Add every step when the sequence starts, and give each motion one step. `SettledOnce` says `true` once, on the first frame at or after a motion's or a sequence's end: for code that runs once at the end. A motion's end moves with a new target, so the old end never comes and the new one is reported instead; a target taken at once, as under reduced motion, ends when it is set, or when its delay ends. A sequence's end is the time it was given and does not move: to wait for a new end, start another sequence. A step is a `Motion` with a transition or a `LookMotion` with `Transitions` (both are `Retarget`), so a look can follow a value: for a look, the step starts when the first of its values starts. In a GPUI view, a sequence moves engine motions (`ginga::Motion`, also reachable as `ginga_gpui::ginga::Motion`) and looks, read with the view's `ginga_gpui::Clock`, as any motion the view's own logic keeps. Start it with `ginga_gpui::sequence(now, cx)`: under reduced motion it is `Sequence::without_motion`, with no motion and no schedule, so each step is at its target once its own delay is over. Use one `SettledOnce` for each thing you wait for: two ends at the same time are one end to it.

```rust
use std::time::Duration;
use ginga::{Color, Easing, Look, LookMotion, Map, Motion, Sequence, SettledOnce, Transitions, Tween};

let ms = Duration::from_millis;
let fade = Tween::new(ms(100), Easing::Linear);

// A width from a progress, and a color from the width, held to its ends.
let mut progress = Motion::at(0.0);
progress.set_target(1.0, fade, ms(0));
let width = Map::new(0.0, 1.0, 40.0, 240.0)?;
let fill = Map::new(40.0, 240.0, Color::hex(0xe5484d), Color::hex(0x30a46c))?.clamped();
let now = width.of(&progress).sample(ms(50)).value;
assert_eq!(now, 140.0);
assert_eq!(fill.at(300.0), Color::hex(0x30a46c));

// Three steps: one after another, the last a look overlapping by 40 ms.
let (mut title, mut body) = (Motion::at(0.0), Motion::at(0.0));
let mut button = LookMotion::at(Look::new().offset(0.0, 24.0).opacity(0.0));
let mut entrance = Sequence::starting_at(ms(0));
entrance.then(&mut title, 1.0, fade);
let second = entrance.then(&mut body, 1.0, fade);
let here = Look::new().offset(0.0, 0.0).opacity(1.0);
let third = entrance.at(second.ends.saturating_sub(ms(40)), &mut button, here, Transitions::new().all(fade));
assert_eq!(third.to_string(), "160 ms to 260 ms");
assert_eq!(button.sample(ms(210)).value, Look::new().offset(0.0, 12.0).opacity(0.5));

// Without motion, as under reduced motion: each step is there after its own delay.
let mut still = Sequence::starting_at(ms(0)).without_motion();
still.then(&mut Motion::at(0.0), 1.0, fade);
assert_eq!(still.settles_at(), ms(0));

let mut done = SettledOnce::default();
assert!(!done.check(Some(entrance.settles_at()), ms(250)));
assert!(done.check(Some(entrance.settles_at()), ms(266)));
# Ok::<(), ginga::MapError>(())
```
