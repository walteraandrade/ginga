//! A parent's named look that its children follow: the handle a view keeps
//! and passes down to each element.

use std::{cell::RefCell, rc::Rc, time::Duration};

use ginga::Order;
use gpui::{App, EntityId};

use crate::Clock;

/// Which named look a parent shows, kept by the view and handed by hand to
/// the parent and to each child that follows it. A [`ginga::Variant`] read
/// on the frame's clock.
///
/// The parent declares its look with [`Looked::leads`](crate::Looked::leads)
/// and each child with [`Looked::follows`](crate::Looked::follows), giving
/// its place. Each one picks its own look for [`name`](Self::name). When the
/// name changes with [`set`](Self::set), each starts toward its new look at
/// its place in the schedule, and the views that drew them are rendered
/// again, cached ones too.
///
/// ```ignore
/// #[derive(Clone, Copy, PartialEq)]
/// enum Menu { Hidden, Shown }
///
/// // In the view's state:
/// let menu = Variant::new(Menu::Hidden).stagger(ms(40)).order(Order::ParentFirst);
///
/// // While rendering:
/// let shown = self.menu.name() == Menu::Shown;
/// div()
///     .look("menu", Look::new().opacity(if shown { 1. } else { 0. }), Spring::SNAPPY)
///     .leads(&self.menu)
///     .children((0..3).map(|n| {
///         let row = if shown { Look::new().offset(0., 0.).opacity(1.) } else { Look::new().offset(0., 8.).opacity(0.) };
///         div().look(("row", n), row, Spring::SNAPPY).follows(&self.menu, n)
///     }))
///
/// // On a click:
/// self.menu.set(Menu::Shown, cx);
/// ```
///
/// Clones share one variant. Under reduced motion every element takes its
/// look at once, with no schedule, after only its own transition's delay.
#[derive(Debug)]
pub struct Variant<N>(Rc<RefCell<State<N>>>);

impl<N> Clone for Variant<N> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug)]
struct State<N> {
    variant: ginga::Variant<N>,
    /// How many times the name has changed: an element changes course on
    /// the schedule only for the first change of look after a new name.
    changes: u64,
    clock: Clock,
    /// The views that drew an element of this variant since the name last
    /// changed: they are rendered again when it changes.
    views: Vec<EntityId>,
}

impl<N: Copy + PartialEq + 'static> Variant<N> {
    /// A parent showing `name`, with its children all together and the
    /// parent with them.
    #[must_use]
    pub fn new(name: N) -> Self {
        Self(Rc::new(RefCell::new(State {
            variant: ginga::Variant::new(name),
            changes: 0,
            clock: Clock::default(),
            views: Vec::new(),
        })))
    }

    /// The same variant, with each child starting `step` after the one before.
    #[must_use]
    pub fn stagger(self, step: Duration) -> Self {
        self.change(|variant| variant.stagger(step));
        self
    }

    /// The same variant, with the parent and its children in `order`.
    #[must_use]
    pub fn order(self, order: Order) -> Self {
        self.change(|variant| variant.order(order));
        self
    }

    fn change(&self, f: impl FnOnce(ginga::Variant<N>) -> ginga::Variant<N>) {
        let mut state = self.0.borrow_mut();
        state.variant = f(state.variant);
    }

    /// The name the parent shows.
    #[must_use]
    pub fn name(&self) -> N {
        self.0.borrow().variant.name()
    }

    /// Shows `name` from this frame. A new name renders again every view that
    /// drew an element of this variant; the same name changes nothing.
    pub fn set(&self, name: N, cx: &mut App) {
        let mut views = {
            let mut state = self.0.borrow_mut();
            let now = state.clock.now(cx);
            if !state.variant.set(name, now) {
                return;
            }
            state.changes += 1;
            std::mem::take(&mut state.views)
        };
        // Notifying renders nothing yet, so no view registers meanwhile.
        for view in views.drain(..) {
            cx.notify(view);
        }
        // The buffer is kept for the views to register in again.
        self.0.borrow_mut().views = views;
    }

    /// The schedule an element of this variant reads.
    pub(crate) fn schedule(&self) -> Rc<dyn Schedule> {
        self.0.clone()
    }
}

/// Where an element stands in a variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Leads,
    Follows(u32),
}

/// A variant as an element reads it, whatever its names are.
pub(crate) trait Schedule {
    /// How long from this frame the element should wait before it starts.
    fn delay(&self, role: Role, cx: &App) -> Duration;
    /// The element will be settled `after` this frame.
    fn ends(&self, role: Role, after: Duration, cx: &App);
    /// The element starts only once its children said when they end, so it
    /// changes course after they are laid out.
    fn waits_for_children(&self, role: Role) -> bool;
    /// `view` drew an element of this variant.
    fn drawn_by(&self, view: EntityId);
    /// How many times the name has changed.
    fn changes(&self) -> u64;
}

impl<N: Copy + PartialEq> Schedule for RefCell<State<N>> {
    fn delay(&self, role: Role, cx: &App) -> Duration {
        let mut state = self.borrow_mut();
        let now = state.clock.now(cx);
        match role {
            Role::Leads => state.variant.parent_delay(now),
            Role::Follows(index) => state.variant.child_delay(index, now),
        }
    }

    fn ends(&self, role: Role, after: Duration, cx: &App) {
        let mut state = self.borrow_mut();
        let at = state.clock.now(cx).saturating_add(after);
        match role {
            Role::Leads => state.variant.parent_ends(at),
            Role::Follows(_) => state.variant.child_ends(at),
        }
    }

    fn waits_for_children(&self, role: Role) -> bool {
        role == Role::Leads && self.borrow().variant.parent_waits_for_children()
    }

    fn drawn_by(&self, view: EntityId) {
        let mut state = self.borrow_mut();
        // Elements of one view come in a row: the last is checked first.
        if state.views.last() != Some(&view) && !state.views.contains(&view) {
            state.views.push(view);
        }
    }

    fn changes(&self) -> u64 {
        self.borrow().changes
    }
}
