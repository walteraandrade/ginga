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

/// How the mark sits in its tab.
#[derive(Clone, Copy)]
enum Mark {
    /// In the flow, at the bottom.
    Plain,
    /// In the flow, with a margin above it.
    Margin,
    /// Out of the flow, pinned to the tab's bottom.
    Absolute,
}

/// Three tabs, 100 px wide, side by side; the selected one draws a 4 px mark
/// under its label, shared between the tabs. A 10 px label sits above the
/// mark in every tab, and paints itself into `beside`. No tab is selected
/// when `selected` is 3.
struct Tabs {
    selected: Rc<Cell<usize>>,
    painted: Painted,
    beside: Painted,
    mark: Mark,
}

impl Render for Tabs {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected.get();
        div().flex().children((0..3).map(|tab| {
            div()
                .id(tab)
                .w(px(100.))
                .h(px(30.))
                .relative()
                .flex()
                .flex_col()
                .justify_end()
                .child(div().h(px(10.)).w_full().child(marker(&self.beside)))
                .children((tab == selected).then(|| {
                    let mark = div().h(px(4.)).w_full();
                    let mark = match self.mark {
                        Mark::Plain => mark,
                        Mark::Margin => mark.mt(px(10.)),
                        Mark::Absolute => mark.absolute().bottom_0().left_0(),
                    };
                    mark.child(marker(&self.painted)).shared("mark", glide())
                }))
        }))
    }
}

/// One box that layout can move: what is before it is `left` wide.
struct Slide {
    left: Rc<Cell<f32>>,
    painted: Painted,
}

impl Render for Slide {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().pl(px(self.left.get())).child(
            div()
                .w(px(50.))
                .h(px(10.))
                .child(marker(&self.painted))
                .shared("box", glide()),
        )
    }
}

/// A row in a list that opens into a card further down: the same thing.
/// The card is as wide as the box it is in, which is `card` wide.
struct List {
    open: Rc<Cell<bool>>,
    card: Rc<Cell<f32>>,
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
                div().id("card").w(px(self.card.get())).child(
                    div()
                        .w_full()
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

fn text(bounds: Option<&Bounds<Pixels>>) -> String {
    bounds.map_or(String::from("-"), |b| {
        format!(
            "{} {} {} {}",
            f32::from(b.origin.x),
            f32::from(b.origin.y),
            f32::from(b.size.width),
            f32::from(b.size.height)
        )
    })
}

impl Stage<'_> {
    /// The thing's bounds after the clock moves `millis` and a frame is
    /// drawn, as `x y width height`, or `-` when it was not painted.
    fn after(&mut self, millis: u64) -> String {
        self.painted.borrow_mut().clear();
        self.cx.executor().advance_clock(ms(millis));
        self.cx.update(Window::simulate_next_frame);
        self.cx.run_until_parked();
        text(self.painted.borrow().last())
    }

    /// The thing's bounds in the frame drawn now, with no time passing: the
    /// frame that follows a change in the view.
    fn redraw(&mut self) -> String {
        self.painted.borrow_mut().clear();
        self.cx.update(|window, _| window.refresh());
        self.cx.run_until_parked();
        text(self.painted.borrow().last())
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
    let (stage, selected, _) = tabs_with(cx, Mark::Plain);
    (stage, selected)
}

/// The tabs, with the mark under tab 0, and where the labels were painted.
fn tabs_with(cx: &mut TestAppContext, mark: Mark) -> (Stage<'_>, Rc<Cell<usize>>, Painted) {
    let (selected, painted, beside) = (
        Rc::new(Cell::new(0)),
        Painted::default(),
        Painted::default(),
    );
    let (_, cx) = cx.add_window_view({
        let (selected, painted, beside) = (selected.clone(), painted.clone(), beside.clone());
        move |_, _| Tabs {
            selected,
            painted,
            beside,
            mark,
        }
    });
    cx.run_until_parked();
    (Stage { cx, painted }, selected, beside)
}

#[gpui::test]
fn the_mark_of_the_selected_tab_travels_to_the_next_one(cx: &mut TestAppContext) {
    let (mut stage, selected) = tabs(cx);
    assert_eq!(stage.after(16), "-", "nothing moves: no frame asked for");
    // The frame the new mark appears in measures where it goes and draws
    // nothing; each frame after it draws the mark on its way.
    selected.set(2);
    assert_eq!(stage.redraw(), "-", "the frame it appears in only measures");
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
    assert_eq!(stage.redraw(), "-");
    stage.after(0);
    assert_eq!(stage.after(50), "100 26 100 4");
    selected.set(0);
    assert_eq!(stage.redraw(), "-");
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

fn list(cx: &mut TestAppContext) -> (Stage<'_>, Rc<Cell<bool>>, Rc<Cell<f32>>) {
    let (open, card, painted) = (
        Rc::new(Cell::new(false)),
        Rc::new(Cell::new(200.)),
        Painted::default(),
    );
    let (_, cx) = cx.add_window_view({
        let (open, card, painted) = (open.clone(), card.clone(), painted.clone());
        move |_, _| List {
            open,
            card,
            painted,
        }
    });
    cx.run_until_parked();
    (Stage { cx, painted }, open, card)
}

#[gpui::test]
fn a_row_opens_into_a_card_that_starts_at_the_rows_bounds(cx: &mut TestAppContext) {
    let (mut stage, open, _) = list(cx);
    open.set(true);
    assert_eq!(stage.redraw(), "-");
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

/// Every label's bounds while the mark travels, one line per frame: the
/// same on every line when nothing around the mark moves.
fn labels_while_it_travels(cx: &mut TestAppContext, mark: Mark) -> (String, Vec<String>) {
    let (mut stage, selected, beside) = tabs_with(cx, mark);
    selected.set(2);
    let mut marks = String::new();
    let mut labels = Vec::new();
    stage.redraw();
    for _ in 0..5 {
        // A frame that is not drawn paints no label, and adds an empty line.
        beside.borrow_mut().clear();
        let at = stage.after(25);
        writeln!(marks, "{at}").unwrap();
        labels.push(
            beside
                .borrow()
                .iter()
                .map(|b| text(Some(b)))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    (marks, labels)
}

#[gpui::test]
fn its_place_keeps_its_margin_so_nothing_around_it_moves(cx: &mut TestAppContext) {
    let (marks, labels) = labels_while_it_travels(cx, Mark::Margin);
    assert_eq!(
        marks, "50 26 100 4\n100 26 100 4\n150 26 100 4\n200 26 100 4\n-\n",
        "the mark keeps its height on the way"
    );
    assert!(
        labels[..4]
            .iter()
            .all(|line| *line == "0 20 100 10, 100 20 100 10, 200 6 100 10"),
        "the label in tab 2 stays above the mark's margin: {labels:#?}"
    );
}

#[gpui::test]
fn an_absolute_mark_takes_no_room_on_its_way_either(cx: &mut TestAppContext) {
    let (marks, labels) = labels_while_it_travels(cx, Mark::Absolute);
    assert_eq!(
        marks,
        "50 26 100 4\n100 26 100 4\n150 26 100 4\n200 26 100 4\n-\n"
    );
    assert!(
        labels[..4]
            .iter()
            .all(|line| *line == "0 20 100 10, 100 20 100 10, 200 20 100 10"),
        "the labels stay at the bottom, with no mark in the flow: {labels:#?}"
    );
}

#[gpui::test]
fn under_reduced_motion_it_is_in_its_new_place_at_once(cx: &mut TestAppContext) {
    let (mut stage, selected) = tabs(cx);
    stage.cx.update(|_, cx| cx.set_reduce_motion(true));
    selected.set(2);
    assert_eq!(stage.redraw(), "200 26 100 4");
    assert_eq!(stage.after(16), "-", "and asks for no frame");
}

#[gpui::test]
fn a_key_whose_last_element_went_frames_ago_is_no_place_to_come_from(cx: &mut TestAppContext) {
    let (mut stage, selected) = tabs(cx);
    selected.set(3);
    stage.redraw();
    stage.after(16);
    stage.after(16);
    selected.set(2);
    assert_eq!(stage.redraw(), "200 26 100 4", "it appears in its place");
    assert_eq!(stage.after(16), "-");
}

#[gpui::test]
fn a_settled_thing_that_layout_moves_takes_its_new_place_at_once(cx: &mut TestAppContext) {
    let (left, painted) = (Rc::new(Cell::new(0.)), Painted::default());
    let (_, cx) = cx.add_window_view({
        let (left, painted) = (left.clone(), painted.clone());
        move |_, _| Slide { left, painted }
    });
    cx.run_until_parked();
    let mut stage = Stage { cx, painted };
    left.set(50.);
    assert_eq!(
        stage.redraw(),
        "50 0 50 10",
        "a move of its place is no trip"
    );
    assert_eq!(stage.after(16), "-");
}

#[gpui::test]
fn a_second_window_does_not_take_its_marks_from_the_first(cx: &mut TestAppContext) {
    let _first = tabs(cx);
    let (selected, painted, beside) = (
        Rc::new(Cell::new(2)),
        Painted::default(),
        Painted::default(),
    );
    let (_, second) = cx.add_window_view({
        let (selected, painted, beside) = (selected.clone(), painted.clone(), beside.clone());
        move |_, _| Tabs {
            selected,
            painted,
            beside,
            mark: Mark::Plain,
        }
    });
    second.run_until_parked();
    let mut stage = Stage {
        cx: second,
        painted,
    };
    assert_eq!(
        text(stage.painted.borrow().last()),
        "200 26 100 4",
        "its first frame draws the mark in place"
    );
    assert_eq!(stage.after(16), "-", "and nothing travels");
}

#[gpui::test]
fn a_card_that_changes_size_on_the_way_is_reached_with_no_jump(cx: &mut TestAppContext) {
    let (mut stage, open, card) = list(cx);
    open.set(true);
    stage.redraw();
    assert_eq!(stage.after(50), "0 50 250 120");
    card.set(100.);
    assert_eq!(
        stage.trace(&[0, 50, 50]),
        "\
0    0 50 250 120
50   0 75 175 160
100  0 100 100 200
",
        "it heads for the card's new width from where it is, and lands on it"
    );
    assert_eq!(stage.after(16), "-");
}
