//! An element opened part of the way: a height that goes from nothing to its
//! content's own, by a progress, as an accordion opens.

use std::mem;

use ginga::Transition;

use gpui::{
    AnyElement, App, Bounds, ContentMask, Display, Element, ElementId, FlexDirection,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, Length, Pixels, Refineable, Style,
    StyleRefinement, Styled, Window, point, px, size,
};

use crate::{Clock, unless_reduced};

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
            glide: None,
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
    /// How the height it opens to heads for a new height of the content.
    glide: Option<Transition>,
    element: Option<E>,
}

impl<E> Opened<E> {
    /// When its content's own height changes, the height it opens to heads
    /// for the new one with `transition`, from where it is and keeping its
    /// speed, instead of taking it at once: a toast whose text grows, a
    /// panel whose rows change. Open all the way, give it a progress of
    /// `1.0`. The frame that finds the change draws the height before; what
    /// is above or below the box is cut while it glides. Under reduced
    /// motion the new height is taken after the transition's delay, at once.
    ///
    /// ```ignore
    /// div().p_2().child(self.message.clone())
    ///     .opened("toast", 1.)
    ///     .height_glides(Spring::SNAPPY)
    /// ```
    #[must_use]
    pub fn height_glides(mut self, transition: impl Into<Transition>) -> Self {
        self.glide = Some(transition.into());
        self
    }
}

/// What an opened element keeps under its id: its own height, as last
/// measured, and the height it opens to, when that glides.
#[derive(Clone, Copy, Default)]
struct Kept {
    height: Option<Pixels>,
    clock: Clock,
    /// The height it opens to, heading for the content's own: once it has
    /// been measured, and only when its height glides.
    glide: Option<ginga::Motion<f32>>,
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
    /// The box is not as tall as the content: what is past it is cut.
    cut: bool,
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
        let (kept, gliding) = window.with_element_state(id, |kept: Option<Kept>, _| {
            let mut kept = kept.unwrap_or_default();
            let now = kept.clock.now(cx);
            let gliding = self
                .glide
                .and(kept.glide)
                .map(|glide| px(glide.sample(now).value));
            ((kept, gliding), kept)
        });
        let mut element = self.element.take().expect("an element is laid out once");
        let progress = if self.progress.is_nan() {
            0.0
        } else {
            self.progress.clamp(0.0, 1.0)
        };
        let partly = progress < 1.0 || gliding.is_some();
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
            let open_to = gliding.or(kept.height).map_or(0.0, f32::from);
            style.size.height = Length::from(px(open_to * progress));
        }
        let layout = window.request_layout(style, [content], cx);
        (
            layout,
            OpenedLayout {
                element,
                content,
                progress,
                cut: partly,
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
            window.with_content_mask(cut(bounds, layout.cut), |window| {
                layout.element.prepaint(window, cx);
            });
        }
        let progress = layout.progress;
        let glide = self.glide.map(|transition| unless_reduced(transition, cx));
        let next_frame = window.with_element_state(id, |kept: Option<Kept>, _| {
            let mut kept = kept.unwrap_or_default();
            let before = kept.height.map_or(0.0, f32::from);
            let height = f32::from(measured);
            kept.height = Some(measured);
            let Some(transition) = glide else {
                kept.glide = None;
                // Part of the way open, the box was laid out at the height
                // before: the next frame lays it out at the new one, if that
                // moves it.
                let moved = (height - before).abs() * progress;
                return (progress < 1.0 && moved >= LEAST_CHANGE, kept);
            };
            let now = kept.clock.now(cx);
            let motion = kept.glide.get_or_insert(ginga::Motion::at(height));
            // The box was laid out at the height before: a change that
            // moves it is drawn from the next frame, even one taken at once.
            let moves = (height - motion.target()).abs() * progress >= LEAST_CHANGE;
            if moves {
                motion.set_target(height, transition, now);
            } else if motion.target() != height {
                // Too small a change to move the box: it is taken at once.
                motion.jump_to(height);
            }
            (moves || !motion.sample(now).settled, kept)
        });
        if next_frame {
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
            window.with_content_mask(cut(bounds, layout.cut), |window| {
                layout.element.paint(window, cx);
            });
        }
    }
}

/// How far past the box's sides a cut lets the content draw: only its
/// height is cut, so a shadow or a focus ring at the sides stays.
const SIDES: f32 = 100_000.;

/// The cut of a box not as tall as its content: what is above or below the
/// box is cut off.
fn cut(bounds: Bounds<Pixels>, cut: bool) -> Option<ContentMask<Pixels>> {
    cut.then(|| ContentMask {
        bounds: Bounds {
            origin: point(bounds.origin.x - px(SIDES), bounds.origin.y),
            size: size(bounds.size.width + px(2. * SIDES), bounds.size.height),
        },
    })
}
