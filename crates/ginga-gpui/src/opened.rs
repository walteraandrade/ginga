//! An element opened part of the way: a height that goes from nothing to its
//! content's own, by a progress, as an accordion opens.

use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, ParentElement, Pixels, Styled, Window, div, px,
};

/// A change of the content's height shorter than this, in pixels, is not a
/// change: no frame is asked for it.
const SAME: f32 = 0.5;

/// Declares on any styled element how much of its height is open; see
/// [`Opened`].
pub trait OpenExt: Styled + IntoElement + 'static {
    /// The element takes `progress` of its own height: `0.0` none of it,
    /// `1.0` all of it, as it would be laid out with nothing around it
    /// cut. What is past the open part is cut off. `id` keeps the height
    /// it measured last, for as long as it is drawn.
    ///
    /// The progress comes from whatever moves it: a presence's, a motion's.
    ///
    /// ```ignore
    /// // `details` is a `ginga_gpui::Presence`: shown, it opens; hidden, it closes.
    /// let open = self.details.frame(window, cx).map(|shown| (shown.progress, *shown.content));
    /// div().children(open.map(|(progress, text)| {
    ///     div().p_2().child(text).opened("details", progress)
    /// }))
    /// ```
    fn opened(self, id: impl Into<ElementId>, progress: f32) -> Opened<Self> {
        Opened {
            id: id.into(),
            progress,
            element: Some(self),
        }
    }
}

impl<E: Styled + IntoElement + 'static> OpenExt for E {}

/// An element opened part of the way: made by [`OpenExt::opened`].
///
/// Fully open, it is the element as it is, with nothing added and nothing
/// cut. Part of the way, it is held in a box of that share of its own
/// height, laid out at the width it would have, with what is past the box
/// cut off and taking no clicks; closed, it takes no room. Its own height
/// is measured as it is laid out, and is the last one measured: a frame
/// where the content's height changes draws the height before, and asks for
/// the next frame.
///
/// A height given as a share of its parent's, such as `h_full`, has no
/// height of its own to open to. What it costs is layout: while it opens
/// the box's height changes on each frame, so its parent is laid out again,
/// and the views it pushes are rendered again, cached ones too, since a
/// cached view is reused only where its place and size are the same.
pub struct Opened<E> {
    id: ElementId,
    progress: f32,
    element: Option<E>,
}

/// What an opened element keeps under its id: its own height, as last
/// measured.
#[derive(Clone, Copy, Default)]
struct Kept {
    height: Option<Pixels>,
}

/// What one frame of an opened element carries from layout to prepaint.
/// `pub` in a private module: it appears in the `Element` impl, and cannot
/// be named outside the crate.
pub struct Frame {
    element: AnyElement,
    /// The element's own height, written as it is prepainted.
    measured: Rc<Cell<Option<Pixels>>>,
    /// Whether it was laid out part of the way open.
    partly: bool,
}

impl<E: Styled + IntoElement + 'static> IntoElement for Opened<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Styled + IntoElement + 'static> Element for Opened<E> {
    type RequestLayoutState = Frame;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
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
    ) -> (LayoutId, Frame) {
        let id = id.expect("an opened element has an id");
        let kept = window.with_element_state(id, |kept: Option<Kept>, _| {
            let kept = kept.unwrap_or_default();
            (kept, kept)
        });
        let element = self.element.take().expect("an element is laid out once");
        let measured = Rc::new(Cell::new(None));
        let progress = if self.progress.is_nan() {
            0.0
        } else {
            self.progress.clamp(0.0, 1.0)
        };
        let partly = progress < 1.0;
        let mut element = if partly {
            let height = kept.height.map_or(0.0, f32::from) * progress;
            // A column that does not shrink its content: the content keeps
            // its own height, and the box cuts it.
            div()
                .flex()
                .flex_col()
                .overflow_hidden()
                .h(px(height))
                .child(Measured {
                    element: Some(element.flex_shrink_0()),
                    measured: measured.clone(),
                })
                .into_any_element()
        } else {
            Measured {
                element: Some(element),
                measured: measured.clone(),
            }
            .into_any_element()
        };
        let layout = element.request_layout(window, cx);
        (
            layout,
            Frame {
                element,
                measured,
                partly,
            },
        )
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        frame: &mut Frame,
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.element.prepaint(window, cx);
        let id = id.expect("an opened element has an id");
        let measured = frame.measured.get();
        let changed = window.with_element_state(id, |kept: Option<Kept>, _| {
            let mut kept = kept.unwrap_or_default();
            let changed = match (kept.height, measured) {
                (Some(before), Some(now)) => f32::from(now - before).abs() >= SAME,
                (None, Some(_)) => true,
                (_, None) => false,
            };
            if changed {
                kept.height = measured;
            }
            (changed, kept)
        });
        // Part of the way open, the box was laid out at the height before:
        // the next frame lays it out at the new one.
        if changed && frame.partly {
            window.request_animation_frame();
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        frame: &mut Frame,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.element.paint(window, cx);
    }
}

impl<E: ParentElement> ParentElement for Opened<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

/// An element that writes down its height as it is prepainted.
struct Measured<E> {
    element: Option<E>,
    measured: Rc<Cell<Option<Pixels>>>,
}

impl<E: IntoElement + 'static> IntoElement for Measured<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: IntoElement + 'static> Element for Measured<E> {
    type RequestLayoutState = AnyElement;
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
    ) -> (LayoutId, AnyElement) {
        let element = self.element.take().expect("an element is laid out once");
        let mut element = element.into_any_element();
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
    ) {
        self.measured.set(Some(bounds.size.height));
        element.prepaint(window, cx);
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
