//! A drag, checked on GPUI's test platform: the test moves the pointer and
//! the clock, and delivers the frames.

use std::time::Duration;

use ginga_gpui::{Drag, DragExt, DragState};
use gpui::{
    Axis, Context, IntoElement, Modifiers, MouseButton, ParentElement, Pixels, Point, Render,
    Styled, TestAppContext, VisualTestContext, Window, div, point, px,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

fn at(x: f32, y: f32) -> Point<Pixels> {
    point(px(x), px(y))
}

/// A 100 px card at the window's corner, that the pointer drags.
struct Board {
    card: DragState,
    drag: Drag,
}

impl Render for Board {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            div()
                .size(px(100.))
                .draggable(&self.card, self.drag.clone()),
        )
    }
}

fn open_board(drag: Drag, cx: &mut TestAppContext) -> (DragState, &mut VisualTestContext) {
    let card = DragState::default();
    let (_, cx) = cx.add_window_view({
        let card = card.clone();
        move |_, _| Board { card, drag }
    });
    cx.run_until_parked();
    (card, cx)
}

/// Delivers the next frame and returns how many frame requests it answered.
fn next_frame(cx: &mut VisualTestContext) -> usize {
    let requests = cx.update(Window::simulate_next_frame);
    cx.run_until_parked();
    requests
}

/// Delivers frames 16 ms apart until none is asked for, and returns how many.
fn frames_until_still(cx: &mut VisualTestContext) -> usize {
    let mut frames = 0;
    loop {
        cx.executor().advance_clock(ms(16));
        if next_frame(cx) == 0 {
            return frames;
        }
        frames += 1;
        assert!(frames < 500, "it settles");
    }
}

fn press(cx: &mut VisualTestContext, pointer: Point<Pixels>) {
    cx.simulate_mouse_down(pointer, MouseButton::Left, Modifiers::none());
}

/// Moves the pointer, held down, to `pointer` after `after`.
fn move_to(cx: &mut VisualTestContext, pointer: Point<Pixels>, after: Duration) {
    cx.executor().advance_clock(after);
    cx.simulate_mouse_move(pointer, MouseButton::Left, Modifiers::none());
}

fn release(cx: &mut VisualTestContext, pointer: Point<Pixels>) {
    cx.simulate_mouse_up(pointer, MouseButton::Left, Modifiers::none());
}

/// Drags from `from` by `step` every 10 ms, `steps` times: 100 × `step` px/s.
fn throw(cx: &mut VisualTestContext, from: Point<Pixels>, step: Point<Pixels>, steps: u8) {
    press(cx, from);
    let mut pointer = from;
    for _ in 0..steps {
        pointer = point(pointer.x + step.x, pointer.y + step.y);
        move_to(cx, pointer, ms(10));
    }
    release(cx, pointer);
}

#[gpui::test]
fn a_dragged_element_stays_exactly_under_the_pointer(cx: &mut TestAppContext) {
    let (card, cx) = open_board(Drag::new(), cx);
    press(cx, at(50., 50.));
    assert!(card.is_held());
    move_to(cx, at(80., 70.), ms(16));
    assert_eq!(card.offset(), at(30., 20.));
    move_to(cx, at(300., 260.), ms(16));
    assert_eq!(card.offset(), at(250., 210.));
    // Held, it asks for no frames of its own.
    assert_eq!(next_frame(cx), 0);
    // Let go of a still pointer, it stays where it was set down.
    cx.executor().advance_clock(ms(200));
    release(cx, at(300., 260.));
    assert!(!card.is_held());
    assert_eq!(card.rest(), at(250., 210.));
    assert_eq!(frames_until_still(cx), 0);
    // It is drawn there: a press where it now is takes it again, and a press
    // where it was does not.
    press(cx, at(20., 20.));
    assert!(!card.is_held());
    release(cx, at(20., 20.));
    press(cx, at(300., 260.));
    assert!(card.is_held());
}

#[gpui::test]
fn a_released_element_carries_the_pointers_speed_and_coasts_to_a_stop(cx: &mut TestAppContext) {
    let (card, cx) = open_board(Drag::new(), cx);
    // 1,000 px/s right and 500 px/s down: with QUICK's 0.1 s, it rests 100
    // and 50 px on from where it was let go.
    throw(cx, at(50., 50.), at(10., 5.), 5);
    assert_eq!(card.rest(), at(150., 75.));
    next_frame(cx);
    assert_eq!(
        card.offset(),
        at(50., 25.),
        "it leaves from where it was let go"
    );
    cx.executor().advance_clock(ms(100));
    next_frame(cx);
    // One time constant on: 1 − 1/e of the way, 63.2 and 31.6 px.
    let one_on = card.offset();
    assert!((f32::from(one_on.x) - 113.2).abs() < 0.1, "{one_on:?}");
    assert!((f32::from(one_on.y) - 56.6).abs() < 0.1, "{one_on:?}");
    let frames = frames_until_still(cx);
    assert!(frames > 10, "it glides there over frames: {frames}");
    assert_eq!(card.offset(), at(150., 75.));
}

#[gpui::test]
fn a_drag_along_an_axis_leaves_the_other_out(cx: &mut TestAppContext) {
    let (card, cx) = open_board(Drag::new().along(Axis::Horizontal), cx);
    press(cx, at(50., 50.));
    move_to(cx, at(90., 80.), ms(16));
    assert_eq!(card.offset(), at(40., 0.));
    // Pressed again without a release, it goes on from where it is held:
    // 40 + 50, and 100 on from there.
    throw(cx, at(90., 80.), at(10., 10.), 5);
    assert_eq!(card.rest(), at(190., 0.));
}

#[gpui::test]
fn an_element_let_go_past_a_limit_springs_back_inside_it(cx: &mut TestAppContext) {
    let track = Drag::new()
        .along(Axis::Horizontal)
        .within(at(0., 0.), at(200., 0.));
    let (card, cx) = open_board(track, cx);
    // Dragged 60 past the end and let go still.
    press(cx, at(50., 50.));
    move_to(cx, at(310., 50.), ms(16));
    assert_eq!(
        card.offset(),
        at(260., 0.),
        "past the limit, still under the pointer"
    );
    cx.executor().advance_clock(ms(200));
    release(cx, at(310., 50.));
    assert_eq!(card.rest(), at(200., 0.));
    assert!(frames_until_still(cx) > 0);
    assert_eq!(card.offset(), at(200., 0.));
    // Thrown hard toward the start, it stops there too.
    throw(cx, at(250., 50.), at(-20., 0.), 5);
    assert_eq!(card.rest(), at(0., 0.));
    frames_until_still(cx);
    assert_eq!(card.offset(), at(0., 0.));
}

#[gpui::test]
fn with_snap_points_it_comes_to_rest_on_the_nearest(cx: &mut TestAppContext) {
    let places = Drag::new().snap_to([at(0., 0.), at(240., 0.), at(480., 0.)]);
    let (card, cx) = open_board(places, cx);
    // It would coast to 150: 240 is nearer than 0.
    throw(cx, at(50., 50.), at(10., 0.), 5);
    assert_eq!(card.rest(), at(240., 0.));
    frames_until_still(cx);
    assert_eq!(card.offset(), at(240., 0.));
    // Nudged back a little and let go still, it returns to 240.
    press(cx, at(290., 50.));
    move_to(cx, at(250., 50.), ms(16));
    cx.executor().advance_clock(ms(200));
    release(cx, at(250., 50.));
    assert_eq!(card.rest(), at(240., 0.));
    frames_until_still(cx);
    assert_eq!(card.offset(), at(240., 0.));
}

#[gpui::test]
fn under_reduced_motion_a_thrown_element_is_at_rest_at_once(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (card, cx) = open_board(Drag::new(), cx);
    throw(cx, at(50., 50.), at(10., 0.), 5);
    assert_eq!(card.rest(), at(150., 0.));
    assert_eq!(next_frame(cx), 0, "no frames asked for");
    assert_eq!(card.offset(), at(150., 0.));
}

#[gpui::test]
fn a_throw_without_a_pointer_moves_as_a_pointers_would(cx: &mut TestAppContext) {
    let tray = Drag::new().within(at(0., 0.), at(300., 300.));
    let (card, cx) = open_board(tray.clone(), cx);
    cx.update(|window, cx| {
        card.throw(at(1_000., 6_000.), &tray, cx);
        window.refresh();
    });
    // 100 on, and down past the tray's floor: back to 300.
    assert_eq!(card.rest(), at(100., 300.));
    assert!(frames_until_still(cx) > 10);
    assert_eq!(card.offset(), at(100., 300.));
}
