use core::time::Duration;

use crate::{InOut, Motion, Phase};

/// A list whose items arrive and leave, and stay to be drawn while they leave.
///
/// Tell it what the list holds now with [`PresenceList::sync`]. An item whose
/// key is new arrives; an item whose key is gone starts its way out and keeps
/// its place, with the content it had, until that is over. Keys must be unique.
///
/// The first `sync` only fills the list: nothing arrives.
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
}

impl<K, T> ShownItem<'_, K, T> {
    /// It is still arriving or leaving: draw another frame.
    #[must_use]
    pub fn moving(&self) -> bool {
        self.phase != Phase::Present
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
        self.entries.iter().filter_map(move |entry| {
            let amount = entry.amount.sample(now);
            let phase = match (entry.wanted, amount.settled) {
                (true, true) => Phase::Present,
                (true, false) => Phase::Entering,
                (false, false) => Phase::Exiting,
                (false, true) => return None,
            };
            Some(ShownItem {
                key: &entry.key,
                content: &entry.content,
                phase,
                progress: amount.value,
            })
        })
    }

    /// The items to draw at `now`, in order. This is the one call a frame
    /// needs: it also lets go of the items whose way out is over.
    pub fn frame(&mut self, now: Duration) -> impl Iterator<Item = ShownItem<'_, K, T>> {
        self.sweep(now);
        self.sample(now)
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
    use super::*;
    use crate::{Easing, Tween};

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
