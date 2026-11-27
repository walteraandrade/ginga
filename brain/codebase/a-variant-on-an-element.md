# A variant on an element

`.leads(&variant)` and `.follows(&variant, n)` put an `Rc<dyn Schedule>` and a role on a `Looked`. The schedule hides the variant's name type, so `Looked` stays generic over its element only.

## When the look changes

- `Looked::frame` reads the transitions only when the wanted look differs from the motion's target. A variant's delay is applied then: `transitions.or(app).delay(schedule.delay(role))`. The delay goes on last, because `Transitions::delay` resolves every value's transition, ginga's defaults included, and anything laid under it afterwards does nothing.
- A hover or a focus also changes the look. It gets the variant's delay too, but that delay is zero once the schedule's times have passed.
- After it changes course, the element tells the schedule when it will be settled (`tell_the_end`). Times cross clocks as "this long after now": each element and the handle have their own `Clock`, with its own epoch.

## A parent after its children

GPUI lays out a parent's children inside the parent's `request_layout`. A parent with `Order::ChildrenFirst` keeps its old course for this frame and returns a `Held` change. Its value this frame is the one it would have anyway, because a delayed motion holds still. `lay_out` then lays out the children, which say when they end, and `finish` sets the target at the frame's time with the right delay. All of it runs inside one `with_element_state` call, which is not re-entered for the same id.

## Rendering again

Each element tells the handle which view drew it (`window.current_view()`). `Variant::set` notifies those views and forgets them, and they register again when they draw. This is how a cached view of children learns of the change. Without it no view renders, and the tests in `tests/variant.rs` fail.

Related: [[codebase/a-look-on-an-element]], [[decisions/2026-11-27-variants-are-a-handle-passed-by-hand]].
