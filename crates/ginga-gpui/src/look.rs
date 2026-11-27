//! A look declared on an element: the element gets there by itself.

use std::{cell::RefCell, rc::Rc};

use ginga::{Look, LookMotion, Transition};
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, ParentElement, Pixels, Styled, Window, px,
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
/// A value the look names replaces the element's own style for it: the offset
/// places the element `relative`ly, moved by `left` and `top`, so give an
/// absolutely placed element its place as the offset instead. A value the look
/// does not name is left as the element's own style.
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
    /// look, and asks for a frame while it moves.
    fn frame(&self, moving: &mut Option<Moving>, window: &mut Window, cx: &App) -> Look {
        let moving = moving.get_or_insert_with(|| Moving {
            motion: LookMotion::at(self.from.unwrap_or(self.look)),
            clock: Clock::default(),
        });
        let now = moving.clock.now(cx);
        let transition = unless_reduced(self.transition, cx);
        let sample = moving.motion.toward(self.look, transition, now);
        if !sample.settled {
            window.request_animation_frame();
        }
        sample.value
    }
}

/// `element` with each value that `look` names.
fn styled<E: Styled>(mut element: E, look: Look) -> E {
    if let Some((x, y)) = look.offset {
        element = element.relative().left(px(x)).top(px(y));
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
    element
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
                let state = state.clone();
                self.frame(&mut state.0.borrow_mut(), window, cx)
            }
            (Keep::Id(_), Some(id)) => {
                window.with_element_state(id, |moving: Option<Option<Moving>>, window| {
                    let mut moving = moving.flatten();
                    let look = self.frame(&mut moving, window, cx);
                    (look, moving)
                })
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
