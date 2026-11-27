//! An element the pointer drags, and that coasts when it is let go.

use std::{cell::RefCell, rc::Rc, time::Duration};

use ginga::{Coast, Trail};
use gpui::{
    AnyElement, App, Axis, Bounds, DispatchPhase, Element, ElementId, GlobalElementId, Hitbox,
    HitboxBehavior, InspectorElementId, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Position, Styled, Window, point,
    px,
};

use crate::{Animate, Clock, InPixels};

/// How an element is dragged: along which axis, inside which limits, onto
/// which snap points, and how it coasts when it is let go.
///
/// ```ignore
/// // A knob along a 200 px track, that springs back inside it:
/// Drag::new().along(Axis::Horizontal).within(point(px(0.), px(0.)), point(px(200.), px(0.)))
/// // A card that comes to rest on one of three places:
/// Drag::new().snap_to([point(px(0.), px(0.)), point(px(240.), px(0.)), point(px(480.), px(0.))])
/// ```
#[derive(Debug, Clone)]
pub struct Drag {
    coast: Coast,
    along: Option<Axis>,
    within: Option<(InPixels<2>, InPixels<2>)>,
    snaps: Option<Rc<[InPixels<2>]>>,
}

impl Default for Drag {
    fn default() -> Self {
        Self::new()
    }
}

impl Drag {
    /// Both ways, with no limits and no snap points, coasting with
    /// [`Coast::QUICK`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            coast: Coast::QUICK,
            along: None,
            within: None,
            snaps: None,
        }
    }

    /// Only along `axis`: the pointer's movement across it is left out.
    #[must_use]
    pub fn along(mut self, axis: Axis) -> Self {
        self.along = Some(axis);
        self
    }

    /// Comes to rest with its offset inside `min..=max`, on each axis: let go
    /// on its way past a limit, or past it already, it springs back to it.
    /// While dragged it stays under the pointer, past a limit too.
    #[must_use]
    pub fn within(mut self, min: Point<Pixels>, max: Point<Pixels>) -> Self {
        self.within = Some((min.to_repr(), max.to_repr()));
        self
    }

    /// Comes to rest on whichever of `points` is nearest to where it would
    /// coast, as offsets. With snap points, the limits of
    /// [`within`](Self::within) are left out: put the points inside them.
    #[must_use]
    pub fn snap_to(mut self, points: impl IntoIterator<Item = Point<Pixels>>) -> Self {
        let points: Rc<[InPixels<2>]> = points.into_iter().map(Animate::to_repr).collect();
        self.snaps = (!points.is_empty()).then_some(points);
        self
    }

    /// How it slows to a stop when it is let go, and the spring it takes to a
    /// limit or a snap point.
    #[must_use]
    pub fn coast(mut self, coast: Coast) -> Self {
        self.coast = coast;
        self
    }

    /// `movement` with what lies across the axis it is dragged along left out.
    fn keep_along(&self, movement: Point<Pixels>) -> Point<Pixels> {
        match self.along {
            Some(Axis::Horizontal) => point(movement.x, px(0.)),
            Some(Axis::Vertical) => point(px(0.), movement.y),
            None => movement,
        }
    }
}

/// Where a dragged element is, owned by a view: its offset from its place in
/// the layout, and the pointer that holds it.
///
/// Clones share one element's place.
#[derive(Debug, Clone, Default)]
pub struct DragState(Rc<RefCell<Dragging>>);

#[derive(Debug)]
struct Dragging {
    motion: ginga::Motion<InPixels<2>>,
    clock: Clock,
    held: Option<Held>,
    /// The offset it was drawn at last.
    drawn: Point<Pixels>,
}

impl Default for Dragging {
    fn default() -> Self {
        Self::at(point(px(0.), px(0.)))
    }
}

impl Dragging {
    fn at(offset: Point<Pixels>) -> Self {
        Self {
            motion: ginga::Motion::at(offset.to_repr()),
            clock: Clock::default(),
            held: None,
            drawn: offset,
        }
    }
}

/// An element in the pointer's hold.
#[derive(Debug, Clone, Copy)]
struct Held {
    /// Where the pointer took it.
    pointer: Point<Pixels>,
    /// Its offset then.
    from: Point<Pixels>,
    /// Its offset now.
    at: Point<Pixels>,
    /// The pointer's last places, for the speed it lets go with.
    trail: Trail<InPixels<2>>,
}

impl DragState {
    /// An element at `offset` from its place in the layout, at rest.
    #[must_use]
    pub fn at(offset: Point<Pixels>) -> Self {
        Self(Rc::new(RefCell::new(Dragging::at(offset))))
    }

    /// Its offset from its place in the layout as it was last drawn, or
    /// where the pointer holds it.
    #[must_use]
    pub fn offset(&self) -> Point<Pixels> {
        let dragging = self.0.borrow();
        dragging.held.map_or(dragging.drawn, |held| held.at)
    }

    /// The offset it comes to rest at: where the pointer holds it, or where
    /// it coasts to.
    #[must_use]
    pub fn rest(&self) -> Point<Pixels> {
        let dragging = self.0.borrow();
        dragging.held.map_or_else(
            || Point::from_repr(dragging.motion.target()),
            |held| held.at,
        )
    }

    /// The pointer holds the element.
    #[must_use]
    pub fn is_held(&self) -> bool {
        self.0.borrow().held.is_some()
    }

    /// This frame's offset; asks for another frame while the element coasts.
    fn frame(&self, window: &mut Window, cx: &App) -> Point<Pixels> {
        let mut dragging = self.0.borrow_mut();
        if let Some(held) = dragging.held {
            dragging.drawn = held.at;
            return held.at;
        }
        let now = dragging.clock.now(cx);
        let sample = dragging.motion.sample(now);
        if !sample.settled {
            window.request_animation_frame();
        }
        dragging.drawn = Point::from_repr(sample.value);
        dragging.drawn
    }

    /// The pointer takes the element at `pointer`, catching it if it coasts.
    fn press(&self, pointer: Point<Pixels>, cx: &App) {
        let mut dragging = self.0.borrow_mut();
        let now = dragging.clock.now(cx);
        // Pressed again while held, as when a release was lost outside the
        // window, it is where the pointer held it.
        let from = match dragging.held {
            Some(held) => held.at,
            None => Point::from_repr(dragging.motion.sample(now).value),
        };
        dragging.motion.jump_to(from.to_repr());
        let mut held = Held {
            pointer,
            from,
            at: from,
            trail: Trail::default(),
        };
        held.trail.note(now, pointer.to_repr());
        dragging.held = Some(held);
    }

    /// The pointer moves to `pointer`, and the element with it.
    fn move_to(&self, pointer: Point<Pixels>, drag: &Drag, cx: &App) -> bool {
        let mut dragging = self.0.borrow_mut();
        let now = dragging.clock.now(cx);
        let Some(held) = &mut dragging.held else {
            return false;
        };
        let movement = drag.keep_along(point(
            pointer.x - held.pointer.x,
            pointer.y - held.pointer.y,
        ));
        held.at = point(held.from.x + movement.x, held.from.y + movement.y);
        held.trail.note(now, pointer.to_repr());
        true
    }

    /// Lets the element go with `velocity`, in pixels per second, as if the
    /// pointer had thrown it: it coasts, to a limit or a snap point if `drag`
    /// names them. Under reduced motion it is at once where it comes to rest.
    ///
    /// For a key or a button that throws what the pointer could. Notify the
    /// view that draws it, and it moves from its next frame on.
    pub fn throw(&self, velocity: Point<Pixels>, drag: &Drag, cx: &App) {
        let mut dragging = self.0.borrow_mut();
        let now = dragging.clock.now(cx);
        let from = match dragging.held.take() {
            Some(held) => held.at,
            None => Point::from_repr(dragging.motion.sample(now).value),
        };
        dragging.let_go(from, velocity, drag, now, cx);
    }

    /// The pointer lets go: the element keeps the pointer's speed.
    fn release(&self, drag: &Drag, cx: &App) -> bool {
        let mut dragging = self.0.borrow_mut();
        let now = dragging.clock.now(cx);
        let Some(held) = dragging.held.take() else {
            return false;
        };
        let speed = Point::from_repr(held.trail.speed(now));
        dragging.let_go(held.at, speed, drag, now, cx);
        true
    }
}

impl Dragging {
    /// Lets the element go at `from` with `velocity`, as `drag` says.
    fn let_go(
        &mut self,
        from: Point<Pixels>,
        velocity: Point<Pixels>,
        drag: &Drag,
        now: Duration,
        cx: &App,
    ) {
        let speed = drag.keep_along(velocity).to_repr();
        let motion = &mut self.motion;
        motion.jump_to(from.to_repr());
        if let Some(snaps) = &drag.snaps {
            motion.coast_to_nearest(speed, drag.coast, snaps, now);
        } else if let Some((min, max)) = drag.within {
            motion.coast_within(speed, drag.coast, min, max, now);
        } else {
            motion.coast(speed, drag.coast, now);
        }
        if cx.reduce_motion() {
            let rest = motion.target();
            motion.jump_to(rest);
        }
    }
}

/// Makes any styled element one the pointer can drag; see [`Dragged`].
pub trait DragExt: Styled + IntoElement + 'static {
    /// The pointer can drag the element, which is kept in `state` and moves
    /// as `drag` says.
    ///
    /// ```ignore
    /// div().size(px(120.)).bg(rgb(0x3b6ef5)).draggable(&self.card, Drag::new())
    /// ```
    fn draggable(self, state: &DragState, drag: Drag) -> Dragged<Self> {
        Dragged {
            element: Some(self),
            state: state.clone(),
            drag,
        }
    }
}

impl<E: Styled + IntoElement + 'static> DragExt for E {}

/// An element the pointer can drag. Made by [`DragExt::draggable`].
///
/// The left button takes it where the pointer presses on it. While held, it
/// is exactly under the pointer, with no spring between them, wherever the
/// pointer goes in the window. Let go, it keeps the pointer's speed and
/// coasts; under reduced motion it is at once where it would come to rest.
///
/// Its offset moves it by `left` and `top`, as a [`Looked`](crate::Looked)
/// element's does: from its place in the layout, made `relative`, or for an
/// `absolute` element, from its parent's corner.
pub struct Dragged<E> {
    element: Option<E>,
    state: DragState,
    drag: Drag,
}

impl<E: ParentElement> ParentElement for Dragged<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E: Styled + IntoElement + 'static> IntoElement for Dragged<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Styled + IntoElement + 'static> Element for Dragged<E> {
    type RequestLayoutState = AnyElement;
    type PrepaintState = Hitbox;

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
    ) -> (LayoutId, AnyElement) {
        let offset = self.state.frame(window, cx);
        let mut element = self.element.take().expect("an element is laid out once");
        if element.style().position != Some(Position::Absolute) {
            element = element.relative();
        }
        let mut element = element.left(offset.x).top(offset.y).into_any_element();
        (element.request_layout(window, cx), element)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        element: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        element.prepaint(window, cx);
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        element: &mut AnyElement,
        hitbox: &mut Hitbox,
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx);
        // The view that draws the element is drawn again after each change,
        // and not the whole window: views cached beside it stay as they are.
        let view = window.current_view();
        let (state, hitbox) = (self.state.clone(), hitbox.clone());
        window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
            if phase == DispatchPhase::Bubble
                && event.button == MouseButton::Left
                && hitbox.is_hovered(window)
            {
                state.press(event.position, cx);
                // The topmost draggable under the pointer takes it, and no
                // other beneath it.
                cx.stop_propagation();
                cx.notify(view);
            }
        });
        let (state, drag) = (self.state.clone(), self.drag.clone());
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
            if phase != DispatchPhase::Capture {
                return;
            }
            // A move with the button up is a release that was lost, as
            // outside the window: the element is let go there.
            let changed = if event.pressed_button == Some(MouseButton::Left) {
                state.move_to(event.position, &drag, cx)
            } else {
                state.release(&drag, cx)
            };
            if changed {
                cx.notify(view);
            }
        });
        let (state, drag) = (self.state.clone(), self.drag.clone());
        window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
            if phase == DispatchPhase::Capture
                && event.button == MouseButton::Left
                && state.release(&drag, cx)
            {
                cx.notify(view);
            }
        });
    }
}
