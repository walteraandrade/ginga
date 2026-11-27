use core::{fmt, time::Duration};

use crate::{Animatable, Spring};

/// How a value let go with a speed slows to a stop: its speed falls by the
/// same share every instant, so it glides and never stops with a jolt.
///
/// The speed falls to about a third (1/e) in each *time constant*; the value
/// comes to rest a time constant's worth of its first speed from where it
/// was let go. Past a limit, or to reach a snap point, the value moves with
/// a spring instead, which [`spring`](Coast::spring) picks.
///
/// ```
/// use std::time::Duration;
/// use ginga::Coast;
///
/// // Let go at 0 with 1,000 px/s: rests 100 px on with QUICK, 500 with SLOW.
/// assert_eq!(Coast::QUICK.rest(0.0, 1_000.0), 100.0);
/// assert_eq!(Coast::SLOW.rest(0.0, 1_000.0), 500.0);
/// let mine = Coast::new(Duration::from_millis(250))?;
/// # let _ = mine;
/// # Ok::<(), ginga::CoastError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coast {
    /// How much of its speed the value loses each second, as a rate: the
    /// inverse of the time constant.
    pub(crate) rate: f32,
    pub(crate) spring: Spring,
}

/// Why a [`Coast`] could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoastError {
    /// The time constant is zero.
    Zero,
}

impl fmt::Display for CoastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => write!(
                f,
                "a coast needs a time constant above zero; to stop at once, use jump_to"
            ),
        }
    }
}

impl core::error::Error for CoastError {}

impl Coast {
    /// Stops soon: a time constant of 0.1 s. For a card thrown aside or a
    /// slider's knob.
    pub const QUICK: Self = Self {
        rate: 10.0,
        spring: Spring::SNAPPY,
    };

    /// Glides far: a time constant of 0.5 s. For a long list.
    pub const SLOW: Self = Self {
        rate: 2.0,
        spring: Spring::SNAPPY,
    };

    /// A coast whose speed falls to about a third each `time_constant`.
    ///
    /// # Errors
    ///
    /// When `time_constant` is zero.
    pub fn new(time_constant: Duration) -> Result<Self, CoastError> {
        if time_constant.is_zero() {
            return Err(CoastError::Zero);
        }
        Ok(Self {
            rate: 1.0 / time_constant.as_secs_f32(),
            spring: Spring::SNAPPY,
        })
    }

    /// The spring that brings the value back inside a limit or onto a snap
    /// point. [`Spring::SNAPPY`] unless set.
    #[must_use]
    pub const fn spring(mut self, spring: Spring) -> Self {
        self.spring = spring;
        self
    }

    /// Where a value let go at `from` with `velocity` (in units per second)
    /// comes to rest: `from + velocity × time constant`, channel by channel.
    #[must_use]
    pub fn rest<T: Animatable>(&self, from: T, velocity: T) -> T {
        let mut rest = from.to_channels();
        for (place, speed) in rest
            .as_mut()
            .iter_mut()
            .zip(velocity.to_channels().as_ref())
        {
            *place += speed / self.rate;
        }
        T::from_channels(rest)
    }

    /// How far a channel still has to go `seconds` after it was let go with
    /// `speed`, and its speed then.
    pub(crate) fn left(self, speed: f32, seconds: f32) -> (f32, f32) {
        let decay = (-self.rate * seconds).exp();
        (speed / self.rate * decay, speed * decay)
    }

    /// How long a channel let go with `speed` takes to come within
    /// `precision` of where it rests.
    pub(crate) fn settle_secs(self, speed: f32, precision: f32) -> f32 {
        let distance = speed.abs() / self.rate;
        if distance <= precision {
            0.0
        } else {
            (distance / precision).ln() / self.rate
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Motion, Trace};

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn thrown(velocity: f32) -> Motion<f32> {
        let mut card = Motion::at(0.0);
        card.coast(velocity, Coast::QUICK, ms(0));
        card
    }

    #[test]
    fn a_value_let_go_keeps_its_speed_and_slows_by_the_same_share_each_instant() {
        let card = thrown(1_000.0);
        assert_eq!(card.sample(ms(0)).value, 0.0);
        assert!((card.velocity(ms(0))[0] - 1_000.0).abs() < 1e-3);
        // One time constant on: 1 - 1/e of the way, at 1/e of the speed.
        let e = core::f32::consts::E;
        assert!((card.sample(ms(100)).value - 100.0 * (1.0 - 1.0 / e)).abs() < 1e-3);
        assert!((card.velocity(ms(100))[0] - 1_000.0 / e).abs() < 1e-2);
        let thrown_back = thrown(-1_000.0);
        assert!((thrown_back.sample(ms(100)).value + 100.0 * (1.0 - 1.0 / e)).abs() < 1e-3);
    }

    #[test]
    fn a_coast_settles_exactly_where_rest_says_once_within_precision() {
        let card = thrown(1_000.0);
        let settle = card.settles_at().unwrap();
        // ln(100 / 0.001) / 10 s: where what is left falls under a thousandth.
        assert_eq!(settle.as_millis(), 1_151);
        let before = card.sample(settle - ms(1));
        assert!(!before.settled);
        assert!(100.0 - before.value < 0.001 + 1e-4);
        let after = card.sample(settle);
        assert!(after.settled);
        assert_eq!(after.value, Coast::QUICK.rest(0.0, 1_000.0));
        assert_eq!(after.value, 100.0);
        assert_eq!(card.target(), 100.0);
    }

    #[test]
    fn a_skipped_frame_changes_nothing() {
        let card = thrown(1_000.0);
        let read_every_frame: Vec<f32> = (0..=20).map(|f| card.sample(ms(f * 16)).value).collect();
        assert_eq!(card.sample(ms(160)).value, read_every_frame[10]);
    }

    #[test]
    fn a_coast_shows_in_a_trace() {
        let trace = Trace::of(&thrown(1_000.0), ms(0), ms(200));
        // 100 × (1 − e^(−t / 0.1 s)), until less than a thousandth is left.
        assert_eq!(
            trace.to_string(),
            "\
ms           value
0            0.000
200         86.466
400         98.168
600         99.752
800         99.966
1000        99.995
1200       100.000  settled
"
        );
    }

    #[test]
    fn a_slow_speed_rests_at_once() {
        let mut still = Motion::at(5.0);
        still.coast(0.005, Coast::QUICK, ms(0));
        let sample = still.sample(ms(0));
        assert!(sample.settled);
        assert_eq!(sample.value, Coast::QUICK.rest(5.0, 0.005));
        assert_eq!(still.settles_at(), None);
    }

    #[test]
    fn a_value_let_go_toward_a_limit_springs_back_to_it_keeping_its_speed() {
        let mut knob = Motion::at(0.0);
        knob.coast_within(3_000.0, Coast::QUICK, 0.0, 50.0, ms(0));
        assert!(
            (knob.velocity(ms(0))[0] - 3_000.0).abs() < 1e-2,
            "no jolt at release"
        );
        let farthest = (0..1_000)
            .map(|t| knob.sample(ms(t)).value)
            .fold(f32::MIN, f32::max);
        assert!(
            farthest > 50.0,
            "thrown hard, it passes the limit: {farthest}"
        );
        let end = knob.sample(knob.settles_at().unwrap());
        assert!(end.settled);
        assert_eq!(end.value, 50.0);
        let mut low = Motion::at(10.0);
        low.coast_within(-1_000.0, Coast::QUICK, 0.0, 50.0, ms(0));
        assert_eq!(low.sample(low.settles_at().unwrap()).value, 0.0);
    }

    #[test]
    fn a_value_let_go_past_a_limit_springs_back_inside() {
        // Dragged 20 past the end, let go still.
        let mut knob = Motion::at(70.0);
        knob.coast_within(0.0, Coast::QUICK, 0.0, 50.0, ms(0));
        assert_eq!(knob.target(), 50.0);
        let halfway = knob.sample(ms(100)).value;
        assert!(halfway > 50.0 && halfway < 70.0, "{halfway}");
        assert_eq!(knob.sample(knob.settles_at().unwrap()).value, 50.0);
    }

    #[test]
    fn a_value_that_rests_inside_its_limits_coasts_as_if_there_were_none() {
        let mut knob = Motion::at(0.0);
        knob.coast_within(300.0, Coast::QUICK, 0.0, 50.0, ms(0));
        let free = thrown(300.0);
        for t in [0, 50, 100, 300] {
            assert_eq!(knob.sample(ms(t)), free.sample(ms(t)));
        }
        assert_eq!(knob.target(), 30.0);
    }

    #[test]
    fn a_value_let_go_comes_to_rest_on_the_nearest_snap_point() {
        let points = [0.0, 80.0, 200.0];
        // It would coast to 100: 80 is nearest.
        let mut card = Motion::at(0.0);
        card.coast_to_nearest(1_000.0, Coast::QUICK, &points, ms(0));
        assert!(
            (card.velocity(ms(0))[0] - 1_000.0).abs() < 1e-3,
            "no jolt at release"
        );
        assert_eq!(card.target(), 80.0);
        assert_eq!(card.sample(card.settles_at().unwrap()).value, 80.0);
        // Thrown hard enough, it reaches the next one.
        card.coast_to_nearest(1_600.0, Coast::QUICK, &points, ms(2_000));
        assert_eq!(card.target(), 200.0);
        // With no points it coasts.
        let mut free = Motion::at(0.0);
        free.coast_to_nearest(1_000.0, Coast::QUICK, &[], ms(0));
        assert_eq!(free.target(), 100.0);
    }

    #[test]
    fn a_coast_with_no_time_constant_does_not_build() {
        let error = Coast::new(Duration::ZERO).unwrap_err();
        assert_eq!(error, CoastError::Zero);
        assert!(error.to_string().contains("jump_to"));
        let quarter = Coast::new(ms(250)).unwrap();
        assert_eq!(quarter.rest(0.0, 100.0), 25.0);
    }

    #[test]
    fn two_channels_coast_each_with_its_own_speed() {
        let mut card = Motion::at((0.0, 0.0));
        card.coast((1_000.0, -200.0), Coast::QUICK, ms(0));
        assert_eq!(card.target(), (100.0, -20.0));
        let end = card.sample(card.settles_at().unwrap());
        assert_eq!(end.value, (100.0, -20.0));
    }
}
