use core::time::Duration;

/// Which named look a parent shows, and when the children that follow it
/// change with it: a schedule, kept by the view and handed to each element.
///
/// A name is any value the app chooses, such as an enum of `Hidden` and
/// `Shown`. Each element picks its own look for the name; the variant only
/// says when each one starts to head there. When the name changes, the
/// parent and its children are delayed by their place in the schedule:
/// child `n` one step later than child `n - 1`, and the parent before them
/// ([`Order::ParentFirst`]), after them ([`Order::ChildrenFirst`]) or at
/// the same time as the first ([`Order::Together`]).
///
/// It sets no targets itself, like a [`Sequence`](crate::Sequence): each
/// element asks for its delay when it changes course, and says when it will
/// be settled, so the side that waits knows how long.
///
/// ```
/// use std::time::Duration;
/// use ginga::{Easing, Look, LookMotion, Order, Transitions, Tween, Variant};
///
/// #[derive(Clone, Copy, PartialEq)]
/// enum Menu { Hidden, Shown }
///
/// let ms = Duration::from_millis;
/// let fade = Transitions::from(Tween::new(ms(100), Easing::Linear));
/// let look = |menu| match menu {
///     Menu::Hidden => Look::new().opacity(0.0),
///     Menu::Shown => Look::new().opacity(1.0),
/// };
/// let mut menu = Variant::new(Menu::Hidden).stagger(ms(50)).order(Order::ParentFirst);
/// let mut panel = LookMotion::at(look(menu.name()));
/// let mut rows = [LookMotion::at(look(menu.name())); 3];
///
/// menu.set(Menu::Shown, ms(0));
/// panel.set_target(look(menu.name()), fade.delay(menu.parent_delay(ms(0))), ms(0));
/// menu.parent_ends(panel.settles_at().unwrap());
/// for (n, row) in (0..).zip(&mut rows) {
///     row.set_target(look(menu.name()), fade.delay(menu.child_delay(n, ms(0))), ms(0));
/// }
///
/// // The panel takes 100 ms, then the rows start 50 ms apart.
/// let starts: Vec<_> = rows.iter().map(|row| row.settles_at().unwrap().saturating_sub(ms(100))).collect();
/// assert_eq!(starts, [ms(100), ms(150), ms(200)]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variant<N> {
    name: N,
    /// When the name last changed.
    since: Duration,
    step: Duration,
    /// The order of a change made with [`set`](Self::set).
    order: Order,
    /// The order of the change running now.
    running: Order,
    /// When the parent is settled on the new name, once it says so.
    parent_ends: Option<Duration>,
    /// When the last of the children is settled on it, once they say so.
    children_end: Option<Duration>,
}

/// Whether a [`Variant`]'s parent changes before its children, after them,
/// or with them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Order {
    /// The parent starts at once, and so does the first child.
    #[default]
    Together,
    /// The children start when the parent is settled.
    ParentFirst,
    /// The parent starts when the last child is settled.
    ChildrenFirst,
}

impl<N: Copy + PartialEq> Variant<N> {
    /// A parent showing `name`, with its children all together and the
    /// parent with them.
    #[must_use]
    pub const fn new(name: N) -> Self {
        Self {
            name,
            since: Duration::ZERO,
            step: Duration::ZERO,
            order: Order::Together,
            running: Order::Together,
            parent_ends: None,
            children_end: None,
        }
    }

    /// The same variant, with each child starting `step` after the one before.
    #[must_use]
    pub const fn stagger(self, step: Duration) -> Self {
        Self { step, ..self }
    }

    /// The same variant, with the parent and its children in `order`.
    #[must_use]
    pub const fn order(self, order: Order) -> Self {
        Self {
            order,
            running: order,
            ..self
        }
    }

    /// The name the parent shows.
    #[must_use]
    pub fn name(&self) -> N {
        self.name
    }

    /// The parent waits for its children ([`Order::ChildrenFirst`]): ask
    /// for its delay only after they said when they end.
    #[must_use]
    pub fn parent_waits_for_children(&self) -> bool {
        self.running == Order::ChildrenFirst
    }

    /// Shows `name` from `now`. A new name starts the schedule again, and
    /// forgets when the parent and the children said they would settle; the
    /// same name changes nothing. Returns whether the name changed.
    pub fn set(&mut self, name: N, now: Duration) -> bool {
        self.set_in_order(name, self.order, now)
    }

    /// Shows `name` from `now`, as [`set`](Self::set) does, with the parent
    /// and its children in `order` for this change only: a menu that opens
    /// with [`Order::ParentFirst`] and closes with [`Order::ChildrenFirst`].
    /// The order given to [`order`](Self::order) is the one for `set`, and
    /// is not changed. The same name changes nothing, the running order
    /// included. Returns whether the name changed.
    ///
    /// ```
    /// use std::time::Duration;
    /// use ginga::{Order, Variant};
    ///
    /// let ms = Duration::from_millis;
    /// let mut menu = Variant::new(false).stagger(ms(50));
    ///
    /// // Open: the panel first, then the rows.
    /// menu.set_in_order(true, Order::ParentFirst, ms(0));
    /// menu.parent_ends(ms(100));
    /// assert_eq!(menu.child_delay(0, ms(0)), ms(100));
    ///
    /// // Close: the rows first, then the panel.
    /// menu.set_in_order(false, Order::ChildrenFirst, ms(500));
    /// assert_eq!(menu.child_delay(0, ms(500)), ms(0));
    /// menu.child_ends(ms(650));
    /// assert_eq!(menu.parent_delay(ms(500)), ms(150));
    /// ```
    pub fn set_in_order(&mut self, name: N, order: Order, now: Duration) -> bool {
        if name == self.name {
            return false;
        }
        self.name = name;
        self.since = now;
        self.running = order;
        self.parent_ends = None;
        self.children_end = None;
        true
    }

    /// How long after `now` the parent should start toward the name's look.
    /// With [`Order::ChildrenFirst`], it waits for the last child that said
    /// when it ends ([`child_ends`](Self::child_ends)), so ask after them:
    /// asked before any child said, it does not wait at all.
    #[must_use]
    pub fn parent_delay(&self, now: Duration) -> Duration {
        let starts = match self.running {
            Order::ChildrenFirst => self.children_end.unwrap_or(self.since),
            Order::Together | Order::ParentFirst => self.since,
        };
        starts.saturating_sub(now)
    }

    /// How long after `now` child `index` should start: its place times the
    /// step, after the parent with [`Order::ParentFirst`] once the parent
    /// said when it ends ([`parent_ends`](Self::parent_ends)): asked before
    /// the parent said, it does not wait for it. A child that asks after its
    /// place has passed starts at once.
    #[must_use]
    pub fn child_delay(&self, index: u32, now: Duration) -> Duration {
        let first = match self.running {
            Order::ParentFirst => self.parent_ends.unwrap_or(self.since),
            Order::Together | Order::ChildrenFirst => self.since,
        };
        first
            .saturating_add(self.step.saturating_mul(index))
            .saturating_sub(now)
    }

    /// The parent says when it will be settled on the name: children that
    /// follow it ask for their delay after this.
    pub fn parent_ends(&mut self, at: Duration) {
        self.parent_ends = Some(at);
    }

    /// A child says when it will be settled on the name; the parent that
    /// goes after the children waits for the latest.
    pub fn child_ends(&mut self, at: Duration) {
        self.children_end = self.children_end.max(Some(at));
    }
}

#[cfg(test)]
mod tests {
    use core::fmt::Write;

    use super::*;
    use crate::{Easing, Look, LookMotion, Transitions, Tween};

    const fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Menu {
        Hidden,
        Shown,
    }

    fn look(menu: Menu) -> Look {
        match menu {
            Menu::Hidden => Look::new().opacity(0.0),
            Menu::Shown => Look::new().opacity(1.0),
        }
    }

    fn fade() -> Transitions {
        Tween::new(ms(100), Easing::Linear).into()
    }

    /// A parent and three children switched to `Shown` at 0 ms, in the order
    /// a view draws them: the parent first, unless it waits for the children.
    fn switch(order: Order) -> (LookMotion, [LookMotion; 3]) {
        let mut menu = Variant::new(Menu::Hidden).stagger(ms(50)).order(order);
        let mut parent = LookMotion::at(look(menu.name()));
        let mut children = [LookMotion::at(look(menu.name())); 3];
        assert!(menu.set(Menu::Shown, ms(0)));
        let mut set_parent = |menu: &mut Variant<Menu>| {
            parent.set_target(
                look(menu.name()),
                fade().delay(menu.parent_delay(ms(0))),
                ms(0),
            );
            menu.parent_ends(parent.settles_at().unwrap());
        };
        if order != Order::ChildrenFirst {
            set_parent(&mut menu);
        }
        for (n, child) in (0..).zip(&mut children) {
            child.set_target(
                look(menu.name()),
                fade().delay(menu.child_delay(n, ms(0))),
                ms(0),
            );
            menu.child_ends(child.settles_at().unwrap());
        }
        if order == Order::ChildrenFirst {
            set_parent(&mut menu);
        }
        (parent, children)
    }

    /// Each element's opacity every 50 ms, as a table.
    fn schedule((parent, children): &(LookMotion, [LookMotion; 3])) -> String {
        let mut table = String::from("  ms  parent  child0  child1  child2\n");
        for t in (0..=300).step_by(50) {
            write!(table, "{t:>4}").unwrap();
            for element in core::iter::once(parent).chain(children) {
                let opacity = element.sample(ms(t)).value.opacity.unwrap();
                write!(table, "  {opacity:>6.2}").unwrap();
            }
            table.push('\n');
        }
        table
    }

    #[test]
    fn the_children_follow_the_parent_one_step_apart() {
        let table = schedule(&switch(Order::ParentFirst));
        assert_eq!(
            table,
            "  ms  parent  child0  child1  child2
   0    0.00    0.00    0.00    0.00
  50    0.50    0.00    0.00    0.00
 100    1.00    0.00    0.00    0.00
 150    1.00    0.50    0.00    0.00
 200    1.00    1.00    0.50    0.00
 250    1.00    1.00    1.00    0.50
 300    1.00    1.00    1.00    1.00
"
        );
    }

    #[test]
    fn the_parent_goes_after_its_children() {
        let table = schedule(&switch(Order::ChildrenFirst));
        assert_eq!(
            table,
            "  ms  parent  child0  child1  child2
   0    0.00    0.00    0.00    0.00
  50    0.00    0.50    0.00    0.00
 100    0.00    1.00    0.50    0.00
 150    0.00    1.00    1.00    0.50
 200    0.00    1.00    1.00    1.00
 250    0.50    1.00    1.00    1.00
 300    1.00    1.00    1.00    1.00
"
        );
    }

    #[test]
    fn together_the_parent_moves_with_the_first_child() {
        let (parent, children) = switch(Order::Together);
        let starts: Vec<_> = core::iter::once(&parent)
            .chain(&children)
            .map(|element| element.settles_at().unwrap().saturating_sub(ms(100)))
            .collect();
        assert_eq!(starts, [ms(0), ms(0), ms(50), ms(100)]);
    }

    #[test]
    fn a_side_that_asks_before_the_other_said_does_not_wait() {
        let mut rows_first = Variant::new(Menu::Hidden).order(Order::ChildrenFirst);
        rows_first.set(Menu::Shown, ms(0));
        assert!(rows_first.parent_waits_for_children());
        assert_eq!(rows_first.parent_delay(ms(0)), Duration::ZERO);
        rows_first.child_ends(ms(120));
        rows_first.child_ends(ms(80));
        assert_eq!(rows_first.parent_delay(ms(0)), ms(120), "the latest child");

        let mut menu_first = Variant::new(Menu::Hidden)
            .stagger(ms(10))
            .order(Order::ParentFirst);
        menu_first.set(Menu::Shown, ms(0));
        assert!(!menu_first.parent_waits_for_children());
        assert_eq!(menu_first.child_delay(1, ms(0)), ms(10));
        menu_first.parent_ends(ms(100));
        assert_eq!(menu_first.child_delay(1, ms(0)), ms(110));
    }

    #[test]
    fn a_child_late_to_ask_keeps_its_place_in_time() {
        let mut menu = Variant::new(Menu::Hidden).stagger(ms(50));
        menu.set(Menu::Shown, ms(1000));
        // A child drawn one frame late still starts at 1100 ms, not 1116 ms.
        assert_eq!(menu.child_delay(2, ms(1016)), ms(84));
        // One whose place has passed starts at once.
        assert_eq!(menu.child_delay(0, ms(1016)), Duration::ZERO);
    }

    #[test]
    fn the_same_name_keeps_the_schedule_and_a_new_one_restarts_it() {
        let mut menu = Variant::new(Menu::Hidden).order(Order::ParentFirst);
        assert!(menu.set(Menu::Shown, ms(0)));
        menu.parent_ends(ms(300));
        assert!(!menu.set(Menu::Shown, ms(100)));
        assert_eq!(menu.child_delay(0, ms(100)), ms(200));
        assert!(menu.set(Menu::Hidden, ms(150)));
        assert_eq!(
            menu.child_delay(1, ms(150)),
            Duration::ZERO,
            "no step without a stagger"
        );
        // The parent has not said when it ends yet: the children do not wait.
        assert_eq!(menu.child_delay(0, ms(150)), Duration::ZERO);
    }

    #[test]
    fn a_menu_opens_parent_first_and_closes_children_first() {
        let mut menu = Variant::new(Menu::Hidden).stagger(ms(50));
        assert!(menu.set_in_order(Menu::Shown, Order::ParentFirst, ms(0)));
        assert!(!menu.parent_waits_for_children());
        assert_eq!(menu.parent_delay(ms(0)), Duration::ZERO);
        menu.parent_ends(ms(100));
        assert_eq!(menu.child_delay(0, ms(0)), ms(100));
        assert_eq!(menu.child_delay(2, ms(0)), ms(200));

        assert!(menu.set_in_order(Menu::Hidden, Order::ChildrenFirst, ms(500)));
        assert!(menu.parent_waits_for_children());
        assert_eq!(menu.child_delay(0, ms(500)), Duration::ZERO);
        assert_eq!(menu.child_delay(2, ms(500)), ms(100));
        menu.child_ends(ms(700));
        assert_eq!(menu.parent_delay(ms(500)), ms(200));
    }

    #[test]
    fn set_keeps_the_order_the_variant_was_built_with() {
        let mut menu = Variant::new(Menu::Hidden)
            .stagger(ms(50))
            .order(Order::ParentFirst);
        menu.set_in_order(Menu::Shown, Order::ChildrenFirst, ms(0));
        assert!(menu.set(Menu::Hidden, ms(500)));
        assert!(!menu.parent_waits_for_children());
        menu.parent_ends(ms(600));
        assert_eq!(menu.child_delay(0, ms(500)), ms(100));
    }

    #[test]
    fn the_same_name_in_another_order_keeps_the_running_schedule() {
        let mut menu = Variant::new(Menu::Hidden).stagger(ms(50));
        menu.set_in_order(Menu::Shown, Order::ParentFirst, ms(0));
        menu.parent_ends(ms(100));
        assert!(!menu.set_in_order(Menu::Shown, Order::ChildrenFirst, ms(50)));
        assert!(!menu.parent_waits_for_children());
        assert_eq!(menu.child_delay(0, ms(50)), ms(50));
    }
}
