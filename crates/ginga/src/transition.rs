use core::time::Duration;

use crate::{Easing, Spring};

/// Motion with a fixed length and a curve.
///
/// A tween retargeted mid-flight starts again from where the value is, at the
/// curve's own starting speed. Use a [`Spring`] when the target changes often.
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, Tween};
///
/// let enter = Tween::new(Duration::from_millis(260), Easing::OutExpo);
/// # let _ = enter;
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Tween {
    pub(crate) duration: Duration,
    pub(crate) easing: Easing,
    pub(crate) delay: Duration,
}

impl Tween {
    /// A tween that takes `duration` and follows `easing`. A zero `duration`
    /// moves the value at once.
    #[must_use]
    pub const fn new(duration: Duration, easing: Easing) -> Self {
        Self {
            duration,
            easing,
            delay: Duration::ZERO,
        }
    }

    /// The same tween, starting `delay` after the target is set. Until then
    /// the value holds still where it was.
    ///
    /// ```
    /// use std::time::Duration;
    /// use ginga::{Easing, Motion, Tween};
    ///
    /// let ms = Duration::from_millis;
    /// let mut motion = Motion::at(0.0);
    /// motion.set_target(100.0, Tween::new(ms(100), Easing::Linear).delay(ms(50)), ms(0));
    /// assert_eq!(motion.sample(ms(50)).value, 0.0);
    /// assert_eq!(motion.sample(ms(100)).value, 50.0);
    /// ```
    #[must_use]
    pub const fn delay(self, delay: Duration) -> Self {
        Self { delay, ..self }
    }
}

/// How a [`Motion`](crate::Motion) gets to its target.
#[derive(Debug, Clone, Copy)]
pub enum Transition {
    /// No motion: the value is at its target at once. For reduced motion,
    /// [`without_motion`](Self::without_motion) keeps a transition's delay.
    Instant,
    /// A spring. Keeps the value's speed across a change of target.
    Spring(Spring),
    /// A fixed length and a curve.
    Tween(Tween),
}

impl Transition {
    /// The same transition, starting `delay` after the target is set. Until
    /// then the value holds still where it was, and a spring starts from rest.
    /// A delayed [`Transition::Instant`] jumps to the target when the delay is
    /// over: it comes back as a zero-length [`Tween`] with that delay, so it no
    /// longer matches `Transition::Instant`.
    ///
    /// To let items arrive one after another, give each a delay one step longer
    /// than the one before:
    ///
    /// ```
    /// use std::time::Duration;
    /// use ginga::{Motion, Spring, Transition};
    ///
    /// let ms = Duration::from_millis;
    /// let mut bars = [Motion::at(0.0); 5];
    /// for (n, bar) in (0..).zip(&mut bars) {
    ///     // 40 ms after the one before, and never more than 120 ms late.
    ///     let delay = (ms(40) * n).min(ms(120));
    ///     bar.set_target(1.0, Transition::from(Spring::SMOOTH).delay(delay), ms(0));
    /// }
    ///
    /// let starts: Vec<_> = bars.iter().map(|bar| bar.sample(ms(60)).value > 0.0).collect();
    /// assert_eq!(starts, [true, true, false, false, false]);
    /// ```
    #[must_use]
    pub const fn delay(self, delay: Duration) -> Self {
        match self {
            Self::Instant if delay.is_zero() => Self::Instant,
            Self::Instant => Self::Tween(Tween::new(Duration::ZERO, Easing::Linear).delay(delay)),
            Self::Spring(spring) => Self::Spring(spring.delay(delay)),
            Self::Tween(tween) => Self::Tween(tween.delay(delay)),
        }
    }

    /// The same timing with no motion: it holds still for this transition's
    /// delay, then is at its target at once. For reduced motion, where a
    /// delay can carry meaning of its own, such as hover intent.
    ///
    /// ```
    /// use std::time::Duration;
    /// use ginga::{Easing, Motion, Transition, Tween};
    ///
    /// let ms = Duration::from_millis;
    /// let intent = Transition::from(Tween::new(ms(260), Easing::OutExpo).delay(ms(160)));
    /// let mut card = Motion::at(0.0);
    /// card.set_target(1.0, intent.without_motion(), ms(0));
    /// assert_eq!(card.sample(ms(159)).value, 0.0);
    /// assert_eq!(card.sample(ms(160)).value, 1.0);
    /// assert!(card.sample(ms(160)).settled);
    /// ```
    #[must_use]
    pub const fn without_motion(self) -> Self {
        Self::Instant.delay(self.start_delay())
    }

    /// How long after the target is set the motion starts.
    pub(crate) const fn start_delay(self) -> Duration {
        match self {
            Self::Instant => Duration::ZERO,
            Self::Spring(spring) => spring.delay,
            Self::Tween(tween) => tween.delay,
        }
    }
}

impl From<Spring> for Transition {
    fn from(spring: Spring) -> Self {
        Self::Spring(spring)
    }
}

impl From<Tween> for Transition {
    fn from(tween: Tween) -> Self {
        Self::Tween(tween)
    }
}
