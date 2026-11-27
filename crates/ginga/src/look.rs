use core::time::Duration;

use crate::{Motion, Sample, Transition, trace::Row};

/// How a thing should look: a set of visual values, each one optional.
///
/// A value the look does not name is left to whoever draws, who keeps the
/// value it has without the look. Build a look by naming values one at a time:
///
/// ```
/// use ginga::Look;
///
/// let card = Look::new().size(240.0, 160.0).opacity(1.0);
/// assert_eq!(card.width, Some(240.0));
/// assert_eq!(card.radius, None); // not named
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
    offset: Motion<Pixels<2>>,
    opacity: Motion<f32>,
    width: Motion<Pixels<1>>,
    height: Motion<Pixels<1>>,
    radius: Motion<Pixels<1>>,
}

/// Pixels, or any unit where a tenth is too small to see: an offset, a size
/// and a radius settle a tenth from their target, where an opacity settles a
/// thousandth from it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Pixels<const N: usize>([f32; N]);

impl<const N: usize> crate::Animatable for Pixels<N>
where
    [f32; N]: Default,
{
    type Channels = [f32; N];

    const PRECISION: f32 = 0.1;

    fn to_channels(self) -> [f32; N] {
        self.0
    }

    fn from_channels(channels: [f32; N]) -> Self {
        Self(channels)
    }
}

fn one(value: Option<f32>) -> Option<Pixels<1>> {
    value.map(|value| Pixels([value]))
}

fn two(value: Option<(f32, f32)>) -> Option<Pixels<2>> {
    value.map(|(x, y)| Pixels([x, y]))
}

impl LookMotion {
    /// A look at rest.
    #[must_use]
    pub fn at(look: Look) -> Self {
        Self {
            target: look,
            offset: Motion::at(two(look.offset).unwrap_or(Pixels([0.0; 2]))),
            opacity: Motion::at(look.opacity.unwrap_or_default()),
            width: Motion::at(one(look.width).unwrap_or(Pixels([0.0]))),
            height: Motion::at(one(look.height).unwrap_or(Pixels([0.0]))),
            radius: Motion::at(one(look.radius).unwrap_or(Pixels([0.0]))),
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
        retarget(
            &mut self.offset,
            two(was.offset),
            two(look.offset),
            transition,
            now,
        );
        retarget(
            &mut self.opacity,
            was.opacity,
            look.opacity,
            transition,
            now,
        );
        retarget(
            &mut self.width,
            one(was.width),
            one(look.width),
            transition,
            now,
        );
        retarget(
            &mut self.height,
            one(was.height),
            one(look.height),
            transition,
            now,
        );
        retarget(
            &mut self.radius,
            one(was.radius),
            one(look.radius),
            transition,
            now,
        );
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
            offset: read(target.offset, &self.offset, now, &mut settled)
                .map(|Pixels([x, y])| (x, y)),
            opacity: read(target.opacity, &self.opacity, now, &mut settled),
            width: read(target.width, &self.width, now, &mut settled).map(|Pixels([value])| value),
            height: read(target.height, &self.height, now, &mut settled)
                .map(|Pixels([value])| value),
            radius: read(target.radius, &self.radius, now, &mut settled)
                .map(|Pixels([value])| value),
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
fn read<T: crate::Animatable, U>(
    named: Option<U>,
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

impl crate::trace::sealed::Sealed for LookMotion {}

impl Look {
    /// Each value the look names, with its column's name, in the order a
    /// trace prints them. The one place that lists them for a trace.
    fn each_named(&self, mut visit: impl FnMut(&'static str, f32)) {
        if let Some((x, y)) = self.offset {
            visit("x", x);
            visit("y", y);
        }
        for (name, value) in [
            ("opacity", self.opacity),
            ("width", self.width),
            ("height", self.height),
            ("radius", self.radius),
        ] {
            if let Some(value) = value {
                visit(name, value);
            }
        }
    }
}

impl crate::trace::Traceable for LookMotion {
    fn row(&self, at: Duration) -> Row {
        let sample = self.sample(at);
        let mut values = Vec::new();
        sample.value.each_named(|_, value| values.push(value));
        Row {
            at,
            values,
            settled: sample.settled,
        }
    }

    fn columns(&self) -> Vec<&'static str> {
        let mut columns = Vec::new();
        self.target.each_named(|name, _| columns.push(name));
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
        let width = |card: &LookMotion, at| card.sample(ms(at)).value.width.unwrap();
        let mut card = LookMotion::at(Look::new().width(100.0));
        card.set_target(Look::new().width(200.0), Spring::SNAPPY, ms(0));
        let old_course = card;
        let before = width(&card, 100);
        card.set_target(Look::new().width(0.0), Spring::SNAPPY, ms(100));
        assert_eq!(width(&card, 100), before, "no jump");
        // In the next millisecond it moves as far as the old course would:
        // the speed is the same, only the pull has turned.
        let step = width(&card, 101) - before;
        let old_step = width(&old_course, 101) - before;
        assert!(step > 0.3, "it keeps going up for a moment: {step}");
        assert!((step - old_step).abs() < 0.1, "{step} against {old_step}");
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

    #[test]
    fn pixels_settle_a_tenth_from_their_target_and_opacity_a_thousandth() {
        let mut card = LookMotion::at(Look::new().size(180.0, 110.0));
        card.set_target(Look::new().size(360.0, 220.0), Spring::SNAPPY, ms(0));
        let trace = Trace::of(&card, ms(0), ms(48));
        assert_eq!(
            trace.rows().last().unwrap().at,
            ms(432),
            "not 624 ms, a thousandth away"
        );

        let mut fade = LookMotion::at(Look::new().opacity(0.0));
        fade.set_target(Look::new().opacity(1.0), Spring::SNAPPY, ms(0));
        // At 240 ms the opacity is within a tenth of 1.0, but not a thousandth.
        let close = fade.sample(ms(240));
        assert!(close.value.opacity.unwrap() > 0.9);
        assert!(!close.settled, "an opacity still moves a thousandth away");
    }

    #[test]
    fn a_trace_of_every_value_has_a_column_over_each_one_in_order() {
        let every = Look::new()
            .offset(1.0, 2.0)
            .opacity(0.5)
            .size(3.0, 4.0)
            .radius(5.0);
        let trace = Trace::of(&LookMotion::at(every), ms(0), ms(16));
        assert_eq!(
            trace.to_string(),
            "ms               x           y     opacity       width      height      radius\n\
             0            1.000       2.000       0.500       3.000       4.000       5.000  settled\n"
        );
    }

    #[test]
    fn samples_out_of_order_or_far_apart_give_the_same_look() {
        let mut card = LookMotion::at(Look::new().offset(0.0, 0.0).opacity(0.0));
        card.set_target(Look::new().offset(80.0, 40.0).opacity(1.0), glide(), ms(0));
        let in_order: Vec<_> = [20, 50, 80].map(|t| card.sample(ms(t)).value).into();
        let shuffled = [80, 20, 50].map(|t| (t, card.sample(ms(t)).value));
        for (t, look) in shuffled {
            let index = [20, 50, 80].iter().position(|&u| u == t).unwrap();
            assert_eq!(look, in_order[index], "at {t} ms");
        }
        assert_eq!(
            card.sample(ms(50)).value,
            Look::new().offset(40.0, 20.0).opacity(0.5)
        );
    }

    #[test]
    fn a_change_to_one_value_does_not_start_the_others_over() {
        let mut card = LookMotion::at(Look::new().width(100.0).opacity(0.0));
        card.set_target(Look::new().width(200.0).opacity(0.0), glide(), ms(0));
        card.set_target(Look::new().width(200.0).opacity(1.0), glide(), ms(50));
        let sample = card.sample(ms(100)).value;
        assert_eq!(
            sample.width,
            Some(200.0),
            "the width ends at 100 ms, not 150"
        );
        assert_eq!(sample.opacity, Some(0.5), "the opacity started at 50 ms");
    }

    #[test]
    fn a_value_dropped_while_it_moves_and_named_again_takes_its_value_at_once() {
        let mut card = LookMotion::at(Look::new().width(0.0).opacity(0.0));
        card.set_target(Look::new().width(100.0).opacity(1.0), glide(), ms(0));
        card.set_target(Look::new().opacity(1.0), glide(), ms(20));
        card.set_target(Look::new().width(300.0).opacity(1.0), glide(), ms(40));
        assert_eq!(card.sample(ms(40)).value.width, Some(300.0));
        assert_eq!(card.target(), Look::new().width(300.0).opacity(1.0));
    }

    #[test]
    fn jump_to_puts_the_look_at_rest_with_no_motion() {
        let mut card = LookMotion::at(Look::new().width(0.0));
        card.set_target(Look::new().width(100.0), glide(), ms(0));
        card.jump_to(Look::new().width(40.0).radius(4.0));
        let sample = card.sample(ms(10));
        assert_eq!(sample.value, Look::new().width(40.0).radius(4.0));
        assert!(sample.settled);
    }
}
