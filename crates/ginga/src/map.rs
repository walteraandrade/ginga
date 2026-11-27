use core::{fmt, time::Duration};

use crate::{Animatable, Motion, Sample, trace::Row};

/// A value computed from another: a number in one range, mapped into a
/// range of any value that can move, such as a progress from 0 to 1 into a
/// width or into a color.
///
/// ```
/// use ginga::{Color, Map};
///
/// let width = Map::new(0.0, 1.0, 40.0, 240.0)?;
/// assert_eq!(width.at(0.5), 140.0);
/// assert_eq!(width.at(1.5), 340.0); // past the end, it goes on
/// assert_eq!(width.clamped().at(1.5), 240.0); // clamped, it stops at the end
///
/// let fill = Map::new(40.0, 240.0, Color::hex(0xe5484d), Color::hex(0x30a46c))?;
/// assert_eq!(fill.at(240.0), Color::hex(0x30a46c));
/// # Ok::<(), ginga::MapError>(())
/// ```
///
/// Each channel is mapped on its own, in a straight line: a color mixes in
/// linear light, as when it moves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Map<T: Animatable> {
    from: (f32, f32),
    to: (T, T),
    /// The ends' channels, worked out once: a color's are not cheap.
    channels: (T::Channels, T::Channels),
    clamped: bool,
}

/// Why a [`Map`] could not be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    /// The range mapped from starts and ends at the same number.
    Empty,
    /// An end of either range is not a finite number.
    NotFinite,
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(
                f,
                "a map needs a range with two different ends, such as 0.0 and 1.0"
            ),
            Self::NotFinite => write!(
                f,
                "a map needs finite ends, such as 0.0 and 1.0 mapped onto 40.0 and 240.0"
            ),
        }
    }
}

impl core::error::Error for MapError {}

impl<T: Animatable> Map<T> {
    /// Maps `from_start..from_end` onto `to_start..to_end`: `from_start`
    /// gives `to_start`, `from_end` gives `to_end`, and a number between gives
    /// a value as far between. Either range may run backwards.
    ///
    /// # Errors
    ///
    /// [`MapError::Empty`] when `from_start` and `from_end` are the same,
    /// and [`MapError::NotFinite`] when any end of either range, or any
    /// channel of one, is not finite.
    pub fn new(from_start: f32, from_end: f32, to_start: T, to_end: T) -> Result<Self, MapError> {
        if !from_start.is_finite() || !from_end.is_finite() {
            return Err(MapError::NotFinite);
        }
        let channels = (to_start.to_channels(), to_end.to_channels());
        let finite = |channels: &T::Channels| channels.as_ref().iter().all(|c| c.is_finite());
        if !finite(&channels.0) || !finite(&channels.1) {
            return Err(MapError::NotFinite);
        }
        if from_start == from_end {
            return Err(MapError::Empty);
        }
        if !(from_end - from_start).is_finite() {
            return Err(MapError::NotFinite);
        }
        Ok(Self {
            from: (from_start, from_end),
            to: (to_start, to_end),
            channels,
            clamped: false,
        })
    }

    /// The same map, held to its ends: a number past an end gives that end's
    /// value. Without it, a number past an end goes on in the same line, as a
    /// spring's overshoot should.
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            clamped: true,
            ..self
        }
    }

    /// The value for `number`. NaN gives the start. An infinite number
    /// gives the end it passes when the map is clamped, and the start when
    /// it is not, as there is no value that far.
    #[must_use]
    pub fn at(&self, number: f32) -> T {
        let (start, end) = self.from;
        let mut part = (number - start) / (end - start);
        if part.is_nan() {
            part = 0.0;
        }
        if self.clamped {
            part = part.clamp(0.0, 1.0);
        } else if !part.is_finite() {
            part = 0.0;
        }
        // The ends come back exactly as they were given.
        if part == 0.0 {
            return self.to.0;
        }
        if part == 1.0 {
            return self.to.1;
        }
        let mut channels = self.channels.0;
        for (channel, to) in channels.as_mut().iter_mut().zip(self.channels.1.as_ref()) {
            *channel += (to - *channel) * part;
        }
        T::from_channels(channels)
    }

    /// This map read from `motion` at each time: a value derived from a
    /// motion, which can be sampled and written down as a [`Trace`](crate::Trace).
    #[must_use]
    pub fn of<'a>(&'a self, motion: &'a Motion<f32>) -> Mapped<'a, T> {
        Mapped { map: self, motion }
    }
}

/// A [`Map`] read from a motion: made by [`Map::of`].
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, Map, Motion, Tween};
///
/// let ms = Duration::from_millis;
/// let mut progress = Motion::at(0.0);
/// progress.set_target(1.0, Tween::new(ms(100), Easing::Linear), ms(0));
/// let width = Map::new(0.0, 1.0, 40.0, 240.0)?;
/// assert_eq!(width.of(&progress).sample(ms(50)).value, 140.0);
/// # Ok::<(), ginga::MapError>(())
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Mapped<'a, T: Animatable> {
    map: &'a Map<T>,
    motion: &'a Motion<f32>,
}

impl<T: Animatable> Mapped<'_, T> {
    /// The derived value at `now`, settled when the motion is. Reading it
    /// changes nothing and allocates nothing.
    #[must_use]
    pub fn sample(&self, now: Duration) -> Sample<T> {
        let sample = self.motion.sample(now);
        Sample {
            value: self.map.at(sample.value),
            settled: sample.settled,
        }
    }
}

impl<T: Animatable> crate::trace::sealed::Sealed for Mapped<'_, T> {}

impl<T: Animatable> crate::trace::Traceable for Mapped<'_, T> {
    fn row(&self, at: Duration) -> Row {
        let sample = self.sample(at);
        Row {
            at,
            values: sample.value.to_channels().as_ref().to_vec(),
            settled: sample.settled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Easing, Spring, Trace, Tween};

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn a_range_with_one_end_or_an_end_not_finite_does_not_build() {
        assert_eq!(Map::new(1.0, 1.0, 0.0, 10.0), Err(MapError::Empty));
        assert_eq!(Map::new(0.0, f32::NAN, 0.0, 10.0), Err(MapError::NotFinite));
        assert_eq!(
            Map::new(f32::INFINITY, 1.0, 0.0, 10.0),
            Err(MapError::NotFinite)
        );
        assert!(MapError::Empty.to_string().contains("0.0 and 1.0"));
    }

    #[test]
    fn a_progress_is_mapped_into_a_width_as_a_trace_shows() {
        let mut progress = Motion::at(0.0);
        progress.set_target(1.0, Tween::new(ms(100), Easing::Linear), ms(0));
        let width = Map::new(0.0, 1.0, 40.0, 240.0).unwrap();
        assert_eq!(
            Trace::of(&width.of(&progress), ms(0), ms(25)).to_string(),
            "\
ms           value
0           40.000
25          90.000
50         140.000
75         190.000
100        240.000  settled
"
        );
    }

    #[test]
    fn unclamped_an_overshoot_goes_on_and_clamped_it_stops_at_the_end() {
        let mut progress = Motion::at(0.0);
        progress.set_target(1.0, Spring::BOUNCY, ms(0));
        let width = Map::new(0.0, 1.0, 0.0, 100.0).unwrap();
        let farthest = |map: &Map<f32>| {
            (0..1_000)
                .map(|t| map.of(&progress).sample(ms(t)).value)
                .fold(f32::MIN, f32::max)
        };
        assert!(farthest(&width) > 100.0, "a bouncy spring passes the end");
        assert_eq!(farthest(&width.clamped()), 100.0);
    }

    #[test]
    fn ranges_may_run_backwards_and_the_ends_are_exact() {
        let fade = Map::new(200.0, 0.0, 1.0, 0.0).unwrap();
        assert_eq!(fade.at(200.0), 1.0);
        assert_eq!(fade.at(50.0), 0.25);
        assert_eq!(fade.at(0.0), 0.0);
        let blue = Color::hex(0x3b_6ef5);
        let fill = Map::new(0.0, 1.0, Color::TRANSPARENT, blue).unwrap();
        assert_eq!(fill.at(1.0), blue, "not rebuilt from its channels");
        assert_eq!(fill.at(f32::NAN), Color::TRANSPARENT);
        // The start comes back exactly too, a color that its channels lose.
        let back = Map::new(0.0, 1.0, blue, Color::TRANSPARENT).unwrap();
        assert_eq!(back.at(0.0), blue);
    }

    #[test]
    fn nan_gives_the_start_and_infinity_the_end_it_passes_when_clamped() {
        let width = Map::new(0.0, 1.0, 40.0, 240.0).unwrap();
        assert_eq!(width.at(f32::NAN), 40.0);
        assert_eq!(
            width.at(f32::INFINITY),
            40.0,
            "unclamped: no value that far"
        );
        assert_eq!(width.clamped().at(f32::INFINITY), 240.0);
        assert_eq!(width.clamped().at(f32::NEG_INFINITY), 40.0);
        assert_eq!(width.clamped().at(f32::NAN), 40.0);
    }

    #[test]
    fn a_range_too_wide_for_a_number_does_not_build_and_a_tiny_one_clamps() {
        assert_eq!(Map::new(-2e38, 2e38, 0.0, 100.0), Err(MapError::NotFinite));
        let tiny = Map::new(0.0, 1e-30, 0.0, 100.0).unwrap();
        assert_eq!(tiny.clamped().at(1e10), 100.0, "far past the end: the end");
    }

    #[test]
    fn a_range_onto_values_that_are_not_finite_does_not_build() {
        assert_eq!(Map::new(0.0, 1.0, 0.0, f32::NAN), Err(MapError::NotFinite));
        assert_eq!(
            Map::new(0.0, 1.0, f32::INFINITY, 1.0),
            Err(MapError::NotFinite)
        );
    }

    #[test]
    fn a_value_follows_another_with_a_spring_by_heading_for_it_each_frame() {
        let mut leader = Motion::at(0.0);
        leader.set_target(100.0, Tween::new(ms(200), Easing::Linear), ms(0));
        let mut follower = Motion::at(0.0);
        let mut last = 0.0;
        let mut frame = 0;
        let settled_at = loop {
            assert!(frame < 200, "the follower never settled");
            let now = ms(frame * 16);
            let lead = leader.sample(now);
            let ahead = lead.value;
            let sample = follower.toward(ahead, Spring::SNAPPY, now);
            assert!(sample.value >= last, "no jump back at {now:?}");
            assert!(sample.value - last < 20.0, "no jump ahead at {now:?}");
            if (16..200).contains(&now.as_millis()) {
                assert!(sample.value < ahead, "it trails the leader at {now:?}");
            }
            last = sample.value;
            if sample.settled && lead.settled {
                break now;
            }
            frame += 1;
        };
        assert_eq!(last, 100.0, "settled exactly where the leader rests");
        assert!(settled_at > ms(200));
    }

    #[test]
    fn a_derived_value_is_settled_when_its_motion_is() {
        let mut progress = Motion::at(0.0);
        progress.set_target(1.0, Spring::SNAPPY, ms(0));
        let color = Map::new(0.0, 1.0, Color::hex(0xe5_484d), Color::hex(0x30_a46c)).unwrap();
        let derived = color.of(&progress);
        assert!(!derived.sample(ms(100)).settled);
        let end = progress.settles_at().unwrap();
        assert_eq!(
            derived.sample(end),
            Sample {
                value: Color::hex(0x30_a46c),
                settled: true
            }
        );
    }
}
