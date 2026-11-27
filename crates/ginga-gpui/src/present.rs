//! Ways in and out declared on an element: it arrives from one look, leaves
//! to another, and is drawn until its way out is over.

use ginga::{InOut, Look, Phase, Shown, Slot, Swap, Transition};
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, FocusHandle, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, ParentElement, Pixels, Styled, Window, div,
};

use crate::{
    Clock,
    look::{PaintedAt, offset_of, painted_at, styled},
    unless_reduced,
};

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
    /// The look content arrives from: its offset and its opacity, which go
    /// to no offset and fully opaque as it arrives (see [`Look::mix`]). Any
    /// other value the look names is left out: the element's own size and
    /// colors are its content's to give. Without it, content appears at once.
    #[must_use]
    pub fn arrive_from(mut self, look: Look) -> Self {
        self.arrive_from = Some(look);
        self
    }

    /// The look content leaves to: its offset and its opacity, as for
    /// [`arrive_from`](Self::arrive_from). Content that comes back on its way
    /// out returns from this look, so it turns around from where it is.
    /// Without it, content is gone at once.
    #[must_use]
    pub fn leave_to(mut self, look: Look) -> Self {
        self.leave_to = Some(look);
        self
    }

    /// How other content takes the place of what is there:
    /// [`Swap::Sync`], the default, crosses them; [`Swap::Wait`] lets the
    /// old one go first, unseen until then. A change applies to the next swap.
    #[must_use]
    pub fn swap(mut self, swap: Swap) -> Self {
        self.swap = swap;
        self
    }

    /// The ways to use this frame: none for a look that is not given, and
    /// none with motion when the desktop asks for reduced motion, each after
    /// its delay.
    fn ways(&self, cx: &App) -> InOut {
        InOut {
            enter: if moves(self.arrive_from) {
                unless_reduced(self.ways.enter, cx)
            } else {
                Transition::Instant
            },
            exit: if moves(self.leave_to) {
                unless_reduced(self.ways.exit, cx)
            } else {
                Transition::Instant
            },
        }
    }

    /// One content as it is drawn this frame: in a box of its own, kept by
    /// the content's number so what it holds lasts while it swaps, with the
    /// look of where it is, and covered while it leaves. What a swap sends
    /// away lies at the top left, out of the layout. `focus` is tracked on
    /// content that leaves.
    fn drawn(
        &self,
        shown: &Shown<'_, Numbered<T>>,
        swapped_out: bool,
        came_back: Option<u64>,
        focus: Option<FocusHandle>,
    ) -> PaintedAt {
        let coming_back = came_back == Some(shown.content.number);
        let look = match shown.phase {
            Phase::Present => Look::new(),
            Phase::Entering if coming_back => self
                .leave_to
                .map_or(Look::new(), |to| to.mix(Look::new(), shown.progress)),
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
        if swapped_out {
            drawn = drawn.absolute().top_0().left_0();
        }
        if shown.phase == Phase::Exiting {
            // Over the content, it takes the pointer, so what leaves takes
            // no clicks and gets no focus from one.
            if !swapped_out {
                drawn = drawn.relative();
            }
            drawn = drawn.child(div().absolute().inset_0().occlude());
        }
        if let Some(focus) = focus {
            drawn = drawn.track_focus(&focus);
        }
        painted_at(styled(drawn, look), offset_of(look))
    }
}

/// A way in or out with this look moves something: only an offset and an
/// opacity are mixed in, so a look without either is no way at all, and
/// asks for no frames.
fn moves(look: Option<Look>) -> bool {
    look.is_some_and(|look| {
        look.offset
            .is_some_and(|(x, y)| x.is_finite() && y.is_finite())
            || look.opacity.is_some_and(f32::is_finite)
    })
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
    /// The content that turned around on its way out: it comes back from
    /// the look it was leaving to.
    came_back: Option<u64>,
    /// A focus handle for each content on its way out, to tell whether the
    /// focus is inside it.
    focus: Vec<(u64, FocusHandle)>,
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
                came_back: None,
                focus: Vec::new(),
            });
            kept.slot.set_swap(self.swap);
            let now = kept.clock.now(cx);
            match content {
                Some(content) => {
                    let held = kept
                        .slot
                        .sample(now)
                        .into_iter()
                        .flatten()
                        .find(|shown| shown.content.content == content)
                        .map(|shown| (shown.content.number, shown.phase));
                    // Shown again on its way out: it turns around.
                    if let Some((number, Phase::Exiting)) = held {
                        kept.came_back = Some(number);
                    }
                    let number = held.map_or(kept.next, |(number, _)| number);
                    if held.is_none() {
                        kept.next += 1;
                    }
                    kept.slot.show(Numbered { number, content }, ways, now);
                }
                None => kept.slot.hide(ways.exit, now),
            }
            let shown = kept.slot.frame(now);
            if shown.iter().flatten().any(Shown::moving) {
                window.request_animation_frame();
            }
            let [first, second, here] = shown;
            if here
                .as_ref()
                .is_some_and(|here| here.phase == Phase::Present)
            {
                kept.came_back = None;
            }
            // While it leaves, the focus is taken from inside it.
            let leaving: Vec<u64> = [&first, &second, &here]
                .into_iter()
                .flatten()
                .filter(|shown| shown.phase == Phase::Exiting)
                .map(|shown| shown.content.number)
                .collect();
            kept.focus.retain(|(number, _)| leaving.contains(number));
            for number in &leaving {
                if !kept.focus.iter().any(|(held, _)| held == number) {
                    kept.focus.push((*number, cx.focus_handle()));
                }
            }
            if kept
                .focus
                .iter()
                .any(|(_, focus)| focus.contains_focused(window, cx))
            {
                window.defer(cx, Window::blur);
            }
            let came_back = kept.came_back;
            let focus = |number: u64| {
                kept.focus
                    .iter()
                    .find(|(held, _)| *held == number)
                    .map(|(_, focus)| focus.clone())
            };
            let drawn = |shown: &Shown<'_, Numbered<T>>, swapped_out: bool| {
                self.drawn(shown, swapped_out, came_back, focus(shown.content.number))
            };
            let leaving = [first, second]
                .into_iter()
                .flatten()
                .map(|shown| drawn(&shown, true));
            let here = here.map(|shown| drawn(&shown, false));
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

#[cfg(test)]
mod tests {
    use ginga::Color;

    use super::*;

    #[test]
    fn only_a_look_with_an_offset_or_an_opacity_is_a_way_that_moves() {
        assert!(moves(Some(Look::new().offset(0., 16.))));
        assert!(moves(Some(Look::new().opacity(0.))));
        assert!(!moves(Some(
            Look::new().width(0.).background(Color::TRANSPARENT)
        )));
        assert!(!moves(Some(Look::new().opacity(f32::NAN))));
        assert!(!moves(None));
    }
}
