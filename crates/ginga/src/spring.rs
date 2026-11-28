use core::{f32::consts::TAU, fmt, time::Duration};

/// Springs within this distance of a damping ratio of one use the critical form.
const CRITICAL_BAND: f32 = 1e-3;
/// No spring is asked to settle later than this many seconds.
const LONGEST_SETTLE: f32 = 60.0;

/// A spring: motion that keeps its speed when its target changes.
///
/// Start from a preset. Build your own with [`Spring::new`] and [`Spring::bounce`].
///
/// ```
/// use std::time::Duration;
/// use ginga::Spring;
///
/// let quick = Spring::SNAPPY;
/// let playful = Spring::new(Duration::from_millis(400)).bounce(0.3)?;
/// # let _ = (quick, playful);
/// # Ok::<(), ginga::SpringError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    /// The natural frequency, in radians per second.
    omega: f32,
    /// The damping ratio: one does not overshoot, less than one does.
    zeta: f32,
    /// How long after the target is set the spring is released.
    pub(crate) delay: Duration,
}

/// Why a [`Spring`] could not be built.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpringError {
    /// The bounce is not a number strictly between `-1.0` and `1.0`.
    BounceOutOfRange(f32),
    /// A stiffness, damping and mass that make no spring: stiffness and mass
    /// must be finite and more than zero, damping finite and more than zero.
    NotASpring {
        /// The stiffness given.
        stiffness: f32,
        /// The damping given.
        damping: f32,
        /// The mass given.
        mass: f32,
    },
}

impl fmt::Display for SpringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BounceOutOfRange(bounce) => write!(
                f,
                "spring bounce must be more than -1.0 and less than 1.0, got {bounce}; \
                 use 0.0 for no overshoot and about 0.3 for a playful one"
            ),
            Self::NotASpring {
                stiffness,
                damping,
                mass,
            } => write!(
                f,
                "no spring has stiffness {stiffness}, damping {damping} and mass {mass}; \
                 each must be finite and more than zero (with no damping a spring never stops), \
                 such as stiffness 100.0, damping 10.0 and mass 1.0"
            ),
        }
    }
}

impl core::error::Error for SpringError {}

impl Spring {
    /// Arrives in about 0.4 s without overshoot. For panels and surfaces.
    pub const SMOOTH: Self = Self::from_secs(0.4, 1.0);
    /// Arrives in about 0.25 s without overshoot. For what follows the pointer or a selection.
    pub const SNAPPY: Self = Self::from_secs(0.25, 1.0);
    /// Passes its target a little and comes back. For what has room to overshoot.
    pub const BOUNCY: Self = Self::from_secs(0.4, 0.7);

    const fn from_secs(response: f32, zeta: f32) -> Self {
        Self {
            omega: TAU / response,
            zeta,
            delay: Duration::ZERO,
        }
    }

    /// A spring that does not overshoot and feels about `response` long.
    ///
    /// `response` is the pace of the motion, not the moment it stops: the last
    /// fraction of the distance keeps settling a little after it. A zero
    /// `response` moves the value at once.
    #[must_use]
    pub fn new(response: Duration) -> Self {
        Self::from_secs(response.as_secs_f32(), 1.0)
    }

    /// Sets how far the spring passes its target: `0.0` not at all, `0.3` playfully.
    /// A negative bounce makes the arrival slower and flatter.
    ///
    /// # Errors
    ///
    /// [`SpringError::BounceOutOfRange`] when `bounce` is not strictly between
    /// `-1.0` and `1.0`: at `1.0` a spring would never stop.
    pub fn bounce(self, bounce: f32) -> Result<Self, SpringError> {
        if !(bounce > -1.0 && bounce < 1.0) {
            return Err(SpringError::BounceOutOfRange(bounce));
        }
        let zeta = if bounce >= 0.0 {
            1.0 - bounce
        } else {
            1.0 / (1.0 + bounce)
        };
        Ok(Self { zeta, ..self })
    }

    /// A spring given as a stiffness, a damping and a mass, the way Motion,
    /// react-spring (tension, friction and mass), `SwiftUI`'s
    /// `interpolatingSpring` and physics give it. It is the same spring as a
    /// response and a bounce, and costs the same: the response is
    /// `2π·√(mass / stiffness)`, and the damping ratio
    /// `damping / (2·√(stiffness·mass))` is `1 − bounce` up to one and
    /// `1 / (1 + bounce)` past it.
    ///
    /// ```
    /// use std::time::Duration;
    /// use ginga::{Motion, Spring};
    ///
    /// let ms = Duration::from_millis;
    /// // Motion's default spring: a ratio of 0.5, so a bounce of 0.5.
    /// let spring = Spring::physical(100.0, 10.0, 1.0)?;
    /// let mut x = Motion::at(0.0);
    /// x.set_target(100.0, spring, ms(0));
    /// assert!(x.sample(ms(300)).value > 100.0); // past its target, on the way back
    /// # Ok::<(), ginga::SpringError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`SpringError::NotASpring`] when a value is not finite or not more
    /// than zero, or when together they make a spring too fast to compute.
    pub fn physical(stiffness: f32, damping: f32, mass: f32) -> Result<Self, SpringError> {
        let positive = |value: f32| value.is_finite() && value > 0.0;
        let omega = (stiffness / mass).sqrt();
        let zeta = damping / (2.0 * (stiffness * mass).sqrt());
        if positive(stiffness)
            && positive(damping)
            && positive(mass)
            && positive(omega)
            && positive(zeta)
        {
            Ok(Self {
                omega,
                zeta,
                delay: Duration::ZERO,
            })
        } else {
            Err(SpringError::NotASpring {
                stiffness,
                damping,
                mass,
            })
        }
    }

    /// The same spring, released `delay` after the target is set. Until then
    /// the value holds still where it was, and the spring starts from rest.
    ///
    /// ```
    /// use std::time::Duration;
    /// use ginga::{Motion, Spring};
    ///
    /// let ms = Duration::from_millis;
    /// let mut motion = Motion::at(0.0);
    /// motion.set_target(100.0, Spring::SNAPPY.delay(ms(80)), ms(0));
    /// assert_eq!(motion.sample(ms(79)).value, 0.0);
    /// assert!(motion.sample(ms(100)).value > 0.0);
    /// ```
    #[must_use]
    pub const fn delay(self, delay: Duration) -> Self {
        Self { delay, ..self }
    }

    pub(crate) fn is_instant(self) -> bool {
        !self.omega.is_finite()
    }

    /// The spring's shape `seconds` after it was released. The costly math
    /// happens once here, whatever the number of channels.
    pub(crate) fn phase(self, seconds: f32) -> Phase {
        let (w, z, t) = (self.omega, self.zeta, seconds);
        if (z - 1.0).abs() < CRITICAL_BAND {
            Phase::Critical {
                w,
                t,
                decay: (-w * t).exp(),
            }
        } else if z < 1.0 {
            let wd = w * (1.0 - z * z).sqrt();
            let (sin, cos) = (wd * t).sin_cos();
            Phase::Under {
                zw: z * w,
                wd,
                decay: (-z * w * t).exp(),
                sin,
                cos,
            }
        } else {
            let root = w * (z * z - 1.0).sqrt();
            let (slow, fast) = (-z * w + root, -z * w - root);
            Phase::Over {
                slow,
                fast,
                slow_decay: (slow * t).exp(),
                fast_decay: (fast * t).exp(),
            }
        }
    }

    /// Seconds until a channel released `offset` away from its target, moving
    /// at `velocity`, stays within `precision` of it. Never too early.
    pub(crate) fn settle_secs(self, offset: f32, velocity: f32, precision: f32) -> f32 {
        let (w, z) = (self.omega, self.zeta);
        let after = |amplitude: f32, rate: f32| (amplitude / precision).max(1.0).ln() / rate;
        let seconds = if (z - 1.0).abs() < CRITICAL_BAND {
            let (a, b) = (offset.abs(), (velocity + w * offset).abs());
            (0..4).fold(1.0 / w, |t, _| after(a + b * t, w))
        } else if z < 1.0 {
            let wd = w * (1.0 - z * z).sqrt();
            let b = (velocity + z * w * offset) / wd;
            after(offset.hypot(b), z * w)
        } else {
            let root = w * (z * z - 1.0).sqrt();
            let (slow, fast) = (-z * w + root, -z * w - root);
            let c1 = (velocity - fast * offset) / (slow - fast);
            after(c1.abs() + (offset - c1).abs(), -slow)
        };
        if seconds.is_finite() {
            seconds.min(LONGEST_SETTLE)
        } else {
            LONGEST_SETTLE
        }
    }
}

/// A spring at one instant, ready to be applied to any number of channels.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Phase {
    Critical {
        w: f32,
        t: f32,
        decay: f32,
    },
    Under {
        zw: f32,
        wd: f32,
        decay: f32,
        sin: f32,
        cos: f32,
    },
    Over {
        slow: f32,
        fast: f32,
        slow_decay: f32,
        fast_decay: f32,
    },
}

impl Phase {
    /// The distance to the target of a channel released `offset` away from
    /// its target at `velocity`. The first half of [`Phase::apply`], for a
    /// frame that only draws: the velocity costs as much again.
    ///
    /// It repeats `apply`'s arithmetic, operation for operation, so the two
    /// agree to the bit; a test checks it. Sharing the code through a helper
    /// cost a four-channel frame 252 instructions against 165 (valgrind), so
    /// the copy stays: change both together.
    pub(crate) fn left(&self, offset: f32, velocity: f32) -> f32 {
        match *self {
            Self::Critical { w, t, decay } => (offset + (velocity + w * offset) * t) * decay,
            Self::Under {
                zw,
                wd,
                decay,
                sin,
                cos,
            } => {
                let b = (velocity + zw * offset) / wd;
                decay * (offset * cos + b * sin)
            }
            Self::Over {
                slow,
                fast,
                slow_decay,
                fast_decay,
            } => {
                let c1 = (velocity - fast * offset) / (slow - fast);
                c1 * slow_decay + (offset - c1) * fast_decay
            }
        }
    }

    /// The distance to the target and the velocity of a channel released
    /// `offset` away from its target at `velocity`.
    pub(crate) fn apply(&self, offset: f32, velocity: f32) -> (f32, f32) {
        match *self {
            Self::Critical { w, t, decay } => {
                let b = velocity + w * offset;
                let reach = offset + b * t;
                (reach * decay, (b - w * reach) * decay)
            }
            Self::Under {
                zw,
                wd,
                decay,
                sin,
                cos,
            } => {
                let b = (velocity + zw * offset) / wd;
                (
                    decay * (offset * cos + b * sin),
                    decay * ((b * wd - zw * offset) * cos - (offset * wd + zw * b) * sin),
                )
            }
            Self::Over {
                slow,
                fast,
                slow_decay,
                fast_decay,
            } => {
                let c1 = (velocity - fast * offset) / (slow - fast);
                let c2 = offset - c1;
                (
                    c1 * slow_decay + c2 * fast_decay,
                    c1 * slow * slow_decay + c2 * fast * fast_decay,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(spring: Spring, seconds: f32) -> (f32, f32) {
        spring.phase(seconds).apply(-100.0, 0.0)
    }

    fn over() -> Spring {
        Spring::SMOOTH.bounce(-0.5).unwrap()
    }

    #[test]
    fn a_spring_physical_moves_as_the_same_response_and_bounce() {
        // (stiffness, damping, mass) and the response in seconds and bounce they make.
        let cases = [
            ((100.0, 10.0, 1.0), (TAU / 10.0, 0.5)),  // a ratio of 0.5
            ((400.0, 40.0, 1.0), (TAU / 20.0, 0.0)),  // critical
            ((100.0, 40.0, 1.0), (TAU / 10.0, -0.5)), // a ratio of 2
            ((200.0, 20.0, 2.0), (TAU / 10.0, 0.5)),  // a heavier mass, the same spring
        ];
        for ((stiffness, damping, mass), (response, bounce)) in cases {
            let given = Spring::physical(stiffness, damping, mass).unwrap();
            let same = Spring::new(Duration::from_secs_f32(response))
                .bounce(bounce)
                .unwrap();
            for step in 0..60 {
                let seconds = step as f32 / 40.0;
                let (a, b) = (at(given, seconds), at(same, seconds));
                assert!(
                    (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-2,
                    "{stiffness}, {damping}, {mass} at {seconds} s: {a:?} against {b:?}"
                );
            }
        }
    }

    #[test]
    fn values_that_make_no_spring_do_not_build() {
        let not = [
            (0.0, 10.0, 1.0),
            (-100.0, 10.0, 1.0),
            (100.0, 0.0, 1.0), // no damping: it would never stop
            (100.0, -10.0, 1.0),
            (100.0, 10.0, 0.0),
            (f32::NAN, 10.0, 1.0),
            (100.0, f32::INFINITY, 1.0),
            (1e38, 10.0, 1e-38), // too fast to compute
        ];
        for (stiffness, damping, mass) in not {
            let error = Spring::physical(stiffness, damping, mass).unwrap_err();
            assert!(matches!(error, SpringError::NotASpring { .. }), "{error:?}");
            assert!(
                error.to_string().contains("such as stiffness 100.0"),
                "{error}"
            );
        }
    }

    #[test]
    fn every_kind_of_spring_starts_where_it_was_released() {
        for spring in [Spring::SMOOTH, Spring::BOUNCY, over()] {
            let (offset, velocity) = spring.phase(0.0).apply(-100.0, 30.0);
            assert!((offset + 100.0).abs() < 1e-3, "{spring:?}");
            assert!((velocity - 30.0).abs() < 1e-2, "{spring:?}");
        }
    }

    #[test]
    fn a_spring_without_bounce_never_passes_its_target() {
        for spring in [Spring::SMOOTH, Spring::SNAPPY, over()] {
            for step in 0..400 {
                let (offset, _) = at(spring, step as f32 / 200.0);
                assert!(
                    offset <= 1e-4,
                    "{spring:?} passed its target at step {step}"
                );
            }
        }
    }

    #[test]
    fn a_bouncy_spring_passes_its_target_and_comes_back() {
        let furthest = (0..400)
            .map(|step| at(Spring::BOUNCY, step as f32 / 200.0).0)
            .fold(f32::MIN, f32::max);
        assert!(furthest > 1.0, "it overshoots by {furthest}");
        assert!(at(Spring::BOUNCY, 2.0).0.abs() < 0.01);
    }

    #[test]
    fn the_distance_alone_is_the_distance_with_the_velocity() {
        for spring in [Spring::SMOOTH, Spring::BOUNCY, over()] {
            for seconds in [0.0, 0.05, 0.1, 0.2, 0.7] {
                let phase = spring.phase(seconds);
                for (offset, velocity) in [(-100.0, 0.0), (0.0, 900.0), (40.0, -2000.0)] {
                    assert_eq!(
                        phase.left(offset, velocity),
                        phase.apply(offset, velocity).0,
                        "{spring:?} at {seconds}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_velocity_is_the_slope_of_the_position() {
        const STEP: f32 = 1e-3;
        for spring in [Spring::SMOOTH, Spring::BOUNCY, over()] {
            for seconds in [0.05, 0.1, 0.2] {
                let (before, _) = at(spring, seconds - STEP);
                let (after, _) = at(spring, seconds + STEP);
                let (_, velocity) = at(spring, seconds);
                let slope = (after - before) / (2.0 * STEP);
                assert!(
                    (velocity - slope).abs() < slope.abs() * 0.02 + 0.5,
                    "{spring:?} at {seconds}: {velocity} against {slope}"
                );
            }
        }
    }

    #[test]
    fn a_spring_has_settled_by_the_time_it_says() {
        for spring in [Spring::SMOOTH, Spring::SNAPPY, Spring::BOUNCY, over()] {
            for (offset, velocity) in [(-100.0, 0.0), (0.0, 900.0), (40.0, -2000.0)] {
                let settle = spring.settle_secs(offset, velocity, 0.1);
                assert!(settle < 10.0, "{spring:?} takes {settle}s");
                for step in 0..200 {
                    let later = settle + step as f32 / 100.0;
                    let (left, _) = spring.phase(later).apply(offset, velocity);
                    assert!(
                        left.abs() <= 0.1 + 1e-3,
                        "{spring:?}: {left} left at {later}s"
                    );
                }
            }
        }
    }

    #[test]
    fn a_spring_at_rest_on_its_target_needs_no_time() {
        assert_eq!(Spring::SMOOTH.settle_secs(0.0, 0.0, 0.1), 0.0);
        assert_eq!(Spring::BOUNCY.settle_secs(0.01, 0.0, 0.1), 0.0);
    }

    #[test]
    fn a_bounce_out_of_range_says_what_to_use() {
        for bounce in [1.0, -1.0, 1.5, f32::NAN] {
            let error = Spring::SMOOTH.bounce(bounce).unwrap_err();
            assert!(error.to_string().contains("use 0.0 for no overshoot"));
        }
        assert!(Spring::SMOOTH.bounce(0.3).is_ok());
    }

    #[test]
    fn a_zero_response_is_instant() {
        assert!(Spring::new(Duration::ZERO).is_instant());
        assert!(!Spring::new(Duration::from_millis(1)).is_instant());
    }
}
