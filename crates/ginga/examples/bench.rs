//! How long one frame of motion takes, in nanoseconds.
//!
//! ```sh
//! cargo run --release --example bench
//! ```

use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use ginga::{Animatable, Easing, Motion, Spring, Transition, Tween};

const FRAMES: u32 = 400_000;
const ROUNDS: u32 = 9;

/// The best of a few rounds: the machine's other work only ever adds time.
fn per_frame<T: Animatable>(
    name: &str,
    from: T,
    to: T,
    transition: impl Into<Transition>,
    retarget_every: u32,
) {
    let transition = transition.into();
    let best = (0..ROUNDS)
        .map(|_| {
            let mut motion = Motion::at(from);
            let started = Instant::now();
            for frame in 0..FRAMES {
                let now = Duration::from_nanos(u64::from(frame % 30_000) * 16_666);
                let target = if (frame / retarget_every).is_multiple_of(2) {
                    to
                } else {
                    from
                };
                black_box(motion.toward(black_box(target), transition, black_box(now)));
            }
            started.elapsed().as_nanos() as f64 / f64::from(FRAMES)
        })
        .fold(f64::MAX, f64::min);
    println!("{name:<44} {best:>7.1} ns per frame");
}

fn main() {
    let tween = Tween::new(Duration::from_millis(560), Easing::OutExpo);
    let bezier = Tween::new(
        Duration::from_millis(560),
        Easing::Bezier(0.16, 1.0, 0.3, 1.0),
    );
    per_frame("at rest, 1 channel", 0.0, 0.0, Spring::SNAPPY, u32::MAX);
    per_frame("spring, 1 channel", 0.0, 300.0, Spring::SNAPPY, 60);
    per_frame("bouncy spring, 1 channel", 0.0, 300.0, Spring::BOUNCY, 60);
    per_frame(
        "spring, 4 channels",
        (0.0, 0.0, 0.0, 1.0),
        (300.0, 80.0, 12.0, 0.0),
        Spring::SNAPPY,
        60,
    );
    per_frame(
        "spring, 4 channels, new target every frame",
        (0.0, 0.0, 0.0, 1.0),
        (300.0, 80.0, 12.0, 0.0),
        Spring::SNAPPY,
        1,
    );
    per_frame(
        "tween out-expo, 2 channels",
        (0.0, 0.0),
        (300.0, 80.0),
        tween,
        60,
    );
    per_frame(
        "tween bezier, 2 channels",
        (0.0, 0.0),
        (300.0, 80.0),
        bezier,
        60,
    );
}
