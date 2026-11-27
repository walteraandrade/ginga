//! A look declared on an element: the element gets there by itself.

use std::{cell::RefCell, rc::Rc};

use ginga::{Color, Look, LookMotion, Transition};
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, ParentElement, Pixels, Position, Rgba, Styled, Window, px,
};

use crate::{Clock, unless_reduced};

/// Declares on any styled element how it should look now; see [`Looked`].
pub trait LookExt: Styled + IntoElement + 'static {
    /// The element takes `look`, and gets there with `transition` whenever
    /// `look` changes.
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
        transition: impl Into<Transition>,
    ) -> Looked<Self> {
        Looked::new(self, Keep::Id(id.into()), look, transition.into())
    }

    /// The same as [`LookExt::look`], with the motion kept in `state`, which
    /// the view owns.
    fn look_with(
        self,
        state: &LookState,
        look: Look,
        transition: impl Into<Transition>,
    ) -> Looked<Self> {
        Looked::new(self, Keep::Owned(state.clone()), look, transition.into())
    }
}

impl<E: Styled + IntoElement + 'static> LookExt for E {}

/// The motion of an element's look, owned by a view, for an element that
/// must keep its motion while it is not drawn.
///
/// Clones share one motion.
#[derive(Debug, Clone, Default)]
pub struct LookState(Rc<RefCell<Option<Moving>>>);

/// An element's look on its way, and the clock it is read on.
#[derive(Debug)]
struct Moving {
    motion: LookMotion,
    clock: Clock,
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
/// A value the look names replaces the element's own style for it. The offset
/// moves the element by `left` and `top`: from its place in the layout, made
/// `relative`, or for an `absolute` element, from its parent's corner, so give
/// an absolutely placed element its place as the offset. A value the look does
/// not name is left as the element's own style.
///
/// It asks for frames only while the look moves, and takes each new look at
/// once when the desktop asks for reduced motion.
pub struct Looked<E> {
    element: Option<E>,
    keep: Keep,
    look: Look,
    from: Option<Look>,
    transition: Transition,
}

impl<E: Styled + IntoElement + 'static> Looked<E> {
    fn new(element: E, keep: Keep, look: Look, transition: Transition) -> Self {
        Self {
            element: Some(element),
            keep,
            look,
            from: None,
            transition,
        }
    }

    /// The look the element starts from when it first appears, to animate in.
    /// Without it, the element takes its look at once.
    #[must_use]
    pub fn from(mut self, start: Look) -> Self {
        self.from = Some(start);
        self
    }

    /// This frame's look: moves the motion in `moving` toward the declared
    /// look, or starts one when there is none, and asks for a frame while it
    /// moves.
    fn frame(&self, moving: Option<Moving>, window: &mut Window, cx: &App) -> (Look, Moving) {
        let mut moving = moving.unwrap_or_else(|| Moving {
            motion: LookMotion::at(self.from.unwrap_or(self.look)),
            clock: Clock::default(),
        });
        let now = moving.clock.now(cx);
        let transition = unless_reduced(self.transition, cx);
        let sample = moving.motion.toward(self.look, transition, now);
        if !sample.settled {
            window.request_animation_frame();
        }
        (sample.value, moving)
    }
}

/// `element` with each value that `look` names.
fn styled<E: Styled>(mut element: E, look: Look) -> E {
    if let Some((x, y)) = look.offset {
        if element.style().position != Some(Position::Absolute) {
            element = element.relative();
        }
        element = element.left(px(x)).top(px(y));
    }
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

/// A color of the engine as GPUI's.
fn rgba(color: Color) -> Rgba {
    Rgba {
        r: color.red,
        g: color.green,
        b: color.blue,
        a: color.alpha,
    }
}

/// A GPUI color as a color a [`Look`] can name: `color(rgb(0x3b6ef5))`.
pub fn color(color: impl Into<Rgba>) -> Color {
    let Rgba { r, g, b, a } = color.into();
    Color::new(r, g, b, a)
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
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

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
    ) -> (LayoutId, AnyElement) {
        let look = match (&self.keep, id) {
            (Keep::Owned(state), _) => {
                let mut slot = state.0.borrow_mut();
                let (look, moving) = self.frame(slot.take(), window, cx);
                *slot = Some(moving);
                look
            }
            (Keep::Id(_), Some(id)) => {
                window.with_element_state(id, |moving, window| self.frame(moving, window, cx))
            }
            (Keep::Id(_), None) => unreachable!("an element with an id is laid out with one"),
        };
        let element = self.element.take().expect("an element is laid out once");
        let mut element = styled(element, look).into_any_element();
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

#[cfg(test)]
mod tests {
    use gpui::{AbsoluteLength, DefiniteLength, Length, div};

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
        assert_eq!(style.position, Some(Position::Relative));
        assert_eq!(pixels(style.inset.left), Some(px(12.)));
        assert_eq!(pixels(style.inset.top), Some(px(-4.)));
        assert_eq!(style.opacity, Some(0.25));
        assert_eq!(pixels(style.size.width), Some(px(80.)));
        assert_eq!(pixels(style.size.height), Some(px(40.)));
        assert_eq!(
            style.corner_radii.top_left,
            Some(AbsoluteLength::Pixels(px(6.)))
        );
    }

    #[test]
    fn an_absolute_element_stays_absolute_and_the_offset_is_its_place() {
        let mut element = styled(div().absolute(), Look::new().offset(100., 50.));
        let style = element.style();
        assert_eq!(style.position, Some(Position::Absolute));
        assert_eq!(pixels(style.inset.left), Some(px(100.)));
        assert_eq!(pixels(style.inset.top), Some(px(50.)));
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
            Some(gpui::Hsla::from(rgba(Color::TRANSPARENT)))
        );
        assert_eq!(
            style.border_color,
            Some(gpui::Hsla::from(rgba(blue.with_alpha(0.5))))
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
