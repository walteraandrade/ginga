//! What a presence draws, covered while it leaves: it takes no clicks, and
//! the keyboard focus leaves it.

use ginga::{Phase, Shown};
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, ParentElement, Pixels, Styled, Window, div,
};

/// Covers what a presence draws while it leaves; see [`CoverExt::cover`].
pub trait CoverExt {
    /// `element`, in a box of its own that, while the content leaves, lies
    /// under a cover that takes the pointer, so the element gets no clicks
    /// and no focus from one, and whose keyboard focus is taken away when it
    /// is inside. While the content arrives or rests the element is as it is.
    /// The box is `relative` and as big as its content: an `absolute`
    /// element is placed from where the box is laid out.
    ///
    /// ```ignore
    /// if let Some(shown) = self.panel.frame(window, cx) {
    ///     let panel = div().child(*shown.content).opacity(shown.progress);
    ///     page = page.child(shown.cover(panel));
    /// }
    /// ```
    fn cover<E: IntoElement>(&self, element: E) -> Covered<E>;
}

impl<T> CoverExt for Shown<'_, T> {
    fn cover<E: IntoElement>(&self, element: E) -> Covered<E> {
        Covered {
            element: Some(element),
            leaving: self.phase == Phase::Exiting,
        }
    }
}

/// An element covered while it leaves. Made by [`CoverExt::cover`].
pub struct Covered<E> {
    element: Option<E>,
    leaving: bool,
}

impl<E: IntoElement + 'static> IntoElement for Covered<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: IntoElement + 'static> Element for Covered<E> {
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
        // The box is there in every phase, so the element is laid out the
        // same way while it leaves.
        let mut covered = div().relative().child(element);
        if self.leaving {
            // A handle of this frame's, tracked only while leaving: tracked
            // always, a click on the content would focus the box. Once the
            // frame is drawn, a focus inside it is taken away.
            let focus = cx.focus_handle();
            covered = covered
                .track_focus(&focus)
                .child(div().absolute().inset_0().occlude());
            window.defer(cx, move |window, cx| {
                if focus.contains_focused(window, cx) {
                    window.blur(cx);
                }
            });
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
