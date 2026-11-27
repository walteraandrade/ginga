//! What the adapter promises, checked on GPUI's test platform: a clock the
//! test moves, frames the test delivers, and no display.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use ginga_gpui::{Easing, Island, Motion, Phase, Presence, Tween};
use gpui::{
    AnyElement, AppContext, Context, IntoElement, ParentElement, Pixels, Render, StyleRefinement,
    Styled, TestAppContext, Window, WindowHandle, div, px, size,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// A linear tween, so the values are exact.
fn glide() -> Tween {
    Tween::new(ms(200), Easing::Linear)
}

/// Delivers the next frame to `window`, as the platform would, and returns how
/// many frame requests it answered.
fn next_frame<V: Render>(window: &WindowHandle<V>, cx: &mut TestAppContext) -> usize {
    let requests = window
        .update(cx, |_, window, cx| window.simulate_next_frame(cx))
        .unwrap();
    cx.run_until_parked();
    requests
}

/// A view that moves a width toward its target and writes down each value it drew.
struct Panel {
    width: Motion<Pixels>,
    target: Pixels,
    drawn: Rc<RefCell<Vec<Pixels>>>,
}

impl Render for Panel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = self.width.toward(self.target, glide(), window, cx);
        self.drawn.borrow_mut().push(width);
        div().w(width)
    }
}

fn open_panel(cx: &mut TestAppContext) -> (WindowHandle<Panel>, Rc<RefCell<Vec<Pixels>>>) {
    let drawn = Rc::new(RefCell::new(Vec::new()));
    let window = cx.open_window(size(px(400.), px(300.)), {
        let drawn = drawn.clone();
        move |_, _| Panel {
            width: Motion::at(px(0.)),
            target: px(0.),
            drawn,
        }
    });
    cx.run_until_parked();
    (window, drawn)
}

fn set_target(window: &WindowHandle<Panel>, target: Pixels, cx: &mut TestAppContext) {
    window
        .update(cx, |panel, _, cx| {
            panel.target = target;
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn toward_draws_the_engines_value_for_the_frames_time(cx: &mut TestAppContext) {
    let (window, drawn) = open_panel(cx);
    set_target(&window, px(100.), cx);
    assert_eq!(*drawn.borrow().last().unwrap(), px(0.));

    cx.executor().advance_clock(ms(50));
    assert_eq!(
        next_frame(&window, cx),
        1,
        "a moving value asks for a frame"
    );
    assert_eq!(*drawn.borrow().last().unwrap(), px(25.), "at 50 ms");

    cx.executor().advance_clock(ms(100));
    assert_eq!(next_frame(&window, cx), 1);
    assert_eq!(*drawn.borrow().last().unwrap(), px(75.), "at 150 ms");

    cx.executor().advance_clock(ms(60));
    assert_eq!(next_frame(&window, cx), 1);
    assert_eq!(
        *drawn.borrow().last().unwrap(),
        px(100.),
        "settled exactly on the target"
    );

    let frames = drawn.borrow().len();
    assert_eq!(
        next_frame(&window, cx),
        0,
        "a settled value asks for no frame"
    );
    assert_eq!(drawn.borrow().len(), frames, "and nothing is drawn again");
}

#[gpui::test]
fn toward_moves_at_once_under_reduced_motion(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (window, drawn) = open_panel(cx);
    set_target(&window, px(100.), cx);
    assert_eq!(*drawn.borrow().last().unwrap(), px(100.));
    assert_eq!(next_frame(&window, cx), 0);
}

/// Each frame of a presence: its phase and progress, or nothing drawn.
type Drawn = Rc<RefCell<Vec<Option<(Phase, f32)>>>>;

/// A view that shows a toast through a presence and writes down each frame of it.
struct Toaster {
    /// The content counts how many of it are alive.
    toast: Presence<Rc<()>>,
    drawn: Drawn,
}

impl Render for Toaster {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shown = self
            .toast
            .frame(window, cx)
            .map(|shown| (shown.phase, shown.progress));
        self.drawn.borrow_mut().push(shown);
        div()
    }
}

fn open_toaster(cx: &mut TestAppContext) -> (WindowHandle<Toaster>, Drawn) {
    let drawn = Rc::new(RefCell::new(Vec::new()));
    let window = cx.open_window(size(px(400.), px(300.)), {
        let drawn = drawn.clone();
        move |_, _| Toaster {
            toast: Presence::absent(),
            drawn,
        }
    });
    cx.run_until_parked();
    (window, drawn)
}

fn last(drawn: &Drawn) -> Option<(Phase, f32)> {
    *drawn.borrow().last().unwrap()
}

#[gpui::test]
fn a_presence_lets_go_of_its_content_once_the_way_out_is_over(cx: &mut TestAppContext) {
    let (window, drawn) = open_toaster(cx);
    let alive = Rc::new(());
    window
        .update(cx, |toaster, _, cx| {
            toaster.toast.show(alive.clone(), glide(), cx);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(last(&drawn), Some((Phase::Entering, 0.0)));
    cx.executor().advance_clock(ms(200));
    next_frame(&window, cx);
    assert_eq!(last(&drawn), Some((Phase::Present, 1.0)));
    assert_eq!(
        next_frame(&window, cx),
        0,
        "a present toast asks for no frame"
    );

    window
        .update(cx, |toaster, _, cx| {
            toaster.toast.hide(glide(), cx);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    cx.executor().advance_clock(ms(100));
    assert_eq!(next_frame(&window, cx), 1);
    assert_eq!(last(&drawn), Some((Phase::Exiting, 0.5)));
    assert_eq!(Rc::strong_count(&alive), 2, "kept while it leaves");

    cx.executor().advance_clock(ms(100));
    assert_eq!(next_frame(&window, cx), 1);
    assert_eq!(
        last(&drawn),
        None,
        "nothing to draw once the way out is over"
    );
    assert_eq!(Rc::strong_count(&alive), 1, "the content is dropped");
    assert_eq!(next_frame(&window, cx), 0);
}

#[gpui::test]
fn show_and_hide_move_at_once_under_reduced_motion(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (window, drawn) = open_toaster(cx);
    let alive = Rc::new(());
    window
        .update(cx, |toaster, _, cx| {
            toaster.toast.show(alive.clone(), glide(), cx);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(last(&drawn), Some((Phase::Present, 1.0)));
    assert_eq!(next_frame(&window, cx), 0);

    window
        .update(cx, |toaster, _, cx| {
            toaster.toast.hide(glide(), cx);
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(last(&drawn), None);
    assert_eq!(Rc::strong_count(&alive), 1);
    assert_eq!(next_frame(&window, cx), 0);
}

/// A view that is expensive to draw, and counts how often it is.
struct Heavy {
    renders: Rc<Cell<usize>>,
}

impl Render for Heavy {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        div()
    }
}

struct Dot {
    x: Motion<Pixels>,
    target: Pixels,
    renders: Rc<Cell<usize>>,
}

/// A heavy view, cached, with an island beside it.
struct Page {
    heavy: gpui::Entity<Heavy>,
    dot: gpui::Entity<Island<Dot>>,
}

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(self.heavy.clone().cached(StyleRefinement::default()))
            .child(self.dot.clone())
    }
}

#[gpui::test]
fn an_island_is_drawn_on_the_next_frame_and_the_cached_view_beside_it_is_not(
    cx: &mut TestAppContext,
) {
    let heavy_renders = Rc::new(Cell::new(0));
    let dot_renders = Rc::new(Cell::new(0));
    let window = cx.open_window(size(px(400.), px(300.)), {
        let heavy_renders = heavy_renders.clone();
        let dot_renders = dot_renders.clone();
        move |_, cx| Page {
            heavy: cx.new(|_| Heavy {
                renders: heavy_renders,
            }),
            dot: cx.new(|_| {
                let dot = Dot {
                    x: Motion::at(px(0.)),
                    target: px(0.),
                    renders: dot_renders,
                };
                Island::new(dot, |dot, window, cx| -> AnyElement {
                    dot.renders.set(dot.renders.get() + 1);
                    let x = dot.x.toward(dot.target, glide(), window, cx);
                    div().w(x).into_any_element()
                })
            }),
        }
    });
    cx.run_until_parked();
    let (heavy_before, dot_before) = (heavy_renders.get(), dot_renders.get());
    assert_eq!((heavy_before, dot_before), (1, 1));

    let dot = window.update(cx, |page, _, _| page.dot.clone()).unwrap();
    dot.update(cx, |island, cx| {
        island.state_mut().target = px(100.);
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(dot_renders.get(), 2);

    for _ in 0..3 {
        cx.executor().advance_clock(ms(16));
        assert_eq!(next_frame(&window, cx), 1);
    }
    assert_eq!(dot_renders.get(), 5, "the island is drawn on each frame");
    assert_eq!(heavy_renders.get(), 1, "the cached view beside it is not");
}
