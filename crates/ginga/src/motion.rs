use core::time::Duration;

use crate::{Animatable, Spring, Transition, Tween};

/// A value on its way to a target.
///
/// A `Motion` is plain data that you own: keep it next to the state it
/// animates. It has no clock and no timer. You hand it the time, as a
/// [`Duration`] since any fixed moment, and it answers with the value at that
/// time. The same times always give the same values, on any machine and at
/// any frame rate, so motion can be tested without a window.
///
/// Each frame, call [`Motion::toward`] with where the value should be:
///
/// ```
/// use std::time::Duration;
/// use ginga::{Motion, Spring};
///
/// let ms = Duration::from_millis;
/// let mut width = Motion::at(180.0);
///
/// let start = width.toward(320.0, Spring::SNAPPY, ms(0));
/// assert_eq!(start.value, 180.0);
///
/// let midway = width.toward(320.0, Spring::SNAPPY, ms(100));
/// assert!(midway.value > 180.0 && midway.value < 320.0);
/// assert!(!midway.settled);
///
/// let end = width.toward(320.0, Spring::SNAPPY, ms(2_000));
/// assert_eq!(end.value, 320.0);
/// assert!(end.settled);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Motion<T: Animatable> {
    target: T::Channels,
    drive: Drive<T::Channels>,
}

/// A [`Motion`] at one instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample<T> {
    /// The value at that instant.
    pub value: T,
    /// The value is at its target and will not move until the target changes.
    /// While this is `false`, draw another frame.
    pub settled: bool,
}

#[derive(Debug, Clone, Copy)]
enum Drive<C> {
    Rest,
    Spring {
        spring: Spring,
        start: Duration,
        /// Where each channel was released, measured from the target.
        offset: C,
        velocity: C,
        settle: Duration,
    },
    Tween {
        tween: Tween,
        start: Duration,
        from: C,
    },
}

struct State<C> {
    value: C,
    velocity: C,
    settled: bool,
}

/// What a reader needs. A spring's speed costs as much again as its place, and
/// a tween's two more looks at its curve, so a frame that only draws does not
/// ask for it.
#[derive(Clone, Copy)]
enum Read {
    /// The value alone. The `velocity` of the [`State`] is then zero, not the
    /// motion's speed: do not read it.
    Value,
    /// The value and its speed.
    ValueAndSpeed,
}

impl<T: Animatable> Motion<T> {
    /// A value at rest.
    #[must_use]
    pub fn at(value: T) -> Self {
        Self {
            target: value.to_channels(),
            drive: Drive::Rest,
        }
    }

    /// Heads for `target` and returns the value at `now`.
    ///
    /// This is the one call a frame needs. It changes course only when
    /// `target` differs from the last one, and then it starts from the value
    /// and the speed the motion has at `now`. `transition` is used for that
    /// change of course and ignored otherwise.
    pub fn toward(
        &mut self,
        target: T,
        transition: impl Into<Transition>,
        now: Duration,
    ) -> Sample<T> {
        if target.to_channels() != self.target {
            self.set_target(target, transition, now);
        }
        self.sample(now)
    }

    /// Changes course at `now`, from the value and the speed the motion has then.
    pub fn set_target(&mut self, target: T, transition: impl Into<Transition>, now: Duration) {
        let state = self.state(now, Read::ValueAndSpeed);
        self.target = target.to_channels();
        self.drive = match transition.into() {
            Transition::Spring(spring) if !spring.is_instant() => self.spring(spring, &state, now),
            Transition::Tween(tween) if !tween.duration.is_zero() => self.tween(tween, &state, now),
            _ => Drive::Rest,
        };
    }

    /// Puts the value at `value`, at rest, with no motion.
    pub fn jump_to(&mut self, value: T) {
        *self = Self::at(value);
    }

    /// The value at `now`. It never changes the motion, so any number of
    /// readers can ask, in any order of time.
    #[must_use]
    pub fn sample(&self, now: Duration) -> Sample<T> {
        let state = self.state(now, Read::Value);
        Sample {
            value: T::from_channels(state.value),
            settled: state.settled,
        }
    }

    /// How fast each channel moves at `now`, in units per second.
    #[must_use]
    pub fn velocity(&self, now: Duration) -> T::Channels {
        self.state(now, Read::ValueAndSpeed).velocity
    }

    /// Where the motion is heading.
    #[must_use]
    pub fn target(&self) -> T {
        T::from_channels(self.target)
    }

    /// When the value reaches its target, or `None` when it is at rest. For a
    /// spring this is a safe bound: the value may arrive a little earlier.
    #[must_use]
    pub fn settles_at(&self) -> Option<Duration> {
        match self.drive {
            Drive::Rest => None,
            Drive::Spring { settle, .. } => Some(settle),
            Drive::Tween { tween, start, .. } => Some(start + tween.duration),
        }
    }

    fn spring(
        &self,
        spring: Spring,
        state: &State<T::Channels>,
        now: Duration,
    ) -> Drive<T::Channels> {
        let mut offset = state.value;
        let mut seconds = 0f32;
        for ((left, speed), target) in offset
            .as_mut()
            .iter_mut()
            .zip(state.velocity.as_ref())
            .zip(self.target.as_ref())
        {
            *left -= target;
            seconds = seconds.max(spring.settle_secs(*left, *speed, T::PRECISION));
        }
        if seconds <= 0.0 {
            return Drive::Rest;
        }
        Drive::Spring {
            spring,
            start: now,
            offset,
            velocity: state.velocity,
            settle: now + Duration::from_secs_f32(seconds),
        }
    }

    fn tween(&self, tween: Tween, state: &State<T::Channels>, now: Duration) -> Drive<T::Channels> {
        if state.value == self.target {
            return Drive::Rest;
        }
        Drive::Tween {
            tween,
            start: now,
            from: state.value,
        }
    }

    fn state(&self, now: Duration, read: Read) -> State<T::Channels> {
        let resting = State {
            value: self.target,
            velocity: T::Channels::default(),
            settled: true,
        };
        match self.drive {
            Drive::Rest => resting,
            Drive::Spring { settle, .. } if now >= settle => resting,
            Drive::Tween { tween, start, .. } if now >= start + tween.duration => resting,
            Drive::Spring {
                spring,
                start,
                offset,
                velocity,
                ..
            } => {
                let phase = spring.phase(now.saturating_sub(start).as_secs_f32());
                let mut value = self.target;
                let velocity = match read {
                    Read::Value => {
                        for ((place, released), speed) in value
                            .as_mut()
                            .iter_mut()
                            .zip(offset.as_ref())
                            .zip(velocity.as_ref())
                        {
                            *place += phase.left(*released, *speed);
                        }
                        T::Channels::default()
                    }
                    Read::ValueAndSpeed => {
                        let mut velocity = velocity;
                        for ((place, speed), released) in value
                            .as_mut()
                            .iter_mut()
                            .zip(velocity.as_mut())
                            .zip(offset.as_ref())
                        {
                            let (left, moving) = phase.apply(*released, *speed);
                            *place += left;
                            *speed = moving;
                        }
                        velocity
                    }
                };
                State {
                    value,
                    velocity,
                    settled: false,
                }
            }
            Drive::Tween { tween, start, from } => {
                let length = tween.duration.as_secs_f32();
                let progress = now.saturating_sub(start).as_secs_f32() / length;
                let eased = tween.easing.at(progress);
                let slope = match read {
                    Read::Value => 0.0,
                    Read::ValueAndSpeed => tween.easing.slope(progress) / length,
                };
                let (mut value, mut velocity) = (from, from);
                for ((place, speed), target) in value
                    .as_mut()
                    .iter_mut()
                    .zip(velocity.as_mut())
                    .zip(self.target.as_ref())
                {
                    let span = target - *place;
                    *speed = span * slope;
                    *place += span * eased;
                }
                State {
                    value,
                    velocity,
                    settled: false,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Easing;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn linear(millis: u64) -> Tween {
        Tween::new(ms(millis), Easing::Linear)
    }

    #[test]
    fn a_value_at_rest_stays_put_and_asks_for_no_frames() {
        let motion = Motion::at(7.0);
        assert_eq!(
            motion.sample(ms(5_000)),
            Sample {
                value: 7.0,
                settled: true
            }
        );
        assert_eq!(motion.settles_at(), None);
    }

    #[test]
    fn a_tween_runs_from_the_value_to_the_target_in_its_time() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, linear(200), ms(1_000));
        assert_eq!(motion.sample(ms(1_000)).value, 0.0);
        assert_eq!(motion.sample(ms(1_050)).value, 25.0);
        assert!(!motion.sample(ms(1_199)).settled);
        assert_eq!(
            motion.sample(ms(1_200)),
            Sample {
                value: 100.0,
                settled: true
            }
        );
        assert_eq!(motion.settles_at(), Some(ms(1_200)));
    }

    #[test]
    fn the_same_time_gives_the_same_value_whatever_was_asked_before() {
        let mut motion = Motion::at((0.0, 0.0));
        motion.set_target((300.0, -40.0), Spring::BOUNCY, ms(0));
        let once = motion.sample(ms(137));
        for millis in [900, 3, 500, 136, 138] {
            let _ = motion.sample(ms(millis));
        }
        assert_eq!(motion.sample(ms(137)), once);
    }

    #[test]
    fn toward_changes_course_only_when_the_target_changes() {
        let mut motion = Motion::at(0.0);
        motion.toward(100.0, linear(200), ms(0));
        let halfway = motion.toward(100.0, linear(200), ms(100));
        assert_eq!(halfway.value, 50.0);
        let turned = motion.toward(0.0, linear(200), ms(100));
        assert_eq!(turned.value, 50.0);
        assert_eq!(motion.toward(0.0, linear(200), ms(200)).value, 25.0);
    }

    #[test]
    fn a_retargeted_spring_keeps_its_place_and_its_speed() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, Spring::SMOOTH, ms(0));
        let before = (motion.sample(ms(80)).value, motion.velocity(ms(80))[0]);
        assert!(before.1 > 100.0, "it is moving at {} per second", before.1);
        motion.set_target(-50.0, Spring::SMOOTH, ms(80));
        let after = (motion.sample(ms(80)).value, motion.velocity(ms(80))[0]);
        assert!((after.0 - before.0).abs() < 1e-3);
        assert!((after.1 - before.1).abs() < 1e-2);
        assert!(
            motion.sample(ms(90)).value > after.0,
            "it still coasts forward"
        );
    }

    #[test]
    fn a_spring_picks_up_the_speed_of_the_tween_it_interrupts() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, linear(200), ms(0));
        motion.set_target(100.0, Spring::SMOOTH, ms(100));
        assert!((motion.velocity(ms(100))[0] - 500.0).abs() < 1.0);
    }

    #[test]
    fn a_spring_lands_exactly_on_its_target() {
        let mut motion = Motion::at((0.0, 10.0));
        motion.set_target((250.0, 10.0), Spring::BOUNCY, ms(0));
        let settle = motion.settles_at().unwrap();
        assert!(
            settle > ms(300) && settle < ms(3_000),
            "settles at {settle:?}"
        );
        assert!(!motion.sample(settle.saturating_sub(ms(1))).settled);
        assert_eq!(
            motion.sample(settle),
            Sample {
                value: (250.0, 10.0),
                settled: true
            }
        );
    }

    #[test]
    fn instant_transitions_and_zero_lengths_do_not_move() {
        let transitions = [
            Transition::Instant,
            linear(0).into(),
            Spring::new(Duration::ZERO).into(),
        ];
        for transition in transitions {
            let mut motion = Motion::at(0.0);
            let sample = motion.toward(100.0, transition, ms(10));
            assert_eq!(
                sample,
                Sample {
                    value: 100.0,
                    settled: true
                },
                "{transition:?}"
            );
        }
    }

    #[test]
    fn a_move_smaller_than_the_precision_is_not_a_motion() {
        let mut motion = Motion::at(1.0);
        motion.set_target(1.0 + f32::PRECISION / 2.0, Spring::SMOOTH, ms(0));
        assert_eq!(motion.settles_at(), None);
        assert!(motion.sample(ms(0)).settled);
    }

    #[test]
    fn a_jump_forgets_the_motion() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, Spring::SMOOTH, ms(0));
        motion.jump_to(40.0);
        assert_eq!(
            motion.sample(ms(1)),
            Sample {
                value: 40.0,
                settled: true
            }
        );
        assert_eq!(motion.target(), 40.0);
    }

    #[test]
    fn time_before_the_start_reads_as_the_start() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, linear(200), ms(1_000));
        assert_eq!(motion.sample(ms(400)).value, 0.0);
    }
}
