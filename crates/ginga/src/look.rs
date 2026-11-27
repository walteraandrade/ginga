use core::time::Duration;

use crate::{Motion, Sample, Transition, trace::Row};

/// How a thing should look: a set of visual values, each one optional.
///
/// A value the look does not name is left to whoever draws: the element keeps
/// its own style for it. Build a look by naming values one at a time:
///
/// ```
/// use ginga::Look;
///
/// let card = Look::new().size(240.0, 160.0).opacity(1.0);
/// assert_eq!(card.width, Some(240.0));
/// assert_eq!(card.radius, None); // left to the element
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Look {
    /// How far the thing sits from its own place, across and down.
    pub offset: Option<(f32, f32)>,
    /// How opaque it is: `0.0` invisible, `1.0` opaque.
    pub opacity: Option<f32>,
    /// Its width.
    pub width: Option<f32>,
    /// Its height.
    pub height: Option<f32>,
    /// The radius of its corners.
    pub radius: Option<f32>,
}

impl Look {
    /// A look that names nothing.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            offset: None,
            opacity: None,
            width: None,
            height: None,
            radius: None,
        }
    }

    /// Names the offset from the thing's own place: `x` across, `y` down.
    #[must_use]
    pub const fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset = Some((x, y));
        self
    }

    /// Names the opacity.
    #[must_use]
    pub const fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity);
        self
    }

    /// Names the width.
    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Names the height.
    #[must_use]
    pub const fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Names the width and the height.
    #[must_use]
    pub const fn size(self, width: f32, height: f32) -> Self {
        self.width(width).height(height)
    }

    /// Names the radius of the corners.
    #[must_use]
    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }
}

/// A [`Look`] on its way to another: one [`Motion`] for each value it names.
///
/// Hand it the look a thing should have now, each frame, as with a motion. A
/// value that the last look named moves from where it is, and a spring keeps
/// its speed. A value the last look did not name has no place to start from,
/// so it takes the new value at once. A value the new look does not name is
/// left out of the sample.
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, Look, LookMotion, Tween};
///
/// let ms = Duration::from_millis;
/// let glide = Tween::new(ms(100), Easing::Linear);
/// let mut card = LookMotion::at(Look::new().width(100.0).opacity(0.0));
///
/// card.toward(Look::new().width(200.0).opacity(1.0), glide, ms(0));
/// let halfway = card.toward(Look::new().width(200.0).opacity(1.0), glide, ms(50));
/// assert_eq!(halfway.value, Look::new().width(150.0).opacity(0.5));
/// assert!(!halfway.settled);
/// ```
///
/// A look motion is written down like a motion, with a column for each value
/// the look names: see [`Trace`](crate::Trace).
#[derive(Debug, Clone, Copy)]
pub struct LookMotion {
    target: Look,
    offset: Motion<(f32, f32)>,
    opacity: Motion<f32>,
    width: Motion<f32>,
    height: Motion<f32>,
    radius: Motion<f32>,
}

impl LookMotion {
    /// A look at rest.
    #[must_use]
    pub fn at(look: Look) -> Self {
        Self {
            target: look,
            offset: Motion::at(look.offset.unwrap_or_default()),
            opacity: Motion::at(look.opacity.unwrap_or_default()),
            width: Motion::at(look.width.unwrap_or_default()),
            height: Motion::at(look.height.unwrap_or_default()),
            radius: Motion::at(look.radius.unwrap_or_default()),
        }
    }

    /// Heads for `look` and returns the look at `now`.
    ///
    /// Like [`Motion::toward`], it changes course only when `look` differs
    /// from the last one, and `transition` is used for that change of course.
    pub fn toward(
        &mut self,
        look: Look,
        transition: impl Into<Transition>,
        now: Duration,
    ) -> Sample<Look> {
        if look != self.target {
            self.set_target(look, transition, now);
        }
        self.sample(now)
    }

    /// Changes course at `now`: each value the last look named moves from
    /// where it is then, and each value it did not name takes its new value at
    /// once.
    pub fn set_target(&mut self, look: Look, transition: impl Into<Transition>, now: Duration) {
        let transition = transition.into();
        let was = self.target;
        retarget(&mut self.offset, was.offset, look.offset, transition, now);
        retarget(
            &mut self.opacity,
            was.opacity,
            look.opacity,
            transition,
            now,
        );
        retarget(&mut self.width, was.width, look.width, transition, now);
        retarget(&mut self.height, was.height, look.height, transition, now);
        retarget(&mut self.radius, was.radius, look.radius, transition, now);
        self.target = look;
    }

    /// Puts the look at `look`, at rest, with no motion.
    pub fn jump_to(&mut self, look: Look) {
        *self = Self::at(look);
    }

    /// The look at `now`, with the values the target names. It never changes
    /// the motion.
    #[must_use]
    pub fn sample(&self, now: Duration) -> Sample<Look> {
        let mut settled = true;
        let target = self.target;
        let look = Look {
            offset: read(target.offset, &self.offset, now, &mut settled),
            opacity: read(target.opacity, &self.opacity, now, &mut settled),
            width: read(target.width, &self.width, now, &mut settled),
            height: read(target.height, &self.height, now, &mut settled),
            radius: read(target.radius, &self.radius, now, &mut settled),
        };
        Sample {
            value: look,
            settled,
        }
    }

    /// Where the look is heading.
    #[must_use]
    pub fn target(&self) -> Look {
        self.target
    }
}

/// One value of a look at `now`, when the target names it.
fn read<T: crate::Animatable>(
    named: Option<T>,
    motion: &Motion<T>,
    now: Duration,
    settled: &mut bool,
) -> Option<T> {
    named.map(|_| {
        let sample = motion.sample(now);
        *settled &= sample.settled;
        sample.value
    })
}

/// Moves one value of a look toward its new target.
fn retarget<T: crate::Animatable + PartialEq>(
    motion: &mut Motion<T>,
    was: Option<T>,
    now_named: Option<T>,
    transition: Transition,
    now: Duration,
) {
    match (was, now_named) {
        (Some(_), Some(value)) => {
            if motion.target() != value {
                motion.set_target(value, transition, now);
            }
        }
        (None, Some(value)) => motion.jump_to(value),
        (_, None) => {}
    }
}

impl crate::trace::Traceable for LookMotion {
    fn row(&self, at: Duration) -> Row {
        let sample = self.sample(at);
        let look = sample.value;
        let mut values = Vec::new();
        if let Some((x, y)) = look.offset {
            values.extend([x, y]);
        }
        values.extend(
            [look.opacity, look.width, look.height, look.radius]
                .into_iter()
                .flatten(),
        );
        Row {
            at,
            values,
            settled: sample.settled,
        }
    }

    fn columns(&self) -> Vec<&'static str> {
        let target = self.target;
        let mut columns = Vec::new();
        if target.offset.is_some() {
            columns.extend(["x", "y"]);
        }
        for (name, named) in [
            ("opacity", target.opacity.is_some()),
            ("width", target.width.is_some()),
            ("height", target.height.is_some()),
            ("radius", target.radius.is_some()),
        ] {
            if named {
                columns.push(name);
            }
        }
        columns
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Easing, Spring, Trace, Tween};

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn glide() -> Tween {
        Tween::new(ms(100), Easing::Linear)
    }

    #[test]
    fn a_look_is_written_down_with_a_column_for_each_value_it_names() {
        let mut card = LookMotion::at(Look::new().offset(0.0, 0.0).opacity(0.0));
        card.set_target(Look::new().offset(40.0, 20.0).opacity(1.0), glide(), ms(0));
        assert_eq!(
            Trace::of(&card, ms(0), ms(50)).to_string(),
            "\
ms               x           y     opacity
0            0.000       0.000       0.000
50          20.000      10.000       0.500
100         40.000      20.000       1.000  settled
"
        );
    }

    #[test]
    fn a_change_of_course_starts_where_the_look_is_and_a_spring_keeps_its_speed() {
        let mut card = LookMotion::at(Look::new().width(100.0));
        card.set_target(Look::new().width(200.0), Spring::SNAPPY, ms(0));
        let before = card.sample(ms(100)).value.width.unwrap();
        let speed = card.width.velocity(ms(100));
        card.set_target(Look::new().width(0.0), Spring::SNAPPY, ms(100));
        assert_eq!(card.sample(ms(100)).value.width, Some(before), "no jump");
        let [kept] = card.width.velocity(ms(100));
        assert!(
            (kept - speed[0]).abs() < 0.01,
            "the speed is kept: {kept} against {speed:?}"
        );
        assert!(
            card.sample(ms(102)).value.width.unwrap() > before,
            "it keeps going the same way for a moment before it turns"
        );
    }

    #[test]
    fn a_value_the_last_look_did_not_name_takes_its_value_at_once() {
        let mut card = LookMotion::at(Look::new().opacity(0.0));
        let sample = card.toward(Look::new().opacity(1.0).radius(8.0), glide(), ms(0));
        assert_eq!(sample.value, Look::new().opacity(0.0).radius(8.0));
        let later = card.sample(ms(50)).value;
        assert_eq!((later.opacity, later.radius), (Some(0.5), Some(8.0)));
    }

    #[test]
    fn a_value_the_look_does_not_name_is_left_out_and_does_not_hold_it_back() {
        let mut card = LookMotion::at(Look::new().width(0.0).opacity(0.0));
        card.set_target(Look::new().width(100.0).opacity(1.0), Spring::SMOOTH, ms(0));
        card.set_target(Look::new().opacity(1.0), glide(), ms(10));
        let sample = card.sample(ms(2_000));
        assert_eq!(sample.value, Look::new().opacity(1.0));
        assert!(
            sample.settled,
            "the width still moving is not drawn, so it is not waited for"
        );
    }

    #[test]
    fn a_settled_look_is_exactly_on_its_target() {
        let target = Look::new()
            .offset(-12.5, 3.0)
            .size(320.0, 180.0)
            .radius(6.0);
        let mut card = LookMotion::at(Look::new().offset(0.0, 0.0).size(0.0, 0.0).radius(0.0));
        card.set_target(target, Spring::BOUNCY, ms(0));
        let trace = Trace::of(&card, ms(0), ms(16));
        let last = trace.rows().last().unwrap();
        assert!(last.settled);
        assert_eq!(last.values, [-12.5, 3.0, 320.0, 180.0, 6.0]);
        assert_eq!(card.sample(last.at).value, target);
    }
}
