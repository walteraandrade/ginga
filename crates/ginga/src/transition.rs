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
}

impl Tween {
    /// A tween that takes `duration` and follows `easing`. A zero `duration`
    /// moves the value at once.
    #[must_use]
    pub const fn new(duration: Duration, easing: Easing) -> Self {
        Self { duration, easing }
    }
}

/// How a [`Motion`](crate::Motion) gets to its target.
#[derive(Debug, Clone, Copy)]
pub enum Transition {
    /// No motion: the value is at its target at once. Use it for reduced motion.
    Instant,
    /// A spring. Keeps the value's speed across a change of target.
    Spring(Spring),
    /// A fixed length and a curve.
    Tween(Tween),
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
