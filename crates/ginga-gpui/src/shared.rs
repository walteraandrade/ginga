//! A shared element: two elements that declare one key are one thing, which
//! travels from where the first was to where the second is.

use std::{
    collections::HashMap,
    rc::{Rc, Weak},
};

use ginga::Transition;
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, Global, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, ParentElement, Pixels, SharedString, Styled, Window, deferred, div, px,
};

use crate::{Animate, Clock, unless_reduced};

/// Declares on any styled element that it is one thing with every other
/// element of the same key; see [`Shared`].
pub trait SharedExt: Styled + IntoElement + 'static {
    /// The element is the thing called `key`. When it appears while another
    /// element of that key has just gone, it does not appear in its place:
    /// it travels there from where the other one was, with `transition`.
    ///
    /// ```ignore
    /// // In each tab: only the selected one draws the mark.
    /// div().id(tab).children(selected.then(|| {
    ///     div().h(px(2.)).w_full().bg(accent).shared("tab-mark", Spring::SNAPPY)
    /// }))
    /// ```
    fn shared(
        self,
        key: impl Into<SharedString>,
        transition: impl Into<Transition>,
    ) -> Shared<Self> {
        Shared {
            key: key.into(),
            transition: transition.into(),
            element: Some(self),
        }
    }
}

impl<E: Styled + IntoElement + 'static> SharedExt for E {}

/// An element that is one thing with every element of the same key: made by
/// [`SharedExt::shared`].
///
/// While it travels it is drawn above the rest of the window, with its place
/// and its size changing together, and the place it is heading for is kept
/// empty at its size, so nothing around it moves. GPUI cannot scale an
/// element, so its content is laid out again at each size it passes.
///
/// It travels only from an element of its key that was drawn in the frame
/// before, or that is still drawn: a key whose last element went long ago
/// is not a place to come from. The frame it appears in is spent measuring
/// where it goes, and it is not drawn in that frame.
///
/// GPUI tells elements apart by the ids of the elements around them. Two
/// elements of one key must differ there, such as tabs that each have an id:
/// to GPUI, two with the same ids around them are one element that moved,
/// and it takes its new place at once.
pub struct Shared<E> {
    key: SharedString,
    transition: Transition,
    element: Option<E>,
}

/// Where each key was drawn last, by which element, and on what way: the
/// one place two elements of a key meet. One clock for all of them, so a
/// motion handed from one element to another reads the same time.
#[derive(Default)]
struct Places {
    clock: Clock,
    last: HashMap<SharedString, Place>,
}

impl Global for Places {}

struct Place {
    owner: GlobalElementId,
    /// Gone once GPUI drops the owner's state: it was not drawn for a frame.
    alive: Weak<()>,
    motion: ginga::Motion<<Bounds<Pixels> as Animate>::Repr>,
}

/// What a shared element keeps under its id.
#[derive(Clone)]
struct Kept {
    /// Lives as long as GPUI keeps this state, so the key's place knows it.
    token: Rc<()>,
    /// Where it is drawn: at rest in its place, or travelling there.
    motion: Option<ginga::Motion<<Bounds<Pixels> as Animate>::Repr>>,
    /// Its place, as last laid out.
    place: Option<Bounds<Pixels>>,
}

/// How a shared element is drawn this frame. `pub` in a private module: it
/// appears in the `Element` impl, and cannot be named outside the crate.
#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    /// In its place.
    InPlace,
    /// Laid out in its place to learn where that is, and not drawn.
    Measuring,
    /// Above the rest at these bounds, its place kept empty.
    Travelling,
}

impl<E: Styled + IntoElement + 'static> IntoElement for Shared<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Styled + IntoElement + 'static> Element for Shared<E> {
    type RequestLayoutState = (AnyElement, Mode);
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name(self.key.clone()))
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
    ) -> (LayoutId, (AnyElement, Mode)) {
        let id = id.expect("a shared element has an id");
        let now = now(cx);
        let places = cx.default_global::<Places>();
        let kept = window.with_element_state(id, |kept: Option<Kept>, _| {
            let kept = kept.unwrap_or_else(|| Kept {
                token: Rc::new(()),
                // Where the last element of the key was, if it is still
                // there or was drawn in the frame before.
                motion: places
                    .last
                    .get(&self.key)
                    .filter(|place| place.owner != *id && place.alive.upgrade().is_some())
                    .map(|place| place.motion),
                place: None,
            });
            (kept.clone(), kept)
        });
        let element = self.element.take().expect("an element is laid out once");
        let (element, mode) = match (kept.motion, kept.place) {
            (Some(_), None) => (element.into_any_element(), Mode::Measuring),
            (Some(motion), Some(place)) if !motion.sample(now).settled => {
                let at = Bounds::from_repr(motion.sample(now).value);
                let traveller = element
                    .absolute()
                    .left(at.origin.x - place.origin.x)
                    .top(at.origin.y - place.origin.y)
                    .w(at.size.width.max(px(0.)))
                    .h(at.size.height.max(px(0.)));
                let element = div()
                    .relative()
                    .w(place.size.width)
                    .h(place.size.height)
                    .child(deferred(traveller).with_priority(1))
                    .into_any_element();
                (element, Mode::Travelling)
            }
            _ => (element.into_any_element(), Mode::InPlace),
        };
        if mode != Mode::InPlace {
            window.request_animation_frame();
        }
        let mut element = element;
        (element.request_layout(window, cx), (element, mode))
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (element, _): &mut (AnyElement, Mode),
        window: &mut Window,
        cx: &mut App,
    ) {
        let id = id.expect("a shared element has an id");
        element.prepaint(window, cx);
        let now = now(cx);
        let transition = unless_reduced(self.transition, cx);
        let kept = window.with_element_state(id, |kept: Option<Kept>, _| {
            let mut kept = kept.expect("laid out before it is prepainted");
            let target = bounds.to_repr();
            match &mut kept.motion {
                // A place to head for, the first time or because it moved.
                Some(motion) if kept.place != Some(bounds) => {
                    if kept.place.is_none() || !motion.sample(now).settled {
                        motion.set_target(target, transition, now);
                    } else {
                        motion.jump_to(target);
                    }
                }
                Some(_) => {}
                None => kept.motion = Some(ginga::Motion::at(target)),
            }
            kept.place = Some(bounds);
            (kept.clone(), kept)
        });
        if let Some(motion) = kept.motion {
            cx.default_global::<Places>().last.insert(
                self.key.clone(),
                Place {
                    owner: id.clone(),
                    alive: Rc::downgrade(&kept.token),
                    motion,
                },
            );
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        (element, mode): &mut (AnyElement, Mode),
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        if *mode != Mode::Measuring {
            element.paint(window, cx);
        }
    }
}

/// The time on the clock every shared element reads.
fn now(cx: &mut App) -> core::time::Duration {
    let mut clock = cx.default_global::<Places>().clock;
    let now = clock.now(cx);
    cx.global_mut::<Places>().clock = clock;
    now
}
