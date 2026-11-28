# Variants are a handle passed by hand

**Chosen.** A parent's name and its schedule are a `Variant` the view keeps. The view hands it by hand to the parent, with `.leads(&variant)`, and to each child, with `.follows(&variant, n)`. Each element picks its own look from `variant.name()`. Issue #31.

**Options.** GPUI gives a library no way to pass context down the element tree, so Waltinho chose between two ways for a child to learn its parent's name:

- A handle the view passes down by hand. It is explicit and wordier.
- A scope kept in the app while the tree is laid out, as one GPUI motion crate does. It reads like Motion's variants and hides state in a global.

**Why.** A child in a cached view of its own skips render, so it would never see a scope set while its parent renders. A handle reaches it: `Variant::set` renders again every view that drew an element of the variant, cached ones too. The handle also keeps the state where the rest of ginga keeps it, in the view ([[principles/time-is-an-argument]], and gpui-ce-motion's "the state has an owner in plain sight" in the README's inspirations).

**What follows from it.**

- The engine's `Variant` is a schedule of delays, like a `Sequence` ([[decisions/2026-11-27-a-sequence-is-a-schedule]]): it sets no targets. Each element asks for its delay when its look changes and says when it will be settled (`parent_ends`, `child_ends`), so the side that waits knows how long. It is `Copy` and does not allocate.
- Delays are counted from the time the name changed, not from the frame each element is drawn in. A child drawn a frame late keeps its place in time; one whose place has passed starts at once.
- `Order::ChildrenFirst` needs the children to say when they end before the parent changes course. So the parent holds its change from layout to its own prepaint, after its children's. That covers children laid out inside it, and cached views of their own, which GPUI lays out in prepaint. Children drawn beside the parent do not hold it.
- Only the first change of look after a new name follows the schedule. A hover, or any other change, goes at once and tells the schedule nothing, so it cannot move the other side's wait.
- The schedule's delay is added to the one a transition has of its own (`Transitions::delay`), so a stagger written on a value is kept.
- Under reduced motion the schedule is dropped and every element keeps only its fades, as any look does (see [[decisions/2026-11-28-reduced-motion-keeps-fades]]).
- `Transitions::delay` and `LookMotion::settles_at` came with it: the parent and the children delay a whole look and say when it ends.
