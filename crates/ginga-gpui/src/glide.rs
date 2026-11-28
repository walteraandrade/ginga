//! An element that glides to where layout puts it: when its place in the
//! layout changes, it is painted where it was and travels to the new place.

use ginga::Transition;
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, Global, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, ParentElement, Pixels, Point, Size, Window,
};

use crate::{Animate, Clock, unless_reduced};

/// A move in the layout shorter than this, in pixels, is not a move: the
/// place is exact, and this only absorbs float error in what is subtracted
/// from it.
const STILL: f32 = 0.5;

/// Declares on any element that it glides to where layout puts it;
/// see [`Glides`].
pub trait GlideExt: IntoElement + 'static {
    /// When layout moves the element, it is painted where it was and
    /// travels to its new place with `transition`, instead of jumping
    /// there. The motion is kept under `id` for as long as the element is
    /// drawn.
    ///
    /// ```ignore
    /// // Cards in a wrapping row: one that is added or removed reflows the others.
    /// div().flex().flex_wrap().children(self.cards.iter().map(|card| {
    ///     div().w(px(120.)).h(px(80.)).glides(("card", card.id), Spring::SNAPPY)
    /// }))
    /// ```
    fn glides(self, id: impl Into<ElementId>, transition: impl Into<Transition>) -> Glides<Self> {
        Glides {
            id: id.into(),
            transition: transition.into(),
            element: Some(self),
        }
    }
}

impl<E: IntoElement + 'static> GlideExt for E {}

/// An element that glides to where layout puts it: made by
/// [`GlideExt::glides`].
///
/// Only what is painted moves, and where the pointer finds it: layout is not
/// touched, so the elements around it take their places at once and
/// nothing is laid out again while it glides. A frame of a glide costs a
/// sample of its motion. Its size is not moved: it takes its new size at
/// once.
///
/// A move is a change of its place in the layout of the window. A scroll,
/// an offset that a look, a drag or another element paints it with, and a
/// glide of an element around it are not: an element that glides inside
/// another that glides moves with it, and glides only by how its place in
/// the outer one changes. A change of the window's size is not a move
/// either: everything takes its new place at once.
///
/// Its first frame is in place. A second move on the way turns from where
/// it is, with its speed. Under reduced motion it is in its new place at
/// once, after the transition's delay. The rows of GPUI's `list` and
/// `uniform_list` are each laid out on their own, so their place in the
/// list is not a place in the window: use
/// [`PresenceList::place_all`](crate::PresenceList::place_all) there. A
/// cached view (`.cached(..)`), a `deferred` element and an `anchored` one
/// are laid out or drawn on their own too: a glide inside one measures its
/// place there, so it does not see the view move, and inside an element
/// that glides it does not know the glide around it. Put the glide on the
/// cached view, or around it.
pub struct Glides<E> {
    id: ElementId,
    transition: Transition,
    element: Option<E>,
}

/// What a gliding element keeps under its id.
#[derive(Clone, Copy)]
struct Kept {
    clock: Clock,
    /// Where it is painted, as a place in the layout, from the gliding
    /// element around it. Its target is its place, as last laid out.
    motion: ginga::Motion<<Point<Pixels> as Animate>::Repr>,
    /// The window's size when it was laid out there.
    viewport: Size<Pixels>,
}

/// The places of the gliding elements being prepainted around the one now
/// prepainted, innermost last.
#[derive(Default)]
struct GlidePlaces(Vec<Point<Pixels>>);

impl Global for GlidePlaces {}

impl<E: IntoElement + 'static> IntoElement for Glides<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: IntoElement + 'static> Element for Glides<E> {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
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
        let element = self.element.take().expect("an element is laid out once");
        let mut element = element.into_any_element();
        (element.request_layout(window, cx), element)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        element: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        let id = id.expect("a gliding element has an id");
        // Its place in the window's layout, without what is painted on top
        // of layout (a scroll, a look's offset, a drag, a glide around it).
        // GPUI adds the offset snapped to the pixel, so it is taken away
        // snapped too, and the place is exact.
        let in_window = bounds.origin - window.pixel_snap_point(window.element_offset());
        let around = cx
            .try_global::<GlidePlaces>()
            .and_then(|places| places.0.last().copied())
            .unwrap_or_default();
        let place = in_window - around;
        let viewport = window.viewport_size();
        let transition = unless_reduced(self.transition, cx);
        let (offset, moving) = window.with_element_state(id, |kept: Option<Kept>, _| {
            let mut kept = kept.unwrap_or_else(|| Kept {
                clock: Clock::default(),
                motion: ginga::Motion::at(place.to_repr()),
                viewport,
            });
            let now = kept.clock.now(cx);
            let moved = (place - Point::from_repr(kept.motion.target())).map(f32::from);
            if moved.x.abs() >= STILL || moved.y.abs() >= STILL {
                if viewport == kept.viewport {
                    kept.motion.set_target(place.to_repr(), transition, now);
                } else {
                    kept.motion.jump_to(place.to_repr());
                }
            }
            kept.viewport = viewport;
            let sample = kept.motion.sample(now);
            let offset = if sample.settled {
                Point::default()
            } else {
                Point::from_repr(sample.value) - place
            };
            ((offset, !sample.settled), kept)
        });
        if moving {
            window.request_animation_frame();
        }
        cx.default_global::<GlidePlaces>().0.push(in_window);
        window.with_element_offset(offset, |window| element.prepaint(window, cx));
        cx.global_mut::<GlidePlaces>().0.pop();
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        element: &mut AnyElement,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        element.paint(window, cx);
    }
}

impl<E: ParentElement> ParentElement for Glides<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}
