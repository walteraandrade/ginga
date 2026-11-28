//! A height that glides when the content's own changes, checked on GPUI's
//! test platform: how much room the box takes at given times, and the
//! frames it asks for.

use std::{cell::RefCell, rc::Rc, time::Duration};

use ginga_gpui::{Easing, OpenExt, Spring, Transition, Tween};
use gpui::{
    Bounds, Context, IntoElement, ParentElement, Pixels, Render, Styled, TestAppContext,
    VisualTestContext, Window, canvas, div, px,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Where the line under the box was painted, one entry per frame.
type Drawn = Rc<RefCell<Vec<Bounds<Pixels>>>>;

/// A box opened all the way, whose content is `height` tall and whose height
/// glides, then a line whose place says how much room the box took.
struct Toast {
    height: f32,
    glide: Transition,
    drawn: Drawn,
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
                    .child(div().h(px(self.height)))
                    .opened("toast", 1.)
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
}

fn open(cx: &mut TestAppContext, glide: impl Into<Transition>) -> Stage<'_> {
    let drawn = Drawn::default();
    let glide = glide.into();
    let (view, cx) = cx.add_window_view({
        let drawn = drawn.clone();
        move |_, _| Toast {
            height: 40.,
            glide,
            drawn,
        }
    });
    cx.run_until_parked();
    Stage { cx, view, drawn }
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
