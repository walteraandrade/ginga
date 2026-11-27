//! A list presence on a view's clock: frames asked for while an item moves,
//! none once it is settled, and reduced motion applied to every way.

use std::{cell::RefCell, rc::Rc, time::Duration};

use ginga_gpui::{Easing, InOut, Phase, PresenceList, Tween};
use gpui::{Context, IntoElement, Render, TestAppContext, Window, WindowHandle, div, px, size};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// A linear tween, so the values are exact.
fn glide() -> Tween {
    Tween::new(ms(200), Easing::Linear)
}

/// What one row was drawn as: its key, phase, progress and offset.
type Row = (u32, Phase, f32, f32);
type Drawn = Rc<RefCell<Vec<Vec<Row>>>>;

/// A view that lays its rows 40 px apart and writes down each frame of them.
struct Rows {
    rows: PresenceList<u32, ()>,
    drawn: Drawn,
}

impl Render for Rows {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[expect(clippy::cast_precision_loss, reason = "a handful of rows")]
        self.rows
            .place_all(|index, _| index as f32 * 40., glide(), cx);
        let frame = self
            .rows
            .frame(window, cx)
            .map(|row| (*row.key, row.phase, row.progress, row.offset))
            .collect();
        self.drawn.borrow_mut().push(frame);
        div()
    }
}

fn open(keys: &[u32], cx: &mut TestAppContext) -> (WindowHandle<Rows>, Drawn) {
    let drawn = Rc::new(RefCell::new(Vec::new()));
    let keys = keys.to_vec();
    let window = cx.open_window(size(px(400.), px(300.)), {
        let drawn = drawn.clone();
        move |_, cx| {
            let mut rows = PresenceList::new();
            rows.sync(keys.into_iter().map(|key| (key, ())), InOut::INSTANT, cx);
            Rows { rows, drawn }
        }
    });
    cx.run_until_parked();
    (window, drawn)
}

/// Makes the list hold `keys`, arriving and leaving with the glide.
fn sync(window: &WindowHandle<Rows>, keys: &[u32], cx: &mut TestAppContext) {
    window
        .update(cx, |view, _, cx| {
            let in_out = InOut::new(glide(), glide());
            view.rows
                .sync(keys.iter().map(|&key| (key, ())), in_out, cx);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
}

/// Delivers the next frame, as the platform would, and returns how many frame
/// requests it answered.
fn next_frame(window: &WindowHandle<Rows>, cx: &mut TestAppContext) -> usize {
    let requests = window
        .update(cx, |_, window, cx| window.simulate_next_frame(cx))
        .unwrap();
    cx.run_until_parked();
    requests
}

fn last(drawn: &Drawn) -> Vec<Row> {
    drawn.borrow().last().unwrap().clone()
}

#[gpui::test]
fn a_leaving_row_asks_for_frames_until_it_is_gone(cx: &mut TestAppContext) {
    let (window, drawn) = open(&[1, 2], cx);
    assert_eq!(
        last(&drawn),
        [(1, Phase::Present, 1.0, 0.0), (2, Phase::Present, 1.0, 0.0)],
        "the first sync only fills the list"
    );
    assert_eq!(next_frame(&window, cx), 0, "a still list asks for no frame");

    sync(&window, &[2], cx);
    cx.executor().advance_clock(ms(100));
    assert_eq!(next_frame(&window, cx), 1, "a leaving row asks for a frame");
    assert_eq!(
        last(&drawn),
        [(1, Phase::Exiting, 0.5, 0.0), (2, Phase::Present, 1.0, 0.0)]
    );

    cx.executor().advance_clock(ms(100));
    assert_eq!(next_frame(&window, cx), 1);
    let keys: Vec<u32> = last(&drawn).iter().map(|row| row.0).collect();
    assert_eq!(keys, [2], "a row whose way out is over is let go");

    // Row 2 then takes 1's place; once it is there, nothing asks for a frame.
    cx.executor().advance_clock(ms(200));
    next_frame(&window, cx);
    assert_eq!(last(&drawn), [(2, Phase::Present, 1.0, 0.0)]);
    assert_eq!(
        next_frame(&window, cx),
        0,
        "a settled list asks for no frame"
    );
}

#[gpui::test]
fn a_row_that_changes_place_glides_there_and_then_asks_for_no_frame(cx: &mut TestAppContext) {
    let (window, drawn) = open(&[1, 2], cx);
    sync(&window, &[2, 1], cx);
    cx.executor().advance_clock(ms(100));
    assert_eq!(
        next_frame(&window, cx),
        1,
        "a travelling row asks for a frame"
    );
    assert_eq!(
        last(&drawn),
        [
            (2, Phase::Present, 1.0, 20.0),
            (1, Phase::Present, 1.0, -20.0)
        ]
    );

    cx.executor().advance_clock(ms(100));
    next_frame(&window, cx);
    assert_eq!(
        last(&drawn),
        [(2, Phase::Present, 1.0, 0.0), (1, Phase::Present, 1.0, 0.0)]
    );
    assert_eq!(next_frame(&window, cx), 0, "settled rows ask for no frame");
}

#[gpui::test]
fn under_reduced_motion_rows_come_go_and_change_place_at_once(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (window, drawn) = open(&[1, 2], cx);

    sync(&window, &[2, 3], cx);
    assert_eq!(
        last(&drawn),
        [(2, Phase::Present, 1.0, 0.0), (3, Phase::Present, 1.0, 0.0)],
        "1 is gone, 3 is here and 2 is in its new place, all at once"
    );
    assert_eq!(next_frame(&window, cx), 0);
}

#[gpui::test]
fn under_reduced_motion_a_row_keeps_the_delay_of_its_way_in(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (window, drawn) = open(&[1], cx);
    window
        .update(cx, |view, _, cx| {
            let enter = glide().delay(ms(100));
            view.rows
                .sync([(1, ()), (2, ())], InOut::new(enter, glide()), cx);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(last(&drawn)[1], (2, Phase::Entering, 0.0, 0.0));
    cx.executor().advance_clock(ms(50));
    assert_eq!(next_frame(&window, cx), 1, "a waiting row asks for a frame");
    assert_eq!(last(&drawn)[1], (2, Phase::Entering, 0.0, 0.0));
    cx.executor().advance_clock(ms(50));
    next_frame(&window, cx);
    assert_eq!(last(&drawn)[1], (2, Phase::Present, 1.0, 0.0));
    assert_eq!(next_frame(&window, cx), 0);
}

#[gpui::test]
fn a_list_is_read_without_a_window_as_its_frame_would_be(cx: &mut TestAppContext) {
    let (window, drawn) = open(&[1, 2], cx);
    sync(&window, &[2], cx);
    cx.executor().advance_clock(ms(100));
    let sampled: Vec<Row> = window
        .update(cx, |view, _, cx| {
            view.rows
                .sample(cx)
                .map(|row| (*row.key, row.phase, row.progress, row.offset))
                .collect()
        })
        .unwrap();
    next_frame(&window, cx);
    assert_eq!(sampled, last(&drawn));
}
