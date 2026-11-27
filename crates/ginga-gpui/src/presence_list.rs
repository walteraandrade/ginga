//! A list presence read on a view's clock.

use gpui::{App, Window};

use crate::{Clock, InOut, ShownItem, Transition, unless_reduced};

/// A list with keys whose items come and go, owned by a view: the engine's
/// [`ginga::PresenceList`], read on the frame's clock.
///
/// ```ignore
/// // When the list changes:
/// self.rows.sync(items, InOut::new(fade, fade), cx);
///
/// // While rendering:
/// let rows: Vec<_> = self
///     .rows
///     .frame(window, cx)
///     .map(|row| div().h(px(40. * row.progress)).child(row.content.clone()))
///     .collect();
/// ```
///
/// [`frame`](Self::frame) asks GPUI for another frame while any item arrives,
/// leaves or travels to its place. Under reduced motion each way, and each
/// travel to a new place, is over at once after its delay (see
/// [`unless_reduced`]).
///
/// In a render, tell the places first and then take the frame: both read the
/// clock, and the frame's read is the later one, so it is what is drawn.
#[derive(Debug, Clone)]
pub struct PresenceList<K, T> {
    list: ginga::PresenceList<K, T>,
    clock: Clock,
}

impl<K, T> Default for PresenceList<K, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, T> PresenceList<K, T> {
    /// An empty list that has not been filled yet: its first
    /// [`sync`](Self::sync) only fills it, and nothing arrives.
    #[must_use]
    pub fn new() -> Self {
        Self {
            list: ginga::PresenceList::new(),
            clock: Clock::default(),
        }
    }

    /// The items to draw this frame, in order. It also lets go of the items
    /// whose way out is over.
    ///
    /// Call it once while rendering the view that owns the list. While any
    /// item is arriving, leaving or travelling to its place it asks GPUI to
    /// render that view again on the next frame.
    pub fn frame(
        &mut self,
        window: &Window,
        cx: &App,
    ) -> impl Iterator<Item = ShownItem<'_, K, T>> {
        let now = self.clock.now(cx);
        if self.list.frame(now).any(|item| item.moving()) {
            window.request_animation_frame();
        }
        self.list.sample(now)
    }

    /// What [`frame`](Self::frame) would hand back now, read from `&self`
    /// with no window: for an event handler, between frames. It asks for no
    /// frame and lets go of nothing. While rendering, call `frame` once and
    /// hand on the [`ShownItem`]s it returns: a second read of the time in
    /// one render can disagree with the first.
    #[must_use = "the items are only read"]
    pub fn sample(&self, cx: &App) -> impl Iterator<Item = ShownItem<'_, K, T>> {
        self.list.sample(self.clock.peek(cx))
    }

    /// Tells the list where its items are laid out now, as
    /// [`ginga::PresenceList::place_all`] does: an item whose place changed
    /// travels there with `transition`.
    pub fn place_all(
        &mut self,
        place: impl FnMut(usize, &ShownItem<'_, K, T>) -> f32,
        transition: impl Into<Transition>,
        cx: &App,
    ) {
        let now = self.clock.now(cx);
        self.list
            .place_all(place, unless_reduced(transition, cx), now);
    }
}

impl<K: PartialEq, T> PresenceList<K, T> {
    /// Makes the list hold `items`, in that order, as
    /// [`ginga::PresenceList::sync`] does: a new key arrives with
    /// `in_out.enter`, a missing key leaves with `in_out.exit`.
    pub fn sync(&mut self, items: impl IntoIterator<Item = (K, T)>, in_out: InOut, cx: &App) {
        let now = self.clock.now(cx);
        let in_out = InOut {
            enter: unless_reduced(in_out.enter, cx),
            exit: unless_reduced(in_out.exit, cx),
        };
        self.list.sync(items, in_out, now);
    }
}
