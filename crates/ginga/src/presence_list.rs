use core::time::Duration;

use crate::{Animatable, InOut, Motion, Phase, Transition};

/// A list whose items arrive and leave, and stay to be drawn while they leave.
///
/// Tell it what the list holds now with [`PresenceList::sync`]. An item whose
/// key is new arrives; an item whose key is gone starts its way out and keeps
/// its place, with the content it had, until that is over. Keys must be unique.
///
/// The first `sync` only fills the list: nothing arrives.
///
/// An item whose place changes can travel there instead of jumping: tell the
/// list where its items are laid out with [`PresenceList::place_all`], and
/// draw each one [`ShownItem::offset`] away from its place.
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, InOut, Phase, PresenceList, Tween};
///
/// let ms = Duration::from_millis;
/// let fade = Tween::new(ms(100), Easing::Linear);
/// let in_out = InOut::new(fade, fade);
/// let mut rows = PresenceList::new();
///
/// rows.sync([(1, "one"), (2, "two"), (3, "three")], in_out, ms(0));
/// rows.sync([(1, "one"), (3, "three"), (4, "four")], in_out, ms(1_000));
///
/// let drawn: Vec<_> = rows
///     .frame(ms(1_050))
///     .map(|row| (*row.content, row.phase))
///     .collect();
/// assert_eq!(
///     drawn,
///     [
///         ("one", Phase::Present),
///         ("two", Phase::Exiting),
///         ("three", Phase::Present),
///         ("four", Phase::Entering),
///     ]
/// );
/// ```
#[derive(Debug, Clone)]
pub struct PresenceList<K, T> {
    entries: Vec<Entry<K, T>>,
    filled: bool,
}

#[derive(Debug, Clone)]
struct Entry<K, T> {
    key: K,
    content: T,
    wanted: bool,
    /// How much of the item is here, from zero to one.
    amount: Motion<f32>,
    /// Where the item is drawn along the list, heading for its place. `None`
    /// until its place is first told.
    position: Option<Motion<Place>>,
}

impl<K, T> Entry<K, T> {
    /// The item as drawn at `now`, or `None` once its way out is over.
    fn shown(&self, now: Duration) -> Option<ShownItem<'_, K, T>> {
        let amount = self.amount.sample(now);
        let phase = match (self.wanted, amount.settled) {
            (true, true) => Phase::Present,
            (true, false) => Phase::Entering,
            (false, false) => Phase::Exiting,
            (false, true) => return None,
        };
        let (offset, travelling) = self.position.as_ref().map_or((0.0, false), |position| {
            let drawn = position.sample(now);
            (drawn.value.0 - position.target().0, !drawn.settled)
        });
        Some(ShownItem {
            key: &self.key,
            content: &self.content,
            phase,
            progress: amount.value,
            offset,
            travelling,
        })
    }
}

/// A place along a list, in pixels or any unit where a tenth is too small to see.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Place(f32);

impl Animatable for Place {
    type Channels = [f32; 1];

    const PRECISION: f32 = 0.1;

    fn to_channels(self) -> [f32; 1] {
        [self.0]
    }

    fn from_channels([place]: [f32; 1]) -> Self {
        Self(place)
    }
}

/// An item of a [`PresenceList`] to draw this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShownItem<'a, K, T> {
    /// The item's key.
    pub key: &'a K,
    /// What to draw. While it leaves, this is what it was when it left the list.
    pub content: &'a T,
    /// Where it is in its life.
    pub phase: Phase,
    /// How much of it is here: `0.0` gone, `1.0` fully here.
    pub progress: f32,
    /// How far from its place to draw it, along the list: where it is drawn
    /// minus where it is laid out. `0.0` unless its place changed with
    /// [`PresenceList::place_all`]; after that it heads for `0.0`.
    pub offset: f32,
    /// It is on its way to its place: its offset is not settled yet. The
    /// offset can pass `0.0` while it travels, so [`ShownItem::moving`]
    /// checks this, not the offset.
    travelling: bool,
}

impl<K, T> ShownItem<'_, K, T> {
    /// It is still arriving, leaving or travelling to its place: draw another frame.
    #[must_use]
    pub fn moving(&self) -> bool {
        self.phase != Phase::Present || self.travelling
    }
}

impl<K, T> Default for PresenceList<K, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, T> PresenceList<K, T> {
    /// An empty list that has not been filled yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            filled: false,
        }
    }

    /// The items to draw at `now`, in order, without changing anything.
    pub fn sample(&self, now: Duration) -> impl Iterator<Item = ShownItem<'_, K, T>> {
        self.entries
            .iter()
            .filter_map(move |entry| entry.shown(now))
    }

    /// The items to draw at `now`, in order. This is the one call a frame
    /// needs: it also lets go of the items whose way out is over.
    pub fn frame(&mut self, now: Duration) -> impl Iterator<Item = ShownItem<'_, K, T>> {
        self.sweep(now);
        self.sample(now)
    }

    /// Tells the list where its items are laid out now. `place` is asked, for
    /// each item [`PresenceList::frame`] gives at `now` and in that order,
    /// leaving ones included, its index and the item, and answers its place
    /// along the list, such as its top in pixels. Places are told apart to a
    /// tenth: a move smaller than that is no motion, so use a unit where a
    /// tenth is too small to see.
    ///
    /// When an item's place changes, it travels there with `transition` from
    /// where it is drawn, keeping its speed, and [`ShownItem::offset`] says how
    /// far it still is. The first place told for an item is where it is: it
    /// does not travel. A place that is not a finite number is ignored.
    ///
    /// A place can follow the items before it, such as below a row whose
    /// height follows its progress: `place` can keep a running total. A place
    /// that changes on every frame makes the item chase it: a tween starts
    /// over at each change, a spring follows it smoothly, keeping its speed.
    ///
    /// ```
    /// use std::time::Duration;
    /// use ginga::{Easing, InOut, PresenceList, Tween};
    ///
    /// let ms = Duration::from_millis;
    /// let glide = Tween::new(ms(100), Easing::Linear);
    /// let mut rows = PresenceList::new();
    ///
    /// rows.sync([("a", ()), ("b", ())], InOut::INSTANT, ms(0));
    /// rows.place_all(|index, _| index as f32 * 40.0, glide, ms(0));
    ///
    /// // "b" moves to the top.
    /// rows.sync([("b", ()), ("a", ())], InOut::INSTANT, ms(1_000));
    /// rows.place_all(|index, _| index as f32 * 40.0, glide, ms(1_000));
    ///
    /// let offsets: Vec<_> = rows.frame(ms(1_050)).map(|row| (*row.key, row.offset)).collect();
    /// assert_eq!(offsets, [("b", 20.0), ("a", -20.0)]);
    /// ```
    pub fn place_all(
        &mut self,
        mut place: impl FnMut(usize, &ShownItem<'_, K, T>) -> f32,
        transition: impl Into<Transition>,
        now: Duration,
    ) {
        let transition = transition.into();
        self.sweep(now);
        for (index, entry) in self.entries.iter_mut().enumerate() {
            let Some(at) = entry.shown(now).map(|item| place(index, &item)) else {
                continue;
            };
            if !at.is_finite() {
                continue;
            }
            match &mut entry.position {
                Some(position) => {
                    position.toward(Place(at), transition, now);
                }
                None => entry.position = Some(Motion::at(Place(at))),
            }
        }
    }

    fn sweep(&mut self, now: Duration) {
        self.entries
            .retain(|entry| entry.wanted || !entry.amount.sample(now).settled);
    }
}

impl<K: PartialEq, T> PresenceList<K, T> {
    /// Makes the list hold `items`, in that order. A list with nothing on its
    /// way out, given what it already holds, only takes the new contents and
    /// allocates nothing.
    pub fn sync(&mut self, items: impl IntoIterator<Item = (K, T)>, in_out: InOut, now: Duration) {
        self.sweep(now);
        let mut items = items.into_iter();
        let mut same = 0;
        let first_change = loop {
            match (items.next(), self.entries.get_mut(same)) {
                (Some((key, content)), Some(entry)) if entry.wanted && entry.key == key => {
                    entry.content = content;
                    same += 1;
                }
                (Some(item), _) => break Some(item),
                (None, Some(_)) => break None,
                (None, None) => {
                    self.filled = true;
                    return;
                }
            }
        };
        let incoming: Vec<(K, T)> = first_change.into_iter().chain(items).collect();
        let mut old: Vec<Option<Entry<K, T>>> = self.entries.drain(same..).map(Some).collect();
        let stays: Vec<bool> = old
            .iter()
            .flatten()
            .map(|entry| incoming.iter().any(|(key, _)| *key == entry.key))
            .collect();
        for (key, content) in incoming {
            let known = old
                .iter()
                .position(|entry| entry.as_ref().is_some_and(|entry| entry.key == key));
            let Some(index) = known else {
                let next_staying = old
                    .iter()
                    .zip(&stays)
                    .position(|(entry, stays)| entry.is_some() && *stays)
                    .unwrap_or(old.len());
                self.release(&mut old[..next_staying], &stays, in_out, now);
                let amount = self.arriving(in_out, now);
                self.entries.push(Entry {
                    key,
                    content,
                    wanted: true,
                    amount,
                    position: None,
                });
                continue;
            };
            self.release(&mut old[..index], &stays, in_out, now);
            let Some(mut entry) = old[index].take() else {
                continue;
            };
            entry.content = content;
            if !entry.wanted {
                entry.wanted = true;
                entry.amount.set_target(1.0, in_out.enter, now);
            }
            self.entries.push(entry);
        }
        self.release(&mut old, &stays, in_out, now);
        self.filled = true;
    }

    /// Moves the entries of `old` that are not in the new list into the list,
    /// on their way out, so they keep their place.
    fn release(
        &mut self,
        old: &mut [Option<Entry<K, T>>],
        stays: &[bool],
        in_out: InOut,
        now: Duration,
    ) {
        for (entry, _) in old.iter_mut().zip(stays).filter(|(_, stays)| !**stays) {
            let Some(mut entry) = entry.take() else {
                continue;
            };
            if entry.wanted {
                entry.wanted = false;
                entry.amount.set_target(0.0, in_out.exit, now);
            }
            self.entries.push(entry);
        }
    }

    fn arriving(&self, in_out: InOut, now: Duration) -> Motion<f32> {
        if !self.filled {
            return Motion::at(1.0);
        }
        let mut amount = Motion::at(0.0);
        amount.set_target(1.0, in_out.enter, now);
        amount
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Write;

    use super::*;
    use crate::{Easing, Spring, Tween};

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn in_out() -> InOut {
        let fade = Tween::new(ms(100), Easing::Linear);
        InOut::new(fade, fade)
    }

    fn list(keys: &[u32], now: u64, rows: &mut PresenceList<u32, String>) {
        rows.sync(
            keys.iter().map(|key| (*key, format!("row {key}"))),
            in_out(),
            ms(now),
        );
    }

    /// The list at `now` as text: each key, with `+` while it arrives and `-` while it leaves.
    fn drawn(rows: &mut PresenceList<u32, String>, now: u64) -> String {
        rows.frame(ms(now))
            .map(|row| match row.phase {
                Phase::Entering => format!("+{}", row.key),
                Phase::Present => row.key.to_string(),
                Phase::Exiting => format!("-{}", row.key),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn glide() -> Tween {
        Tween::new(ms(100), Easing::Linear)
    }

    /// Tells `rows` that its items are laid out 40 apart, in their order now.
    fn lay_out(rows: &mut PresenceList<u32, String>, transition: impl Into<Transition>, now: u64) {
        rows.place_all(|index, _| index as f32 * 40.0, transition, ms(now));
    }

    /// Where each item is drawn at `now`, by key: its place, 40 times its index
    /// as [`lay_out`] told it, plus its offset.
    fn drawn_at(rows: &PresenceList<u32, String>, now: u64) -> Vec<(u32, f32)> {
        let mut drawn: Vec<(u32, f32)> = rows
            .sample(ms(now))
            .enumerate()
            .map(|(index, row)| (*row.key, index as f32 * 40.0 + row.offset))
            .collect();
        drawn.sort_by_key(|(key, _)| *key);
        drawn
    }

    /// The offsets at `now`, as text: each key and how far from its place it is drawn.
    fn offsets(rows: &mut PresenceList<u32, String>, now: u64) -> String {
        rows.frame(ms(now))
            .map(|row| format!("{}:{}", row.key, row.offset))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn a_moved_item_starts_its_old_place_minus_its_new_one_away_and_reaches_it() {
        let mut rows = PresenceList::new();
        list(&[1, 2, 3], 0, &mut rows);
        lay_out(&mut rows, glide(), 0);
        list(&[3, 1, 2], 1_000, &mut rows);
        lay_out(&mut rows, glide(), 1_000);
        let mut trace = String::from("ms        3       1       2\n");
        for now in (1_000..=1_100).step_by(25) {
            let offsets: Vec<String> = rows
                .frame(ms(now))
                .map(|row| format!("{:>6.1}", row.offset))
                .collect();
            writeln!(trace, "{:<6}{}", now - 1_000, offsets.join("  ")).unwrap();
        }
        assert_eq!(
            trace,
            "\
ms        3       1       2
0       80.0   -40.0   -40.0
25      60.0   -30.0   -30.0
50      40.0   -20.0   -20.0
75      20.0   -10.0   -10.0
100      0.0     0.0     0.0
"
        );
        assert!(rows.frame(ms(1_100)).all(|row| !row.moving()));
    }

    #[test]
    fn an_item_on_its_way_to_its_place_is_moving_though_present() {
        let mut rows = PresenceList::new();
        list(&[1, 2], 0, &mut rows);
        lay_out(&mut rows, glide(), 0);
        list(&[2, 1], 1_000, &mut rows);
        lay_out(&mut rows, glide(), 1_000);
        assert!(
            rows.frame(ms(1_050))
                .all(|row| row.phase == Phase::Present && row.travelling && row.moving())
        );
    }

    #[test]
    fn an_offset_that_passes_zero_is_still_travelling() {
        let mut rows = PresenceList::new();
        list(&[1, 2], 0, &mut rows);
        lay_out(&mut rows, Spring::BOUNCY, 0);
        list(&[2, 1], 1_000, &mut rows);
        lay_out(&mut rows, Spring::BOUNCY, 1_000);
        // Item 2 starts 40 below its place and overshoots it: find where its offset changes sign.
        let crossing = (1_000..3_000)
            .find(|now| {
                let row = rows.sample(ms(*now)).next().unwrap();
                row.offset <= 0.0
            })
            .unwrap();
        let row = rows.sample(ms(crossing)).next().unwrap();
        assert!(row.offset.abs() < 2.0, "{} at {crossing} ms", row.offset);
        assert!(row.travelling && row.moving());
    }

    #[test]
    fn a_first_place_is_not_a_move() {
        let mut rows = PresenceList::new();
        list(&[1, 2], 0, &mut rows);
        lay_out(&mut rows, glide(), 0);
        assert!(
            rows.frame(ms(0))
                .all(|row| row.offset == 0.0 && !row.moving())
        );
    }

    #[test]
    fn a_second_reorder_during_a_move_does_not_jump_and_keeps_its_speed() {
        let mut rows = PresenceList::new();
        list(&[1, 2, 3], 0, &mut rows);
        lay_out(&mut rows, Spring::SMOOTH, 0);
        list(&[3, 1, 2], 1_000, &mut rows);
        lay_out(&mut rows, Spring::SMOOTH, 1_000);
        let before = drawn_at(&rows, 1_119);
        let at = drawn_at(&rows, 1_120);
        list(&[2, 3, 1], 1_120, &mut rows);
        lay_out(&mut rows, Spring::SMOOTH, 1_120);
        let after = drawn_at(&rows, 1_120);
        let next = drawn_at(&rows, 1_121);
        for (((key, was), (_, is)), ((_, earlier), (_, later))) in
            at.iter().zip(&after).zip(before.iter().zip(&next))
        {
            assert!(
                (was - is).abs() < 1e-3,
                "item {key} jumped from {was} to {is}"
            );
            let (speed_before, speed_after) = (was - earlier, later - is);
            assert!(
                (speed_after - speed_before).abs() < speed_before.abs() * 0.1 + 0.01,
                "item {key} moved {speed_before} in the millisecond before and {speed_after} after"
            );
        }
        assert!(rows.frame(ms(1_121)).all(|row| row.travelling));
        let settle = ms(1_120 + 2_000);
        assert!(
            rows.frame(settle)
                .all(|row| row.offset == 0.0 && !row.moving())
        );
    }

    #[test]
    fn items_that_arrive_and_leave_during_a_move_keep_their_order_and_their_offsets() {
        let mut rows = PresenceList::new();
        list(&[1, 2, 3, 4], 0, &mut rows);
        lay_out(&mut rows, glide(), 0);
        list(&[4, 1, 2, 3], 1_000, &mut rows);
        lay_out(&mut rows, glide(), 1_000);
        list(&[4, 1, 9, 3], 1_030, &mut rows);
        lay_out(&mut rows, glide(), 1_030);
        assert_eq!(drawn(&mut rows, 1_050), "4 1 -2 +9 3");
        // 4, 1 and 2 keep their places at 1_030 and go on with their glides,
        // halfway at 1_050. 9 arrives where 3 was, and 3 is pushed a place down:
        // from -28 at 1_030 it starts a new glide from -68, a fifth done at 1_050.
        assert_eq!(offsets(&mut rows, 1_050), "4:60 1:-20 2:-20 9:0 3:-54.4");
        assert_eq!(drawn(&mut rows, 1_200), "4 1 9 3");
        assert_eq!(offsets(&mut rows, 1_200), "4:0 1:0 9:0 3:0");
    }

    #[test]
    fn a_leaving_item_travels_and_is_let_go_when_its_way_out_ends() {
        let mut rows = PresenceList::new();
        list(&[1, 2], 0, &mut rows);
        lay_out(&mut rows, glide(), 0);
        list(&[2], 1_000, &mut rows);
        // 1 is leaving, still drawn first; 2 moves up to the top, 1 is placed below it.
        rows.place_all(
            |_, row| if *row.key == 1 { 40.0 } else { 0.0 },
            glide(),
            ms(1_000),
        );
        assert_eq!(offsets(&mut rows, 1_050), "1:-20 2:20");
        assert_eq!(drawn(&mut rows, 1_100), "2");
        assert_eq!(offsets(&mut rows, 1_100), "2:0");
    }

    #[test]
    fn an_item_that_comes_back_mid_move_keeps_where_it_is_drawn() {
        let mut rows = PresenceList::new();
        list(&[1, 2], 0, &mut rows);
        lay_out(&mut rows, glide(), 0);
        list(&[2, 1], 1_000, &mut rows);
        lay_out(&mut rows, glide(), 1_000);
        list(&[2], 1_020, &mut rows);
        list(&[2, 1], 1_040, &mut rows);
        assert_eq!(offsets(&mut rows, 1_040), "2:24 1:-24");
    }

    #[test]
    fn places_that_are_not_finite_are_ignored() {
        let mut rows = PresenceList::new();
        list(&[1, 2], 0, &mut rows);
        lay_out(&mut rows, glide(), 0);
        rows.place_all(
            |index, _| [f32::NAN, f32::INFINITY][index],
            glide(),
            ms(1_000),
        );
        assert_eq!(offsets(&mut rows, 1_000), "1:0 2:0");
        assert!(rows.frame(ms(1_000)).all(|row| !row.moving()));
    }

    #[test]
    fn the_first_fill_does_not_arrive() {
        let mut rows = PresenceList::new();
        list(&[1, 2, 3], 0, &mut rows);
        assert_eq!(drawn(&mut rows, 0), "1 2 3");
    }

    #[test]
    fn a_new_key_arrives_and_a_missing_one_leaves_in_its_place() {
        let mut rows = PresenceList::new();
        list(&[1, 2, 3], 0, &mut rows);
        list(&[1, 3, 4], 1_000, &mut rows);
        assert_eq!(drawn(&mut rows, 1_050), "1 -2 3 +4");
        assert_eq!(drawn(&mut rows, 1_100), "1 3 4");
    }

    #[test]
    fn an_item_that_leaves_keeps_the_content_it_had() {
        let mut rows = PresenceList::new();
        rows.sync([(1, "before")], in_out(), ms(0));
        rows.sync(core::iter::empty(), in_out(), ms(10));
        let kept: Vec<_> = rows.frame(ms(50)).map(|row| *row.content).collect();
        assert_eq!(kept, ["before"]);
    }

    #[test]
    fn items_that_leave_stay_before_the_neighbor_that_followed_them() {
        let mut rows = PresenceList::new();
        list(&[1, 2, 3, 4, 5], 0, &mut rows);
        list(&[1, 4, 9], 1_000, &mut rows);
        assert_eq!(drawn(&mut rows, 1_050), "1 -2 -3 4 -5 +9");
    }

    #[test]
    fn a_reorder_moves_the_items_and_nothing_arrives_or_leaves() {
        let mut rows = PresenceList::new();
        list(&[1, 2, 3], 0, &mut rows);
        list(&[3, 1, 2], 1_000, &mut rows);
        assert_eq!(drawn(&mut rows, 1_050), "3 1 2");
    }

    #[test]
    fn an_item_that_comes_back_on_its_way_out_turns_around() {
        let mut rows = PresenceList::new();
        list(&[1, 2], 0, &mut rows);
        list(&[1], 1_000, &mut rows);
        list(&[1, 2], 1_040, &mut rows);
        assert_eq!(drawn(&mut rows, 1_040), "1 +2");
        let back = rows.frame(ms(1_040)).nth(1).unwrap();
        assert!((back.progress - 0.6).abs() < 1e-3);
        assert_eq!(drawn(&mut rows, 1_140), "1 2");
    }

    #[test]
    fn the_same_list_again_only_takes_the_new_contents() {
        let mut rows = PresenceList::new();
        rows.sync([(1, "a"), (2, "b")], in_out(), ms(0));
        rows.sync([(1, "A"), (2, "B")], in_out(), ms(10));
        let contents: Vec<_> = rows.frame(ms(10)).map(|row| *row.content).collect();
        assert_eq!(contents, ["A", "B"]);
        assert!(rows.frame(ms(10)).all(|row| !row.moving()));
    }

    #[test]
    fn items_added_after_an_empty_first_fill_arrive() {
        let mut rows = PresenceList::new();
        list(&[], 0, &mut rows);
        list(&[7], 1_000, &mut rows);
        assert_eq!(drawn(&mut rows, 1_050), "+7");
    }

    #[test]
    fn a_list_emptied_draws_its_items_out_and_then_nothing() {
        let mut rows = PresenceList::new();
        list(&[1, 2], 0, &mut rows);
        list(&[], 1_000, &mut rows);
        assert_eq!(drawn(&mut rows, 1_050), "-1 -2");
        assert_eq!(drawn(&mut rows, 1_100), "");
    }
}
