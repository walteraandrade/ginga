use core::time::Duration;

use crate::{Motion, Transition};

/// How something arrives and how it leaves.
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, InOut, Tween};
///
/// let ms = Duration::from_millis;
/// let toast = InOut::new(
///     Tween::new(ms(320), Easing::OutExpo),
///     Tween::new(ms(180), Easing::OutQuart),
/// );
/// # let _ = toast;
/// ```
#[derive(Debug, Clone, Copy)]
pub struct InOut {
    /// How it arrives.
    pub enter: Transition,
    /// How it leaves.
    pub exit: Transition,
}

impl InOut {
    /// Arrives and leaves at once. Use it for reduced motion.
    pub const INSTANT: Self = Self {
        enter: Transition::Instant,
        exit: Transition::Instant,
    };

    /// Arrives with `enter` and leaves with `exit`.
    pub fn new(enter: impl Into<Transition>, exit: impl Into<Transition>) -> Self {
        Self {
            enter: enter.into(),
            exit: exit.into(),
        }
    }
}

/// Where something on screen is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// On its way in.
    Entering,
    /// Here, and still.
    Present,
    /// On its way out. It should no longer take clicks or the focus.
    Exiting,
}

/// Something to draw this frame: here, arriving or leaving.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shown<'a, T> {
    /// What to draw. While it leaves, this is what it was when it was hidden.
    pub content: &'a T,
    /// Where it is in its life.
    pub phase: Phase,
    /// How much of it is here: `0.0` gone, `1.0` fully here. A spring with
    /// bounce passes both ends a little.
    pub progress: f32,
}

impl<T> Shown<'_, T> {
    /// It is still arriving or leaving: draw another frame.
    #[must_use]
    pub fn moving(&self) -> bool {
        self.phase != Phase::Present
    }
}

/// Something that can be shown and hidden, and stays to be drawn while it leaves.
///
/// Hiding does not drop the content. The presence keeps it, as it was, until
/// its way out is over, so there is always something to draw the exit with.
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, Phase, Presence, Tween};
///
/// let ms = Duration::from_millis;
/// let fade = Tween::new(ms(100), Easing::Linear);
/// let mut toast = Presence::absent();
///
/// toast.show("Saved", fade, ms(0));
/// assert_eq!(toast.frame(ms(50)).unwrap().progress, 0.5);
/// assert_eq!(toast.frame(ms(100)).unwrap().phase, Phase::Present);
///
/// toast.hide(fade, ms(1_000));
/// let leaving = toast.frame(ms(1_050)).unwrap();
/// assert_eq!((*leaving.content, leaving.phase), ("Saved", Phase::Exiting));
///
/// assert!(toast.frame(ms(1_100)).is_none());
/// ```
#[derive(Debug, Clone)]
pub struct Presence<T> {
    content: Option<T>,
    wanted: bool,
    /// How much of the content is here, from zero to one.
    amount: Motion<f32>,
}

impl<T> Default for Presence<T> {
    fn default() -> Self {
        Self::absent()
    }
}

impl<T> Presence<T> {
    /// Nothing is shown.
    #[must_use]
    pub fn absent() -> Self {
        Self {
            content: None,
            wanted: false,
            amount: Motion::at(0.0),
        }
    }

    /// `content` is shown, and has been: it does not arrive.
    #[must_use]
    pub fn present(content: T) -> Self {
        Self {
            content: Some(content),
            wanted: true,
            amount: Motion::at(1.0),
        }
    }

    /// Shows `content`. What was hidden or leaving arrives with `enter`, from
    /// as far in as it still is; what was already shown only changes content.
    pub fn show(&mut self, content: T, enter: impl Into<Transition>, now: Duration) {
        self.content = Some(content);
        if !self.wanted {
            self.wanted = true;
            self.amount.set_target(1.0, enter, now);
        }
    }

    /// Starts the way out with `exit`. The content stays, to be drawn, until
    /// the way out is over.
    pub fn hide(&mut self, exit: impl Into<Transition>, now: Duration) {
        if self.wanted {
            self.wanted = false;
            self.amount.set_target(0.0, exit, now);
        }
    }

    /// What to draw at `now`, or `None` when nothing is here. This is the one
    /// call a frame needs: it also lets go of content whose way out is over.
    pub fn frame(&mut self, now: Duration) -> Option<Shown<'_, T>> {
        if !self.wanted && self.amount.sample(now).settled {
            self.content = None;
        }
        self.sample(now)
    }

    /// What to draw at `now`, without changing anything.
    #[must_use]
    pub fn sample(&self, now: Duration) -> Option<Shown<'_, T>> {
        let content = self.content.as_ref()?;
        let amount = self.amount.sample(now);
        let phase = match (self.wanted, amount.settled) {
            (true, true) => Phase::Present,
            (true, false) => Phase::Entering,
            (false, false) => Phase::Exiting,
            (false, true) => return None,
        };
        Some(Shown {
            content,
            phase,
            progress: amount.value,
        })
    }

    /// The content should be on screen: it was shown and not hidden since.
    #[must_use]
    pub fn is_wanted(&self) -> bool {
        self.wanted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Easing, Spring, Tween};

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn fade() -> Tween {
        Tween::new(ms(100), Easing::Linear)
    }

    fn life<T: Copy>(presence: &mut Presence<T>, now: u64) -> Option<(T, Phase, f32)> {
        presence
            .frame(ms(now))
            .map(|shown| (*shown.content, shown.phase, thousandths(shown.progress)))
    }

    fn thousandths(value: f32) -> f32 {
        (value * 1000.0).round() / 1000.0
    }

    #[test]
    fn nothing_is_drawn_before_anything_is_shown() {
        let mut presence = Presence::<&str>::absent();
        assert_eq!(life(&mut presence, 0), None);
        assert!(!presence.is_wanted());
    }

    #[test]
    fn what_is_shown_arrives_stays_leaves_and_is_gone() {
        let mut presence = Presence::absent();
        presence.show("toast", fade(), ms(0));
        assert_eq!(
            life(&mut presence, 0),
            Some(("toast", Phase::Entering, 0.0))
        );
        assert_eq!(
            life(&mut presence, 25),
            Some(("toast", Phase::Entering, 0.25))
        );
        assert_eq!(
            life(&mut presence, 100),
            Some(("toast", Phase::Present, 1.0))
        );
        presence.hide(fade(), ms(500));
        assert_eq!(
            life(&mut presence, 500),
            Some(("toast", Phase::Exiting, 1.0))
        );
        assert_eq!(
            life(&mut presence, 575),
            Some(("toast", Phase::Exiting, 0.25))
        );
        assert_eq!(life(&mut presence, 600), None);
    }

    #[test]
    fn what_was_there_from_the_start_does_not_arrive() {
        let mut presence = Presence::present("panel");
        assert_eq!(life(&mut presence, 0), Some(("panel", Phase::Present, 1.0)));
        assert!(!presence.frame(ms(0)).unwrap().moving());
    }

    #[test]
    fn shown_again_on_its_way_out_it_turns_around_from_where_it_is() {
        let mut presence = Presence::present("panel");
        presence.hide(fade(), ms(0));
        presence.show("panel", fade(), ms(40));
        assert_eq!(
            life(&mut presence, 40),
            Some(("panel", Phase::Entering, 0.6))
        );
        assert_eq!(
            life(&mut presence, 140),
            Some(("panel", Phase::Present, 1.0))
        );
    }

    #[test]
    fn showing_what_is_shown_changes_the_content_and_not_the_motion() {
        let mut presence = Presence::absent();
        presence.show("one", fade(), ms(0));
        presence.show("two", fade(), ms(50));
        assert_eq!(life(&mut presence, 50), Some(("two", Phase::Entering, 0.5)));
    }

    #[test]
    fn the_content_is_let_go_once_the_way_out_is_over() {
        let mut presence = Presence::present(vec![1, 2, 3]);
        presence.hide(fade(), ms(0));
        assert!(presence.sample(ms(50)).is_some());
        assert!(presence.frame(ms(100)).is_none());
        assert!(presence.content.is_none());
    }

    #[test]
    fn hiding_what_is_hidden_and_instant_ways_out_leave_nothing() {
        let mut presence = Presence::<&str>::absent();
        presence.hide(fade(), ms(0));
        assert_eq!(life(&mut presence, 0), None);
        let mut presence = Presence::present("menu");
        presence.hide(InOut::INSTANT.exit, ms(0));
        assert_eq!(life(&mut presence, 0), None);
    }

    #[test]
    fn a_delayed_way_in_is_entering_with_nothing_shown_until_it_starts() {
        let mut presence = Presence::absent();
        presence.show("row", fade().delay(ms(40)), ms(0));
        assert_eq!(life(&mut presence, 20), Some(("row", Phase::Entering, 0.0)));
        assert_eq!(life(&mut presence, 90), Some(("row", Phase::Entering, 0.5)));
        assert_eq!(life(&mut presence, 140), Some(("row", Phase::Present, 1.0)));
    }

    #[test]
    fn a_spring_carries_its_speed_through_a_change_of_mind() {
        let mut presence = Presence::absent();
        presence.show("panel", Spring::SNAPPY, ms(0));
        let arriving = presence.frame(ms(60)).unwrap().progress;
        presence.hide(Spring::SNAPPY, ms(60));
        let just_after = presence.frame(ms(70)).unwrap();
        assert_eq!(just_after.phase, Phase::Exiting);
        assert!(just_after.progress > arriving, "it still coasts in");
    }
}
