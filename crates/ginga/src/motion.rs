use core::time::Duration;

use crate::{Animatable, Coast, Easing, Spring, Transition, Tween};

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
    /// The target as it was given, handed back once settled: a value rebuilt
    /// from its channels can be off in the last bit, as a color is.
    goal: T,
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
    Coast {
        coast: Coast,
        start: Duration,
        /// The speed each channel was let go with.
        velocity: C,
        settle: Duration,
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
            goal: value,
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
    ///
    /// A transition with a [delay](Transition::delay) holds the value still
    /// where it is until the delay is over, then moves from rest.
    pub fn set_target(&mut self, target: T, transition: impl Into<Transition>, now: Duration) {
        let transition = transition.into();
        let delay = transition.start_delay();
        // Only a spring that starts now carries the speed on. A delayed motion
        // waits still, so it starts from rest, and `Read::Value` reads no speed.
        let read = match transition {
            Transition::Spring(spring) if delay.is_zero() && !spring.is_instant() => {
                Read::ValueAndSpeed
            }
            _ => Read::Value,
        };
        let state = self.state(now, read);
        self.goal = target;
        self.target = target.to_channels();
        let start = now.saturating_add(delay);
        // Three ways to say "jump": no motion, a zero-length tween, a spring
        // with no response. Without a delay a jump rests on the target at once;
        // with one, it is a zero-length tween at the end of the delay.
        let jumps = match transition {
            Transition::Instant => true,
            Transition::Spring(spring) => spring.is_instant(),
            Transition::Tween(tween) => tween.duration.is_zero(),
        };
        self.drive = match transition {
            _ if jumps && delay.is_zero() => Drive::Rest,
            _ if jumps => self.tween(Tween::new(Duration::ZERO, Easing::Linear), &state, start),
            Transition::Spring(spring) => self.spring(spring, &state, start),
            Transition::Tween(tween) => self.tween(tween, &state, start),
            Transition::Instant => Drive::Rest,
        };
    }

    /// Lets the value go at `now` with `velocity`, in units per second: it
    /// coasts on and slows to a stop where [`Coast::rest`] says.
    ///
    /// ```
    /// use std::time::Duration;
    /// use ginga::{Coast, Motion};
    ///
    /// let ms = Duration::from_millis;
    /// let mut card = Motion::at(0.0);
    /// card.coast(1_000.0, Coast::QUICK, ms(0));
    /// assert_eq!(card.target(), 100.0);
    /// assert!(card.sample(ms(100)).value > 60.0); // most of the way in one time constant
    /// assert_eq!(card.sample(ms(2_000)).value, 100.0);
    /// ```
    pub fn coast(&mut self, velocity: T, coast: Coast, now: Duration) {
        let from = self.state(now, Read::Value).value;
        let velocity = velocity.to_channels();
        let mut seconds = 0f32;
        for speed in velocity.as_ref() {
            seconds = seconds.max(coast.settle_secs(*speed, T::PRECISION));
        }
        self.goal = coast.rest(T::from_channels(from), T::from_channels(velocity));
        self.target = self.goal.to_channels();
        self.drive = if seconds > 0.0 {
            Drive::Coast {
                coast,
                start: now,
                velocity,
                settle: now.saturating_add(Duration::from_secs_f32(seconds)),
            }
        } else {
            Drive::Rest
        };
    }

    /// As [`coast`](Self::coast), but it comes to rest with every channel
    /// within `min..=max`: a value that would coast past a limit moves with
    /// the coast's spring to that limit instead, keeping its speed, so it
    /// passes the limit a little and comes back.
    pub fn coast_within(&mut self, velocity: T, coast: Coast, min: T, max: T, now: Duration) {
        let from = T::from_channels(self.state(now, Read::Value).value);
        let mut rest = coast.rest(from, velocity).to_channels();
        let mut past = false;
        for ((place, low), high) in rest
            .as_mut()
            .iter_mut()
            .zip(min.to_channels().as_ref())
            .zip(max.to_channels().as_ref())
        {
            let inside = place.clamp(*low, high.max(*low));
            past |= inside != *place;
            *place = inside;
        }
        if past {
            let rest = T::from_channels(rest);
            self.fling(rest, coast.spring, velocity.to_channels(), now);
        } else {
            self.coast(velocity, coast, now);
        }
    }

    /// As [`coast`](Self::coast), but it comes to rest on whichever of
    /// `points` is nearest to where it would coast to, moving there with the
    /// coast's spring and keeping its speed. With no points it coasts.
    pub fn coast_to_nearest(&mut self, velocity: T, coast: Coast, points: &[T], now: Duration) {
        let from = T::from_channels(self.state(now, Read::Value).value);
        let rest = coast.rest(from, velocity).to_channels();
        let distance = |point: &T| -> f32 {
            point
                .to_channels()
                .as_ref()
                .iter()
                .zip(rest.as_ref())
                .map(|(a, b)| (a - b) * (a - b))
                .sum()
        };
        let nearest = points
            .iter()
            .min_by(|a, b| distance(a).total_cmp(&distance(b)));
        match nearest {
            Some(&point) => self.fling(point, coast.spring, velocity.to_channels(), now),
            None => self.coast(velocity, coast, now),
        }
    }

    /// Heads for `target` with `spring`, from where the value is at `now` and
    /// with `velocity` instead of the speed it had.
    fn fling(&mut self, target: T, spring: Spring, velocity: T::Channels, now: Duration) {
        let state = State {
            value: self.state(now, Read::Value).value,
            velocity,
            settled: false,
        };
        self.goal = target;
        self.target = target.to_channels();
        self.drive = self.spring(spring, &state, now);
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
            value: if state.settled {
                self.goal
            } else {
                T::from_channels(state.value)
            },
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
        self.goal
    }

    /// When the value reaches its target, or `None` when it is at rest. For a
    /// spring this is a safe bound: the value may arrive a little earlier.
    #[must_use]
    pub fn settles_at(&self) -> Option<Duration> {
        match self.drive {
            Drive::Rest => None,
            Drive::Spring { settle, .. } | Drive::Coast { settle, .. } => Some(settle),
            Drive::Tween { tween, start, .. } => Some(start.saturating_add(tween.duration)),
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
            settle: now.saturating_add(Duration::from_secs_f32(seconds)),
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

    /// Where a tween from `from` is `elapsed` after it starts, and how fast it
    /// goes there when `read` asks for the speed.
    fn tweening(
        &self,
        tween: Tween,
        elapsed: Duration,
        from: T::Channels,
        read: Read,
    ) -> State<T::Channels> {
        let length = tween.duration.as_secs_f32();
        let progress = elapsed.as_secs_f32() / length;
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

    /// Where a coast is `elapsed` after its release at `velocity`: short of
    /// its rest by what it has yet to travel, and as fast as it still goes.
    fn coasting(
        &self,
        coast: Coast,
        elapsed: Duration,
        velocity: T::Channels,
    ) -> State<T::Channels> {
        let seconds = elapsed.as_secs_f32();
        let (mut value, mut speeds) = (self.target, velocity);
        for (place, speed) in value.as_mut().iter_mut().zip(speeds.as_mut()) {
            let (left, moving) = coast.left(*speed, seconds);
            *place -= left;
            *speed = moving;
        }
        State {
            value,
            velocity: speeds,
            settled: false,
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
            Drive::Spring { settle, .. } | Drive::Coast { settle, .. } if now >= settle => resting,
            Drive::Tween { tween, start, .. } if now >= start.saturating_add(tween.duration) => {
                resting
            }
            Drive::Spring {
                start,
                offset,
                velocity,
                ..
            } if now < start => {
                let mut value = self.target;
                for (place, released) in value.as_mut().iter_mut().zip(offset.as_ref()) {
                    *place += released;
                }
                State {
                    value,
                    velocity,
                    settled: false,
                }
            }
            Drive::Tween { start, from, .. } if now < start => State {
                value: from,
                velocity: T::Channels::default(),
                settled: false,
            },
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
            Drive::Coast {
                coast,
                start,
                velocity,
                ..
            } => self.coasting(coast, now.saturating_sub(start), velocity),
            Drive::Tween { tween, start, from } => {
                self.tweening(tween, now.saturating_sub(start), from, read)
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
    fn drawing_reads_the_same_values_as_reading_the_speed_too() {
        let over = Spring::SMOOTH.bounce(-0.5).unwrap();
        for spring in [Spring::SMOOTH, Spring::BOUNCY, over] {
            let mut motion = Motion::at((0.0, 10.0, -40.0, 1.0));
            motion.set_target((300.0, 80.0, 12.0, 0.0), spring, ms(0));
            // A change of course mid-flight: each channel carries its own speed.
            motion.set_target((-20.0, 200.0, 12.0, 0.5), spring, ms(90));
            for millis in (90..1_500).step_by(7) {
                let now = ms(millis);
                let drawn = motion.state(now, Read::Value).value;
                let read = motion.state(now, Read::ValueAndSpeed).value;
                assert_eq!(drawn, read, "{spring:?} at {millis} ms");
            }
        }
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
            assert_eq!(motion.settles_at(), None, "{transition:?} rests at once");
        }
    }

    #[test]
    fn a_delay_too_long_to_add_holds_the_value_and_does_not_panic() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, Spring::SMOOTH.delay(Duration::MAX), ms(10));
        let sample = motion.sample(ms(1_000));
        assert_eq!(sample.value, 0.0);
        assert!(!sample.settled);
        motion.set_target(100.0, linear(200).delay(Duration::MAX), ms(10));
        assert_eq!(motion.sample(ms(1_000)).value, 0.0);
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
    fn a_delayed_tween_holds_still_then_runs_its_time() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, linear(200).delay(ms(50)), ms(1_000));
        assert_eq!(
            motion.sample(ms(1_049)),
            Sample {
                value: 0.0,
                settled: false
            }
        );
        assert_eq!(motion.sample(ms(1_100)).value, 25.0);
        assert_eq!(motion.settles_at(), Some(ms(1_250)));
        assert!(motion.sample(ms(1_250)).settled);
    }

    #[test]
    fn a_delayed_spring_waits_where_it_is_and_starts_from_rest() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, Spring::SMOOTH, ms(0));
        let caught = motion.sample(ms(80)).value;
        motion.set_target(-50.0, Spring::SMOOTH.delay(ms(100)), ms(80));
        // The spring is kept as an offset from its target, so the value it
        // holds is `caught` to within rounding, and the same all through.
        let held = motion.sample(ms(80)).value;
        assert!((held - caught).abs() < 1e-4, "{held} holds {caught}");
        for millis in [80, 120, 179, 180] {
            assert_eq!(motion.sample(ms(millis)).value, held, "at {millis} ms");
            assert!(!motion.sample(ms(millis)).settled, "waiting is not settled");
            assert_eq!(motion.velocity(ms(millis)), [0.0]);
        }
        assert!(motion.sample(ms(200)).value < caught);
        let settle = motion.settles_at().unwrap();
        assert!(settle > ms(180), "settles at {settle:?}, after the delay");
        assert_eq!(motion.sample(settle).value, -50.0);
    }

    #[test]
    fn a_delayed_motion_reads_the_same_in_any_order() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, linear(100).delay(ms(50)), ms(1_000));
        let times = [1_100, 1_049, 1_075, 1_020, 1_100, 1_049];
        let values: Vec<_> = times.iter().map(|&t| motion.sample(ms(t)).value).collect();
        assert_eq!(values, [50.0, 0.0, 25.0, 0.0, 50.0, 0.0]);
    }

    #[test]
    fn a_new_course_during_a_delay_starts_from_the_held_value() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, linear(100), ms(0));
        motion.set_target(-100.0, linear(100).delay(ms(100)), ms(50));
        // Halfway through the wait, a new target with no delay.
        motion.set_target(150.0, linear(100), ms(100));
        assert_eq!(motion.sample(ms(100)).value, 50.0);
        assert_eq!(motion.sample(ms(150)).value, 100.0);
        assert_eq!(motion.sample(ms(200)).value, 150.0);
    }

    #[test]
    fn a_delayed_instant_jumps_when_the_delay_is_over() {
        let mut motion = Motion::at(0.0);
        let late = Transition::Instant.delay(ms(30));
        motion.set_target(1.0, late, ms(0));
        assert_eq!(motion.sample(ms(29)).value, 0.0);
        assert!(!motion.sample(ms(29)).settled);
        assert_eq!(
            motion.sample(ms(30)),
            Sample {
                value: 1.0,
                settled: true
            }
        );
        let mut motion = Motion::at(0.0);
        motion.set_target(1.0, Spring::new(Duration::ZERO).delay(ms(30)), ms(0));
        assert_eq!(motion.sample(ms(29)).value, 0.0);
        assert_eq!(motion.sample(ms(30)).value, 1.0);
    }

    #[test]
    fn staggered_items_each_start_one_step_after_the_one_before() {
        let step = ms(40);
        let mut items = [Motion::at(0.0); 5];
        for (n, item) in (0..).zip(&mut items) {
            item.set_target(100.0, linear(100).delay(step * n), ms(0));
        }
        let traces: Vec<crate::Trace> = items
            .iter()
            .map(|item| crate::Trace::of(item, ms(0), ms(20)))
            .collect();
        let starts: Vec<Duration> = traces
            .iter()
            .map(|trace| {
                trace
                    .rows()
                    .iter()
                    .find(|row| row.values[0] > 0.0)
                    .unwrap()
                    .at
            })
            .collect();
        assert_eq!(starts, [ms(20), ms(60), ms(100), ms(140), ms(180)]);
        assert_eq!(
            traces[2].to_string(),
            "\
ms           value
0            0.000
20           0.000
40           0.000
60           0.000
80           0.000
100         20.000
120         40.000
140         60.000
160         80.000
180        100.000  settled
"
        );
    }

    #[test]
    fn time_before_the_start_reads_as_the_start() {
        let mut motion = Motion::at(0.0);
        motion.set_target(100.0, linear(200), ms(1_000));
        assert_eq!(motion.sample(ms(400)).value, 0.0);
    }
}
