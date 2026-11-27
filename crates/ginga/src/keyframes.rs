use core::{fmt, time::Duration};

use crate::{Animatable, Easing, Sample};

/// One stop of [`Keyframes`]: the value at a time, and the easing that
/// arrives there from the stop before.
#[derive(Debug, Clone, Copy)]
pub struct Stop<T> {
    /// When the value is here, counted from the start of a round.
    pub at: Duration,
    /// The value at that time.
    pub value: T,
    /// How the value arrives here from the stop before. The first stop's
    /// easing is not used.
    pub easing: Easing,
}

impl<T> Stop<T> {
    /// A stop: `value` at `at`, arrived at with `easing`.
    #[must_use]
    pub const fn new(at: Duration, value: T, easing: Easing) -> Self {
        Self { at, value, easing }
    }
}

/// Why [`Keyframes`] could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyframesError {
    /// There are no stops.
    NoStops,
    /// The first stop is not at zero.
    FirstNotAtZero(Duration),
    /// A stop comes before the one listed ahead of it.
    OutOfOrder {
        /// The position of the stop in the list.
        index: usize,
        /// Its time.
        at: Duration,
        /// The time of the stop before it.
        before: Duration,
    },
}

impl fmt::Display for KeyframesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoStops => write!(
                f,
                "keyframes need at least one stop; for a value at rest use Motion::at"
            ),
            Self::FirstNotAtZero(at) => write!(
                f,
                "the first stop must be at zero, got {at:?}; to wait before the motion, start it later with `starting_at`"
            ),
            Self::OutOfOrder { index, at, before } => write!(
                f,
                "stop {index} is at {at:?}, before the stop ahead of it at {before:?}; list the stops in order of time"
            ),
        }
    }
}

impl core::error::Error for KeyframesError {}

/// How many rounds keyframes play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rounds {
    Times(u32),
    Forever,
}

/// A motion through several stops, each at its time with its own easing, that
/// can repeat.
///
/// The stops are kept in an array of `N`, so keyframes are plain data, copied
/// and read without allocating. Like a [`Motion`](crate::Motion), they are
/// read at any time and answer with the value then, computed from their start.
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, Keyframes, Stop};
///
/// let ms = Duration::from_millis;
/// // Up to 1 and back to 0 in 200 ms, three times.
/// let pulse = Keyframes::new([
///     Stop::new(ms(0), 0.0, Easing::Linear),
///     Stop::new(ms(100), 1.0, Easing::Linear),
///     Stop::new(ms(200), 0.0, Easing::Linear),
/// ])?
/// .times(3);
///
/// assert_eq!(pulse.sample(ms(50)).value, 0.5);
/// assert_eq!(pulse.sample(ms(350)).value, 0.5); // the second round, halfway down
/// assert!(pulse.sample(ms(600)).settled);
/// # Ok::<(), ginga::KeyframesError>(())
/// ```
///
/// Keyframes that repeat forever are never settled, and
/// [`settles_at`](Keyframes::settles_at) is `None`: whoever draws them asks
/// for frames for as long as they are drawn.
#[derive(Debug, Clone, Copy)]
pub struct Keyframes<T: Animatable, const N: usize> {
    stops: [Stop<T>; N],
    rounds: Rounds,
    mirrored: bool,
    pause: Duration,
    start: Duration,
}

impl<T: Animatable, const N: usize> Keyframes<T, N> {
    /// Keyframes through `stops`, played once from time zero.
    ///
    /// # Errors
    ///
    /// When there are no stops, when the first is not at zero, or when a stop
    /// is listed before one at an earlier time. Two stops at the same time are
    /// a jump.
    pub fn new(stops: [Stop<T>; N]) -> Result<Self, KeyframesError> {
        let first = stops.first().ok_or(KeyframesError::NoStops)?;
        if !first.at.is_zero() {
            return Err(KeyframesError::FirstNotAtZero(first.at));
        }
        for (index, pair) in stops.windows(2).enumerate() {
            if pair[1].at < pair[0].at {
                return Err(KeyframesError::OutOfOrder {
                    index: index + 1,
                    at: pair[1].at,
                    before: pair[0].at,
                });
            }
        }
        Ok(Self {
            stops,
            rounds: Rounds::Times(1),
            mirrored: false,
            pause: Duration::ZERO,
            start: Duration::ZERO,
        })
    }

    /// Plays `rounds` rounds in all. Zero rounds rest on the first stop.
    #[must_use]
    pub const fn times(mut self, rounds: u32) -> Self {
        self.rounds = Rounds::Times(rounds);
        self
    }

    /// Plays round after round, forever.
    #[must_use]
    pub const fn forever(mut self) -> Self {
        self.rounds = Rounds::Forever;
        self
    }

    /// Plays every second round backward, from the last stop to the first,
    /// so the rounds go back and forth.
    #[must_use]
    pub const fn mirrored(mut self) -> Self {
        self.mirrored = true;
        self
    }

    /// Holds still for `pause` between one round and the next.
    #[must_use]
    pub const fn pause(mut self, pause: Duration) -> Self {
        self.pause = pause;
        self
    }

    /// Starts at `start` instead of at zero. Before then the value rests on
    /// the first stop.
    #[must_use]
    pub const fn starting_at(mut self, start: Duration) -> Self {
        self.start = start;
        self
    }

    /// The value at `now`. It never changes the keyframes.
    #[must_use]
    pub fn sample(&self, now: Duration) -> Sample<T> {
        let round = self.round_length();
        let since = now.saturating_sub(self.start);
        let done = self.settles_at().is_some_and(|end| now >= end);
        if done || round.is_zero() {
            return Sample {
                value: self.last_value(),
                settled: self.rounds != Rounds::Forever || round.is_zero(),
            };
        }
        let period = round + self.pause;
        let (index, within) = split(since, period);
        // In the pause, the value holds where the round ended.
        let within = within.min(round);
        let backward = self.mirrored && index % 2 == 1;
        let at = if backward { round - within } else { within };
        Sample {
            value: self.value_at(at),
            settled: false,
        }
    }

    /// When the keyframes have played all their rounds, or `None` when they
    /// repeat forever.
    #[must_use]
    pub fn settles_at(&self) -> Option<Duration> {
        let Rounds::Times(rounds) = self.rounds else {
            return None;
        };
        if rounds == 0 {
            return Some(self.start);
        }
        let played = self.round_length() * rounds + self.pause * (rounds - 1);
        Some(self.start.saturating_add(played))
    }

    /// How long one round takes: the time of the last stop.
    fn round_length(&self) -> Duration {
        self.stops.last().map_or(Duration::ZERO, |stop| stop.at)
    }

    /// Where the keyframes rest once played: the last stop, or the first when
    /// the last round played backward or there was none.
    fn last_value(&self) -> T {
        let first = self.stops[0].value;
        let last = self.stops[N - 1].value;
        match self.rounds {
            Rounds::Times(0) => first,
            Rounds::Times(rounds) if self.mirrored && rounds % 2 == 0 => first,
            _ => last,
        }
    }

    /// The value `at` into a round played forward.
    fn value_at(&self, at: Duration) -> T {
        // The first stop at or after `at`; the one before it is where the
        // segment starts.
        let to = self
            .stops
            .iter()
            .position(|stop| stop.at >= at)
            .unwrap_or(N - 1);
        if to == 0 {
            return self.stops[0].value;
        }
        let (from, to) = (&self.stops[to - 1], &self.stops[to]);
        let length = (to.at - from.at).as_secs_f32();
        let progress = if length > 0.0 {
            (at - from.at).as_secs_f32() / length
        } else {
            1.0
        };
        let eased = to.easing.at(progress.clamp(0.0, 1.0));
        let mut value = from.value.to_channels();
        for (place, target) in value
            .as_mut()
            .iter_mut()
            .zip(to.value.to_channels().as_ref())
        {
            *place += (target - *place) * eased;
        }
        T::from_channels(value)
    }
}

/// How many whole `period`s fit in `since`, and what is left.
fn split(since: Duration, period: Duration) -> (u128, Duration) {
    let period = period.as_nanos();
    let since = since.as_nanos();
    let left = u64::try_from(since % period).unwrap_or(u64::MAX);
    (since / period, Duration::from_nanos(left))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// 0 to 100 in 100 ms, then to 50 in 100 ms more, all linear.
    fn up_and_half_back() -> Keyframes<f32, 3> {
        Keyframes::new([
            Stop::new(ms(0), 0.0, Easing::Linear),
            Stop::new(ms(100), 100.0, Easing::Linear),
            Stop::new(ms(200), 50.0, Easing::Linear),
        ])
        .unwrap()
    }

    fn values(keyframes: &Keyframes<f32, 3>, times: &[u64]) -> Vec<(f32, bool)> {
        times
            .iter()
            .map(|&t| {
                let sample = keyframes.sample(ms(t));
                (sample.value, sample.settled)
            })
            .collect()
    }

    #[test]
    fn keyframes_pass_each_stop_at_its_time() {
        assert_eq!(
            values(&up_and_half_back(), &[0, 50, 100, 150, 199, 200, 900]),
            [
                (0.0, false),
                (50.0, false),
                (100.0, false),
                (75.0, false),
                (50.5, false),
                (50.0, true),
                (50.0, true)
            ]
        );
    }

    #[test]
    fn each_stop_has_its_own_easing() {
        let keyframes = Keyframes::new([
            Stop::new(ms(0), 0.0, Easing::Linear),
            Stop::new(ms(100), 100.0, Easing::Custom(|t| t * t)),
        ])
        .unwrap();
        assert_eq!(keyframes.sample(ms(50)).value, 25.0);
    }

    #[test]
    fn rounds_start_over_and_the_last_one_ends_on_the_last_stop() {
        let keyframes = up_and_half_back().times(2);
        assert_eq!(keyframes.settles_at(), Some(ms(400)));
        assert_eq!(
            values(&keyframes, &[250, 300, 399, 400]),
            [(50.0, false), (100.0, false), (50.5, false), (50.0, true)]
        );
    }

    #[test]
    fn mirrored_rounds_go_back_and_forth_and_an_even_count_ends_on_the_first_stop() {
        let keyframes = up_and_half_back().times(2).mirrored();
        assert_eq!(
            values(&keyframes, &[200, 250, 300, 350, 400]),
            [
                (50.0, false),
                (75.0, false),
                (100.0, false),
                (50.0, false),
                (0.0, true)
            ]
        );
    }

    #[test]
    fn a_pause_holds_where_the_round_ended_and_comes_only_between_rounds() {
        let keyframes = up_and_half_back().times(2).pause(ms(100));
        assert_eq!(keyframes.settles_at(), Some(ms(500)));
        assert_eq!(
            values(&keyframes, &[200, 250, 299, 300, 400, 500]),
            [
                (50.0, false),
                (50.0, false),
                (50.0, false),
                (0.0, false),
                (100.0, false),
                (50.0, true)
            ]
        );
    }

    #[test]
    fn keyframes_that_repeat_forever_are_never_settled() {
        let keyframes = up_and_half_back().forever().mirrored();
        assert_eq!(keyframes.settles_at(), None);
        let year = 365 * 24 * 3_600 * 1_000;
        let later = keyframes.sample(ms(year + 50));
        assert!(!later.settled);
        // A year is a whole number of 200 ms rounds, and an even one: forward.
        assert_eq!(later.value, 50.0);
    }

    #[test]
    fn before_its_start_it_rests_on_the_first_stop_without_being_settled() {
        let keyframes = up_and_half_back().starting_at(ms(1_000));
        assert_eq!(
            values(&keyframes, &[0, 1_050]),
            [(0.0, false), (50.0, false)]
        );
    }

    #[test]
    fn two_stops_at_one_time_are_a_jump() {
        let keyframes = Keyframes::new([
            Stop::new(ms(0), 0.0, Easing::Linear),
            Stop::new(ms(100), 0.0, Easing::Linear),
            Stop::new(ms(100), 100.0, Easing::Linear),
            Stop::new(ms(200), 100.0, Easing::Linear),
        ])
        .unwrap();
        assert_eq!(keyframes.sample(ms(99)).value, 0.0);
        assert_eq!(keyframes.sample(ms(101)).value, 100.0);
    }

    #[test]
    fn a_single_stop_is_a_value_at_rest_even_forever() {
        let keyframes = Keyframes::new([Stop::new(ms(0), 7.0, Easing::Linear)])
            .unwrap()
            .forever();
        let sample = keyframes.sample(ms(500));
        assert_eq!((sample.value, sample.settled), (7.0, true));
    }

    #[test]
    fn stops_out_of_order_do_not_build_and_say_what_to_do() {
        let error = Keyframes::new([
            Stop::new(ms(0), 0.0, Easing::Linear),
            Stop::new(ms(200), 1.0, Easing::Linear),
            Stop::new(ms(100), 0.0, Easing::Linear),
        ])
        .unwrap_err();
        assert_eq!(
            error,
            KeyframesError::OutOfOrder {
                index: 2,
                at: ms(100),
                before: ms(200)
            }
        );
        assert!(
            error
                .to_string()
                .contains("list the stops in order of time")
        );
        let late = Keyframes::new([Stop::new(ms(10), 0.0, Easing::Linear)]).unwrap_err();
        assert!(late.to_string().contains("starting_at"));
        let none = Keyframes::<f32, 0>::new([]).unwrap_err();
        assert!(none.to_string().contains("Motion::at"));
    }
}
