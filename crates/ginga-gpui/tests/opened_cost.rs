//! What an opening element costs, counted on GPUI's test platform: which
//! cached views around it render again while it opens, and how long a frame
//! of many takes. The numbers are in `docs/measurements/2026-11-28-accordion`.

use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

use ginga_gpui::{Easing, OpenExt, Presence, Tween};
use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, StyleRefinement, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// A view that counts its renders.
struct Counted {
    renders: Rc<Cell<usize>>,
}

impl Render for Counted {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        div().h(px(30.)).w_full()
    }
}

/// An answer that opens, with a cached view in it, one below it in the same
/// column, and one beside the column.
struct Page {
    answer: Presence<()>,
    inside: Entity<Counted>,
    below: Entity<Counted>,
    beside: Entity<Counted>,
}

impl Render for Page {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cached = |view: &Entity<Counted>| view.clone().cached(StyleRefinement::default());
        let answer = self.answer.frame(window, cx).map(|shown| {
            div()
                .child(div().h(px(40.)))
                .child(cached(&self.inside))
                .opened("answer", shown.progress)
        });
        div()
            .flex()
            .child(
                div()
                    .w(px(200.))
                    .flex()
                    .flex_col()
                    .children(answer)
                    .child(cached(&self.below)),
            )
            .child(div().w(px(200.)).child(cached(&self.beside)))
    }
}

fn counter(cx: &mut App) -> (Entity<Counted>, Rc<Cell<usize>>) {
    let renders = Rc::new(Cell::new(0));
    let view = cx.new(|_| Counted {
        renders: renders.clone(),
    });
    (view, renders)
}

use gpui::App;

fn next_frame(cx: &mut VisualTestContext) -> usize {
    let requests = cx.update(Window::simulate_next_frame);
    cx.run_until_parked();
    requests
}

#[gpui::test]
fn while_it_opens_the_cached_views_in_it_and_below_it_render_again(cx: &mut TestAppContext) {
    let counts = Rc::new(Cell::new(None));
    let (page, cx) = cx.add_window_view({
        let counts = counts.clone();
        move |_, cx| {
            let (inside, inside_renders) = counter(cx);
            let (below, below_renders) = counter(cx);
            let (beside, beside_renders) = counter(cx);
            counts.set(Some((inside_renders, below_renders, beside_renders)));
            Page {
                answer: Presence::absent(),
                inside,
                below,
                beside,
            }
        }
    });
    cx.run_until_parked();
    let (inside, below, beside) = counts.take().unwrap();
    let before = (inside.get(), below.get(), beside.get());

    page.update(cx, |page, cx| {
        page.answer
            .show((), Tween::new(ms(160), Easing::Linear), cx);
        cx.notify();
    });
    cx.run_until_parked();
    let mut frames = 1;
    loop {
        cx.executor().advance_clock(ms(16));
        if next_frame(cx) == 0 {
            break;
        }
        frames += 1;
    }
    let renders = (
        inside.get() - before.0,
        below.get() - before.1,
        beside.get() - before.2,
    );
    println!("frames {frames}, renders inside/below/beside {renders:?}");
    assert_eq!(renders.2, 0, "the view beside the column never renders");
    assert!(
        renders.1 >= frames - 1,
        "the view below renders on each frame that moves it"
    );
    // Its place and size stay, but the cut around it changes, and a cached
    // view is reused only under the same cut.
    assert!(
        renders.0 >= frames - 1,
        "the view inside renders on each frame the cut changes"
    );

    let settled = (inside.get(), below.get(), beside.get());
    cx.executor().advance_clock(ms(16));
    assert_eq!(next_frame(cx), 0);
    assert_eq!(
        (inside.get(), below.get(), beside.get()),
        settled,
        "fully open, nothing renders again"
    );
}

/// Many questions, each with its answer.
struct Many {
    answers: Vec<Presence<()>>,
    /// Draws each answer as it is, with no `.opened` around it.
    plain: bool,
}

impl Render for Many {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let plain = self.plain;
        div()
            .w(px(400.))
            .flex()
            .flex_col()
            .children(self.answers.iter_mut().enumerate().map(|(index, answer)| {
                let open = answer.frame(window, cx).map(|shown| {
                    let answer = div().p_2().child(div().h(px(36.)));
                    if plain {
                        answer.into_any_element()
                    } else {
                        answer
                            .opened(("answer", index), shown.progress)
                            .into_any_element()
                    }
                });
                div().child(div().h(px(20.))).children(open)
            }))
    }
}

/// Run with `cargo test --release -p ginga-gpui --test opened_cost -- --ignored --nocapture`.
#[gpui::test]
#[ignore = "a measurement: it prints times"]
fn time_a_frame_of_many_opening(cx: &mut TestAppContext) {
    for count in [10, 100, 400] {
        let (many, cx) = open_many(cx, count, true, true);
        let plain = time_frames(cx, &many, 60, false);
        let (many, cx) = open_many(cx, count, false, true);
        let full = time_frames(cx, &many, 60, false);
        let (many, cx) = open_many(cx, count, false, false);
        many.update(cx, |many, cx| {
            for answer in &mut many.answers {
                answer.show((), Tween::new(ms(100_000), Easing::Linear), cx);
            }
            cx.notify();
        });
        cx.run_until_parked();
        let opening = time_frames(cx, &many, 60, true);
        println!(
            "{count} answers, µs a frame: {plain:.0} drawn as they are, {full:.0} opened at 1.0, {opening:.0} opening"
        );
    }
}
/// A window of `count` questions, their answers shown or not.
fn open_many(
    cx: &mut TestAppContext,
    count: usize,
    plain: bool,
    shown: bool,
) -> (Entity<Many>, &mut VisualTestContext) {
    let (many, cx) = cx.add_window_view(move |_, _| Many {
        answers: (0..count)
            .map(|_| {
                if shown {
                    Presence::present(())
                } else {
                    Presence::absent()
                }
            })
            .collect(),
        plain,
    });
    cx.run_until_parked();
    (many, cx)
}

/// Renders `frames` frames and returns the mean time of one, in µs.
fn time_frames(cx: &mut VisualTestContext, many: &Entity<Many>, frames: u32, moving: bool) -> f64 {
    let start = Instant::now();
    for _ in 0..frames {
        cx.executor().advance_clock(ms(16));
        if !moving {
            many.update(cx, |_, cx| cx.notify());
        }
        cx.update(Window::simulate_next_frame);
        cx.run_until_parked();
    }
    start.elapsed().as_secs_f64() * 1e6 / f64::from(frames)
}
