//! How long one frame of motion takes, in nanoseconds.
//!
//! ```sh
//! cargo run --release --example bench
//! ```
//!
//! A frame costs one thing while the value moves and another once it is
//! settled, so each case is timed twice: on the frames of its way to the
//! target, and on frames after it settled. A mix of the two would hide a cost
//! behind the share of frames spent at rest.
//!
//! The rounds of all the cases are interleaved and each keeps its best: the
//! machine's other work only ever adds time, and a slow spell of the machine
//! then falls on every case alike.

use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use ginga::{Animatable, Easing, Motion, Spring, Transition, Tween};

/// Frames timed in each round of each case.
const FRAMES: usize = 200_000;
const ROUNDS: u32 = 31;
/// One frame at 60 Hz.
const FRAME: Duration = Duration::from_nanos(16_666_667);

/// A round of a case: times its frames and returns nanoseconds per frame.
type Round = Box<dyn FnMut() -> f64>;

struct Case {
    name: &'static str,
    /// Frames from the target being set until the value is settled.
    frames: Option<u32>,
    moving: Option<Round>,
    settled: Option<Round>,
}

/// Times `frame` over `times`, again and again, for one round.
fn round(times: Vec<Duration>, mut frame: impl FnMut(Duration) + 'static) -> Round {
    Box::new(move || {
        let started = Instant::now();
        for &now in times.iter().cycle().take(FRAMES) {
            frame(black_box(now));
        }
        started.elapsed().as_nanos() as f64 / FRAMES as f64
    })
}

/// A motion from `from` to `to`, timed on the frames of its way there and on
/// frames after it settled.
fn motion<T: Animatable + 'static>(
    name: &'static str,
    from: T,
    to: T,
    transition: impl Into<Transition>,
) -> Case {
    let transition = transition.into();
    let mut motion = Motion::at(from);
    motion.set_target(to, transition, Duration::ZERO);
    let settle = motion.settles_at().unwrap_or_default();
    let frames = settle.as_nanos().div_ceil(FRAME.as_nanos()) as u32;
    // The motion is hidden from the optimizer, as an app's would be: what it
    // knows of one case's motion would otherwise make that case alone cheaper.
    let frame = move |mut motion: Motion<T>| {
        move |now| {
            black_box(black_box(&mut motion).toward(black_box(to), transition, now));
        }
    };
    Case {
        name,
        frames: Some(frames),
        moving: (frames > 0)
            .then(|| round((0..frames).map(|n| FRAME * n).collect(), frame(motion))),
        settled: Some(round(
            (0..60).map(|n| settle + FRAME * n).collect(),
            frame(motion),
        )),
    }
}

/// A motion whose target changes every frame, so it is always moving.
fn retargeted<T: Animatable + 'static>(
    name: &'static str,
    from: T,
    to: T,
    transition: impl Into<Transition>,
) -> Case {
    let transition = transition.into();
    let mut motion = Motion::at(from);
    let mut flip = false;
    let frame = move |now| {
        flip = !flip;
        let target = if flip { to } else { from };
        black_box(black_box(&mut motion).toward(black_box(target), transition, now));
    };
    Case {
        name,
        frames: None,
        moving: Some(round((0..600).map(|n| FRAME * n).collect(), frame)),
        settled: None,
    }
}

fn main() {
    let tween = Tween::new(Duration::from_millis(560), Easing::OutExpo);
    let bezier = Tween::new(
        Duration::from_millis(560),
        Easing::Bezier(0.16, 1.0, 0.3, 1.0),
    );
    let four = ((0.0, 0.0, 0.0, 1.0), (300.0, 80.0, 12.0, 0.0));
    let mut cases = [
        motion("at rest, 1 channel", 0.0, 0.0, Spring::SNAPPY),
        motion("spring, 1 channel", 0.0, 300.0, Spring::SNAPPY),
        motion("spring, 4 channels", four.0, four.1, Spring::SNAPPY),
        motion("bouncy spring, 1 channel", 0.0, 300.0, Spring::BOUNCY),
        motion("bouncy spring, 4 channels", four.0, four.1, Spring::BOUNCY),
        motion(
            "tween out-expo, 2 channels",
            (0.0, 0.0),
            (300.0, 80.0),
            tween,
        ),
        motion(
            "tween bezier, 2 channels",
            (0.0, 0.0),
            (300.0, 80.0),
            bezier,
        ),
        retargeted(
            "spring, 4 channels, new target",
            four.0,
            four.1,
            Spring::SNAPPY,
        ),
    ];
    // Each case's best time, moving and settled; `None` for what it doesn't time.
    let mut best = cases.each_ref().map(|_| [None::<f64>; 2]);
    for _ in 0..ROUNDS {
        for (case, best) in cases.iter_mut().zip(&mut best) {
            for (round, best) in [&mut case.moving, &mut case.settled].into_iter().zip(best) {
                if let Some(round) = round {
                    let ns = round();
                    *best = Some(best.map_or(ns, |old| old.min(ns)));
                }
            }
        }
    }
    println!(
        "{:<34} {:>7} {:>9} {:>8}",
        "ns per frame of", "moving", "settled", "frames"
    );
    for (case, best) in cases.iter().zip(best) {
        let [moving, settled] =
            best.map(|ns| ns.map_or_else(|| "-".to_owned(), |ns| format!("{ns:.1}")));
        let frames = case
            .frames
            .map_or_else(|| "every".to_owned(), |n| n.to_string());
        println!("{:<34} {moving:>7} {settled:>9} {frames:>8}", case.name);
    }
}
