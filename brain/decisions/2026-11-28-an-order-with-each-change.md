# An order with each change

**Chosen.** A variant's order can be given with one change: `Variant::set_in_order(name, order, now)` in the engine and `set_in_order(name, order, cx)` in the adapter. `set` keeps the order the variant was built with (`.order(..)`). A menu opens with `ParentFirst` and closes with `ChildrenFirst`. Issue #83, PR #92.

**Options.** Issue #83 left the shape to decide:

- The order given with each change, as one call. This is what was chosen.
- An order per name (`.order_for(Menu::Open, ParentFirst)`), kept in the variant, as Motion's `when` sits on each variant.

**Why.**

- The existing API could not do it. `.order(..)` is a builder that consumes the variant, and the adapter's clones share one variant, so rebuilding it on a click is not a way to do it ([[principles/a-small-api]]).
- An order per name needs a table from names to orders. The engine's `Variant` is `Copy`, has no dependencies and does not allocate. The per-change order needs one field.
- The order is read where the change is made, in the click handler, which is the line an agent writes.

**What follows from it.**

- The engine keeps two orders: `order`, for `set`, and `running`, the order of the change running now. `running` is `None` until the first change, so a variant that was built and never changed reads the built order.
- A running schedule is never changed. The same name given again changes nothing, its order included, and `.order(..)` after a change leaves the running one alone.
