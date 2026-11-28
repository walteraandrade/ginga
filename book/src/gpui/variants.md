# A parent and its children

A `Variant` is a name a parent shows, such as `Closed` or `Open`, that its children follow one after another. The view keeps it and hands it by hand to the parent, with `.leads(&menu)`, and to each child, with `.follows(&menu, n)`. Each element picks its own look for the name. When the name changes, child `n` starts one step after child `n - 1`, and the parent goes before them (`Order::ParentFirst`), after them (`Order::ChildrenFirst`) or with the first one (`Order::Together`, the default).

```rust,ignore
// In the view's state:
let menu = Variant::new(Menu::Closed).stagger(ms(40)).order(Order::ParentFirst);

// While rendering:
let open = self.menu.name() == Menu::Open;
div()
    .look("menu", if open { Look::new().height(176.) } else { Look::new().height(32.) }, Spring::SNAPPY)
    .leads(&self.menu)
    .children((0..4).map(|n| {
        let row = if open { Look::new().offset(0., 0.).opacity(1.) } else { Look::new().offset(-12., 0.).opacity(0.) };
        div().look(("row", n), row, Spring::SNAPPY).follows(&self.menu, n)
    }))

// On a click: every view that drew the menu renders again, cached ones too.
self.menu.set(Menu::Open, cx);
```

A menu that opens with the panel first and closes with the rows first gives the order with each change, with `set_in_order`; `set` keeps the one the variant was built with:

```rust,ignore
let (name, order) = match self.menu.name() {
    Menu::Closed => (Menu::Open, Order::ParentFirst),
    Menu::Open => (Menu::Closed, Order::ChildrenFirst),
};
self.menu.set_in_order(name, order, cx);
```

Only a change of look that comes with a new name follows the schedule: a hover, or a change of the app's own, goes at once. The schedule's delay is added to any a transition has of its own. A parent that goes after its children waits for the ones laid out inside it, cached views of their own included; children drawn beside it do not hold it. Times count from the change of name, so a child drawn a frame late keeps its place. Under reduced motion the schedule is dropped: every element keeps only its fades, as any look does, after only its own transition's delay.

In the engine, a `Variant` is the schedule alone. Each element asks for its delay and says when it will be settled, so the side that waits knows how long:

```rust
use std::time::Duration;
use ginga::{Easing, Look, LookMotion, Order, Transitions, Tween, Variant};

let ms = Duration::from_millis;
let fade = Transitions::from(Tween::new(ms(100), Easing::Linear));
let mut menu = Variant::new(false).stagger(ms(50)).order(Order::ParentFirst);
let (mut panel, mut rows) = (LookMotion::at(Look::new().opacity(0.0)), [LookMotion::at(Look::new().opacity(0.0)); 3]);

menu.set(true, ms(0));
panel.set_target(Look::new().opacity(1.0), fade.delay(menu.parent_delay(ms(0))), ms(0));
menu.parent_ends(panel.settles_at().unwrap());
for (n, row) in (0..).zip(&mut rows) {
    row.set_target(Look::new().opacity(1.0), fade.delay(menu.child_delay(n, ms(0))), ms(0));
}
// The panel fades in over 100 ms; then the rows, 50 ms apart.
assert_eq!(rows.map(|row| row.sample(ms(175)).value.opacity.unwrap()), [0.75, 0.25, 0.0]);

// Closing gives its own order: the rows first, then the panel.
menu.set_in_order(false, Order::ChildrenFirst, ms(500));
assert!(menu.parent_waits_for_children());
```
