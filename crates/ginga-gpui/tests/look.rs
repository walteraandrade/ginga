//! A look declared on an element, checked on GPUI's test platform: the
//! element's laid-out bounds, frame by frame.

use std::{cell::RefCell, rc::Rc, time::Duration};

use ginga_gpui::{Easing, Look, LookExt, LookState, Trace, Transitions, Tween};
use gpui::{
    Bounds, Context, IntoElement, ParentElement, Pixels, Render, Styled, TestAppContext, Window,
    WindowHandle, canvas, div, point, px, size,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// A linear tween, so the values are exact.
fn glide() -> Tween {
    Tween::new(ms(200), Easing::Linear)
}

/// The bounds the card was laid out at, one entry per frame drawn.
type Drawn = Rc<RefCell<Vec<Bounds<Pixels>>>>;

/// A view with one card whose look is declared on it.
struct Cards {
    look: Look,
    from: Option<Look>,
    /// Keep the motion in the view instead of with the card's id.
    owned: Option<LookState>,
    shown: bool,
    transitions: Transitions,
    drawn: Drawn,
}

impl Render for Cards {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let drawn = self.drawn.clone();
        let card = div().w(px(10.)).h(px(30.)).child(
            canvas(
                move |bounds, _, _| drawn.borrow_mut().push(bounds),
                |_, (), _, _| {},
            )
            .size_full(),
        );
        let card = match &self.owned {
            Some(state) => card.look_with(state, self.look, self.transitions),
            None => card.look("card", self.look, self.transitions),
        };
        let card = match self.from {
            Some(from) => card.from(from),
            None => card,
        };
        div().children(self.shown.then_some(card))
    }
}

fn open(
    cx: &mut TestAppContext,
    look: Look,
    from: Option<Look>,
    owned: bool,
) -> (WindowHandle<Cards>, Drawn) {
    let drawn = Drawn::default();
    let window = cx.open_window(size(px(400.), px(300.)), {
        let drawn = drawn.clone();
        move |_, _| Cards {
            look,
            from,
            owned: owned.then(LookState::default),
            shown: true,
            transitions: glide().into(),
            drawn,
        }
    });
    cx.run_until_parked();
    (window, drawn)
}

fn change(window: &WindowHandle<Cards>, cx: &mut TestAppContext, f: impl FnOnce(&mut Cards)) {
    window
        .update(cx, |cards, _, cx| {
            f(cards);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
}

/// Moves the clock by `millis`, delivers the next frame, and returns how many
/// frame requests it answered.
fn after(millis: u64, window: &WindowHandle<Cards>, cx: &mut TestAppContext) -> usize {
    cx.executor().advance_clock(ms(millis));
    let requests = window
        .update(cx, |_, window, cx| window.simulate_next_frame(cx))
        .unwrap();
    cx.run_until_parked();
    requests
}

fn last(drawn: &Drawn) -> Bounds<Pixels> {
    *drawn.borrow().last().unwrap()
}

#[gpui::test]
fn a_change_of_look_animates_and_settles_on_it(cx: &mut TestAppContext) {
    let (window, drawn) = open(cx, Look::new().width(100.), None, false);
    assert_eq!(
        last(&drawn).size.width,
        px(100.),
        "the look replaces the element's own width"
    );

    change(&window, cx, |cards| cards.look = Look::new().width(200.));
    assert_eq!(after(50, &window, cx), 1, "a moving look asks for a frame");
    assert_eq!(last(&drawn).size.width, px(125.), "at 50 ms");
    assert_eq!(after(150, &window, cx), 1);
    assert_eq!(
        last(&drawn).size.width,
        px(200.),
        "settled on the look, at 200 ms"
    );

    let frames = drawn.borrow().len();
    assert_eq!(
        after(16, &window, cx),
        0,
        "a settled look asks for no frame"
    );
    assert_eq!(drawn.borrow().len(), frames, "and nothing is drawn again");
}

#[gpui::test]
fn a_value_the_look_does_not_name_is_the_elements_own(cx: &mut TestAppContext) {
    let (window, drawn) = open(cx, Look::new().width(100.).offset(0., 0.), None, false);
    change(&window, cx, |cards| {
        cards.look = Look::new().width(100.).offset(40., 20.);
    });
    after(100, &window, cx);
    let bounds = last(&drawn);
    assert_eq!(
        bounds.origin,
        point(px(20.), px(10.)),
        "moved halfway from its own place"
    );
    assert_eq!(
        bounds.size,
        size(px(100.), px(30.)),
        "the height is the element's own 30"
    );
}

#[gpui::test]
fn a_change_in_the_middle_does_not_jump(cx: &mut TestAppContext) {
    let (window, drawn) = open(cx, Look::new().width(100.), None, false);
    change(&window, cx, |cards| cards.look = Look::new().width(200.));
    after(100, &window, cx);
    assert_eq!(last(&drawn).size.width, px(150.));
    change(&window, cx, |cards| cards.look = Look::new().width(0.));
    assert_eq!(
        last(&drawn).size.width,
        px(150.),
        "it turns around from where it is"
    );
    after(100, &window, cx);
    assert_eq!(last(&drawn).size.width, px(75.), "and heads for 0 from 150");
}

#[gpui::test]
fn on_first_appearance_it_takes_its_look_unless_it_starts_from_one(cx: &mut TestAppContext) {
    let (window, drawn) = open(cx, Look::new().width(200.), None, false);
    assert_eq!(last(&drawn).size.width, px(200.));
    assert_eq!(after(16, &window, cx), 0, "nothing to animate");

    let (window, drawn) = open(
        cx,
        Look::new().width(200.),
        Some(Look::new().width(0.)),
        false,
    );
    assert_eq!(
        last(&drawn).size.width,
        px(0.),
        "it starts from the given look"
    );
    after(100, &window, cx);
    assert_eq!(last(&drawn).size.width, px(100.), "and animates in");
}

#[gpui::test]
fn the_apps_default_transitions_fill_in_what_the_element_leaves_out(cx: &mut TestAppContext) {
    cx.update(|cx| {
        ginga_gpui::set_default_look_transitions(Tween::new(ms(100), Easing::Linear), cx);
    });
    let (window, drawn) = open(cx, Look::new().size(100., 100.), None, false);
    change(&window, cx, |cards| {
        cards.transitions = Transitions::new().height(glide());
        cards.look = Look::new().size(200., 200.);
    });
    after(50, &window, cx);
    assert_eq!(
        last(&drawn).size,
        size(px(150.), px(125.)),
        "the width on the app's 100 ms, the height on its own 200 ms"
    );
}

#[gpui::test]
fn under_reduced_motion_it_takes_each_look_at_once(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (window, drawn) = open(cx, Look::new().width(100.), None, false);
    change(&window, cx, |cards| cards.look = Look::new().width(200.));
    assert_eq!(last(&drawn).size.width, px(200.));
    assert_eq!(after(16, &window, cx), 0);
}

#[gpui::test]
fn under_reduced_motion_a_values_own_transition_moves_at_once_too(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_reduce_motion(true);
        ginga_gpui::set_default_look_transitions(Tween::new(ms(100), Easing::Linear), cx);
    });
    let (window, drawn) = open(cx, Look::new().size(100., 100.), None, false);
    change(&window, cx, |cards| {
        cards.transitions = Transitions::new().height(glide());
        cards.look = Look::new().size(200., 200.);
    });
    assert_eq!(last(&drawn).size, size(px(200.), px(200.)));
    assert_eq!(after(16, &window, cx), 0);
}

#[gpui::test]
fn a_motion_the_view_owns_outlasts_a_frame_without_the_element(cx: &mut TestAppContext) {
    let (window, drawn) = open(cx, Look::new().width(100.), None, true);
    change(&window, cx, |cards| cards.look = Look::new().width(200.));
    after(50, &window, cx);
    change(&window, cx, |cards| cards.shown = false);
    after(50, &window, cx);
    change(&window, cx, |cards| cards.shown = true);
    assert_eq!(
        last(&drawn).size.width,
        px(150.),
        "back where the motion is at 100 ms, not at its look at once"
    );
}

#[gpui::test]
fn a_motion_kept_with_the_id_ends_when_the_element_is_not_drawn(cx: &mut TestAppContext) {
    let (window, drawn) = open(cx, Look::new().width(100.), None, false);
    change(&window, cx, |cards| cards.look = Look::new().width(200.));
    after(50, &window, cx);
    change(&window, cx, |cards| cards.shown = false);
    after(50, &window, cx);
    change(&window, cx, |cards| cards.shown = true);
    assert_eq!(
        last(&drawn).size.width,
        px(200.),
        "it appears again, taking its look at once"
    );
}

#[gpui::test]
fn under_reduced_motion_a_delay_is_kept_and_then_the_look_is_taken_at_once(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (window, drawn) = open(cx, Look::new().width(100.), None, false);
    change(&window, cx, |cards| {
        cards.transitions = Tween::new(ms(260), Easing::OutExpo).delay(ms(160)).into();
        cards.look = Look::new().width(200.);
    });
    assert_eq!(last(&drawn).size.width, px(100.), "it holds its start look");
    after(150, &window, cx);
    assert_eq!(last(&drawn).size.width, px(100.), "still holding at 150 ms");
    assert_eq!(after(10, &window, cx), 1, "a frame for the end of the wait");
    assert_eq!(
        last(&drawn).size.width,
        px(200.),
        "at rest at once at 160 ms"
    );
    assert_eq!(after(16, &window, cx), 0, "and settled");
}

#[gpui::test]
fn a_look_the_view_owns_can_be_written_down(cx: &mut TestAppContext) {
    let (window, drawn) = open(cx, Look::new().width(100.), None, true);
    let state = window
        .update(cx, |cards, _, _| cards.owned.clone().unwrap())
        .unwrap();
    let (motion, now) = cx.update(|cx| state.inspect(cx)).unwrap();
    assert_eq!(now, ms(0), "the first frame is the clock's start");
    assert_eq!(motion.target(), Look::new().width(100.));

    change(&window, cx, |cards| cards.look = Look::new().width(200.));
    after(50, &window, cx);
    let (motion, now) = cx.update(|cx| state.inspect(cx)).unwrap();
    assert_eq!(now, ms(50));
    assert_eq!(
        motion.sample(now).value.width,
        Some(last(&drawn).size.width.into()),
        "what it reads is what was drawn"
    );
    assert_eq!(
        Trace::of(&motion, now, ms(50)).to_string(),
        "ms           width\n\
         50         125.000\n\
         100        150.000\n\
         150        175.000\n\
         200        200.000  settled\n"
    );

    let frames = drawn.borrow().len();
    let _ = cx.update(|cx| state.inspect(cx));
    cx.run_until_parked();
    assert_eq!(drawn.borrow().len(), frames, "reading it draws nothing");
}

#[gpui::test]
fn a_look_state_no_element_drew_has_nothing_to_write_down(cx: &mut TestAppContext) {
    let state = LookState::default();
    assert!(cx.update(|cx| state.inspect(cx)).is_none());
}
