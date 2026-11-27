use core::time::Duration;

use crate::{InOut, Presence, Shown};

/// How a [`Slot`] swaps one content for another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Swap {
    /// The old content leaves while the new one arrives.
    #[default]
    Sync,
    /// The new content arrives only when the old one is gone.
    Wait,
}

/// One place on screen whose content is swapped: the old content leaves and
/// the new one arrives, at once or one after the other ([`Swap`]).
///
/// It holds what arrives and what leaves, each a [`Presence`], so both can
/// be drawn while they cross. A third content shown while two are crossing
/// takes the place of the one arriving, which starts its way out at once;
/// the one that was leaving is let go.
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, InOut, Phase, Slot, Swap, Tween};
///
/// let ms = Duration::from_millis;
/// let fade = Tween::new(ms(100), Easing::Linear);
/// let ways = InOut::new(fade, fade);
/// let mut title = Slot::new(Swap::Wait);
///
/// title.show("Inbox", ways, ms(0));
/// title.show("Sent", ways, ms(100));
/// let [leaving, arriving] = title.frame(ms(150));
/// assert_eq!(leaving.unwrap().progress, 0.5);
/// assert_eq!(arriving.unwrap().progress, 0.0, "it waits");
///
/// let [leaving, arriving] = title.frame(ms(250));
/// assert!(leaving.is_none());
/// assert_eq!((*arriving.unwrap().content, arriving.unwrap().phase), ("Sent", Phase::Entering));
/// ```
#[derive(Debug, Clone)]
pub struct Slot<T> {
    swap: Swap,
    /// What is shown, arriving, or the last thing hidden.
    current: Presence<T>,
    /// What a swap sent away, on its way out.
    leaving: Presence<T>,
}

impl<T: PartialEq> Default for Slot<T> {
    fn default() -> Self {
        Self::new(Swap::Sync)
    }
}

impl<T: PartialEq> Slot<T> {
    /// An empty slot that swaps its content as `swap` says.
    #[must_use]
    pub fn new(swap: Swap) -> Self {
        Self {
            swap,
            current: Presence::absent(),
            leaving: Presence::absent(),
        }
    }

    /// Shows `content`. The same content as now does not move, or turns
    /// around from where it is when it was on its way out. Other content
    /// swaps: what was there leaves with `ways.exit`, and `content` arrives
    /// with `ways.enter`, at once with [`Swap::Sync`] or once the old one
    /// is gone with [`Swap::Wait`]. Content that was leaving and is shown
    /// again turns around, in either mode.
    pub fn show(&mut self, content: T, ways: InOut, now: Duration) {
        if self.current.content() == Some(&content) {
            self.current.come_back(ways.enter, now);
            return;
        }
        if self.leaving.content() == Some(&content) {
            core::mem::swap(&mut self.current, &mut self.leaving);
            self.current.come_back(ways.enter, now);
            self.leaving.hide(ways.exit, now);
            return;
        }
        let mut enter = ways.enter;
        if self.current.content().is_some() {
            self.current.hide(ways.exit, now);
            self.leaving = core::mem::take(&mut self.current);
            if self.swap == Swap::Wait {
                enter = enter.delay(self.leaving.gone_at(now).saturating_sub(now));
            }
        }
        self.current.show(content, enter, now);
    }

    /// Starts the way out of what is shown, with `exit`.
    pub fn hide(&mut self, exit: impl Into<crate::Transition>, now: Duration) {
        self.current.hide(exit, now);
    }

    /// What to draw at `now`: what leaves, then what arrives or is here, each
    /// `None` when there is nothing. Draw them in that order, so what arrives
    /// lies over what leaves. Like [`Presence::frame`], it lets go of content
    /// whose way out is over.
    pub fn frame(&mut self, now: Duration) -> [Option<Shown<'_, T>>; 2] {
        // Let go first, then read: both reads borrow the slot.
        let _ = self.leaving.frame(now);
        let _ = self.current.frame(now);
        self.sample(now)
    }

    /// What to draw at `now`, as [`frame`](Self::frame) gives it, without
    /// changing anything.
    #[must_use]
    pub fn sample(&self, now: Duration) -> [Option<Shown<'_, T>>; 2] {
        [self.leaving.sample(now), self.current.sample(now)]
    }

    /// Something is to be on screen: it was shown and not hidden since.
    #[must_use]
    pub fn is_wanted(&self) -> bool {
        self.current.is_wanted()
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt::Write, string::String};

    use super::*;
    use crate::{Easing, Phase, Tween};

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn ways() -> InOut {
        let fade = Tween::new(ms(100), Easing::Linear);
        InOut::new(fade, fade)
    }

    fn cell(shown: Option<Shown<'_, &str>>) -> String {
        match shown {
            None => String::from("-"),
            Some(shown) => {
                let phase = match shown.phase {
                    Phase::Entering => "in",
                    Phase::Present => "here",
                    Phase::Exiting => "out",
                };
                format!("{} {phase} {:.2}", shown.content, shown.progress)
            }
        }
    }

    /// "Old" is shown at 0 and settled; "New" swaps in at 100. Each line is
    /// a time, what leaves and what arrives.
    fn schedule(swap: Swap) -> String {
        let mut slot = Slot::new(swap);
        slot.show("Old", InOut::INSTANT, ms(0));
        slot.show("New", ways(), ms(100));
        let mut text = String::new();
        for at in (100..=325).step_by(25) {
            let [leaving, arriving] = slot.frame(ms(at));
            writeln!(text, "{at:>3}  {:<12}  {}", cell(leaving), cell(arriving)).unwrap();
        }
        text
    }

    #[test]
    fn in_sync_the_old_leaves_while_the_new_arrives() {
        assert_eq!(
            schedule(Swap::Sync),
            "\
100  Old out 1.00  New in 0.00
125  Old out 0.75  New in 0.25
150  Old out 0.50  New in 0.50
175  Old out 0.25  New in 0.75
200  -             New here 1.00
225  -             New here 1.00
250  -             New here 1.00
275  -             New here 1.00
300  -             New here 1.00
325  -             New here 1.00
"
        );
    }

    #[test]
    fn waiting_the_new_arrives_once_the_old_is_gone() {
        assert_eq!(
            schedule(Swap::Wait),
            "\
100  Old out 1.00  New in 0.00
125  Old out 0.75  New in 0.00
150  Old out 0.50  New in 0.00
175  Old out 0.25  New in 0.00
200  -             New in 0.00
225  -             New in 0.25
250  -             New in 0.50
275  -             New in 0.75
300  -             New here 1.00
325  -             New here 1.00
"
        );
    }

    #[test]
    fn the_same_content_again_does_not_move() {
        let mut slot = Slot::new(Swap::Sync);
        slot.show("Inbox", ways(), ms(0));
        slot.show("Inbox", ways(), ms(50));
        let [leaving, here] = slot.frame(ms(50));
        assert!(leaving.is_none());
        assert_eq!(here.unwrap().progress, 0.5);
    }

    #[test]
    fn hidden_and_shown_again_it_turns_around_from_where_it_is() {
        let mut slot = Slot::new(Swap::Wait);
        slot.show("Toast", InOut::INSTANT, ms(0));
        slot.hide(ways().exit, ms(0));
        slot.show("Toast", ways(), ms(40));
        let [leaving, here] = slot.frame(ms(40));
        assert!(leaving.is_none());
        let here = here.unwrap();
        assert_eq!((here.phase, here.progress), (Phase::Entering, 0.6));
    }

    #[test]
    fn what_was_leaving_and_is_shown_again_turns_around_and_the_other_leaves() {
        let mut slot = Slot::new(Swap::Sync);
        slot.show("A", InOut::INSTANT, ms(0));
        slot.show("B", ways(), ms(0));
        slot.show("A", ways(), ms(25));
        let [leaving, here] = slot.frame(ms(25));
        assert_eq!(cell(leaving), "B out 0.25");
        assert_eq!(cell(here), "A in 0.75");
        assert_eq!(cell(slot.frame(ms(125))[1]), "A here 1.00");
        assert!(slot.frame(ms(125))[0].is_none(), "B is gone");
    }

    #[test]
    fn a_third_content_lets_the_oldest_go_and_sends_the_arriving_one_out() {
        let mut slot = Slot::new(Swap::Sync);
        slot.show("A", InOut::INSTANT, ms(0));
        slot.show("B", ways(), ms(0));
        slot.show("C", ways(), ms(50));
        let [leaving, here] = slot.frame(ms(50));
        assert_eq!(cell(leaving), "B out 0.50");
        assert_eq!(cell(here), "C in 0.00");
    }

    #[test]
    fn hidden_its_way_out_ends_and_nothing_is_left() {
        let mut slot = Slot::new(Swap::Sync);
        slot.show("Toast", ways(), ms(0));
        slot.hide(ways().exit, ms(100));
        assert!(!slot.is_wanted());
        assert_eq!(cell(slot.frame(ms(150))[1]), "Toast out 0.50");
        assert!(slot.frame(ms(200)).iter().all(Option::is_none));
    }
}
