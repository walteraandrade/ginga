//! Keyframes in a view, on GPUI's test platform: a clock the test moves,
//! frames the test delivers, and no display.

use std::{cell::RefCell, rc::Rc, time::Duration};

use ginga_gpui::{Easing, Keyframes, Stop};
use gpui::{
    Context, IntoElement, Render, Styled, TestAppContext, Window, WindowHandle, div, px, size,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// 0 to 100 and back to 0 in 200 ms, linear, so the values are exact.
fn up_and_down() -> Keyframes<f32, 3> {
    Keyframes::new([
        Stop::new(ms(0), 0.0, Easing::Linear),
        Stop::new(ms(100), 100.0, Easing::Linear),
        Stop::new(ms(200), 0.0, Easing::Linear),
    ])
    .unwrap()
}

/// A view that draws keyframes and writes down each value it drew.
struct Pulse {
    keyframes: Keyframes<f32, 3>,
    drawn: Rc<RefCell<Vec<f32>>>,
}

impl Render for Pulse {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let value = self.keyframes.frame(window, cx);
        self.drawn.borrow_mut().push(value);
        div().w(px(value))
    }
}

#[expect(
    clippy::large_types_passed_by_value,
    reason = "the view keeps the keyframes, so they are moved in either way"
)]
fn open(
    keyframes: Keyframes<f32, 3>,
    cx: &mut TestAppContext,
) -> (WindowHandle<Pulse>, Rc<RefCell<Vec<f32>>>) {
    let drawn = Rc::new(RefCell::new(Vec::new()));
    let window = cx.open_window(size(px(400.), px(300.)), {
        let drawn = drawn.clone();
        move |_, _| Pulse { keyframes, drawn }
    });
    cx.run_until_parked();
    (window, drawn)
}

/// Delivers the next frame, as the platform would, and returns how many frame
/// requests it answered.
fn next_frame(window: &WindowHandle<Pulse>, cx: &mut TestAppContext) -> usize {
    let requests = window
        .update(cx, |_, window, cx| window.simulate_next_frame(cx))
        .unwrap();
    cx.run_until_parked();
    requests
}

fn last(drawn: &Rc<RefCell<Vec<f32>>>) -> f32 {
    *drawn.borrow().last().unwrap()
}

#[gpui::test]
fn keyframes_draw_each_frames_value_and_stop_asking_once_played(cx: &mut TestAppContext) {
    let (window, drawn) = open(up_and_down().times(2), cx);
    assert_eq!(last(&drawn), 0.0);

    cx.executor().advance_clock(ms(50));
    assert_eq!(
        next_frame(&window, cx),
        1,
        "moving keyframes ask for a frame"
    );
    assert_eq!(last(&drawn), 50.0);

    cx.executor().advance_clock(ms(100));
    assert_eq!(next_frame(&window, cx), 1);
    assert_eq!(last(&drawn), 50.0, "at 150 ms, on the way down");

    cx.executor().advance_clock(ms(100));
    assert_eq!(next_frame(&window, cx), 1);
    assert_eq!(last(&drawn), 50.0, "at 250 ms, the second round");

    cx.executor().advance_clock(ms(200));
    assert_eq!(next_frame(&window, cx), 1);
    assert_eq!(last(&drawn), 0.0, "played twice, on the last stop");
    assert_eq!(next_frame(&window, cx), 0, "and asking for no more frames");
}

/// 0 to 128 and back in 256 ms, linear: every 32 ms is a value exact in `f32`.
fn ramp() -> Keyframes<f32, 3> {
    Keyframes::new([
        Stop::new(ms(0), 0.0, Easing::Linear),
        Stop::new(ms(128), 128.0, Easing::Linear),
        Stop::new(ms(256), 0.0, Easing::Linear),
    ])
    .unwrap()
}

/// Moves the clock by `millis` and runs what is due, timers included.
fn wait(millis: u64, cx: &mut TestAppContext) {
    cx.executor().advance_clock(ms(millis));
    cx.run_until_parked();
}

#[gpui::test]
fn a_capped_loop_asks_for_a_frame_no_sooner_than_its_cap(cx: &mut TestAppContext) {
    let (window, drawn) = open(ramp().forever().at_most_every(ms(32)), cx);
    let renders = || drawn.borrow().len();
    assert_eq!(renders(), 1);
    assert_eq!(next_frame(&window, cx), 0, "no request for the next frame");

    wait(31, cx);
    assert_eq!(renders(), 1, "not before 32 ms");
    wait(1, cx);
    assert_eq!(renders(), 2, "at 32 ms the view is drawn again");
    assert_eq!(last(&drawn), 32.0, "with the value at its own time");

    for _ in 0..3 {
        wait(10, cx);
    }
    assert_eq!(renders(), 2, "one timer at a time");
    wait(2, cx);
    assert_eq!(renders(), 3);
    assert_eq!(last(&drawn), 64.0);
}

#[gpui::test]
fn copies_of_a_capped_loop_in_two_views_are_each_drawn_again(cx: &mut TestAppContext) {
    let capped = ramp().forever().at_most_every(ms(32));
    let (_, first) = open(capped, cx);
    let (_, second) = open(capped, cx);
    wait(32, cx);
    assert_eq!(first.borrow().len(), 2);
    assert_eq!(second.borrow().len(), 2);
    assert_eq!(last(&second), 32.0);
}

#[gpui::test]
fn capped_keyframes_that_settle_are_drawn_once_settled_and_then_ask_for_nothing(
    cx: &mut TestAppContext,
) {
    let (window, drawn) = open(ramp().at_most_every(ms(64)), cx);
    for _ in 0..4 {
        wait(64, cx);
    }
    assert_eq!(*drawn.borrow(), [0.0, 64.0, 128.0, 64.0, 0.0]);
    wait(500, cx);
    assert_eq!(drawn.borrow().len(), 5, "settled, it asks for no frame");
    assert_eq!(next_frame(&window, cx), 0);
}

#[gpui::test]
fn under_reduced_motion_keyframes_rest_where_they_end(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let rise = || {
        Keyframes::new([
            Stop::new(ms(0), 0.0, Easing::Linear),
            Stop::new(ms(50), 50.0, Easing::Linear),
            Stop::new(ms(100), 100.0, Easing::Linear),
        ])
        .unwrap()
    };
    let (window, drawn) = open(rise().times(3).mirrored(), cx);
    assert_eq!(
        last(&drawn),
        100.0,
        "three mirrored rounds end on the last stop"
    );
    assert_eq!(next_frame(&window, cx), 0, "and ask for no frames");

    let (window, drawn) = open(rise().forever(), cx);
    assert_eq!(last(&drawn), 0.0, "a loop rests on its first stop");
    assert_eq!(next_frame(&window, cx), 0);
}

#[gpui::test]
fn restart_plays_the_keyframes_again_from_now(cx: &mut TestAppContext) {
    let (window, drawn) = open(up_and_down(), cx);
    cx.executor().advance_clock(ms(500));
    next_frame(&window, cx);
    assert_eq!(next_frame(&window, cx), 0, "played once and settled");

    window
        .update(cx, |pulse, _, cx| {
            pulse.keyframes.restart(cx);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(last(&drawn), 0.0, "from the first stop");
    cx.executor().advance_clock(ms(100));
    assert_eq!(next_frame(&window, cx), 1);
    assert_eq!(last(&drawn), 100.0);
}

#[gpui::test]
fn a_cap_of_zero_is_no_cap(cx: &mut TestAppContext) {
    let (window, drawn) = open(ramp().forever().at_most_every(Duration::ZERO), cx);
    assert_eq!(next_frame(&window, cx), 1, "it asks for the next frame");
    wait(32, cx);
    assert_eq!(drawn.borrow().len(), 2, "and no timer draws it in between");
}

#[gpui::test]
fn a_loop_behind_another_is_partway_through_its_round_from_the_first_frame(
    cx: &mut TestAppContext,
) {
    // The ramp is up over 128 ms and down over 128: 32 ms behind, the first
    // frame is 224 ms into the round before, on the way down. Ahead, it would
    // be 32 on the way up.
    let (window, drawn) = open(ramp().forever().behind(ms(32)), cx);
    assert_eq!(
        last(&drawn),
        32.0,
        "224 ms into a round: 32 on the way down"
    );
    cx.executor().advance_clock(ms(64));
    next_frame(&window, cx);
    assert_eq!(
        last(&drawn),
        32.0,
        "32 ms into its first round, on the way up"
    );
    cx.executor().advance_clock(ms(32));
    next_frame(&window, cx);
    assert_eq!(last(&drawn), 64.0);
}

#[gpui::test]
fn a_restart_keeps_the_delay_and_the_lag(cx: &mut TestAppContext) {
    let (window, drawn) = open(up_and_down().starting_at(ms(100)), cx);
    cx.executor().advance_clock(ms(500));
    next_frame(&window, cx);
    window
        .update(cx, |pulse, _, cx| {
            pulse.keyframes.restart(cx);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    cx.executor().advance_clock(ms(125));
    next_frame(&window, cx);
    assert_eq!(last(&drawn), 25.0, "25 ms after the restart's delay");

    let (window, drawn) = open(ramp().forever().behind(ms(32)), cx);
    cx.executor().advance_clock(ms(100));
    next_frame(&window, cx);
    window
        .update(cx, |pulse, _, cx| {
            pulse.keyframes.restart(cx);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(last(&drawn), 32.0, "as far behind as on its first frame");
}

#[gpui::test]
fn starting_at_waits_on_the_first_stop_after_the_first_frame(cx: &mut TestAppContext) {
    let (window, drawn) = open(up_and_down().starting_at(ms(100)), cx);
    assert_eq!(last(&drawn), 0.0);
    cx.executor().advance_clock(ms(50));
    next_frame(&window, cx);
    assert_eq!(
        last(&drawn),
        0.0,
        "still waiting: with no start it would be 50"
    );
    cx.executor().advance_clock(ms(75));
    next_frame(&window, cx);
    assert_eq!(last(&drawn), 25.0, "25 ms after its start; with none, 75");
}
