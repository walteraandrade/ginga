//! An element opened part of the way: a height that goes from nothing to its
//! content's own, by a progress, as an accordion opens.

use std::mem;

use gpui::{
    AnyElement, App, Bounds, ContentMask, Display, Element, ElementId, FlexDirection,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, Length, Pixels, Refineable, Style,
    StyleRefinement, Styled, Window, point, px, size,
};

/// A move of the box, in pixels, shorter than this asks for no frame: a new
/// height of the content that moves the box less is drawn on the next frame
/// that is drawn anyway.
const LEAST_CHANGE: f32 = 0.5;

/// Declares on any styled element how much of its height is open; see
/// [`Opened`].
pub trait OpenExt: Styled + IntoElement + 'static {
    /// The element takes `progress` of its own height: `0.0` none of it,
    /// `1.0` all of it, as it would be laid out with nothing around it
    /// cut. What is past the open part is cut off. A progress below `0.0`
    /// or past `1.0`, such as a bouncy spring's, is held to them, and NaN is
    /// `0.0`. `id` keeps the height it measured last, for as long as it is
    /// drawn.
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
/// It is held in a box that takes its place in the parent: its margins, its
/// width, its share of a flex row and its position are the box's, so it is
/// laid out the same at every progress, at the width it would have. The box
/// is as tall as that share of the element's own height. Part of the way,
/// what is above or below the box is cut off and takes no clicks, and what
/// sticks out at its sides is not cut. Closed, it takes no room but its
/// margins, and nothing of it is drawn: no clicks, no focus, no keys. Fully
/// open, nothing is cut. Its own height is
/// measured as it is laid out, and is the last one measured: a frame where
/// the content's height changes part of the way open draws the height
/// before, and asks for the next frame.
///
/// A height given as a share of its parent's, such as `h_full`, has no
/// height of its own to open to. What is cut part of the way open can still
/// take the keyboard focus. What it costs is layout: while it opens the box's height changes
/// on each frame, so its parent is laid out again, and the views it pushes
/// are rendered again, cached ones too, since a cached view is reused only
/// where its place and size are the same.
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

/// What an opened element carries from layout to prepaint. `pub` in a
/// private module: it appears in the `Element` impl, and cannot be named
/// outside the crate.
pub struct OpenedLayout {
    element: AnyElement,
    /// The element's own layout, inside the box.
    content: LayoutId,
    /// How much of it is open, held to `0.0..=1.0`.
    progress: f32,
}

impl<E: Styled + IntoElement + 'static> IntoElement for Opened<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

/// Takes from `inner` what places it in its parent, for the box around it.
fn place_in_parent(inner: &mut StyleRefinement) -> StyleRefinement {
    let mut outer = StyleRefinement {
        margin: mem::take(&mut inner.margin),
        position: inner.position.take(),
        inset: mem::take(&mut inner.inset),
        flex_grow: inner.flex_grow.take(),
        flex_shrink: inner.flex_shrink.take(),
        flex_basis: inner.flex_basis.take(),
        align_self: inner.align_self.take(),
        ..StyleRefinement::default()
    };
    outer.size.width = inner.size.width.take();
    outer.min_size.width = inner.min_size.width.take();
    outer.max_size.width = inner.max_size.width.take();
    outer
}

impl<E: Styled + IntoElement + 'static> Element for Opened<E> {
    type RequestLayoutState = OpenedLayout;
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
    ) -> (LayoutId, OpenedLayout) {
        let id = id.expect("an opened element has an id");
        let kept = window.with_element_state(id, |kept: Option<Kept>, _| {
            let kept = kept.unwrap_or_default();
            (kept, kept)
        });
        let mut element = self.element.take().expect("an element is laid out once");
        let progress = if self.progress.is_nan() {
            0.0
        } else {
            self.progress.clamp(0.0, 1.0)
        };
        let partly = progress < 1.0;
        let outer = place_in_parent(element.style());
        // The content keeps its own height in the box: a column that does
        // not shrink it, so the box cuts it instead of squeezing it.
        element.style().flex_shrink = Some(0.);
        let mut element = element.into_any_element();
        let content = element.request_layout(window, cx);
        let mut style = Style::default();
        style.refine(&outer);
        style.display = Display::Flex;
        style.flex_direction = FlexDirection::Column;
        if partly {
            let height = kept.height.map_or(0.0, f32::from) * progress;
            style.size.height = Length::from(px(height));
        }
        let layout = window.request_layout(style, [content], cx);
        (
            layout,
            OpenedLayout {
                element,
                content,
                progress,
            },
        )
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut OpenedLayout,
        window: &mut Window,
        cx: &mut App,
    ) {
        let id = id.expect("an opened element has an id");
        let measured = window.layout_bounds(layout.content).size.height;
        // Closed, nothing of it is drawn: it takes no clicks, no focus and
        // no keys, and draws nothing deferred.
        if layout.progress > 0.0 {
            window.with_content_mask(cut(bounds, layout.progress), |window| {
                layout.element.prepaint(window, cx);
            });
        }
        let progress = layout.progress;
        let moved = window.with_element_state(id, |kept: Option<Kept>, _| {
            let mut kept = kept.unwrap_or_default();
            let before = kept.height.map_or(0.0, f32::from);
            let now = f32::from(measured);
            kept.height = Some(measured);
            // How far the box would move with the new height.
            let moved = (now - before).abs() * progress;
            (moved, kept)
        });
        // Part of the way open, the box was laid out at the height before:
        // the next frame lays it out at the new one, if that moves it.
        if progress < 1.0 && moved >= LEAST_CHANGE {
            window.request_animation_frame();
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut OpenedLayout,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        if layout.progress > 0.0 {
            window.with_content_mask(cut(bounds, layout.progress), |window| {
                layout.element.paint(window, cx);
            });
        }
    }
}

/// How far past the box's sides a cut lets the content draw: only its
/// height is cut, so a shadow or a focus ring at the sides stays.
const SIDES: f32 = 100_000.;

/// The cut of a box part of the way open: what is above or below the box
/// is cut off. Fully open, nothing is cut.
fn cut(bounds: Bounds<Pixels>, progress: f32) -> Option<ContentMask<Pixels>> {
    (progress < 1.0).then(|| ContentMask {
        bounds: Bounds {
            origin: point(bounds.origin.x - px(SIDES), bounds.origin.y),
            size: size(bounds.size.width + px(2. * SIDES), bounds.size.height),
        },
    })
}
