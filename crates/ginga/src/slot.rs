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
/// It holds what arrives and what leaves, each a [`Presence`], so all of them
/// can be drawn while they cross. Up to two contents can be leaving at once;
/// a third swap while two are still leaving lets the oldest go at once.
/// With [`Swap::Wait`], content shown while the new one still waits takes its
/// place in the wait, unseen, so nothing leaves twice and nothing skips the wait.
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
/// let [leaving, _, arriving] = title.frame(ms(150));
/// assert_eq!(leaving.unwrap().progress, 0.5);
/// assert!(arriving.is_none(), "it waits, unseen");
///
/// let [leaving, _, arriving] = title.frame(ms(250));
/// assert!(leaving.is_none());
/// let arriving = arriving.unwrap();
/// assert_eq!((*arriving.content, arriving.phase, arriving.progress), ("Sent", Phase::Entering, 0.5));
/// ```
#[derive(Debug, Clone)]
pub struct Slot<T> {
    swap: Swap,
    /// What is shown, arriving, or the last thing hidden.
    current: Presence<T>,
    /// Until when the current content waits, unseen, for what leaves.
    waits_until: Option<Duration>,
    /// What swaps sent away, on their way out, the oldest first.
    leaving: [Presence<T>; 2],
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
            waits_until: None,
            leaving: [Presence::absent(), Presence::absent()],
        }
    }

    /// Swaps the next contents as `swap` says. A wait already set keeps its time.
    pub fn set_swap(&mut self, swap: Swap) {
        self.swap = swap;
    }

    /// Shows `content`. The same content as now does not move, or turns
    /// around from where it is when it was on its way out. Content that was
    /// leaving after a swap turns around too, and what was there leaves.
    /// Other content swaps: what was there leaves with `ways.exit`, and
    /// `content` arrives with `ways.enter`, at once with [`Swap::Sync`] or
    /// once everything leaving is gone with [`Swap::Wait`].
    pub fn show(&mut self, content: T, ways: InOut, now: Duration) {
        if self.current.content() == Some(&content) {
            self.current.come_back(ways.enter, now);
            return;
        }
        if let Some(index) = self
            .leaving
            .iter()
            .position(|leaving| leaving.content() == Some(&content))
        {
            core::mem::swap(&mut self.current, &mut self.leaving[index]);
            self.current.come_back(ways.enter, now);
            self.leaving[index].hide(ways.exit, now);
            self.waits_until = None;
            return;
        }
        let mut enter = ways.enter;
        match self.waits_until {
            // Still unseen: the new content waits in its place.
            Some(until) if now < until && self.current.content().is_some() => {
                enter = enter.delay(until.saturating_sub(now));
            }
            _ if self.current.content().is_some() => {
                self.current.hide(ways.exit, now);
                let leaving = core::mem::take(&mut self.current);
                if self.leaving[1].content().is_some() {
                    // A full pair: the oldest is let go.
                    self.leaving[0] = core::mem::take(&mut self.leaving[1]);
                    self.leaving[1] = leaving;
                } else if self.leaving[0].content().is_some() {
                    self.leaving[1] = leaving;
                } else {
                    self.leaving[0] = leaving;
                }
                self.waits_until = None;
                if self.swap == Swap::Wait {
                    let until = self
                        .leaving
                        .iter()
                        .map(|leaving| leaving.gone_at(now))
                        .fold(now, Duration::max);
                    self.waits_until = Some(until);
                    enter = enter.delay(until.saturating_sub(now));
                }
            }
            _ => self.waits_until = None,
        }
        self.current = Presence::absent();
        self.current.show(content, enter, now);
    }

    /// Starts the way out of what is shown, with `exit`. Content still
    /// waiting is gone at once, never seen.
    pub fn hide(&mut self, exit: impl Into<crate::Transition>, now: Duration) {
        if self.waits_until.is_some_and(|until| now < until) {
            self.current = Presence::absent();
            self.waits_until = None;
        }
        self.current.hide(exit, now);
    }

    /// What to draw at `now`: what leaves, the oldest first, then what arrives
    /// or is here, each `None` when there is nothing. Draw them in that order,
    /// so what arrives lies over what leaves. Content that waits is `None`
    /// until its wait is over. Like [`Presence::frame`], it lets go of content
    /// whose way out is over.
    pub fn frame(&mut self, now: Duration) -> [Option<Shown<'_, T>>; 3] {
        for leaving in &mut self.leaving {
            leaving.let_go(now);
        }
        self.current.let_go(now);
        self.sample(now)
    }

    /// What to draw at `now`, as [`frame`](Self::frame) gives it, without
    /// changing anything.
    #[must_use]
    pub fn sample(&self, now: Duration) -> [Option<Shown<'_, T>>; 3] {
        let waiting = self.waits_until.is_some_and(|until| now < until);
        [
            self.leaving[0].sample(now),
            self.leaving[1].sample(now),
            if waiting {
                None
            } else {
                self.current.sample(now)
            },
        ]
    }

    /// Something is to be on screen: it was shown and not hidden since.
    #[must_use]
    pub fn is_wanted(&self) -> bool {
        self.current.is_wanted()
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt::Write, string::String, vec::Vec};

    use super::*;
    use crate::{Easing, Phase, Spring, Tween};

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn ways() -> InOut {
        let fade = Tween::new(ms(100), Easing::Linear);
        InOut::new(fade, fade)
    }

    fn cell(shown: &Option<Shown<'_, &str>>) -> String {
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

    /// One line of a schedule: what leaves, then what arrives or is here.
    fn line(slot: &mut Slot<&str>, at: u64) -> String {
        let [first, second, current] = slot.frame(ms(at));
        let leaving: Vec<String> = [first, second]
            .iter()
            .filter(|shown| shown.is_some())
            .map(cell)
            .collect();
        let leaving = if leaving.is_empty() {
            String::from("-")
        } else {
            leaving.join(", ")
        };
        format!("{at:>3}  {leaving:<12}  {}", cell(&current))
    }

    /// "Old" is shown at 0 and settled; "New" swaps in at 100. Each line is
    /// a time, what leaves and what arrives.
    fn schedule(swap: Swap) -> String {
        let mut slot = Slot::new(swap);
        slot.show("Old", InOut::INSTANT, ms(0));
        slot.show("New", ways(), ms(100));
        let mut text = String::new();
        for at in (100..=325).step_by(25) {
            writeln!(text, "{}", line(&mut slot, at)).unwrap();
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
    fn waiting_the_new_arrives_once_the_old_is_gone_and_is_unseen_until_then() {
        assert_eq!(
            schedule(Swap::Wait),
            "\
100  Old out 1.00  -
125  Old out 0.75  -
150  Old out 0.50  -
175  Old out 0.25  -
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
    fn a_third_content_while_one_waits_takes_its_place_in_the_wait() {
        let mut slot = Slot::new(Swap::Wait);
        slot.show("A", InOut::INSTANT, ms(0));
        slot.show("B", ways(), ms(0));
        slot.show("C", ways(), ms(50));
        let text: Vec<String> = [50, 75, 100, 150, 200]
            .into_iter()
            .map(|at| line(&mut slot, at))
            .collect();
        assert_eq!(
            text,
            [
                " 50  A out 0.50    -",
                " 75  A out 0.25    -",
                "100  -             C in 0.00",
                "150  -             C in 0.50",
                "200  -             C here 1.00",
            ],
            "A keeps leaving; B is never seen"
        );
    }

    #[test]
    fn a_third_content_in_sync_keeps_both_on_their_way_out() {
        let mut slot = Slot::new(Swap::Sync);
        slot.show("A", InOut::INSTANT, ms(0));
        slot.show("B", ways(), ms(0));
        slot.show("C", ways(), ms(50));
        assert_eq!(
            line(&mut slot, 50),
            " 50  A out 0.50, B out 0.50  C in 0.00"
        );
        assert_eq!(
            line(&mut slot, 75),
            " 75  A out 0.25, B out 0.38  C in 0.25"
        );
        assert_eq!(line(&mut slot, 150), "150  -             C here 1.00");
    }

    #[test]
    fn a_fourth_while_two_leave_lets_the_oldest_go() {
        let mut slot = Slot::new(Swap::Sync);
        slot.show("A", InOut::INSTANT, ms(0));
        slot.show("B", ways(), ms(0));
        slot.show("C", ways(), ms(10));
        slot.show("D", ways(), ms(20));
        assert_eq!(
            line(&mut slot, 20),
            " 20  B out 0.09, C out 0.10  D in 0.00"
        );
    }

    #[test]
    fn waiting_after_a_spring_the_way_in_starts_when_the_spring_settles() {
        let mut slot = Slot::new(Swap::Wait);
        slot.show("Old", InOut::INSTANT, ms(0));
        slot.show("New", InOut::new(ways().enter, Spring::SNAPPY), ms(0));
        let gone = slot.leaving[0].gone_at(ms(0));
        assert!(gone > ms(0));
        let just_before = gone.saturating_sub(ms(1));
        assert!(slot.frame(just_before)[2].is_none(), "still waiting");
        let [_, _, new] = slot.frame(gone);
        assert_eq!(cell(&new), "New in 0.00");
        assert!(slot.frame(gone)[0].is_none(), "the old one is gone");
    }

    #[test]
    fn hidden_while_it_waits_it_is_never_seen() {
        let mut slot = Slot::new(Swap::Wait);
        slot.show("A", InOut::INSTANT, ms(0));
        slot.show("B", ways(), ms(0));
        slot.hide(ways().exit, ms(50));
        assert_eq!(line(&mut slot, 50), " 50  A out 0.50    -");
        assert_eq!(line(&mut slot, 150), "150  -             -");
    }

    #[test]
    fn what_leaves_while_another_waits_turns_around_and_the_waiting_one_is_dropped() {
        let mut slot = Slot::new(Swap::Wait);
        slot.show("A", InOut::INSTANT, ms(0));
        slot.show("B", ways(), ms(0));
        slot.show("A", ways(), ms(50));
        assert_eq!(line(&mut slot, 50), " 50  -             A in 0.50");
        assert_eq!(line(&mut slot, 150), "150  -             A here 1.00");
    }

    #[test]
    fn the_same_content_again_does_not_move() {
        let mut slot = Slot::new(Swap::Sync);
        slot.show("Inbox", ways(), ms(0));
        slot.show("Inbox", ways(), ms(50));
        assert_eq!(line(&mut slot, 50), " 50  -             Inbox in 0.50");
        assert_eq!(
            line(&mut slot, 100),
            "100  -             Inbox here 1.00",
            "the way in was not started again"
        );
    }

    #[test]
    fn hidden_and_shown_again_it_turns_around_from_where_it_is() {
        let mut slot = Slot::new(Swap::Wait);
        slot.show("Toast", InOut::INSTANT, ms(0));
        slot.hide(ways().exit, ms(0));
        slot.show("Toast", ways(), ms(40));
        assert_eq!(line(&mut slot, 40), " 40  -             Toast in 0.60");
    }

    #[test]
    fn what_was_leaving_and_is_shown_again_turns_around_and_the_other_leaves() {
        let mut slot = Slot::new(Swap::Sync);
        slot.show("A", InOut::INSTANT, ms(0));
        slot.show("B", ways(), ms(0));
        slot.show("A", ways(), ms(25));
        assert_eq!(line(&mut slot, 25), " 25  B out 0.25    A in 0.75");
        assert_eq!(line(&mut slot, 125), "125  -             A here 1.00");
    }

    #[test]
    fn hidden_its_way_out_ends_and_nothing_is_left() {
        let mut slot = Slot::new(Swap::Sync);
        slot.show("Toast", ways(), ms(0));
        slot.hide(ways().exit, ms(100));
        assert!(!slot.is_wanted());
        assert_eq!(line(&mut slot, 150), "150  -             Toast out 0.50");
        assert!(slot.frame(ms(200)).iter().all(Option::is_none));
    }

    #[test]
    fn a_change_of_swap_applies_to_the_next_swap() {
        let mut slot = Slot::new(Swap::Sync);
        slot.show("A", InOut::INSTANT, ms(0));
        slot.set_swap(Swap::Wait);
        slot.show("B", ways(), ms(0));
        assert_eq!(line(&mut slot, 50), " 50  A out 0.50    -");
    }
}
