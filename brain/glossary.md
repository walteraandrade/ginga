# Glossary

The words the code, the docs, the issues and the pull requests use, and what each one means here. Use them in that sense; when a new word is needed, add it here in the same change.

Types are in `code`. Unless it says otherwise, a type is in the engine, `ginga`; the adapter, `ginga-gpui`, says so.

## Motion

**Motion** (`Motion`). A value on its way to a target. Plain data that you own: it has no clock and no timer, and it answers with the value at any time you hand it. `ginga_gpui::Motion` is the same thing for a GPUI view: it reads the frame's clock and asks for frames itself.

**Value.** What a motion moves: a number, a point, a size, a color, anything `Animatable`. It is not a position on screen until a view draws it there.

**Target.** Where a motion is heading. Setting a new target is the only thing that changes a motion's future; until then, every time gives a fixed value.

**Change of course.** Setting a new target while the value still moves. The motion starts again from the value and the speed it has at that moment, so nothing jumps.

**Jump.** `jump_to`: putting the value somewhere at rest, with no motion. The opposite of a transition.

**Transition** (`Transition`). How a motion gets to its target: a spring, a tween, or `Transition::Instant`.

**Instant** (`Transition::Instant`). No motion: the value is on its target at once. What reduced motion uses.

**Spring** (`Spring`). A transition with no fixed length that keeps the value's speed across a change of course. Built from a response and a bounce. The presets are `Spring::SNAPPY`, `Spring::SMOOTH` and `Spring::BOUNCY`.

**Response.** The pace of a spring, as a `Duration`: about how long the motion feels. *Not the duration*: a spring has none. The last fraction of the distance keeps settling a little after the response, and a spring's real end is when it is settled.

**Bounce.** How far a spring passes its target before it comes back: `0.0` not at all, `0.3` playfully, a negative bounce slower and flatter. Strictly between `-1.0` and `1.0`; anything else does not build (`SpringError`).

**Tween** (`Tween`). A transition with a fixed duration and an easing. A tween that changes course starts over from where the value is.

**Duration.** How long a tween takes, exactly. Only tweens have one; see *response* for springs.

**Delay** (`Tween::delay`, `Spring::delay`, `Transition::delay`). How long after the target is set a transition starts. Until then the value holds still where it was and is not settled, and a spring starts from rest; *settles at* counts the delay. A delayed `Instant` is a zero-length tween at the end of its delay.

**Stagger.** Items that arrive one after another: each gets a delay one step longer than the one before, usually up to a cap. ginga has no stagger call; it is a delay per item ([the decision](decisions/2026-11-27-stagger-is-a-delay-on-the-transition.md)).

**Keyframes** (`Keyframes`). A motion through several stops instead of to one target, which can repeat. Built from an array of stops, so it is plain data and never allocates; see [the decision](decisions/2026-11-27-keyframes-are-an-array-of-stops.md). `ginga_gpui::Keyframes` is the same for a GPUI view.

**Stop** (`Stop`). One point of keyframes: a value, the time in a round it is reached, and the easing that gets there from the stop before. The first stop is at zero; two stops at one time are a jump.

**Round.** One pass of keyframes from the first stop to the last. `times(n)` plays `n`, `forever()` never stops; *mirrored* rounds go back and forth, and a *pause* holds still between them. Keyframes that repeat forever are never settled, and their *settles at* is `None`.

**Frame cap** (`ginga_gpui::Keyframes::at_most_every`). Asking for the next frame no sooner than a set time after this one, with a timer instead of a frame request, so a slow loop is drawn at a lower rate than the display's.

**Easing** (`Easing`). The shape of a tween: how progress in time becomes progress in value. `Linear`, `Out`, `OutExpo`, a CSS `Bezier`, your own `Custom` curve and others.

**Sample** (`Sample`, `Motion::sample`). The value of a motion at one instant, and whether it is settled. Taking a sample reads the motion and changes nothing, so it can be done at any time, in any order. *Not a frame*: a frame is when something is drawn, and it takes one sample of each motion it draws, but a test or a trace takes samples with no frame at all.

**Settled.** The value is exactly on its target and will not move until the target changes. Whoever draws stops asking for frames then. *Not present*: settled is about a value, present is about something shown (see *phase*); something can be settled on its way to `0.0` and gone.

**Settles at** (`Motion::settles_at`). When a motion will be settled, known from the moment its target is set, a delay included. For a spring it is a safe bound.

**Velocity** (`Motion::velocity`). How fast each channel moves at a time, in units per second. What a spring carries through a change of course.

**Channel.** One `f32` of a value. A point has two channels, a color four. The engine moves each channel on its own with the same transition and keeps a velocity for each.

**Animatable** (`Animatable`). A value the engine can move: a fixed set of channels, a way to split the value into them and to build it back, and a precision. `f32`, tuples of up to four `f32`, `[f32; N]` and your own types.

**Precision** (`Animatable::PRECISION`). How close to its target a channel must be for the motion to stop, in the value's own units. `0.001` by default, for values from zero to one; a tenth of a pixel for pixels. A move smaller than the precision is no motion at all.

**Coast** (`Coast`, `Motion::coast`). A value let go with a speed that slows to a stop: its speed falls by the same share every instant, to about a third (1/e) in each *time constant*, and it rests a time constant's worth of its first speed on (`Coast::rest`). `coast_within` rests inside limits and `coast_to_nearest` on a *snap point*; past a limit, or to reach a snap point, the value moves with the coast's spring instead, keeping its speed.

**Time constant.** How quickly a coast slows: `Coast::QUICK` 0.1 s, `Coast::SLOW` 0.5 s. *Not a duration*: a coast, like a spring, settles when what it has left falls under the precision.

**Trail** (`Trail`). The last places of a value moved by hand and their times, kept at least 15 ms apart in an array of eight: the speed it is let go with, over the last 100 ms, or zero if it was held still for 50 ms.

**Snap point.** A value a coast may come to rest on: of the snap points given, the one nearest to where it would rest.

**Closed-form.** How the engine computes: the value at a time comes from where the motion started, not from stepping the last frame forward. A skipped frame changes nothing.

## Motion as text

**Trace** (`Trace`). A motion written down: its samples at regular times until it settles, one row each, printed as text. How a test, a log or an agent checks motion without a screen.

**Row** (`Row`). One line of a trace: a time, the value of each channel, and whether it is settled.

**Traceable** (`Traceable`). What a trace can write down: a motion, keyframes, or a look motion with a named column for each value.

## Things that come and go

**Presence** (`Presence`). Something you show and hide that stays drawable while it leaves: it keeps its content until its way out is over. It never decides what leaving looks like; it gives a phase and a progress, and the view draws them. `ginga_gpui::Presence` is the same for a GPUI view.

**Content.** What a presence or a list presence shows. While it leaves, it is what it was when it was hidden.

**Show** and **hide.** Starting the way in and the way out. Showing what is already shown only changes the content; showing what is leaving turns it around from where it is.

**Way in** and **way out.** The motion of something arriving and of something leaving. Each is a transition, given together as an `InOut` or one at a time to `show` and `hide`. When the way out is over, the content is dropped.

**In-out** (`InOut`). A pair of transitions: how something arrives and how it leaves. `InOut::INSTANT` for reduced motion.

**Phase** (`Phase`). Where something shown is in its life: `Entering`, `Present` or `Exiting`. Something `Exiting` is still drawn but should no longer take clicks or the focus.

**Present.** The phase of something shown and still. *Not settled*: see *settled*.

**Progress.** How much of something shown is here: `0.0` gone, `1.0` fully here. A view draws it as opacity, scale, height or anything else. A bouncy spring passes both ends a little.

**Shown** (`Shown`), **shown item** (`ShownItem`). What `frame` hands a view to draw: a reference to the content, its phase and its progress; for a list, its key too.

**List presence** (`PresenceList`). A presence for a list with keys. `sync` tells it what the list holds now: a new key arrives, a missing key leaves and keeps its place until it is gone, and a key that comes back turns around.

**Key.** What identifies an item of a list presence across syncs. Keys are unique within a list.

**Sync** (`PresenceList::sync`). Telling a list presence what it holds now, in order. The first sync only fills the list: nothing arrives.

**Place** (`PresenceList::place_all`). Where an item of a list presence is laid out along the list, such as its top in pixels. The view gives `place_all` a function that answers each drawn item's place from its index and the item. Places are told apart to a tenth. When an item's place changes, it travels there instead of jumping. The first place told for an item is where it is: no travel.

**Offset** (`ShownItem::offset`). How far from its place to draw an item: where it is drawn minus where it is laid out. `0.0` unless its place changed; then it heads for `0.0`. The engine animates the offset, never the position, so a list of any layout can use it.

**Travelling** (checked by `ShownItem::moving`). An item is on its way to its place: its offset is not settled. *Not the offset*: an offset can pass `0.0` while it travels, so `moving()` checks this.

## In a GPUI view

**Test platform.** GPUI with its `test-support` feature: windows with no display, a clock the test moves and frames the test delivers. How the adapter's tests run ([[codebase/testing-the-adapter]]).

**Frame.** One time GPUI draws the window. A view that moves something asks for the next frame while it moves. *Not a sample*: see *sample*.

**Frame request.** Asking GPUI to draw the next frame: `window.request_animation_frame()`. `ginga_gpui::Motion` and `ginga_gpui::Presence` do it while their value moves, and stop when it is settled.

**Clock** (`ginga_gpui::Clock`). The frame's time as the `Duration` the engine counts in, from the first frame the clock saw. The engine never reads a clock; the adapter reads one and hands the time on.

**Reduced motion.** The desktop's setting for less motion. Under it, the adapter's motions and presences use `Transition::Instant` (`unless_reduced`).

**Animate** (`ginga_gpui::Animate`) and **stand-in** (`ginga_gpui::InPixels`). The orphan rule keeps the adapter from implementing `Animatable` for GPUI's own types, so each GPUI type that can move names a stand-in that the engine moves instead. `InPixels` is the stand-in for pixels: it stops a tenth of a pixel from the target.

**View.** A GPUI entity that renders. GPUI renders a view again when it asks for a frame or is notified, and every view above it with it.

**Cached view.** A view embedded with `.cached(style)`: unless it was notified, GPUI does not render, lay out or paint it again, and reuses what it painted last time. Its primitives still go to the GPU in every frame. How a heavy view sits beside a motion without being drawn again; see [[codebase/what-a-cached-view-saves]].

**Island** (`ginga_gpui::Island`). A small view of its own that draws what moves, so that the frames its motion asks for render the island and the views above it, and not the heavy, cached views beside it.

**Stale pane.** A cached view that shows old state because it changed without being notified. The risk an island brings.

**Drag** (`ginga_gpui::Drag`, `.draggable`). The pointer takes an element and it follows exactly, with no spring between them. Let go, it is *thrown*: it keeps the pointer's speed, which a *trail* measures, and coasts. A pointer still for 50 ms before it lets go sets the element down instead. `DragState` holds where the element is; `throw` lets it go without a pointer.

**Gallery** and **page**. The adapter's example app (`examples/gallery`): a list of pages on one side, the chosen page on the other. A page shows one thing ginga does and has a Play button. A new feature adds a page; the four first pages cover what existed when the gallery came, the look page came with the look (#25), the color page with colors (#26), the pulse page with keyframes (#30), the drag page with coasting and drag (#32), and a delay (stagger) and a list's travel (`place_all`) have none yet. `cargo test -p ginga-gpui` plays every page on GPUI's test platform and checks that it moves and then settles. *Recording* a page runs it in a virtual display and keeps its frames; see [[codebase/recording-a-gallery-page]].

## Looks

**Look** (`Look`). How a thing should look: an offset from its own place, an opacity, a width, a height, a corner radius, and a background, text and border color, each optional. A value a look does not name is left to whoever draws; on an element, it stays the element's own style.

**Color** (`Color`). Red, green and blue as sRGB and an alpha, each from `0.0` to `1.0`, written with `Color::hex(0xRRGGBB)` or `ginga_gpui::color(rgb(..))`. It moves in linear light with its alpha premultiplied, so between two colors there are only colors between them ([the decision](decisions/2026-11-27-colors-mix-in-linear-light-premultiplied.md)).

**Linear light.** Color as an amount of light, before the sRGB curve that makes equal steps of a written color look even. Mixing in it gives the brightness two lights would give together; mixing written sRGB values gives a darker middle.

**Premultiplied alpha.** A color kept with each of red, green and blue multiplied by its alpha. Transparent is then all zeros whatever its color, so a fade from it never passes through black.

**Look motion** (`LookMotion`). A look on its way to another, one motion for each value. A value the last look named moves from where it is; a value it did not name has no place to start from and takes its new value at once.

**Declared look** (`LookExt::look`, `Looked`). A look written on a GPUI element, with the transition that gets it there. Its look motion is kept with the element's id while the element is drawn each frame, or in a `LookState` the view owns.
