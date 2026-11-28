//! A look declared on an element: the element gets there by itself.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use ginga::{Color, Look, LookMotion, Transitions};
use gpui::{
    AnyElement, App, Bounds, DispatchPhase, Element, ElementId, FocusHandle, Global,
    GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId,
    MouseButton, MouseDownEvent, MouseExitEvent, MouseMoveEvent, MouseUpEvent, ParentElement,
    Pixels, Point, Rgba, Styled, Window, point, px,
};

use crate::{
    Animate, Clock, reduced,
    variant::{Role, Schedule, Variant},
};

/// Declares on any styled element how it should look now; see [`Looked`].
pub trait LookExt: Styled + IntoElement + 'static {
    /// The element takes `look`, and gets there with `transitions` whenever
    /// `look` changes: one [`Transition`] for every value, or a
    /// [`Transitions`] that gives values their own. What they leave out
    /// comes from [`set_default_look_transitions`], then from ginga's defaults.
    ///
    /// Its motion is kept under `id`, so the view needs no field for it. The
    /// motion lasts while the element is drawn in each frame; to keep it
    /// longer, use [`LookExt::look_with`].
    ///
    /// ```ignore
    /// let look = if self.open {
    ///     Look::new().size(320., 200.).opacity(1.)
    /// } else {
    ///     Look::new().size(160., 100.).opacity(0.6)
    /// };
    /// div().bg(rgb(0x2a2f3a)).look("card", look, Spring::SNAPPY)
    /// ```
    fn look(
        self,
        id: impl Into<ElementId>,
        look: Look,
        transitions: impl Into<Transitions>,
    ) -> Looked<Self> {
        Looked::new(self, Keep::Id(id.into()), look, transitions.into())
    }

    /// The element styled with `look` as it is, with no motion and no
    /// element state: for a look the caller already has, such as one mixed
    /// by a presence's progress. Each value maps to the style as with
    /// [`LookExt::look`], the offset painted from the element's own place.
    ///
    /// ```
    /// use ginga_gpui::{Look, LookExt};
    /// use gpui::{Styled, div, px};
    ///
    /// // In a render, with `shown` from a presence's `frame`:
    /// // div().child(label).with_look(from.mix(Look::new(), shown.progress))
    /// let from = Look::new().offset(0., 24.).opacity(0.);
    /// let halfway = div().w(px(80.)).with_look(from.mix(Look::new(), 0.5));
    /// # let _ = halfway;
    /// ```
    fn with_look(self, look: Look) -> WithLook<Self> {
        WithLook {
            element: Some(self),
            look,
        }
    }

    /// The same as [`LookExt::look`], with the motion kept in `state`, which
    /// the view owns.
    fn look_with(
        self,
        state: &LookState,
        look: Look,
        transitions: impl Into<Transitions>,
    ) -> Looked<Self> {
        Looked::new(self, Keep::Owned(state.clone()), look, transitions.into())
    }
}

impl<E: Styled + IntoElement + 'static> LookExt for E {}

/// The motion of an element's look, owned by a view, for an element that
/// must keep its motion while it is not drawn.
///
/// Clones share one motion.
#[derive(Debug, Clone, Default)]
pub struct LookState(Rc<RefCell<Option<Moving>>>);

impl LookState {
    /// The look's motion as the element last set it, and this frame's time
    /// on its clock: read it with [`LookMotion::sample`] or write it down
    /// with [`Trace::of`](ginga::Trace::of), as a test or a log would.
    /// `None` until an element has drawn with it.
    ///
    /// Reading it changes nothing, starts no clock and asks for no frame.
    ///
    /// ```ignore
    /// let (motion, now) = self.card.inspect(cx).unwrap();
    /// println!("{}", Trace::of(&motion, now, Duration::from_millis(16)));
    /// ```
    #[must_use]
    pub fn inspect(&self, cx: &App) -> Option<(LookMotion, std::time::Duration)> {
        let moving = self.0.borrow();
        moving
            .as_ref()
            .map(|moving| (moving.motion, moving.clock.peek(cx)))
    }
}

/// An element's look on its way, and the clock it is read on.
#[derive(Debug)]
struct Moving {
    motion: LookMotion,
    clock: Clock,
    /// Whether the pointer is over the element and presses it, as its
    /// listeners last heard. Shared with them, as they live past the frame.
    gestures: Rc<Cell<Gestures>>,
    /// The variant's count of changes of name when the element was last
    /// drawn: a change of look follows the schedule only after a new name.
    heard: u64,
}

/// What the pointer is doing to an element.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Gestures {
    hovered: bool,
    pressed: bool,
    /// The pointer has moved in the window since it last left it.
    inside: bool,
}

/// Which of the pointer's moments an element has a look for: it keeps track
/// of those only, so a moment without a look redraws nothing.
#[derive(Debug, Clone, Copy)]
struct Hears {
    hover: bool,
    press: bool,
}

/// Where an element's look motion is kept.
enum Keep {
    /// With the element's id, for as long as it is drawn.
    Id(ElementId),
    /// In a [`LookState`] the view owns.
    Owned(LookState),
}

/// An element with a declared look. Made by [`LookExt::look`].
///
/// A value the look names replaces the element's own style for it, except
/// the offset: it moves what is painted, and where the pointer finds the
/// element, from wherever the element is laid out, its own `left` and `top`
/// or an `absolute` place included. The layout does not see it, so nothing
/// around the element moves. A value the look does not name is left as the
/// element's own style.
///
/// It asks for frames only while the look moves, and takes each new look at
/// once when the desktop asks for reduced motion, each value after its
/// transition's delay. A variant's schedule is not kept then.
///
/// It can take another look for a moment: [`hover`](Self::hover) while the
/// pointer is over it, [`pressed`](Self::pressed) while it is pressed, and
/// [`focused`](Self::focused) while it has the keyboard focus. Each is laid
/// over the declared look with [`Look::or`], so it names only what changes;
/// pressed wins over hover, which wins over focused. When the moment ends the
/// element goes back the same way, from where it is.
pub struct Looked<E> {
    element: Option<E>,
    keep: Keep,
    look: Look,
    from: Option<Look>,
    transitions: Transitions,
    hover: Option<Look>,
    pressed: Option<Look>,
    focused: Option<(FocusHandle, Look)>,
    /// The variant the element leads or follows, and its place in it.
    variant: Option<(Rc<dyn Schedule>, Role)>,
    /// A change of course held from layout to prepaint, for a parent that
    /// goes after its children.
    held: Option<Held>,
}

/// A change of course held until the element's children, cached views
/// included, are laid out and said when they end, for a parent that goes
/// after them.
#[derive(Clone, Copy)]
struct Held {
    look: Look,
    transitions: Transitions,
    now: std::time::Duration,
}

impl<E: Styled + IntoElement + 'static> Looked<E> {
    #[expect(
        clippy::large_types_passed_by_value,
        reason = "the transitions are kept in the element, so they are moved in either way"
    )]
    fn new(element: E, keep: Keep, look: Look, transitions: Transitions) -> Self {
        Self {
            element: Some(element),
            keep,
            look,
            from: None,
            transitions,
            hover: None,
            pressed: None,
            focused: None,
            variant: None,
            held: None,
        }
    }

    /// The look the element starts from when it first appears, to animate in.
    /// Without it, the element takes its look at once.
    #[must_use]
    pub fn from(mut self, start: Look) -> Self {
        self.from = Some(start);
        self
    }

    /// The look while the pointer is over the element, laid over its own.
    /// The element's own mouse handlers still run. A size in it moves the
    /// element's edges, and with them where the pointer is over it: a hover
    /// look smaller than the element's own flickers at its edge.
    #[must_use]
    pub fn hover(mut self, look: Look) -> Self {
        self.hover = Some(look);
        self
    }

    /// The look while the element is pressed with the left button, laid over
    /// its own; it wins over [`hover`](Self::hover). A release anywhere ends
    /// it, and a child that stops the press keeps it from the element. The
    /// element's own click handlers still run, but GPUI counts a click only
    /// when the release is over the element: a pressed look that shrinks it
    /// can lose a click near its edge, so shrink it with care or not at all.
    #[must_use]
    pub fn pressed(mut self, look: Look) -> Self {
        self.pressed = Some(look);
        self
    }

    /// The look while `focus` has the keyboard focus, laid over the
    /// element's own. Give the element the same handle with `track_focus`.
    #[must_use]
    pub fn focused(mut self, focus: &FocusHandle, look: Look) -> Self {
        self.focused = Some((focus.clone(), look));
        self
    }

    /// The element is the parent of `variant`: when the variant's name
    /// changes, its look changes at the parent's place in the schedule. Pick
    /// the look from [`Variant::name`]. With [`Order::ChildrenFirst`](crate::Order::ChildrenFirst)
    /// it waits for the children laid out inside it, cached views of their
    /// own included, and not for children drawn beside it.
    ///
    /// Only the first change of look after a new name follows the schedule,
    /// with the schedule's delay added to the transition's own; any other
    /// change, such as a hover, goes at once.
    #[must_use]
    pub fn leads<N: Copy + PartialEq + 'static>(mut self, variant: &Variant<N>) -> Self {
        self.variant = Some((variant.schedule(), Role::Leads));
        self
    }

    /// The element is child `index` of `variant`, counted from 0: when the
    /// variant's name changes, its look changes one step after child
    /// `index - 1`'s. Pick the look from [`Variant::name`].
    #[must_use]
    pub fn follows<N: Copy + PartialEq + 'static>(
        mut self,
        variant: &Variant<N>,
        index: u32,
    ) -> Self {
        self.variant = Some((variant.schedule(), Role::Follows(index)));
        self
    }

    /// The pointer's looks are wanted: the element listens to the pointer.
    fn hears_the_pointer(&self) -> bool {
        self.hover.is_some() || self.pressed.is_some()
    }

    /// The look the element heads for now: its own, with the looks of the
    /// moments it is in laid over it.
    fn wanted(&self, gestures: Gestures, window: &Window) -> Look {
        let mut look = self.look;
        if let Some((focus, focused)) = &self.focused
            && focus.is_focused(window)
        {
            look = focused.or(look);
        }
        if let Some(hover) = self.hover
            && gestures.hovered
        {
            look = hover.or(look);
        }
        if let Some(pressed) = self.pressed
            && gestures.pressed
        {
            look = pressed.or(look);
        }
        look
    }

    /// This frame's look: moves the motion in `moving` toward the wanted
    /// look, or starts one when there is none, and asks for a frame while it
    /// moves. A parent that goes after its children holds its change of
    /// course, to [`finish`](Self::finish) once they are laid out.
    fn frame(
        &self,
        moving: Option<Moving>,
        window: &mut Window,
        cx: &App,
    ) -> (Look, Moving, Option<Held>) {
        let mut moving = moving.unwrap_or_else(|| Moving {
            // Already focused when it first appears, it takes that look at once.
            motion: LookMotion::at(
                self.from
                    .unwrap_or_else(|| self.wanted(Gestures::default(), window)),
            ),
            clock: Clock::default(),
            gestures: Rc::default(),
            heard: self
                .variant
                .as_ref()
                .map_or(0, |(schedule, _)| schedule.changes()),
        });
        // A moment whose look is no longer given is forgotten, so it cannot
        // come back stale when the look is given again.
        let mut gestures = moving.gestures.get();
        gestures.hovered &= self.hover.is_some();
        gestures.pressed &= self.pressed.is_some();
        moving.gestures.set(gestures);
        let look = self.wanted(gestures, window);
        let now = moving.clock.now(cx);
        let changed = moving.motion.target() != look;
        let reduced = reduced(cx);
        // The transitions are read only when the look changes, so they are
        // laid over the app's only then, not on every frame.
        let mut transitions = if changed {
            let app = cx.try_global::<AppTransitions>().map(|app| app.0);
            let transitions = self.transitions.or(app.unwrap_or_default());
            // Every value with no motion, each after its own delay.
            if reduced {
                transitions.without_motion()
            } else {
                transitions
            }
        } else {
            Transitions::new()
        };
        // Only a change of look that comes with a new name follows the
        // schedule: a hover or a change of the app's own goes at once.
        let mut new_name = false;
        if let Some((schedule, _)) = &self.variant {
            schedule.drawn_by(window.current_view());
            let count = schedule.changes();
            new_name = count != moving.heard;
            moving.heard = count;
        }
        let scheduled = self
            .variant
            .as_ref()
            .filter(|_| changed && !reduced && new_name);
        if let Some((schedule, role)) = scheduled {
            if schedule.waits_for_children(*role) {
                window.request_animation_frame();
                let held = Held {
                    look,
                    transitions,
                    now,
                };
                return (moving.motion.sample(now).value, moving, Some(held));
            }
            transitions = transitions.delay(schedule.delay(*role, cx));
        }
        let sample = moving.motion.toward(look, transitions, now);
        if let Some((schedule, role)) = scheduled {
            tell_the_end(&moving.motion, schedule.as_ref(), *role, now, cx);
        }
        if !sample.settled {
            window.request_animation_frame();
        }
        (sample.value, moving, None)
    }

    /// Changes course as `held` says, at the parent's place in the
    /// schedule, now that its children said when they end.
    fn finish(&self, moving: &mut Moving, held: &Held, cx: &App) {
        let Some((schedule, role)) = &self.variant else {
            return;
        };
        let transitions = held.transitions.delay(schedule.delay(*role, cx));
        moving.motion.set_target(held.look, transitions, held.now);
        tell_the_end(&moving.motion, schedule.as_ref(), *role, held.now, cx);
    }

    /// Takes this frame's look and lays the element out with it. A held
    /// change of course waits for prepaint.
    fn lay_out(
        &mut self,
        moving: Option<Moving>,
        window: &mut Window,
        cx: &mut App,
    ) -> ((LayoutId, LaidOut), Moving) {
        let (look, moving, held) = self.frame(moving, window, cx);
        let element = self.element.take().expect("an element is laid out once");
        let mut element = styled(element, look).into_any_element();
        let layout = element.request_layout(window, cx);
        self.held = held;
        let drawn = LaidOut {
            layout,
            element,
            gestures: moving.gestures.clone(),
            offset: offset_of(look),
        };
        ((layout, drawn), moving)
    }
}

/// An element laid out with its look, waiting to be prepainted and painted.
/// `pub` in a private module: it appears in `Looked`'s `Element` impl, and
/// cannot be named outside the crate.
pub struct LaidOut {
    /// The element, styled with this frame's look.
    element: AnyElement,
    layout: LayoutId,
    /// What the pointer is doing to it.
    gestures: Rc<Cell<Gestures>>,
    /// How far from its laid-out place it is painted.
    offset: Point<Pixels>,
}

/// Tells the schedule when an element that just changed course will be settled.
fn tell_the_end(
    motion: &LookMotion,
    schedule: &dyn Schedule,
    role: Role,
    now: std::time::Duration,
    cx: &App,
) {
    if let Some(at) = motion.settles_at() {
        schedule.ends(role, at.saturating_sub(now), cx);
    }
}

/// The transitions set for the whole app.
struct AppTransitions(Transitions);

impl Global for AppTransitions {}

/// Sets the transitions every declared look in the app uses for the values
/// its own transitions leave out. Set them once, as the app starts:
///
/// ```ignore
/// ginga_gpui::set_default_look_transitions(
///     Transitions::new().offset(Spring::BOUNCY).opacity(Tween::new(ms(200), Easing::Out)),
///     cx,
/// );
/// ```
pub fn set_default_look_transitions(transitions: impl Into<Transitions>, cx: &mut App) {
    cx.set_global(AppTransitions(transitions.into()));
}

/// `element` with each value that `look` names but the offset, which is
/// painted: see [`offset_of`].
pub(crate) fn styled<E: Styled>(mut element: E, look: Look) -> E {
    if let Some(opacity) = look.opacity {
        element = element.opacity(opacity.clamp(0., 1.));
    }
    if let Some(width) = look.width {
        element = element.w(px(width.max(0.)));
    }
    if let Some(height) = look.height {
        element = element.h(px(height.max(0.)));
    }
    if let Some(radius) = look.radius {
        element = element.rounded(px(radius.max(0.)));
    }
    if let Some(color) = look.background {
        element = element.bg(rgba(color));
    }
    if let Some(color) = look.text_color {
        element = element.text_color(rgba(color));
    }
    if let Some(color) = look.border_color {
        element = element.border_color(rgba(color));
    }
    element
}

/// How far from its laid-out place an element with `look` is painted: no
/// distance when the look names no offset, or one that is not finite.
pub(crate) fn offset_of(look: Look) -> Point<Pixels> {
    match look.offset {
        Some((x, y)) if x.is_finite() && y.is_finite() => point(px(x), px(y)),
        _ => Point::default(),
    }
}

/// Prepaints `element` moved by `offset`, with its children: what is painted
/// and where the pointer finds it move, and the layout does not. `hitbox`
/// runs first, inside the offset, with where the element is painted, as GPUI
/// snaps it to the pixel; what it returns is handed back.
pub(crate) fn prepaint_at<R>(
    element: &mut AnyElement,
    layout: LayoutId,
    offset: Point<Pixels>,
    hitbox: impl FnOnce(Bounds<Pixels>, &mut Window) -> R,
    window: &mut Window,
    cx: &mut App,
) -> R {
    window.with_element_offset(offset, |window| {
        let painted = window.layout_bounds(layout);
        let hitbox = hitbox(painted, window);
        element.prepaint(window, cx);
        hitbox
    })
}

/// An element styled with a look as it is. Made by [`LookExt::with_look`].
pub struct WithLook<E> {
    element: Option<E>,
    look: Look,
}

impl<E: ParentElement> ParentElement for WithLook<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E: Styled + IntoElement + 'static> IntoElement for WithLook<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Styled + IntoElement + 'static> Element for WithLook<E> {
    type RequestLayoutState = (AnyElement, LayoutId);
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, (AnyElement, LayoutId)) {
        let element = self.element.take().expect("an element is laid out once");
        let mut element = styled(element, self.look).into_any_element();
        let layout = element.request_layout(window, cx);
        (layout, (element, layout))
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        (element, layout): &mut (AnyElement, LayoutId),
        window: &mut Window,
        cx: &mut App,
    ) {
        let offset = offset_of(self.look);
        prepaint_at(element, *layout, offset, |_, _| (), window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        (element, _): &mut (AnyElement, LayoutId),
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx);
    }
}

/// A color of the engine as GPUI's.
fn rgba(color: Color) -> Rgba {
    Rgba::from_repr(color)
}

/// A GPUI color as a color a [`Look`] can name: `color(rgb(0x3b6ef5))`.
pub fn color(gpui_color: impl Into<Rgba>) -> Color {
    gpui_color.into().to_repr()
}

impl<E: ParentElement> ParentElement for Looked<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E: Styled + IntoElement + 'static> IntoElement for Looked<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Styled + IntoElement + 'static> Element for Looked<E> {
    type RequestLayoutState = LaidOut;
    /// Where the pointer is heard, when a look for it is wanted.
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        match &self.keep {
            Keep::Id(id) => Some(id.clone()),
            Keep::Owned(_) => None,
        }
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let ((layout, drawn), moving) = match (&self.keep, id) {
            (Keep::Owned(state), _) => {
                let state = state.clone();
                // Not borrowed while the children are laid out.
                let taken = state.0.borrow_mut().take();
                let (laid_out, moving) = self.lay_out(taken, window, cx);
                (laid_out, Some((state, moving)))
            }
            (Keep::Id(_), Some(id)) => (
                window.with_element_state(id, |moving, window| self.lay_out(moving, window, cx)),
                None,
            ),
            (Keep::Id(_), None) => unreachable!("an element with an id is laid out with one"),
        };
        if let Some((state, moving)) = moving {
            *state.0.borrow_mut() = Some(moving);
        }
        (layout, drawn)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        drawn: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Hitbox> {
        // Inserted before the children's, so theirs lie over it, as GPUI's
        // own elements do; where it is painted.
        let hears = self.hears_the_pointer();
        let hitbox = prepaint_at(
            &mut drawn.element,
            drawn.layout,
            drawn.offset,
            |painted, window| hears.then(|| window.insert_hitbox(painted, HitboxBehavior::Normal)),
            window,
            cx,
        );
        // GPUI lays out a cached view in prepaint, so only now have all the
        // children inside said when they end.
        if let Some(held) = self.held.take() {
            match (&self.keep, id) {
                (Keep::Owned(state), _) => {
                    if let Some(moving) = state.0.borrow_mut().as_mut() {
                        self.finish(moving, &held, cx);
                    }
                }
                (Keep::Id(_), Some(id)) => {
                    window.with_element_state(id, |moving: Option<Moving>, _| {
                        let mut moving = moving.expect("laid out in this frame");
                        self.finish(&mut moving, &held, cx);
                        ((), moving)
                    });
                }
                (Keep::Id(_), None) => {}
            }
        }
        hitbox
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        LaidOut {
            element, gestures, ..
        }: &mut Self::RequestLayoutState,
        hitbox: &mut Option<Hitbox>,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Registered before the children's listeners, so in the bubble phase
        // theirs run first, and a child that takes the press keeps it.
        if let Some(hitbox) = hitbox.take() {
            let hears = Hears {
                hover: self.hover.is_some(),
                press: self.pressed.is_some(),
            };
            // The element can move under a still pointer, as in a scroll:
            // this frame's hit test says whether the pointer is over it now.
            let mut now = gestures.get();
            let hovered = hears.hover && now.inside && hitbox.is_hovered(window);
            if now.hovered != hovered {
                now.hovered = hovered;
                gestures.set(now);
                window.request_animation_frame();
            }
            listen(gestures, hitbox, hears, window);
        }
        element.paint(window, cx);
    }
}

/// Listens to the pointer for an element's hover and pressed looks. After
/// each change it draws again the view that drew the element, and no other.
/// It stops nothing, so the element's own handlers run as they would; a
/// child that stops a press keeps it from the element. It does not read
/// `default_prevented`: GPUI's own focus handling marks every press on a
/// focusable element so.
fn listen(gestures: &Rc<Cell<Gestures>>, hitbox: Hitbox, hears: Hears, window: &mut Window) {
    let view = window.current_view();
    let change = move |gestures: &Cell<Gestures>, cx: &mut App, f: &dyn Fn(&mut Gestures)| {
        let mut now = gestures.get();
        f(&mut now);
        if now != gestures.get() {
            gestures.set(now);
            cx.notify(view);
        }
    };
    let (on_move, over) = (gestures.clone(), hitbox.clone());
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
        if phase == DispatchPhase::Capture {
            let hovered = over.is_hovered(window);
            // A move with the button up is a release that was lost, such as
            // one outside the window.
            let released = event.pressed_button != Some(MouseButton::Left);
            change(&on_move, cx, &|now| {
                now.inside = true;
                now.hovered = hears.hover && hovered;
                now.pressed &= !released;
            });
        }
    });
    // The pointer can leave the window without a last move.
    let on_exit = gestures.clone();
    window.on_mouse_event(move |_: &MouseExitEvent, phase, _, cx| {
        if phase == DispatchPhase::Capture {
            change(&on_exit, cx, &|now| {
                now.inside = false;
                now.hovered = false;
            });
        }
    });
    if !hears.press {
        return;
    }
    let on_down = gestures.clone();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if phase == DispatchPhase::Bubble
            && event.button == MouseButton::Left
            && hitbox.is_hovered(window)
        {
            change(&on_down, cx, &|now| {
                now.inside = true;
                now.pressed = true;
                now.hovered = hears.hover;
            });
        }
    });
    let on_up = gestures.clone();
    window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
        if phase == DispatchPhase::Capture && event.button == MouseButton::Left {
            change(&on_up, cx, &|now| now.pressed = false);
        }
    });
}

#[cfg(test)]
mod tests {
    use gpui::{AbsoluteLength, DefiniteLength, Length, Position, div};

    use super::*;

    fn pixels(length: Option<Length>) -> Option<Pixels> {
        match length? {
            Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(pixels))) => {
                Some(pixels)
            }
            _ => None,
        }
    }

    #[test]
    fn each_named_value_reaches_the_style() {
        let look = Look::new()
            .offset(12., -4.)
            .opacity(0.25)
            .size(80., 40.)
            .radius(6.);
        let mut element = styled(div(), look);
        let style = element.style();
        assert_eq!(style.opacity, Some(0.25));
        assert_eq!(pixels(style.size.width), Some(px(80.)));
        assert_eq!(pixels(style.size.height), Some(px(40.)));
        assert_eq!(
            style.corner_radii.top_left,
            Some(AbsoluteLength::Pixels(px(6.)))
        );
        assert_eq!(offset_of(look), point(px(12.), px(-4.)));
    }

    #[test]
    fn the_offset_leaves_the_elements_own_place_alone() {
        let look = Look::new().offset(8., 0.);
        let mut element = styled(div().absolute().left(px(100.)).top(px(40.)), look);
        let style = element.style();
        assert_eq!(style.position, Some(Position::Absolute));
        assert_eq!(pixels(style.inset.left), Some(px(100.)));
        assert_eq!(pixels(style.inset.top), Some(px(40.)));
        let mut element = styled(div(), look);
        assert_eq!(element.style().position, None, "not made relative");
    }

    #[test]
    fn an_offset_that_is_not_finite_moves_nothing() {
        assert_eq!(
            offset_of(Look::new().offset(f32::NAN, 3.)),
            Point::default()
        );
        assert_eq!(offset_of(Look::new()), Point::default());
    }

    #[test]
    fn a_gpui_color_keeps_its_alpha_on_the_way_in_and_out() {
        let half = color(gpui::rgba(0x3b6e_f580));
        assert_eq!(half.alpha, 128. / 255.);
        assert_eq!(rgba(half), gpui::rgba(0x3b6e_f580));
    }

    #[test]
    fn each_named_color_reaches_the_style() {
        let blue = color(gpui::rgb(0x003b_6ef5));
        let look = Look::new()
            .background(blue)
            .text_color(Color::TRANSPARENT)
            .border_color(blue.with_alpha(0.5));
        let mut element = styled(div(), look);
        let style = element.style();
        assert_eq!(
            style.background,
            Some(gpui::Fill::from(gpui::rgb(0x003b_6ef5)))
        );
        assert_eq!(
            style.text.color,
            Some(gpui::Hsla::from(gpui::Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0
            }))
        );
        assert_eq!(
            style.border_color,
            Some(gpui::Hsla::from(gpui::Rgba {
                a: 0.5,
                ..gpui::rgb(0x003b_6ef5)
            }))
        );
    }

    #[test]
    fn a_value_past_its_range_is_drawn_at_the_end_of_it() {
        let look = Look::new().opacity(1.4).size(-3., -1.).radius(-2.);
        let mut element = styled(div(), look);
        let style = element.style();
        assert_eq!(style.opacity, Some(1.0));
        assert_eq!(pixels(style.size.width), Some(px(0.)));
        assert_eq!(pixels(style.size.height), Some(px(0.)));
        assert_eq!(
            style.corner_radii.bottom_right,
            Some(AbsoluteLength::Pixels(px(0.)))
        );
        let faded = styled(div(), Look::new().opacity(-0.2)).style().opacity;
        assert_eq!(faded, Some(0.0));
        let past = Color {
            red: 1.5,
            green: -0.2,
            blue: 0.5,
            alpha: 1.5,
        };
        let fill = styled(div(), Look::new().background(past))
            .style()
            .background
            .clone();
        let held = gpui::Rgba {
            r: 1.0,
            g: 0.0,
            b: 0.5,
            a: 1.0,
        };
        assert_eq!(fill, Some(gpui::Fill::from(held)));
    }

    #[test]
    fn a_value_the_look_does_not_name_is_left_alone() {
        let mut element = styled(div().w(px(10.)).opacity(0.5), Look::new().height(20.));
        let style = element.style();
        assert_eq!(pixels(style.size.width), Some(px(10.)));
        assert_eq!(style.opacity, Some(0.5));
        assert_eq!(style.position, None);
    }
}
