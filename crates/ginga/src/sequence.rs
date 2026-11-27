use core::{fmt, time::Duration};

use crate::{Animatable, Motion, Transition};

/// Several motions arranged in time and started as one: one after another,
/// together, or a set time apart. A step is a motion given a target and a
/// transition, delayed to start at its place in the sequence; nothing runs
/// between frames, so each motion is read as usual.
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, Motion, Sequence, Tween};
///
/// let ms = Duration::from_millis;
/// let fade = Tween::new(ms(100), Easing::Linear);
/// let (mut title, mut body, mut button) = (Motion::at(0.0), Motion::at(0.0), Motion::at(0.0));
///
/// let mut entrance = Sequence::starting_at(ms(0));
/// let first = entrance.then(&mut title, 1.0, fade);
/// let second = entrance.then(&mut body, 1.0, fade); // when the title has arrived
/// let third = entrance.at(second.ends.saturating_sub(ms(40)), &mut button, 1.0, fade); // overlapping by 40 ms
///
/// assert_eq!(first.to_string(), "0 ms to 100 ms");
/// assert_eq!(second.to_string(), "100 ms to 200 ms");
/// assert_eq!(third.to_string(), "160 ms to 260 ms");
/// assert_eq!(entrance.settles_at(), ms(260));
/// assert_eq!(body.sample(ms(150)).value, 0.5);
/// ```
///
/// A sequence is a schedule: it sets the motions' targets once, when each
/// step is added, and keeps only the times. A motion that is given another
/// target before its step ends leaves the sequence's times behind: the
/// sequence still says it settles when it would have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sequence {
    start: Duration,
    last: Step,
    settles: Duration,
}

/// When one step of a [`Sequence`] starts and when it ends, counted on the
/// same clock as the motions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    /// When the step's motion starts to move.
    pub starts: Duration,
    /// When the step's motion is settled on its target. For a spring this is
    /// a safe bound, as [`Motion::settles_at`] says.
    pub ends: Duration,
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ms to {} ms",
            self.starts.as_millis(),
            self.ends.as_millis()
        )
    }
}

impl Sequence {
    /// An empty sequence that starts at `now`, the time its steps are set.
    #[must_use]
    pub const fn starting_at(now: Duration) -> Self {
        let none = Step {
            starts: now,
            ends: now,
        };
        Self {
            start: now,
            last: none,
            settles: now,
        }
    }

    /// Adds a step that starts when the step before it ends, or when the
    /// sequence starts if it is the first.
    pub fn then<T: Animatable>(
        &mut self,
        motion: &mut Motion<T>,
        target: T,
        transition: impl Into<Transition>,
    ) -> Step {
        self.at(self.last.ends, motion, target, transition)
    }

    /// Adds a step that starts together with the step before it.
    pub fn with<T: Animatable>(
        &mut self,
        motion: &mut Motion<T>,
        target: T,
        transition: impl Into<Transition>,
    ) -> Step {
        self.at(self.last.starts, motion, target, transition)
    }

    /// Adds a step that starts at `starts`: a set time after another step
    /// starts, or before it ends, to overlap. A time before the sequence
    /// starts counts as its start. A delay on `transition` is added to it.
    pub fn at<T: Animatable>(
        &mut self,
        starts: Duration,
        motion: &mut Motion<T>,
        target: T,
        transition: impl Into<Transition>,
    ) -> Step {
        let starts = starts.max(self.start);
        let transition = transition.into();
        let delay = starts
            .saturating_sub(self.start)
            .saturating_add(transition.start_delay());
        motion.set_target(target, transition.delay(delay), self.start);
        let step = Step {
            starts,
            ends: motion.settles_at().unwrap_or(starts).max(starts),
        };
        self.last = step;
        self.settles = self.settles.max(step.ends);
        step
    }

    /// When the last of its steps ends.
    #[must_use]
    pub const fn settles_at(&self) -> Duration {
        self.settles
    }
}

/// Says once that a motion or a sequence has settled: for code that should
/// run once at the end, such as removing what has left or starting what
/// comes next.
///
/// Check it each frame with the time something settles at, such as
/// [`Motion::settles_at`] or [`Sequence::settles_at`]. It answers `true`
/// on the first check at or after that time, and not again for it.
///
/// Interrupted first, by a new target, the end it waited for never comes: it
/// answers `true` once for the new end instead. A motion put at rest at once,
/// with no motion, has no end to wait for, and is not reported.
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, Motion, SettledOnce, Tween};
///
/// let ms = Duration::from_millis;
/// let mut toast = Motion::at(1.0);
/// let mut gone = SettledOnce::default();
/// toast.set_target(0.0, Tween::new(ms(100), Easing::Linear), ms(0));
///
/// assert!(!gone.check(toast.settles_at(), ms(50)));
/// assert!(gone.check(toast.settles_at(), ms(116))); // the first frame at the end
/// assert!(!gone.check(toast.settles_at(), ms(132))); // and only once
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SettledOnce {
    reported: Option<Duration>,
}

impl SettledOnce {
    /// `true` once, on the first check at or after `settles_at`; `false`
    /// before it, after it was reported, and when there is no end.
    pub fn check(&mut self, settles_at: Option<Duration>, now: Duration) -> bool {
        match settles_at {
            Some(end) if now >= end && self.reported != Some(end) => {
                self.reported = Some(end);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use super::*;
    use crate::{Easing, Spring, Tween};

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn linear(millis: u64) -> Tween {
        Tween::new(ms(millis), Easing::Linear)
    }

    #[test]
    fn three_steps_run_in_order_and_may_overlap_as_the_schedule_shows() {
        let (mut card, mut title, mut button) = (Motion::at(0.0), Motion::at(0.0), Motion::at(0.0));
        let mut entrance = Sequence::starting_at(ms(1_000));
        let steps = [
            ("card", entrance.then(&mut card, 1.0, linear(200))),
            ("title", entrance.then(&mut title, 1.0, linear(150))),
        ];
        let overlap = entrance.at(
            steps[1].1.ends.saturating_sub(ms(50)),
            &mut button,
            1.0,
            linear(100),
        );
        let mut schedule = String::new();
        for (name, step) in steps.into_iter().chain([("button", overlap)]) {
            writeln!(schedule, "{name:<8}{step}").unwrap();
        }
        assert_eq!(
            schedule,
            "\
card    1000 ms to 1200 ms
title   1200 ms to 1350 ms
button  1300 ms to 1400 ms
"
        );
        assert_eq!(entrance.settles_at(), ms(1_400));
        // Each motion holds still until its step starts, then moves on time.
        assert_eq!(title.sample(ms(1_199)).value, 0.0);
        assert_eq!(title.sample(ms(1_275)).value, 0.5);
        assert_eq!(button.sample(ms(1_350)).value, 0.5);
        assert!(button.sample(ms(1_400)).settled);
    }

    #[test]
    fn a_step_with_the_one_before_starts_together_and_ends_on_its_own() {
        let (mut x, mut opacity) = (Motion::at(0.0), Motion::at(0.0));
        let mut both = Sequence::starting_at(ms(0));
        let slide = both.then(&mut x, 100.0, Spring::SNAPPY);
        let fade = both.with(&mut opacity, 1.0, linear(100));
        assert_eq!(fade.starts, slide.starts);
        assert_eq!(fade.ends, ms(100));
        assert_eq!(slide.ends, x.settles_at().unwrap());
        assert_eq!(both.settles_at(), slide.ends.max(fade.ends));
    }

    #[test]
    fn a_set_time_apart_and_a_delay_on_the_transition_add_up() {
        let (mut a, mut b) = (Motion::at(0.0), Motion::at(0.0));
        let mut sequence = Sequence::starting_at(ms(0));
        let first = sequence.then(&mut a, 1.0, linear(100));
        let second = sequence.at(
            first.starts + ms(30),
            &mut b,
            1.0,
            linear(100).delay(ms(20)),
        );
        assert_eq!(
            second,
            Step {
                starts: ms(30),
                ends: ms(150)
            }
        );
        assert_eq!(b.sample(ms(50)).value, 0.0);
        assert_eq!(b.sample(ms(100)).value, 0.5);
    }

    #[test]
    fn a_step_that_has_nowhere_to_go_ends_where_it_starts() {
        let mut still = Motion::at(1.0);
        let mut sequence = Sequence::starting_at(ms(0));
        let step = sequence.at(ms(40), &mut still, 1.0, linear(100));
        assert!(step.ends >= step.starts);
        assert_eq!(
            sequence
                .at(ms(0), &mut still, 1.0, Transition::Instant)
                .ends,
            ms(0)
        );
    }

    #[test]
    fn settled_once_answers_once_and_waits_for_the_new_end_when_interrupted() {
        let mut toast = Motion::at(1.0);
        let mut gone = SettledOnce::default();
        assert!(!gone.check(toast.settles_at(), ms(0)), "at rest: no end");
        toast.set_target(0.0, linear(100), ms(0));
        assert!(!gone.check(toast.settles_at(), ms(60)));
        // Wanted back before it ends: the first end never comes.
        toast.set_target(1.0, linear(100), ms(60));
        assert!(!gone.check(toast.settles_at(), ms(100)));
        assert!(!gone.check(toast.settles_at(), ms(150)));
        assert!(gone.check(toast.settles_at(), ms(160)));
        assert!(!gone.check(toast.settles_at(), ms(400)));
        // A sequence's end is checked the same way.
        let mut entrance = Sequence::starting_at(ms(500));
        entrance.then(&mut toast, 0.0, linear(100));
        assert!(!gone.check(Some(entrance.settles_at()), ms(560)));
        assert!(gone.check(Some(entrance.settles_at()), ms(600)));
    }
}
