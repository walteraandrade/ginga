//! Ways in and out declared on an element: it arrives from one look, leaves
//! to another, and is drawn until its way out is over.

use ginga::{InOut, Look, Phase, Shown, Slot, Swap, Transition};
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, ParentElement, Pixels, Styled, Window, div,
};

use crate::{Clock, look::styled, reduced};

/// Draws `content` while it is there, and keeps drawing it on its way out.
///
/// Call it in every frame, with `Some` while the thing should be on screen
/// and `None` once it should not. It keeps what it needs under `id`: the
/// content that is leaving, and where each one is on its way. Content that
/// first appears arrives from [`arrive_from`](Present::arrive_from), and
/// content that is no longer wanted leaves to [`leave_to`](Present::leave_to),
/// with the transitions of `ways`. Wanted again on its way out, it turns
/// around from where it is. Other content in its place swaps, as
/// [`swap`](Present::swap) says.
///
/// While it leaves it takes no clicks, and so no focus from the pointer.
///
/// ```ignore
/// let ways = InOut::new(Spring::SNAPPY, Tween::new(ms(150), Easing::In));
/// present("toast", self.saved.then_some("Saved"), ways, |text| {
///     div().px_3().py_2().bg(rgb(0x2a2f3a)).child(*text)
/// })
/// .arrive_from(Look::new().offset(0., 16.).opacity(0.))
/// .leave_to(Look::new().opacity(0.))
/// ```
pub fn present<T, E>(
    id: impl Into<ElementId>,
    content: Option<T>,
    ways: InOut,
    draw: impl Fn(&T) -> E + 'static,
) -> Present<T>
where
    T: PartialEq + 'static,
    E: IntoElement,
{
    Present {
        id: id.into(),
        content,
        ways,
        draw: Box::new(move |content| draw(content).into_any_element()),
        arrive_from: None,
        leave_to: None,
        swap: Swap::Sync,
    }
}

/// An element with ways in and out: made by [`present`].
pub struct Present<T> {
    id: ElementId,
    content: Option<T>,
    ways: InOut,
    draw: Box<dyn Fn(&T) -> AnyElement>,
    arrive_from: Option<Look>,
    leave_to: Option<Look>,
    swap: Swap,
}

impl<T: PartialEq + 'static> Present<T> {
    /// The look content arrives from, mixed into its own as it arrives (see
    /// [`Look::mix`]): an offset and an opacity go to no offset and fully
    /// opaque. Without it, content appears at once.
    #[must_use]
    pub fn arrive_from(mut self, look: Look) -> Self {
        self.arrive_from = Some(look);
        self
    }

    /// The look content leaves to. Without it, content is gone at once.
    #[must_use]
    pub fn leave_to(mut self, look: Look) -> Self {
        self.leave_to = Some(look);
        self
    }

    /// How other content takes the place of what is there:
    /// [`Swap::Sync`], the default, crosses them; [`Swap::Wait`] lets the
    /// old one go first. The first frame's choice is kept.
    #[must_use]
    pub fn swap(mut self, swap: Swap) -> Self {
        self.swap = swap;
        self
    }

    /// The ways to use this frame: none for a look that is not given, and
    /// none at all when the desktop asks for reduced motion.
    fn ways(&self, cx: &App) -> InOut {
        if reduced(cx) {
            return InOut::INSTANT;
        }
        InOut {
            enter: if self.arrive_from.is_some() {
                self.ways.enter
            } else {
                Transition::Instant
            },
            exit: if self.leave_to.is_some() {
                self.ways.exit
            } else {
                Transition::Instant
            },
        }
    }

    /// One content as it is drawn this frame: in a box of its own, kept by
    /// the content's number so what it holds lasts while it swaps, with the
    /// look of where it is, and covered while it leaves.
    fn drawn(&self, shown: &Shown<'_, Numbered<T>>) -> gpui::Stateful<gpui::Div> {
        let look = match shown.phase {
            Phase::Present => Look::new(),
            Phase::Entering => self
                .arrive_from
                .map_or(Look::new(), |from| from.mix(Look::new(), shown.progress)),
            Phase::Exiting => self
                .leave_to
                .map_or(Look::new(), |to| to.mix(Look::new(), shown.progress)),
        };
        let mut drawn = div()
            .id(ElementId::Integer(shown.content.number))
            .child((self.draw)(&shown.content.content));
        if shown.phase == Phase::Exiting {
            // Over the content, it takes the pointer, so what leaves takes
            // no clicks and gets no focus from one.
            drawn = drawn.relative().child(div().absolute().inset_0().occlude());
        }
        styled(drawn, look)
    }
}

/// Content with the number it was given when it was first shown. Two are
/// the same when their content is.
struct Numbered<T> {
    number: u64,
    content: T,
}

impl<T: PartialEq> PartialEq for Numbered<T> {
    fn eq(&self, other: &Self) -> bool {
        self.content == other.content
    }
}

/// What a [`Present`] keeps under its id.
struct Kept<T> {
    slot: Slot<Numbered<T>>,
    clock: Clock,
    /// The number the next new content gets.
    next: u64,
}

impl<T: PartialEq + 'static> IntoElement for Present<T> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<T: PartialEq + 'static> Element for Present<T> {
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
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let id = id.expect("an element with an id is laid out with one");
        let ways = self.ways(cx);
        let content = self.content.take();
        let mut element = window.with_element_state(id, |kept: Option<Kept<T>>, window| {
            let mut kept = kept.unwrap_or_else(|| Kept {
                slot: Slot::new(self.swap),
                clock: Clock::default(),
                next: 0,
            });
            let now = kept.clock.now(cx);
            match content {
                Some(content) => {
                    let number = kept.next;
                    kept.next += 1;
                    kept.slot.show(Numbered { number, content }, ways, now);
                }
                None => kept.slot.hide(ways.exit, now),
            }
            let [leaving, here] = kept.slot.frame(now);
            if leaving.as_ref().is_some_and(Shown::moving)
                || here.as_ref().is_some_and(Shown::moving)
            {
                window.request_animation_frame();
            }
            let leaving = leaving.map(|shown| self.drawn(&shown).absolute().top_0().left_0());
            let here = here.map(|shown| self.drawn(&shown));
            // What arrives lies over what leaves; what leaves takes no room.
            let element = div()
                .relative()
                .children(leaving)
                .children(here)
                .into_any_element();
            (element, kept)
        });
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
