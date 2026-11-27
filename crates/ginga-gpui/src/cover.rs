//! What a presence draws, covered while it leaves: it takes no clicks, and
//! the keyboard focus leaves it.

use ginga::{Phase, Shown};
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, ParentElement, Pixels, Styled, Window, div,
};

/// Covers what a presence draws while it leaves; see [`CoverExt::cover`].
pub trait CoverExt {
    /// `element`, covered while the content leaves: a child over all of it
    /// takes the pointer, so it gets no clicks and no focus from one, and a
    /// keyboard focus inside it is taken away. While the content arrives or
    /// rests the element is as it is. Nothing wraps it, so its layout is the
    /// same in every phase: an element with no position of its own is made
    /// `relative` while it leaves, so that the cover lies over it, and an
    /// `absolute` one keeps its place.
    ///
    /// A way out that takes no time, such as an undelayed one under reduced
    /// motion, is over before a frame draws it leaving, so there is nothing
    /// to cover: take the focus away where you hide it.
    ///
    /// ```ignore
    /// if let Some(shown) = self.panel.frame(window, cx) {
    ///     let panel = div().child(*shown.content).opacity(shown.progress);
    ///     page = page.child(shown.cover(panel));
    /// }
    /// ```
    fn cover<E: Coverable>(&self, element: E) -> Covered<E>;
}

impl<T> CoverExt for Shown<'_, T> {
    fn cover<E: Coverable>(&self, element: E) -> Covered<E> {
        Covered {
            element: Some(element),
            leaving: self.phase == Phase::Exiting,
        }
    }
}

/// An element that can be covered: one that takes children, a style and a
/// focus handle, such as a `div`.
pub trait Coverable: IntoElement + ParentElement + Styled + InteractiveElement + 'static {}

impl<E: IntoElement + ParentElement + Styled + InteractiveElement + 'static> Coverable for E {}

/// An element covered while it leaves. Made by [`CoverExt::cover`].
pub struct Covered<E> {
    element: Option<E>,
    leaving: bool,
}

/// The element's own style, so a covered element can still be styled, such
/// as with [`LookExt::with_look`](crate::LookExt::with_look).
impl<E: Coverable> Styled for Covered<E> {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.element
            .as_mut()
            .expect("an element is styled before it is laid out")
            .style()
    }
}

impl<E: Coverable> IntoElement for Covered<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Coverable> Element for Covered<E> {
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
        let mut covered = self.element.take().expect("an element is laid out once");
        if self.leaving {
            if covered.style().position.is_none() {
                covered = covered.relative();
            }
            covered = covered.child(div().absolute().inset_0().occlude());
            // With nothing focused in the window there is nothing to take.
            if window.focused(cx).is_some() {
                // A handle of this frame's, tracked only while leaving:
                // tracked always, a click on the content would focus it.
                // The check waits until the frame is drawn, so it reads the
                // focus tree this frame built.
                let focus = cx.focus_handle();
                covered = covered.track_focus(&focus);
                window.defer(cx, move |window, cx| {
                    if focus.contains_focused(window, cx) {
                        window.blur(cx);
                    }
                });
            }
        }
        let mut element = covered.into_any_element();
        (element.request_layout(window, cx), element)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        element: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
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
