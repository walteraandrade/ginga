//! An element opened part of the way, checked on GPUI's test platform: how
//! much room it takes, what of it takes clicks, and the frames it asks for.

use std::{cell::RefCell, rc::Rc, time::Duration};

use ginga_gpui::{Easing, OpenExt, Presence, Tween};
use gpui::{
    App, Bounds, Context, InteractiveElement, IntoElement, Modifiers, MouseButton, ParentElement,
    Pixels, Render, StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext, Window,
    canvas, div, point, px,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Where the line under the opened element was painted, one entry per frame.
type Drawn = Rc<RefCell<Vec<Bounds<Pixels>>>>;

/// A column 100 px wide: an opened box of `tiles` tiles, each 40 px wide and
/// 20 px tall, that wrap two to a row, then a line whose place says how
/// much room the box took. A click on a tile is counted.
struct Column {
    progress: f32,
    tiles: usize,
    /// When set, the box is shown by this presence and opens by its
    /// progress, instead of by `progress`.
    details: Option<Presence<()>>,
    clicks: Rc<RefCell<usize>>,
    drawn: Drawn,
}

impl Column {
    fn tiles(&self) -> gpui::Stateful<gpui::Div> {
        let clicks = self.clicks.clone();
        div()
            .id("tiles")
            .flex()
            .flex_wrap()
            .on_click(move |_, _, _| *clicks.borrow_mut() += 1)
            .children((0..self.tiles).map(|_| div().w(px(40.)).h(px(20.))))
    }
}

impl Render for Column {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let drawn = self.drawn.clone();
        let opened = match &mut self.details {
            Some(details) => details.frame(window, cx).map(|shown| shown.progress),
            None => Some(self.progress),
        };
        let tiles = self.tiles();
        div()
            .w(px(100.))
            .flex()
            .flex_col()
            .children(opened.map(|progress| tiles.opened("tiles", progress)))
            .child(
                div().h(px(10.)).child(
                    canvas(
                        move |bounds, _, _| drawn.borrow_mut().push(bounds),
                        |_, (), _, _| {},
                    )
                    .size_full(),
                ),
            )
    }
}

fn open(
    progress: f32,
    tiles: usize,
    details: Option<Presence<()>>,
    cx: &mut TestAppContext,
) -> (
    gpui::Entity<Column>,
    Drawn,
    Rc<RefCell<usize>>,
    &mut VisualTestContext,
) {
    let drawn = Drawn::default();
    let clicks = Rc::new(RefCell::new(0));
    let (view, cx) = cx.add_window_view({
        let drawn = drawn.clone();
        let clicks = clicks.clone();
        move |_, _| Column {
            progress,
            tiles,
            details,
            clicks,
            drawn,
        }
    });
    cx.run_until_parked();
    (view, drawn, clicks, cx)
}

/// Delivers the next frame and returns how many frame requests it answered.
fn next_frame(cx: &mut VisualTestContext) -> usize {
    let requests = cx.update(Window::simulate_next_frame);
    cx.run_until_parked();
    requests
}

fn change(
    view: &gpui::Entity<Column>,
    cx: &mut VisualTestContext,
    f: impl FnOnce(&mut Column, &App),
) {
    view.update(cx, |column, cx| {
        f(column, cx);
        cx.notify();
    });
    cx.run_until_parked();
}

/// How far down the line under the box was painted, last frame.
fn below(drawn: &Drawn) -> f32 {
    drawn.borrow().last().unwrap().origin.y.into()
}

fn click(cx: &mut VisualTestContext, x: f32, y: f32) {
    let at = point(px(x), px(y));
    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
}

#[gpui::test]
fn fully_open_it_takes_its_own_height_at_its_width(cx: &mut TestAppContext) {
    // Five tiles, two to a row at 100 px: three rows.
    let (_, drawn, clicks, cx) = open(1.0, 5, None, cx);
    assert_eq!(below(&drawn), 60.0);
    // It asks for no frame: it is not part of the way open.
    assert_eq!(next_frame(cx), 0);
    // Nothing is cut: the last row takes clicks.
    click(cx, 10., 50.);
    assert_eq!(*clicks.borrow(), 1);
}

#[gpui::test]
fn part_of_the_way_open_it_takes_that_share_and_cuts_the_rest(cx: &mut TestAppContext) {
    let (view, drawn, clicks, cx) = open(1.0, 4, None, cx);
    assert_eq!(below(&drawn), 40.0);

    change(&view, cx, |column, _| column.progress = 0.5);
    assert_eq!(below(&drawn), 20.0, "half of its 40 px");
    assert_eq!(next_frame(cx), 0, "its height did not change");
    // The first row is in the open part; the second is cut, and the line
    // is drawn where the second row would be.
    click(cx, 10., 10.);
    assert_eq!(*clicks.borrow(), 1);
    click(cx, 10., 30.);
    assert_eq!(*clicks.borrow(), 1, "the cut row takes no click");

    change(&view, cx, |column, _| column.progress = 0.0);
    assert_eq!(below(&drawn), 0.0, "closed, it takes no room");
    click(cx, 10., 5.);
    assert_eq!(*clicks.borrow(), 1, "closed, it takes no click");
}

#[gpui::test]
fn a_progress_out_of_range_is_held_to_it(cx: &mut TestAppContext) {
    let (view, drawn, _, cx) = open(1.0, 4, None, cx);
    change(&view, cx, |column, _| column.progress = 2.0);
    assert_eq!(below(&drawn), 40.0);
    change(&view, cx, |column, _| column.progress = -1.0);
    assert_eq!(below(&drawn), 0.0);
    change(&view, cx, |column, _| column.progress = f32::NAN);
    assert_eq!(below(&drawn), 0.0);
}

#[gpui::test]
fn a_new_height_is_opened_to_on_the_next_frame(cx: &mut TestAppContext) {
    // First drawn half open, it has no height measured yet: it takes none,
    // and asks for the frame that opens it to half of what it measured.
    let (view, drawn, _, cx) = open(0.5, 4, None, cx);
    assert_eq!(below(&drawn), 0.0);
    assert_eq!(next_frame(cx), 1);
    assert_eq!(below(&drawn), 20.0);
    assert_eq!(next_frame(cx), 0);

    // Its content grows by a row while it is half open.
    change(&view, cx, |column, _| column.tiles = 6);
    assert_eq!(
        below(&drawn),
        20.0,
        "the frame of the change keeps the height before"
    );
    assert_eq!(next_frame(cx), 1);
    assert_eq!(below(&drawn), 30.0, "half of its new 60 px");
    assert_eq!(next_frame(cx), 0);
}

#[gpui::test]
fn it_opens_and_closes_with_a_presence(cx: &mut TestAppContext) {
    let (view, drawn, _, cx) = open(0.0, 4, Some(Presence::absent()), cx);
    assert_eq!(below(&drawn), 0.0);

    let way = Tween::new(ms(100), Easing::Linear);
    change(&view, cx, |column, cx| {
        column.details.as_mut().unwrap().show((), way, cx);
    });
    // Its first frame of the way in, at progress 0, measures it.
    assert_eq!(below(&drawn), 0.0);
    cx.executor().advance_clock(ms(25));
    next_frame(cx);
    assert_eq!(below(&drawn), 10.0, "a quarter of the way in");
    cx.executor().advance_clock(ms(75));
    next_frame(cx);
    assert_eq!(below(&drawn), 40.0, "all the way in");

    change(&view, cx, |column, cx| {
        column.details.as_mut().unwrap().hide(way, cx);
    });
    cx.executor().advance_clock(ms(50));
    next_frame(cx);
    assert_eq!(below(&drawn), 20.0, "half of the way out");
    cx.executor().advance_clock(ms(50));
    next_frame(cx);
    assert_eq!(below(&drawn), 0.0, "gone");
    assert_eq!(next_frame(cx), 0);
}
