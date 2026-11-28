//! An element that glides to where layout puts it, checked on GPUI's test
//! platform: where its content is painted, frame by frame.

use std::{cell::RefCell, rc::Rc, time::Duration};

use ginga_gpui::{Easing, GlideExt, Look, LookExt, Tween};
use gpui::{
    Bounds, Context, InteractiveElement, IntoElement, ParentElement, Pixels, Render, ScrollHandle,
    StatefulInteractiveElement, Styled, TestAppContext, Window, WindowHandle, canvas, div, point,
    px, size,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Where the gliding card's content was painted, one entry per frame.
type Drawn = Rc<RefCell<Vec<Bounds<Pixels>>>>;

/// A row with a spacer before a card: the spacer's width moves the card in
/// the layout. The row sits in a scrolled column, and can be moved by a
/// look's offset; the card can sit in a box that glides too.
struct Row {
    spacer: f32,
    /// The row's own offset, painted by a look.
    shift: f32,
    /// A gliding box around the card, moved by its own spacer.
    outer: Option<f32>,
    scroll: ScrollHandle,
    drawn: Drawn,
}

impl Render for Row {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let drawn = self.drawn.clone();
        let card = div()
            .w(px(20.))
            .h(px(20.))
            .child(
                canvas(
                    move |bounds, _, _| drawn.borrow_mut().push(bounds),
                    |_, (), _, _| {},
                )
                .size_full(),
            )
            .glides("card", Tween::new(ms(200), Easing::Linear));
        let inner = div()
            .flex()
            .child(div().w(px(self.spacer)).h(px(20.)))
            .child(card);
        let content = match self.outer {
            Some(outer) => div().flex().child(div().w(px(outer)).h(px(20.))).child(
                div()
                    .child(inner)
                    .glides("outer", Tween::new(ms(200), Easing::Linear)),
            ),
            None => inner,
        };
        div()
            .id("column")
            .h(px(100.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(
                div()
                    .h(px(400.))
                    .child(content.with_look(Look::new().offset(self.shift, 0.))),
            )
    }
}

fn open(cx: &mut TestAppContext, outer: Option<f32>) -> (WindowHandle<Row>, Drawn) {
    let drawn = Drawn::default();
    let window = cx.open_window(size(px(400.), px(300.)), {
        let drawn = drawn.clone();
        move |_, _| Row {
            spacer: 0.,
            shift: 0.,
            outer,
            scroll: ScrollHandle::new(),
            drawn,
        }
    });
    cx.run_until_parked();
    (window, drawn)
}

fn change(window: &WindowHandle<Row>, cx: &mut TestAppContext, f: impl FnOnce(&mut Row)) {
    window
        .update(cx, |row, _, cx| {
            f(row);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
}

/// Moves the clock by `millis`, delivers the next frame, and returns how many
/// frame requests it answered.
fn after(millis: u64, window: &WindowHandle<Row>, cx: &mut TestAppContext) -> usize {
    cx.executor().advance_clock(ms(millis));
    let requests = window
        .update(cx, |_, window, cx| window.simulate_next_frame(cx))
        .unwrap();
    cx.run_until_parked();
    requests
}

fn x(drawn: &Drawn) -> f32 {
    drawn.borrow().last().unwrap().origin.x.into()
}

#[gpui::test]
fn a_move_in_the_layout_glides_there_and_settles_on_it(cx: &mut TestAppContext) {
    let (window, drawn) = open(cx, None);
    assert_eq!(x(&drawn), 0.0, "its first frame is in place");

    change(&window, cx, |row| row.spacer = 100.);
    assert_eq!(
        x(&drawn),
        0.0,
        "the frame of the move: painted where it was"
    );
    assert_eq!(
        after(50, &window, cx),
        1,
        "a gliding element asks for a frame"
    );
    assert_eq!(x(&drawn), 25.0, "a quarter of the way at 50 ms");
    after(150, &window, cx);
    assert_eq!(x(&drawn), 100.0, "exactly in its place at 200 ms");
    assert_eq!(after(16, &window, cx), 0, "settled, it asks for no frame");
}

#[gpui::test]
fn a_second_move_on_the_way_turns_from_where_it_is(cx: &mut TestAppContext) {
    let (window, drawn) = open(cx, None);
    change(&window, cx, |row| row.spacer = 100.);
    after(100, &window, cx);
    assert_eq!(x(&drawn), 50.0);
    change(&window, cx, |row| row.spacer = 0.);
    assert_eq!(x(&drawn), 50.0, "no jump");
    after(100, &window, cx);
    assert_eq!(x(&drawn), 25.0, "heading back to 0 from 50");
}

#[gpui::test]
fn a_scroll_or_an_offset_painted_around_it_is_not_a_move(cx: &mut TestAppContext) {
    let (window, drawn) = open(cx, None);
    let first = drawn.borrow().last().unwrap().origin;

    change(&window, cx, |row| {
        row.scroll.set_offset(point(px(0.), px(-30.)));
    });
    after(16, &window, cx);
    let scrolled = drawn.borrow().last().unwrap().origin;
    assert_eq!(
        scrolled,
        point(first.x, first.y - px(30.)),
        "it scrolls with the rest at once"
    );
    assert_eq!(after(16, &window, cx), 0, "and does not glide");

    change(&window, cx, |row| row.shift = 40.);
    assert_eq!(x(&drawn), 40.0, "a look's offset moves it at once");
    assert_eq!(after(16, &window, cx), 0, "and is not a move");
}

#[gpui::test]
fn inside_a_gliding_box_it_moves_with_the_box(cx: &mut TestAppContext) {
    let (window, drawn) = open(cx, Some(0.));
    change(&window, cx, |row| row.outer = Some(100.));
    after(50, &window, cx);
    assert_eq!(x(&drawn), 25.0, "carried by the box, not gliding twice");
    after(150, &window, cx);
    assert_eq!(x(&drawn), 100.0);

    change(&window, cx, |row| row.spacer = 40.);
    after(100, &window, cx);
    assert_eq!(
        x(&drawn),
        120.0,
        "its own move in the box glides on its own"
    );
}

#[gpui::test]
fn under_reduced_motion_it_is_in_its_place_at_once(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (window, drawn) = open(cx, None);
    change(&window, cx, |row| row.spacer = 100.);
    assert_eq!(x(&drawn), 100.0);
    assert_eq!(after(16, &window, cx), 0);
}
