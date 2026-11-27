//! Looks for a moment: while the pointer is over an element, while it is
//! pressed and while it has the focus, checked on GPUI's test platform.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use ginga_gpui::{Easing, Look, LookExt, LookState, Tween};
use gpui::{
    Bounds, Context, FocusHandle, InteractiveElement, IntoElement, Modifiers, MouseButton,
    MouseExitEvent, ParentElement, Pixels, Render, StatefulInteractiveElement, Styled,
    TestAppContext, VisualTestContext, Window, canvas, div, point, px,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// The bounds the button was laid out at, one entry per frame drawn.
type Drawn = Rc<RefCell<Vec<Bounds<Pixels>>>>;

/// A 100 px button, 150 px under the pointer, 200 px pressed and 120 px
/// with the focus, each on a 100 ms linear tween so the widths are exact.
/// `shift` moves it down; a 10 px corner inside it stops every press.
struct Button {
    focus: FocusHandle,
    clicks: Rc<Cell<u32>>,
    drawn: Drawn,
    owned: Option<LookState>,
    shift: Rc<Cell<f32>>,
}

impl Render for Button {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (drawn, clicks) = (self.drawn.clone(), self.clicks.clone());
        let button = div()
            .id("button")
            .relative()
            .h(px(40.))
            .track_focus(&self.focus)
            .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
            .child(
                canvas(
                    move |bounds, _, _| drawn.borrow_mut().push(bounds),
                    |_, (), _, _| {},
                )
                .size_full(),
            )
            .child(
                div()
                    .id("corner")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size(px(10.))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            );
        let transition = Tween::new(ms(100), Easing::Linear);
        let button = match &self.owned {
            Some(state) => button.look_with(state, Look::new().width(100.), transition),
            None => button.look("button", Look::new().width(100.), transition),
        };
        div().size_full().pt(px(self.shift.get())).child(
            button
                .hover(Look::new().width(150.))
                .pressed(Look::new().width(200.))
                .focused(&self.focus, Look::new().width(120.)),
        )
    }
}

struct Opened<'a> {
    cx: &'a mut VisualTestContext,
    focus: FocusHandle,
    clicks: Rc<Cell<u32>>,
    drawn: Drawn,
    shift: Rc<Cell<f32>>,
}

fn open(cx: &mut TestAppContext) -> Opened<'_> {
    open_as(cx, false)
}

fn open_as(cx: &mut TestAppContext, owned: bool) -> Opened<'_> {
    let (drawn, clicks) = (Drawn::default(), Rc::new(Cell::new(0)));
    let shift = Rc::new(Cell::new(0.));
    let focus = cx.update(|cx| cx.focus_handle());
    let (_, cx) = cx.add_window_view({
        let (drawn, clicks, focus, shift) =
            (drawn.clone(), clicks.clone(), focus.clone(), shift.clone());
        move |_, _| Button {
            focus,
            clicks,
            drawn,
            owned: owned.then(LookState::default),
            shift,
        }
    });
    cx.run_until_parked();
    Opened {
        cx,
        focus,
        clicks,
        drawn,
        shift,
    }
}

impl Opened<'_> {
    /// The button's width after the clock moves `millis` and a frame is drawn.
    fn width_after(&mut self, millis: u64) -> Pixels {
        self.cx.executor().advance_clock(ms(millis));
        self.cx.update(Window::simulate_next_frame);
        self.cx.run_until_parked();
        self.drawn.borrow().last().unwrap().size.width
    }

    fn move_to(&mut self, (x, y): (f32, f32), button: Option<MouseButton>) {
        self.cx
            .simulate_mouse_move(point(px(x), px(y)), button, Modifiers::none());
    }
}

const OVER: (f32, f32) = (20., 20.);
const AWAY: (f32, f32) = (300., 300.);

#[gpui::test]
fn the_hover_look_applies_while_the_pointer_is_over_and_is_left_when_it_goes(
    cx: &mut TestAppContext,
) {
    hover_comes_and_goes(open(cx));
}

#[gpui::test]
fn the_hover_look_works_with_a_look_state_the_view_owns(cx: &mut TestAppContext) {
    hover_comes_and_goes(open_as(cx, true));
}

fn hover_comes_and_goes(mut button: Opened<'_>) {
    assert_eq!(button.width_after(16), px(100.));
    button
        .cx
        .simulate_mouse_move(point(px(OVER.0), px(OVER.1)), None, Modifiers::none());
    assert_eq!(
        button.width_after(50),
        px(125.),
        "halfway to the hover look"
    );
    assert_eq!(button.width_after(50), px(150.));
    button
        .cx
        .simulate_mouse_move(point(px(AWAY.0), px(AWAY.1)), None, Modifiers::none());
    assert_eq!(button.width_after(100), px(100.), "back to its own look");
}

#[gpui::test]
fn a_pointer_that_crosses_quickly_turns_the_motion_around_without_a_jump(cx: &mut TestAppContext) {
    let mut button = open(cx);
    button
        .cx
        .simulate_mouse_move(point(px(OVER.0), px(OVER.1)), None, Modifiers::none());
    assert_eq!(button.width_after(40), px(120.));
    button
        .cx
        .simulate_mouse_move(point(px(AWAY.0), px(AWAY.1)), None, Modifiers::none());
    assert_eq!(button.width_after(0), px(120.), "it turns from where it is");
    assert_eq!(button.width_after(50), px(110.), "and heads back from 120");
}

#[gpui::test]
fn the_pressed_look_wins_and_a_release_over_the_element_returns_to_hover(cx: &mut TestAppContext) {
    let mut button = open(cx);
    let over = point(px(OVER.0), px(OVER.1));
    button.cx.simulate_mouse_move(over, None, Modifiers::none());
    assert_eq!(button.width_after(100), px(150.));
    button
        .cx
        .simulate_mouse_down(over, MouseButton::Left, Modifiers::none());
    assert_eq!(button.width_after(100), px(200.), "pressed wins over hover");
    button
        .cx
        .simulate_mouse_up(over, MouseButton::Left, Modifiers::none());
    assert_eq!(button.width_after(100), px(150.), "released over it: hover");
    assert_eq!(button.clicks.get(), 1, "its own click handler ran");
    button
        .cx
        .simulate_mouse_down(over, MouseButton::Right, Modifiers::none());
    assert_eq!(button.width_after(100), px(150.), "only the left button");
}

#[gpui::test]
fn a_release_away_from_the_element_ends_the_press(cx: &mut TestAppContext) {
    let mut button = open(cx);
    let over = point(px(OVER.0), px(OVER.1));
    button.move_to(OVER, None);
    button
        .cx
        .simulate_mouse_down(over, MouseButton::Left, Modifiers::none());
    assert_eq!(button.width_after(100), px(200.));
    button.move_to(AWAY, Some(MouseButton::Left));
    assert_eq!(button.width_after(100), px(200.), "still held away from it");
    button.cx.simulate_mouse_up(
        point(px(AWAY.0), px(AWAY.1)),
        MouseButton::Left,
        Modifiers::none(),
    );
    assert_eq!(
        button.width_after(100),
        px(120.),
        "the press gave it the focus, and that look stays"
    );
    assert_eq!(button.clicks.get(), 0);
}

#[gpui::test]
fn a_release_lost_outside_the_window_ends_the_press_at_the_next_move(cx: &mut TestAppContext) {
    let mut button = open(cx);
    let over = point(px(OVER.0), px(OVER.1));
    button.move_to(OVER, None);
    button
        .cx
        .simulate_mouse_down(over, MouseButton::Left, Modifiers::none());
    assert_eq!(button.width_after(100), px(200.));
    button.move_to(AWAY, None);
    assert_eq!(
        button.width_after(100),
        px(120.),
        "no button held: released"
    );
}

#[gpui::test]
fn the_pointer_leaving_the_window_ends_the_hover(cx: &mut TestAppContext) {
    let mut button = open(cx);
    button.move_to(OVER, None);
    assert_eq!(button.width_after(100), px(150.));
    button.cx.simulate_event(MouseExitEvent {
        position: point(px(OVER.0), px(OVER.1)),
        pressed_button: None,
        modifiers: Modifiers::none(),
    });
    assert_eq!(button.width_after(100), px(100.));
    assert_eq!(
        button.width_after(100),
        px(100.),
        "the last place the pointer was seen does not bring it back"
    );
    button.move_to(OVER, None);
    assert_eq!(button.width_after(100), px(150.), "back in: hovered again");
}

#[gpui::test]
fn an_element_that_moves_from_under_a_still_pointer_is_no_longer_hovered(cx: &mut TestAppContext) {
    let mut button = open(cx);
    button.move_to(OVER, None);
    assert_eq!(button.width_after(100), px(150.));
    button.shift.set(200.);
    button.cx.update(|window, _| window.refresh());
    assert_eq!(button.width_after(0), px(150.), "the frame it moves in");
    assert_eq!(
        button.width_after(100),
        px(100.),
        "then it leaves the hover"
    );
}

#[gpui::test]
fn a_child_that_takes_the_press_keeps_the_element_from_being_pressed(cx: &mut TestAppContext) {
    let mut button = open(cx);
    let corner = point(px(4.), px(4.));
    button.move_to((4., 4.), None);
    assert_eq!(button.width_after(100), px(150.));
    button
        .cx
        .simulate_mouse_down(corner, MouseButton::Left, Modifiers::none());
    assert_eq!(button.width_after(100), px(150.), "not pressed");
}

#[gpui::test]
fn hover_wins_over_focused(cx: &mut TestAppContext) {
    let mut button = open(cx);
    let focus = button.focus.clone();
    button.cx.update(|window, cx| window.focus(&focus, cx));
    assert_eq!(button.width_after(100), px(120.));
    button.move_to(OVER, None);
    assert_eq!(button.width_after(100), px(150.));
    button.move_to(AWAY, None);
    assert_eq!(button.width_after(100), px(120.), "focused again");
}

#[gpui::test]
fn the_focus_look_applies_while_the_element_has_the_keyboard_focus(cx: &mut TestAppContext) {
    let mut button = open(cx);
    let focus = button.focus.clone();
    button.cx.update(|window, cx| window.focus(&focus, cx));
    assert_eq!(button.width_after(100), px(120.));
    button.cx.update(Window::blur);
    assert_eq!(button.width_after(100), px(100.));
}
