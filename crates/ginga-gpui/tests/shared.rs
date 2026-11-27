//! One thing that travels between two places, checked on GPUI's test platform.

use std::{
    cell::{Cell, RefCell},
    fmt::Write,
    rc::Rc,
    time::Duration,
};

use ginga_gpui::{Easing, SharedExt, Tween};
use gpui::{
    Bounds, Context, InteractiveElement, IntoElement, ParentElement, Pixels, Render, Styled,
    TestAppContext, VisualTestContext, Window, canvas, div, px,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Where the shared thing was painted, one entry per frame that painted it.
type Painted = Rc<RefCell<Vec<Bounds<Pixels>>>>;

fn glide() -> Tween {
    Tween::new(ms(100), Easing::Linear)
}

/// What paints itself into `painted`, to fill the shared element.
fn marker(painted: &Painted) -> impl IntoElement {
    let painted = painted.clone();
    canvas(
        |_, _, _| {},
        move |bounds, (), _, _| painted.borrow_mut().push(bounds),
    )
    .size_full()
}

/// Three tabs, 100 px wide, side by side; the selected one draws a 4 px mark
/// under its label, shared between the tabs.
struct Tabs {
    selected: Rc<Cell<usize>>,
    painted: Painted,
}

impl Render for Tabs {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected.get();
        div().flex().children((0..3).map(|tab| {
            div()
                .id(tab)
                .w(px(100.))
                .h(px(30.))
                .flex()
                .flex_col()
                .justify_end()
                .children((tab == selected).then(|| {
                    div()
                        .h(px(4.))
                        .w_full()
                        .child(marker(&self.painted))
                        .shared("mark", glide())
                }))
        }))
    }
}

/// A row in a list that opens into a card further down: the same thing.
struct List {
    open: Rc<Cell<bool>>,
    painted: Painted,
}

impl Render for List {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let open = self.open.get();
        div()
            .flex()
            .flex_col()
            .child(div().h(px(40.)).w(px(300.)).children((!open).then(|| {
                div()
                    .size_full()
                    .child(marker(&self.painted))
                    .shared("item", glide())
            })))
            .child(div().h(px(60.)))
            .children(open.then(|| {
                div().id("card").child(
                    div()
                        .w(px(200.))
                        .h(px(200.))
                        .child(marker(&self.painted))
                        .shared("item", glide()),
                )
            }))
    }
}

struct Stage<'a> {
    cx: &'a mut VisualTestContext,
    painted: Painted,
}

impl Stage<'_> {
    /// The thing's bounds after the clock moves `millis` and a frame is
    /// drawn, as `x y width height`, or `-` when it was not painted.
    fn after(&mut self, millis: u64) -> String {
        self.painted.borrow_mut().clear();
        self.cx.executor().advance_clock(ms(millis));
        self.cx.update(Window::simulate_next_frame);
        self.cx.run_until_parked();
        self.painted.borrow().last().map_or(String::from("-"), |b| {
            format!(
                "{} {} {} {}",
                f32::from(b.origin.x),
                f32::from(b.origin.y),
                f32::from(b.size.width),
                f32::from(b.size.height)
            )
        })
    }

    fn redraw(&mut self) {
        self.cx.update(|window, _| window.refresh());
    }

    /// Each time in `times`, a step from the one before, and the bounds then.
    fn trace(&mut self, steps: &[u64]) -> String {
        let mut text = String::new();
        let mut at = 0;
        for step in steps {
            at += step;
            let bounds = self.after(*step);
            writeln!(text, "{at:<4} {bounds}").unwrap();
        }
        text
    }
}

fn tabs(cx: &mut TestAppContext) -> (Stage<'_>, Rc<Cell<usize>>) {
    let (selected, painted) = (Rc::new(Cell::new(0)), Painted::default());
    let (_, cx) = cx.add_window_view({
        let (selected, painted) = (selected.clone(), painted.clone());
        move |_, _| Tabs { selected, painted }
    });
    cx.run_until_parked();
    (Stage { cx, painted }, selected)
}

#[gpui::test]
fn the_mark_of_the_selected_tab_travels_to_the_next_one(cx: &mut TestAppContext) {
    let (mut stage, selected) = tabs(cx);
    assert_eq!(stage.after(16), "-", "nothing moves: no frame asked for");
    // The frame the new mark appears in measures where it goes and draws
    // nothing; each frame after it draws the mark on its way.
    selected.set(2);
    stage.redraw();
    assert_eq!(
        stage.trace(&[0, 25, 25, 25, 25, 25]),
        "\
0    0 26 100 4
25   50 26 100 4
50   100 26 100 4
75   150 26 100 4
100  200 26 100 4
125  -
",
        "from tab 0 to tab 2; at rest it asks for no frame"
    );
}

#[gpui::test]
fn a_change_of_destination_in_the_middle_of_the_trip_does_not_jump(cx: &mut TestAppContext) {
    let (mut stage, selected) = tabs(cx);
    selected.set(2);
    stage.redraw();
    stage.after(0);
    assert_eq!(stage.after(50), "100 26 100 4");
    selected.set(0);
    stage.redraw();
    assert_eq!(
        stage.trace(&[0, 50, 50]),
        "\
0    100 26 100 4
50   50 26 100 4
100  0 26 100 4
",
        "it heads back from where it was, 100, not from tab 2"
    );
}

#[gpui::test]
fn a_row_opens_into_a_card_that_starts_at_the_rows_bounds(cx: &mut TestAppContext) {
    let (open, painted) = (Rc::new(Cell::new(false)), Painted::default());
    let (_, cx) = cx.add_window_view({
        let (open, painted) = (open.clone(), painted.clone());
        move |_, _| List { open, painted }
    });
    cx.run_until_parked();
    let mut stage = Stage { cx, painted };
    open.set(true);
    stage.redraw();
    assert_eq!(
        stage.trace(&[0, 1, 49, 50]),
        "\
0    0 0 300 40
1    0 1 299 41.5
50   0 50 250 120
100  0 100 200 200
",
        "it moves and really changes size, from the row's 300 by 40 to the card's 200 by 200"
    );
}
