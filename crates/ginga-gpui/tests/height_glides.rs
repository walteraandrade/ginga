//! A height that glides when the content's own changes, checked on GPUI's
//! test platform: how much room the box takes at given times, and the
//! frames it asks for.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use ginga_gpui::{Easing, OpenExt, Spring, Transition, Tween};
use gpui::{
    Bounds, Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Pixels, Render,
    StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext, Window, canvas, div,
    point, px,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Where the line under the box was painted, one entry per frame.
type Drawn = Rc<RefCell<Vec<Bounds<Pixels>>>>;

/// A box opened `progress` of the way, whose content is `height` tall and
/// whose height glides, then a line whose place says how much room the box
/// took. The content's last 10 px count clicks as `bottom`, and a 10 px tag
/// placed just below the content, as a popover would be, counts them as
/// `below`.
struct Toast {
    height: f32,
    progress: f32,
    glide: Transition,
    drawn: Drawn,
    bottom: Rc<Cell<u32>>,
    below: Rc<Cell<u32>>,
}

impl Render for Toast {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let drawn = self.drawn.clone();
        div()
            .w(px(100.))
            .flex()
            .flex_col()
            .child(
                div()
                    .relative()
                    .child(div().h(px(self.height)))
                    .child(
                        div()
                            .id("bottom")
                            .absolute()
                            .left_0()
                            .bottom_0()
                            .w_full()
                            .h(px(10.))
                            .on_click({
                                let bottom = self.bottom.clone();
                                move |_, _, _| bottom.set(bottom.get() + 1)
                            }),
                    )
                    .child(
                        div()
                            .id("below")
                            .absolute()
                            .left_0()
                            .top(px(self.height))
                            .w_full()
                            .h(px(10.))
                            .on_click({
                                let below = self.below.clone();
                                move |_, _, _| below.set(below.get() + 1)
                            }),
                    )
                    .opened("toast", self.progress)
                    .height_glides(self.glide),
            )
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

struct Stage<'a> {
    cx: &'a mut VisualTestContext,
    view: gpui::Entity<Toast>,
    drawn: Drawn,
    bottom: Rc<Cell<u32>>,
    below: Rc<Cell<u32>>,
}

fn open(cx: &mut TestAppContext, glide: impl Into<Transition>) -> Stage<'_> {
    open_at(cx, glide, 40., 1.)
}

fn open_at(
    cx: &mut TestAppContext,
    glide: impl Into<Transition>,
    height: f32,
    progress: f32,
) -> Stage<'_> {
    let (drawn, bottom, below) = (Drawn::default(), Rc::default(), Rc::default());
    let glide = glide.into();
    let (view, cx) = cx.add_window_view({
        let (drawn, bottom, below) = (drawn.clone(), Rc::clone(&bottom), Rc::clone(&below));
        move |_, _| Toast {
            height,
            progress,
            glide,
            drawn,
            bottom,
            below,
        }
    });
    cx.run_until_parked();
    Stage {
        cx,
        view,
        drawn,
        bottom,
        below,
    }
}

impl Stage<'_> {
    /// The box's height in the frame drawn after the clock moves `millis`,
    /// and how many frame requests that frame answered.
    fn after(&mut self, millis: u64) -> (f32, usize) {
        self.cx.executor().advance_clock(ms(millis));
        let requests = self.cx.update(Window::simulate_next_frame);
        self.cx.run_until_parked();
        (self.height(), requests)
    }

    fn height(&self) -> f32 {
        f32::from(
            self.drawn
                .borrow()
                .last()
                .expect("a frame was drawn")
                .origin
                .y,
        )
    }

    /// Clicks at `y` px from the top of the box.
    fn click(&mut self, y: f32) {
        self.cx
            .simulate_click(point(px(50.), px(y)), Modifiers::none());
        self.cx.run_until_parked();
    }

    /// Makes the content `height` tall, drawn on the next frame.
    fn grow(&mut self, height: f32) {
        self.view.update(self.cx, |toast, cx| {
            toast.height = height;
            cx.notify();
        });
        self.cx.run_until_parked();
    }
}

#[gpui::test]
fn content_that_grows_glides_to_its_new_height_and_settles_on_it(cx: &mut TestAppContext) {
    let mut toast = open(cx, Tween::new(ms(100), Easing::Linear));
    assert_eq!(toast.height(), 40.);
    toast.grow(120.);
    assert_eq!(
        toast.height(),
        40.,
        "the frame that finds it draws the height before"
    );
    let heights: Vec<f32> = [25, 25, 25, 25].map(|t| toast.after(t).0).into();
    assert_eq!(heights, [60., 80., 100., 120.]);
    assert_eq!(
        toast.after(16),
        (120., 0),
        "settled exactly, and no more frames"
    );
}

#[gpui::test]
fn a_second_change_on_the_way_keeps_its_speed(cx: &mut TestAppContext) {
    let spring = Spring::SNAPPY;
    let mut toast = open(cx, spring);
    toast.grow(120.);
    toast.after(50);
    let midway = toast.height();
    toast.grow(200.);
    let turn = toast.after(16).0;
    // The same as an engine motion that heads for 200 from where it was,
    // keeping its speed.
    let mut motion = ginga_gpui::ginga::Motion::at(40.0_f32);
    motion.set_target(120., spring, ms(0));
    motion.set_target(200., spring, ms(50));
    assert!((motion.sample(ms(50)).value - midway).abs() < 0.5);
    assert!(
        (motion.sample(ms(66)).value - turn).abs() < 0.5,
        "{turn} is not {}",
        motion.sample(ms(66)).value
    );
    let mut from_rest = ginga_gpui::ginga::Motion::at(midway);
    from_rest.set_target(200., spring, ms(50));
    assert!(
        (from_rest.sample(ms(66)).value - turn).abs() > 2.,
        "it did not start from rest"
    );
}

#[gpui::test]
fn under_reduced_motion_it_takes_the_new_height_at_once(cx: &mut TestAppContext) {
    let mut toast = open(cx, Tween::new(ms(100), Easing::Linear));
    toast.cx.update(|_, cx| cx.set_reduce_motion(true));
    toast.grow(120.);
    assert_eq!(toast.after(16), (120., 1), "the next frame, at once");
    assert_eq!(toast.after(16).1, 0);
}

#[gpui::test]
fn without_a_change_it_asks_for_no_frame(cx: &mut TestAppContext) {
    let mut toast = open(cx, Tween::new(ms(100), Easing::Linear));
    assert_eq!(toast.after(16), (40., 0));
}

#[gpui::test]
fn content_that_shrinks_glides_down_to_its_new_height(cx: &mut TestAppContext) {
    let mut toast = open_at(cx, Tween::new(ms(100), Easing::Linear), 120., 1.);
    toast.grow(40.);
    assert_eq!(toast.height(), 120.);
    let heights: Vec<f32> = [25, 25, 25, 25].map(|t| toast.after(t).0).into();
    assert_eq!(heights, [100., 80., 60., 40.]);
    assert_eq!(toast.after(16), (40., 0));
}

#[gpui::test]
fn part_of_the_way_open_it_glides_by_that_share(cx: &mut TestAppContext) {
    let mut toast = open_at(cx, Tween::new(ms(100), Easing::Linear), 40., 0.5);
    toast.after(16);
    assert_eq!(toast.height(), 20.);
    toast.grow(120.);
    let heights: Vec<f32> = [25, 25, 25, 25].map(|t| toast.after(t).0).into();
    assert_eq!(heights, [30., 40., 50., 60.]);
    assert_eq!(toast.after(16), (60., 0));
}

#[gpui::test]
fn while_it_glides_what_is_past_the_box_is_cut_and_settled_nothing_is(cx: &mut TestAppContext) {
    let mut toast = open(cx, Tween::new(ms(100), Easing::Linear));
    toast.click(45.);
    assert_eq!(
        toast.below.get(),
        1,
        "settled, a popover below it is not cut"
    );
    toast.grow(120.);
    toast.after(50);
    assert_eq!(toast.height(), 80.);
    toast.click(115.);
    assert_eq!(toast.bottom.get(), 0, "past the box while it glides: cut");
    toast.after(50);
    toast.after(16);
    toast.click(115.);
    assert_eq!(toast.bottom.get(), 1, "there, nothing of it is cut");
}

#[gpui::test]
fn a_small_change_on_the_way_keeps_its_course(cx: &mut TestAppContext) {
    // Half open, a half pixel of content moves the box a quarter: a small
    // change. GPUI lays heights out in half pixels here.
    let mut toast = open_at(cx, Tween::new(ms(100), Easing::Linear), 40., 0.5);
    toast.after(16);
    toast.grow(120.);
    toast.after(50);
    assert_eq!(toast.height(), 40.);
    toast.grow(120.5);
    let (midway, requests) = toast.after(25);
    assert!(midway > 40. && midway < 50., "still on its way: {midway}");
    assert!(requests > 0, "and asking for frames");
    toast.after(100);
    let (there, requests) = toast.after(16);
    assert!((there - 60.25).abs() < 0.5, "{there}");
    assert_eq!(requests, 0);
}

#[gpui::test]
fn a_small_change_at_rest_asks_for_no_frame(cx: &mut TestAppContext) {
    let mut toast = open_at(cx, Tween::new(ms(100), Easing::Linear), 40., 0.5);
    toast.after(16);
    toast.grow(40.5);
    assert_eq!(toast.after(16).1, 0);
}

#[gpui::test]
fn closed_on_its_way_it_asks_for_no_frame(cx: &mut TestAppContext) {
    let mut toast = open(cx, Tween::new(ms(100), Easing::Linear));
    toast.grow(120.);
    toast.after(25);
    toast.view.update(toast.cx, |toast, cx| {
        toast.progress = 0.;
        cx.notify();
    });
    toast.cx.run_until_parked();
    // The frame asked for on the way is drawn; closed, it asks for no other.
    assert_eq!(toast.after(16).0, 0.);
    assert_eq!(toast.after(16), (0., 0));
}
