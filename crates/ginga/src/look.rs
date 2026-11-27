use core::time::Duration;

use crate::{Color, Easing, Motion, Sample, Spring, Transition, Tween, trace::Row};

/// How a thing should look: a set of visual values, each one optional.
///
/// A value the look does not name is left to whoever draws, who keeps the
/// value it has without the look. A value that is not finite, such as
/// `f32::NAN`, counts as not named. Build a look by naming values one at a time:
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
    /// The color it is filled with.
    pub background: Option<Color>,
    /// The color of its text.
    pub text_color: Option<Color>,
    /// The color of its border.
    pub border_color: Option<Color>,
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
            background: None,
            text_color: None,
            border_color: None,
        }
    }

    /// Names the offset from the thing's own place: `x` across, `y` down.
    #[must_use]
    pub const fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset = Some((x, y));
        self
    }

    /// Names the opacity: 0 is unseen, 1 is fully seen. A spring can carry
    /// it past either end on the way; whoever draws it holds it to 0..=1.
    #[must_use]
    pub const fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = Some(opacity);
        self
    }

    /// Names the width, in pixels. A spring can carry it below 0 on the way;
    /// whoever draws it holds it at 0 or more.
    #[must_use]
    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Names the height, in pixels. A spring can carry it below 0 on the way;
    /// whoever draws it holds it at 0 or more.
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

    /// Names the radius of the corners, in pixels. A spring can carry it
    /// below 0 on the way; whoever draws it holds it at 0 or more.
    #[must_use]
    pub const fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Names the color it is filled with.
    #[must_use]
    pub const fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Names the color of its text.
    #[must_use]
    pub const fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Names the color of its border.
    #[must_use]
    pub const fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// This look laid over `under`: each value this one names, and where it
    /// names none, the value `under` names. A look for a moment, such as
    /// while a pointer is over a thing, laid over the look it has otherwise.
    ///
    /// ```
    /// use ginga::Look;
    ///
    /// let rest = Look::new().size(120.0, 40.0).opacity(0.8);
    /// let hover = Look::new().opacity(1.0);
    /// assert_eq!(hover.or(rest), Look::new().size(120.0, 40.0).opacity(1.0));
    /// ```
    #[must_use]
    pub const fn or(self, under: Look) -> Self {
        // `Option::or` is not const yet.
        macro_rules! first {
            ($($field:ident),*) => {
                Self {
                    $($field: match self.$field {
                        Some(value) => Some(value),
                        None => under.$field,
                    },)*
                }
            };
        }
        first!(
            offset,
            opacity,
            width,
            height,
            radius,
            background,
            text_color,
            border_color
        )
    }
}

impl Look {
    /// The look `part` of the way from this one to `to`: `0.0` gives this
    /// one for what both name, `1.0` gives `to` exactly, and a part past either end goes on in
    /// the same line, as a spring's overshoot should. NaN counts as `0.0`.
    ///
    /// A value both looks name moves in a straight line; a color mixes in
    /// linear light. An offset or an opacity that only one of them names
    /// moves from or to where a thing sits without one: no offset, and fully
    /// opaque. Any other value that only `to` names stays as `to` has it, and
    /// any other value that only this look names is left out.
    ///
    /// It is how a way in or out is drawn: the look a thing arrives from,
    /// mixed into its own look by how far it has arrived.
    ///
    /// ```
    /// use ginga::Look;
    ///
    /// let own = Look::new().width(200.0);
    /// let from = Look::new().offset(0.0, 40.0).opacity(0.0);
    /// assert_eq!(
    ///     from.mix(own, 0.25),
    ///     Look::new().offset(0.0, 30.0).opacity(0.25).width(200.0)
    /// );
    /// assert_eq!(from.mix(own, 1.0), own);
    /// ```
    #[must_use]
    pub fn mix(self, to: Look, part: f32) -> Look {
        let (from, to) = (self.finite(), to.finite());
        let part = if part.is_nan() { 0.0 } else { part };
        if part == 1.0 {
            return to;
        }
        let at_rest = |value: Option<f32>, rest| value.or(Some(rest));
        let offset = |value: Option<(f32, f32)>| value.map(|(x, y)| Pixels([x, y]));
        let moving = from.offset.is_some() || to.offset.is_some();
        Look {
            offset: moving
                .then(|| {
                    let rest = Some(Pixels([0.0, 0.0]));
                    mix(
                        offset(from.offset).or(rest),
                        offset(to.offset).or(rest),
                        part,
                    )
                })
                .flatten()
                .map(|Pixels([x, y])| (x, y)),
            opacity: (from.opacity.is_some() || to.opacity.is_some())
                .then(|| mix(at_rest(from.opacity, 1.0), at_rest(to.opacity, 1.0), part))
                .flatten(),
            width: mix(from.width, to.width, part),
            height: mix(from.height, to.height, part),
            radius: mix(from.radius, to.radius, part),
            background: mix(from.background, to.background, part),
            text_color: mix(from.text_color, to.text_color, part),
            border_color: mix(from.border_color, to.border_color, part),
        }
    }
}

/// One value of [`Look::mix`]: in a straight line when both name it, as
/// `to` has it when only `to` does, and left out otherwise.
fn mix<T: crate::Animatable>(from: Option<T>, to: Option<T>, part: f32) -> Option<T> {
    let (from, to) = match (from, to) {
        (Some(from), Some(to)) => (from, to),
        (_, to) => return to,
    };
    if part == 0.0 {
        return Some(from);
    }
    let mut channels = from.to_channels();
    let end = to.to_channels();
    for (channel, end) in channels.as_mut().iter_mut().zip(end.as_ref()) {
        *channel += (end - *channel) * part;
    }
    Some(T::from_channels(channels))
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

/// How long an opacity or a color takes when nothing else is said.
const FADE: Tween = Tween::new(Duration::from_millis(150), Easing::Out);

/// The values of a look, in one table: each one's name, the motion that
/// moves it, how it goes to that motion and back, and the transition it has
/// when nothing else is said. Everything that goes through each value of a
/// [`LookMotion`] or of a [`Transitions`] is written from this table: the
/// motion, the transitions, `at`, `set_target` and `sample`. A new value also
/// needs its field and builder on [`Look`], a line in `Look::finite` and its
/// trace columns, which are written by hand.
macro_rules! values {
    ($($field:ident: $repr:ty = $to:expr, $from:expr, $default:expr, $kind:literal;)*) => {
        /// A [`Look`] on its way to another: one [`Motion`] for each value it names.
        ///
        /// Hand it the look a thing should have now, each frame, as with a
        /// motion. A value that the last look named moves from where it is,
        /// and a spring keeps its speed. A value the last look did not name has
        /// no place to start from, so it takes the new value at once. A value
        /// the new look does not name is left out of the sample.
        ///
        /// Each value moves with its own transition, from [`Transitions`]: a
        /// single [`Transition`] given for the whole look moves every value.
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
        /// A look motion is written down like a motion, with a column for each
        /// value the look names: see [`Trace`](crate::Trace).
        #[derive(Debug, Clone, Copy)]
        pub struct LookMotion {
            target: Look,
            $($field: Motion<$repr>,)*
        }

        /// A transition for each value of a [`Look`].
        ///
        /// A value's own transition wins over the one for the whole look, set
        /// with [`all`](Self::all), which wins over the defaults under it
        /// ([`or`](Self::or)), which win over ginga's own: a spring
        /// ([`Spring::SNAPPY`]) for the offset, the size and the radius, and a
        /// 150 ms tween ([`Easing::Out`]) for the opacity and the colors.
        ///
        /// A single [`Transition`], [`Spring`] or [`Tween`] is the transition
        /// for the whole look.
        ///
        /// ```
        /// use std::time::Duration;
        /// use ginga::{Easing, Look, LookMotion, Spring, Transitions, Tween};
        ///
        /// let ms = Duration::from_millis;
        /// let mut card = LookMotion::at(Look::new().offset(0.0, 40.0).opacity(0.0));
        /// // The card springs to its place and fades in with a linear tween.
        /// let fade = Tween::new(ms(100), Easing::Linear);
        /// let shown = Look::new().offset(0.0, 0.0).opacity(1.0);
        /// card.set_target(shown, Transitions::new().opacity(fade), ms(0));
        /// assert_eq!(card.sample(ms(50)).value.opacity, Some(0.5));
        /// assert!(card.sample(ms(100)).value.offset.unwrap().1 > 0.0, "still on its way");
        /// ```
        #[derive(Debug, Clone, Copy, Default)]
        pub struct Transitions {
            /// The values' own transitions.
            own: Layer,
            /// The transition for the whole look.
            all: Option<Transition>,
            /// The defaults laid under these with [`or`](Self::or), already
            /// resolved value by value.
            under: Layer,
        }

        /// One transition, or none, for each value.
        #[derive(Debug, Clone, Copy, Default)]
        struct Layer {
            $($field: Option<Transition>,)*
        }

        impl Layer {
            const EMPTY: Self = Self { $($field: None,)* };
        }

        impl Transitions {
            /// Names no transition: each value moves with the defaults.
            #[must_use]
            pub const fn new() -> Self {
                Self {
                    own: Layer::EMPTY,
                    all: None,
                    under: Layer::EMPTY,
                }
            }

            /// The transition for every value this does not give one of its own.
            #[must_use]
            pub fn all(mut self, transition: impl Into<Transition>) -> Self {
                self.all = Some(transition.into());
                self
            }

            $(
                #[doc = concat!("The transition for the ", $kind, ".")]
                #[must_use]
                pub fn $field(mut self, transition: impl Into<Transition>) -> Self {
                    self.own.$field = Some(transition.into());
                    self
                }
            )*

            /// These transitions over `defaults`: for each value, its own
            /// transition here, then the one for the whole look here, then
            /// what `defaults` gives it. Keep a set of defaults once and give
            /// each look only what differs. The order holds whatever order the
            /// calls come in: an [`all`](Self::all) after `or` still wins over
            /// the defaults.
            ///
            /// ```
            /// use std::time::Duration;
            /// use ginga::{Look, LookMotion, Spring, Transition, Transitions};
            ///
            /// let ms = Duration::from_millis;
            /// let app = Transitions::new().all(Spring::BOUNCY);
            /// let this_card = Transitions::new().opacity(Transition::Instant);
            ///
            /// let mut card = LookMotion::at(Look::new().width(0.0).opacity(0.0));
            /// card.set_target(Look::new().width(100.0).opacity(1.0), this_card.or(app), ms(0));
            /// let now = card.sample(ms(0)).value;
            /// assert_eq!(now.opacity, Some(1.0)); // its own: at once
            /// assert_eq!(now.width, Some(0.0)); // the app's: a spring, on its way
            /// ```
            #[must_use]
            pub fn or(self, defaults: Transitions) -> Self {
                Self {
                    under: Layer {
                        $($field: self.under.$field.or(defaults.named(|layer| layer.$field)),)*
                    },
                    ..self
                }
            }

            /// The transition this names for one value, layer by layer: its
            /// own, the whole look's, then the defaults under it. The one place
            /// that says the order.
            fn named(&self, value: impl Fn(&Layer) -> Option<Transition>) -> Option<Transition> {
                value(&self.own).or(self.all).or(value(&self.under))
            }
        }

        impl LookMotion {
            /// A look at rest.
            #[must_use]
            pub fn at(look: Look) -> Self {
                use crate::Animatable;
                let look = look.finite();
                Self {
                    target: look,
                    $($field: Motion::at(
                        look.$field
                            .map($to)
                            .unwrap_or_else(|| <$repr>::from_channels(Default::default())),
                    ),)*
                }
            }

            /// Changes course at `now`: each value the last look named moves
            /// from where it is then, with its transition from `transitions`,
            /// and each value it did not name takes its new value at once.
            ///
            /// A value whose target is the same as before keeps its course,
            /// whatever its transition is: a look changes as a whole, and the
            /// values it does not change are left alone. To put every value on
            /// its target at once, use [`jump_to`](Self::jump_to).
            pub fn set_target(&mut self, look: Look, transitions: impl Into<Transitions>, now: Duration) {
                let look = look.finite();
                let transitions = transitions.into();
                let was = self.target;
                $(retarget(
                    &mut self.$field,
                    was.$field.map($to),
                    look.$field.map($to),
                    transitions.named(|layer| layer.$field).unwrap_or($default),
                    now,
                );)*
                self.target = look;
            }

            /// The look at `now`, with the values the target names. It never
            /// changes the motion.
            #[must_use]
            pub fn sample(&self, now: Duration) -> Sample<Look> {
                let mut settled = true;
                let target = self.target;
                let look = Look {
                    $($field: read(target.$field.map($to), &self.$field, now, &mut settled).map($from),)*
                };
                Sample {
                    value: look,
                    settled,
                }
            }
        }
    };
}

values! {
    offset: Pixels<2> = |(x, y)| Pixels([x, y]), |Pixels([x, y])| (x, y), Transition::Spring(Spring::SNAPPY), "offset";
    opacity: f32 = |value| value, |value| value, Transition::Tween(FADE), "opacity";
    width: Pixels<1> = |value| Pixels([value]), |Pixels([value])| value, Transition::Spring(Spring::SNAPPY), "width";
    height: Pixels<1> = |value| Pixels([value]), |Pixels([value])| value, Transition::Spring(Spring::SNAPPY), "height";
    radius: Pixels<1> = |value| Pixels([value]), |Pixels([value])| value, Transition::Spring(Spring::SNAPPY), "radius of the corners";
    background: Color = |value| value, |value| value, Transition::Tween(FADE), "background color";
    text_color: Color = |value| value, |value| value, Transition::Tween(FADE), "text color";
    border_color: Color = |value| value, |value| value, Transition::Tween(FADE), "border color";
}

impl From<Transition> for Transitions {
    fn from(transition: Transition) -> Self {
        Self::new().all(transition)
    }
}

impl From<Spring> for Transitions {
    fn from(spring: Spring) -> Self {
        Self::new().all(spring)
    }
}

impl From<Tween> for Transitions {
    fn from(tween: Tween) -> Self {
        Self::new().all(tween)
    }
}

impl LookMotion {
    /// Heads for `look` and returns the look at `now`.
    ///
    /// Like [`Motion::toward`], it changes course only when `look` differs
    /// from the last one, and `transitions` are used for that change of course.
    pub fn toward(
        &mut self,
        look: Look,
        transitions: impl Into<Transitions>,
        now: Duration,
    ) -> Sample<Look> {
        let look = look.finite();
        if look != self.target {
            self.set_target(look, transitions, now);
        }
        self.sample(now)
    }

    /// Puts the look at `look`, at rest, with no motion.
    pub fn jump_to(&mut self, look: Look) {
        *self = Self::at(look);
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
        (Some(was), Some(value)) => {
            if was != value {
                motion.set_target(value, transition, now);
            }
        }
        (None, Some(value)) => motion.jump_to(value),
        (_, None) => {}
    }
}

impl crate::trace::sealed::Sealed for LookMotion {}

impl Look {
    /// The look with each value that is not finite left out, so that a NaN
    /// from the caller's arithmetic cannot keep a motion from settling.
    fn finite(self) -> Self {
        let finite = |value: f32| value.is_finite();
        Self {
            offset: self.offset.filter(|&(x, y)| finite(x) && finite(y)),
            opacity: self.opacity.filter(|&value| finite(value)),
            width: self.width.filter(|&value| finite(value)),
            height: self.height.filter(|&value| finite(value)),
            radius: self.radius.filter(|&value| finite(value)),
            background: self.background.filter(|color| color.is_finite()),
            text_color: self.text_color.filter(|color| color.is_finite()),
            border_color: self.border_color.filter(|color| color.is_finite()),
        }
    }

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
        for (names, color) in [
            (["bg r", "bg g", "bg b", "bg a"], self.background),
            (["text r", "text g", "text b", "text a"], self.text_color),
            (
                ["border r", "border g", "border b", "border a"],
                self.border_color,
            ),
        ] {
            if let Some(color) = color {
                let values = [color.red, color.green, color.blue, color.alpha];
                for (name, value) in names.into_iter().zip(values) {
                    visit(name, value);
                }
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
        let mut card = LookMotion::at(Look::new().width(0.0).opacity(1.0));
        let slow = Tween::new(ms(1_000), Easing::Linear);
        card.set_target(Look::new().width(100.0).opacity(1.0), slow, ms(0));
        card.set_target(Look::new().opacity(1.0), slow, ms(10));
        let sample = card.sample(ms(150));
        assert!(!card.width.sample(ms(150)).settled, "the width still moves");
        assert_eq!(sample.value, Look::new().opacity(1.0));
        assert!(
            sample.settled,
            "the width still moving is not drawn, so it is not waited for"
        );
    }

    #[test]
    fn a_value_that_is_not_finite_counts_as_not_named() {
        let mut card = LookMotion::at(Look::new().width(10.0));
        let sample = card.toward(Look::new().width(f32::NAN).opacity(0.5), glide(), ms(0));
        assert_eq!(sample.value, Look::new().opacity(0.5));
        assert!(sample.settled, "a NaN does not keep the look moving");
        let again = card.toward(
            Look::new().width(f32::INFINITY).opacity(0.5),
            glide(),
            ms(16),
        );
        assert!(again.settled);
        assert_eq!(card.target(), Look::new().opacity(0.5));
    }

    #[test]
    fn a_value_with_the_same_target_keeps_its_course_whatever_the_transition() {
        let mut card = LookMotion::at(Look::new().width(0.0).opacity(0.0));
        let slow = Tween::new(ms(1_000), Easing::Linear);
        card.set_target(Look::new().width(100.0).opacity(0.0), slow, ms(0));
        card.set_target(
            Look::new().width(100.0).opacity(1.0),
            Transition::Instant,
            ms(50),
        );
        assert_eq!(card.sample(ms(50)).value.width, Some(5.0));
        assert_eq!(card.sample(ms(50)).value.opacity, Some(1.0));
    }

    #[test]
    fn colors_move_between_two_looks_and_settle_exactly_on_theirs() {
        let red = Color::hex(0xff_0000);
        let blue = Color::hex(0x00_00ff);
        let ink = Color::hex(0x16_181c);
        let mut button = LookMotion::at(
            Look::new()
                .background(red)
                .text_color(Color::TRANSPARENT)
                .border_color(red),
        );
        let pressed = Look::new()
            .background(blue)
            .text_color(ink)
            .border_color(blue);
        button.set_target(pressed, glide(), ms(0));
        let halfway = button.sample(ms(50)).value;
        let background = halfway.background.unwrap();
        assert!((background.red - 0.735).abs() < 1e-3);
        assert_eq!(background.green, 0.0, "red to blue never turns green");
        let text = halfway.text_color.unwrap();
        assert!((text.red - ink.red).abs() < 1e-3, "fading in, not darker");
        assert!((text.alpha - 0.5).abs() < 1e-6);

        let end = button.sample(ms(100));
        assert!(end.settled);
        assert_eq!(end.value, pressed, "exactly the colors named");
    }

    #[test]
    fn a_color_that_did_not_change_keeps_its_course_when_another_value_does() {
        // 0x3b6ef5 does not come back from its channels to the last bit; a
        // value rebuilt from them would look changed and start over.
        let blue = Color::hex(0x3b_6ef5);
        let mut card = LookMotion::at(Look::new().background(Color::hex(0xff_0000)).opacity(0.0));
        card.set_target(Look::new().background(blue).opacity(0.0), glide(), ms(0));
        card.set_target(Look::new().background(blue).opacity(1.0), glide(), ms(50));
        let at_end = card.sample(ms(100)).value;
        assert_eq!(
            at_end.background,
            Some(blue),
            "on time, not started over at 50 ms"
        );
    }

    #[test]
    fn a_trace_names_a_column_for_each_channel_of_a_color() {
        let mut button = LookMotion::at(Look::new().opacity(0.0).background(Color::TRANSPARENT));
        button.set_target(
            Look::new().opacity(1.0).background(Color::hex(0xff_ffff)),
            glide(),
            ms(0),
        );
        let trace = Trace::of(&button, ms(0), ms(50)).to_string();
        let lines: Vec<&str> = trace.lines().collect();
        assert_eq!(
            lines[0].split_whitespace().collect::<Vec<_>>(),
            ["ms", "opacity", "bg", "r", "bg", "g", "bg", "b", "bg", "a"]
        );
        assert_eq!(
            lines[2].split_whitespace().collect::<Vec<_>>(),
            ["50", "0.500", "1.000", "1.000", "1.000", "0.500"]
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
    fn with_no_transition_sizes_spring_and_opacity_and_colors_fade() {
        let white = Color::hex(0xff_ffff);
        let mut card = LookMotion::at(
            Look::new()
                .offset(0.0, 0.0)
                .size(0.0, 0.0)
                .radius(0.0)
                .opacity(0.0)
                .background(Color::TRANSPARENT)
                .text_color(Color::TRANSPARENT)
                .border_color(Color::TRANSPARENT),
        );
        let shown = Look::new()
            .offset(100.0, 100.0)
            .size(100.0, 100.0)
            .radius(100.0)
            .opacity(1.0)
            .background(white)
            .text_color(white)
            .border_color(white);
        card.set_target(shown, Transitions::new(), ms(0));
        // Each value against a motion of its own with the default it should have.
        let mut spring = Motion::at(0.0);
        spring.set_target(100.0, Spring::SNAPPY, ms(0));
        let mut fade = Motion::at(0.0);
        fade.set_target(1.0, FADE, ms(0));
        let mut color = Motion::at(Color::TRANSPARENT);
        color.set_target(white, FADE, ms(0));
        for at in [ms(30), ms(60), ms(120)] {
            let (look, spring, fade, color) = (
                card.sample(at).value,
                spring.sample(at).value,
                fade.sample(at).value,
                Some(color.sample(at).value),
            );
            assert_eq!(look.offset, Some((spring, spring)), "at {at:?}");
            assert_eq!(
                (look.width, look.height, look.radius),
                (Some(spring), Some(spring), Some(spring))
            );
            assert_eq!(look.opacity, Some(fade));
            assert_eq!(
                (look.background, look.text_color, look.border_color),
                (color, color, color)
            );
        }
        // The fades are 150 ms tweens: exactly there at 150 ms.
        let at_150 = card.sample(ms(150));
        assert_eq!(at_150.value.opacity, Some(1.0));
        assert_eq!(at_150.value.border_color, Some(white));
        assert!(!at_150.settled, "the spring is still on its way");
    }

    #[test]
    fn an_all_after_or_still_wins_over_the_defaults() {
        let slow = Tween::new(ms(1_000), Easing::Linear);
        let fast = Tween::new(ms(100), Easing::Linear);
        let start = Look::new().width(0.0);
        let end = Look::new().width(100.0);
        for transitions in [
            Transitions::new()
                .all(fast)
                .or(Transitions::new().width(slow)),
            Transitions::new()
                .or(Transitions::new().width(slow))
                .all(fast),
            Transitions::new()
                .all(slow)
                .or(Transitions::new())
                .all(fast),
        ] {
            let mut card = LookMotion::at(start);
            card.set_target(end, transitions, ms(0));
            assert_eq!(
                card.sample(ms(50)).value.width,
                Some(50.0),
                "{transitions:?}"
            );
        }
    }

    #[test]
    fn three_layers_each_give_the_values_they_name() {
        let linear = |millis| Tween::new(ms(millis), Easing::Linear);
        // An element over a view over an app, each naming a value and an all.
        let element = Transitions::new().width(linear(100));
        let view = Transitions::new().opacity(linear(200)).radius(linear(400));
        let app = Transitions::new()
            .all(linear(1_000))
            .radius(Transition::Instant);
        let mut card = LookMotion::at(Look::new().width(0.0).opacity(0.0).radius(0.0).height(0.0));
        card.set_target(
            Look::new()
                .width(100.0)
                .opacity(1.0)
                .radius(100.0)
                .height(100.0),
            element.or(view).or(app),
            ms(0),
        );
        let now = card.sample(ms(50)).value;
        assert_eq!(now.width, Some(50.0), "the element's own");
        assert_eq!(now.opacity, Some(0.25), "the view's");
        assert_eq!(
            now.radius,
            Some(12.5),
            "the view's, over the app's own for it"
        );
        assert_eq!(now.height, Some(5.0), "the app's all");
    }

    #[test]
    fn a_values_own_transition_wins_over_the_looks_which_wins_over_the_defaults() {
        let slow = Tween::new(ms(1_000), Easing::Linear);
        let fast = Tween::new(ms(100), Easing::Linear);
        let defaults = Transitions::new().all(slow).radius(Transition::Instant);
        let start = Look::new().width(0.0).opacity(0.0).radius(0.0).height(0.0);
        let end = Look::new()
            .width(100.0)
            .opacity(1.0)
            .radius(10.0)
            .height(100.0);

        // The value's own over the look's.
        let mut card = LookMotion::at(start);
        card.set_target(end, Transitions::new().all(fast).opacity(slow), ms(0));
        let now = card.sample(ms(50)).value;
        assert_eq!((now.width, now.opacity), (Some(50.0), Some(0.05)));

        // The look's over the defaults', even over a default for one value.
        let mut card = LookMotion::at(start);
        card.set_target(end, Transitions::new().all(fast).or(defaults), ms(0));
        let now = card.sample(ms(50)).value;
        assert_eq!((now.width, now.radius), (Some(50.0), Some(5.0)));

        // With nothing given here, the defaults, the one for a value first.
        let mut card = LookMotion::at(start);
        card.set_target(end, Transitions::new().height(fast).or(defaults), ms(0));
        let now = card.sample(ms(50)).value;
        assert_eq!(
            (now.width, now.radius, now.height),
            (Some(5.0), Some(10.0), Some(50.0))
        );
    }

    #[test]
    fn two_values_of_one_look_arrive_on_their_own_clocks() {
        let mut card = LookMotion::at(Look::new().offset(0.0, 40.0).opacity(0.0));
        card.set_target(
            Look::new().offset(0.0, 0.0).opacity(1.0),
            Transitions::new()
                .offset(Tween::new(ms(200), Easing::Linear))
                .opacity(Tween::new(ms(100), Easing::Linear)),
            ms(0),
        );
        assert_eq!(
            Trace::of(&card, ms(0), ms(50)).to_string(),
            "\
ms               x           y     opacity
0            0.000      40.000       0.000
50           0.000      30.000       0.500
100          0.000      20.000       1.000
150          0.000      10.000       1.000
200          0.000       0.000       1.000  settled
"
        );
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

    #[test]
    fn a_mix_moves_what_both_name_and_brings_offset_and_opacity_from_where_a_thing_sits() {
        let own = Look::new().width(200.0).background(Color::hex(0x00_0000));
        let from = Look::new()
            .offset(0.0, 40.0)
            .opacity(0.0)
            .width(100.0)
            .radius(8.0);
        let half = from.mix(own, 0.5);
        assert_eq!(half.offset, Some((0.0, 20.0)));
        assert_eq!(half.opacity, Some(0.5));
        assert_eq!(half.width, Some(150.0));
        assert_eq!(half.radius, None, "only the start names it: left out");
        assert_eq!(half.background, own.background, "only the end names it");
        assert_eq!(from.mix(own, 1.0), own, "the end, exactly");
        assert_eq!(from.mix(own, f32::NAN), from.mix(own, 0.0));
        assert_eq!(
            from.mix(own, 1.5).width,
            Some(250.0),
            "past the end it goes on"
        );
        let leave_to = Look::new().opacity(0.0);
        assert_eq!(own.mix(leave_to, 0.5).opacity, Some(0.5), "from opaque");
    }
}
