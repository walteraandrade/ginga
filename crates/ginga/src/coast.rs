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
