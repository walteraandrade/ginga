# A variant on an element

`.leads(&variant)` and `.follows(&variant, n)` put an `Rc<dyn Schedule>` and a role on a `Looked`. The schedule hides the variant's name type, so `Looked` stays generic over its element only.

## When the look changes

- `Looked::frame` reads the transitions only when the wanted look differs from the motion's target. A variant's delay is applied then: `transitions.or(app).delay(schedule.delay(role))`. The delay goes on last, because `Transitions::delay` resolves every value's transition, ginga's defaults included, and anything laid under it afterwards does nothing.
- Only a change of look that comes with a new name is scheduled. The handle counts changes of name (`Schedule::changes`), and each element keeps the count it last drew with (`Moving::heard`). A hover or another change of the app's own goes at once and reports nothing; `tests/variant.rs` has a test that fails otherwise.
- `Transitions::delay` adds to each value's own delay, so a stagger on a value is kept.
- After it changes course, the element tells the schedule when it will be settled (`tell_the_end`). Times cross clocks as "this long after now": each element and the handle have their own `Clock`, with its own epoch.

## A parent after its children

GPUI lays out a parent's children inside the parent's `request_layout`, but a cached view of children only in prepaint. A parent with `Order::ChildrenFirst` keeps its old course for this frame and keeps a `Held` change in the element. Its value this frame is the one it would have anyway, because a delayed motion holds still. In its own `prepaint`, after its children's, every child inside it has said when it ends, and `finish` sets the target at the frame's time with the right delay. It reaches the motion again with a second `with_element_state` call on its id in the same frame, which GPUI allows: it looks in the next frame's states first.

## Rendering again

Each element tells the handle which view drew it (`window.current_view()`). `Variant::set` notifies those views and forgets them, and they register again when they draw. This is how a cached view of children learns of the change. Without it no view renders, and the tests in `tests/variant.rs` fail.

Related: [[codebase/a-look-on-an-element]], [[decisions/2026-11-27-variants-are-a-handle-passed-by-hand]].
