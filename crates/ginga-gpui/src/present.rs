//! Ways in and out declared on an element: it arrives from one look, leaves
//! to another, and is drawn until its way out is over.

use ginga::{InOut, Look, Phase, Shown, Slot, Swap, Transition};
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, FocusHandle, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, ParentElement, Pixels, Styled, Window, div,
};

use crate::{
    Clock,
    look::{LookExt, WithLook},
    reduced,
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
        on_gone: None,
    }
}

/// What is called with content that was leaving, once it is gone.
type OnGone<T> = Box<dyn FnMut(T, &mut Window, &mut App)>;

/// An element with ways in and out: made by [`present`].
pub struct Present<T> {
    id: ElementId,
    content: Option<T>,
    ways: InOut,
    draw: Box<dyn Fn(&T) -> AnyElement>,
    arrive_from: Option<Look>,
    leave_to: Option<Look>,
    swap: Swap,
    on_gone: Option<OnGone<T>>,
}

impl<T: PartialEq + 'static> Present<T> {
    /// Calls `gone` with each content once it is gone, on the frame that
    /// finds it gone and after that frame is drawn: to drop what the view
    /// kept for it, or to move the focus. Content is gone when its way out
    /// is over, or when a fourth swap while two contents leave cuts the
    /// oldest short. A way out that is instant, or that reduced motion makes
    /// instant, is over on the frame that starts it. Content wanted again on
    /// its way out is not gone. Neither is content a [`Swap::Wait`] held back
    /// and dropped unseen, or content that was there when this element
    /// stopped being drawn.
    ///
    /// ```ignore
    /// let view = cx.entity().downgrade();
    /// present("toast", self.toast, ways, |toast| div().child(toast.text()))
    ///     .leave_to(Look::new().opacity(0.))
    ///     .on_gone(move |toast, _, cx| {
    ///         view.update(cx, |this, _| this.undo.remove(&toast)).ok();
    ///     })
    /// ```
    #[must_use]
    pub fn on_gone(mut self, gone: impl FnMut(T, &mut Window, &mut App) + 'static) -> Self {
        self.on_gone = Some(Box::new(gone));
        self
    }

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

    /// The ways in and out as they are used this frame: see [`way`].
    fn ways(&self, reduced: bool) -> Ways {
        let (arrive_from, enter) = way(self.arrive_from, self.ways.enter, reduced);
        let (leave_to, exit) = way(self.leave_to, self.ways.exit, reduced);
        // Content that turns around comes back from its leaving look, so
        // under reduced motion that look says whether the way back fades.
        let back = if reduced {
            way(self.leave_to, self.ways.enter, true).1
        } else {
            enter
        };
        Ways {
            arrive_from,
            leave_to,
            transitions: InOut { enter, exit },
            back,
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
        ways: &Ways,
    ) -> WithLook<gpui::Stateful<gpui::Div>> {
        let coming_back = came_back == Some(shown.content.number);
        let (arrive_from, leave_to) = (ways.arrive_from, ways.leave_to);
        let look = match shown.phase {
            Phase::Present => Look::new(),
            Phase::Entering if coming_back => {
                leave_to.map_or(Look::new(), |to| to.mix(Look::new(), shown.progress))
            }
            Phase::Entering => {
                arrive_from.map_or(Look::new(), |from| from.mix(Look::new(), shown.progress))
            }
            Phase::Exiting => {
                leave_to.map_or(Look::new(), |to| to.mix(Look::new(), shown.progress))
            }
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
        drawn.with_look(look)
    }
}

/// The ways in and out of one frame: their looks and their transitions.
struct Ways {
    arrive_from: Option<Look>,
    leave_to: Option<Look>,
    transitions: InOut,
    /// The way in for content that turns around on its way out.
    back: Transition,
}

/// A way in or out as it is used this frame: its look and its transition.
/// A look that moves nothing is no way, and is instant. Under reduced
/// motion a way that fades keeps its transition and loses its offset, so it
/// fades where it is laid out; a way that only moves keeps its look and has
/// no motion, so it waits there for its delay. The one place that says what
/// a way is under reduced motion.
fn way(look: Option<Look>, transition: Transition, reduced: bool) -> (Option<Look>, Transition) {
    match look {
        _ if !moves(look) => (look, Transition::Instant),
        Some(look) if reduced && fades(look) => (
            Some(Look {
                offset: None,
                ..look
            }),
            transition,
        ),
        _ if reduced => (look, transition.without_motion()),
        _ => (look, transition),
    }
}

/// A way in or out with this look fades: it names an opacity that is not
/// the full one content is at when it is there.
fn fades(look: Look) -> bool {
    look.opacity
        .is_some_and(|opacity| opacity.is_finite() && opacity != 1.0)
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

impl<T: PartialEq> Kept<T> {
    /// Shows `content`, numbered as it was if it is held.
    fn show(&mut self, content: T, ways: &Ways, now: std::time::Duration) {
        let held = self
            .slot
            .sample(now)
            .into_iter()
            .flatten()
            .find(|shown| shown.content.content == content)
            .map(|shown| (shown.content.number, shown.phase));
        // Shown again on its way out: it turns around.
        let mut transitions = ways.transitions;
        if let Some((number, Phase::Exiting)) = held {
            self.came_back = Some(number);
            transitions.enter = ways.back;
        }
        // Held content keeps the one it has: the slot does not replace it.
        let number = self.next;
        if held.is_none() {
            self.next += 1;
        }
        self.slot
            .show(Numbered { number, content }, transitions, now);
    }
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
        let reduced = reduced(cx);
        let ways = self.ways(reduced);
        let transitions = ways.transitions;
        let content = self.content.take();
        let (mut element, gone) = window.with_element_state(id, |kept: Option<Kept<T>>, window| {
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
                Some(content) => kept.show(content, &ways, now),
                None => kept.slot.hide(transitions.exit, now),
            }
            // Empty, and so not allocated, unless something is gone.
            let mut gone = Vec::new();
            gone.extend(core::iter::from_fn(|| kept.slot.take_gone()));
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
                self.drawn(
                    shown,
                    swapped_out,
                    came_back,
                    focus(shown.content.number),
                    &ways,
                )
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
            gone.extend(core::iter::from_fn(|| kept.slot.take_gone()));
            ((element, gone), kept)
        });
        let layout = element.request_layout(window, cx);
        // After the frame, so what it changes draws the next one; and after
        // what the contents inside defer, so a focus it moves stays moved.
        if let Some(mut on_gone) = self.on_gone.take().filter(|_| !gone.is_empty()) {
            window.defer(cx, move |window, cx| {
                for gone in gone {
                    on_gone(gone.content, window, cx);
                }
            });
        }
        (layout, element)
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

    /// `Transition` has no `PartialEq`; its debug form names every part.
    fn assert_same(seen: (Option<Look>, Transition), wanted: (Option<Look>, Transition)) {
        assert_eq!(format!("{seen:?}"), format!("{wanted:?}"));
    }

    #[test]
    fn under_reduced_motion_a_way_that_fades_keeps_its_opacity_and_transition_and_loses_its_offset()
    {
        let fade = Transition::from(ginga::Tween::new(
            core::time::Duration::from_millis(100),
            ginga::Easing::Linear,
        ));
        let rise = Look::new().offset(0., 40.).opacity(0.);
        assert_same(way(Some(rise), fade, false), (Some(rise), fade));
        assert_same(
            way(Some(rise), fade, true),
            (Some(Look::new().opacity(0.)), fade),
        );
    }

    #[test]
    fn under_reduced_motion_a_way_that_only_moves_or_names_full_opacity_has_no_motion() {
        let fade = Transition::from(ginga::Tween::new(
            core::time::Duration::from_millis(100),
            ginga::Easing::Linear,
        ));
        for look in [
            Look::new().offset(0., 40.),
            Look::new().offset(0., 40.).opacity(1.),
        ] {
            assert_same(
                way(Some(look), fade, true),
                (Some(look), fade.without_motion()),
            );
        }
        assert_same(way(None, fade, true), (None, Transition::Instant));
    }
}
