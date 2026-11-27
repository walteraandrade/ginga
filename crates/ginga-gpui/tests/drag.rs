//! A drag, checked on GPUI's test platform: the test moves the pointer and
//! the clock, and delivers the frames.

use std::{cell::Cell, rc::Rc, time::Duration};

use ginga_gpui::{Coast, Drag, DragExt, DragState, Spring};
use gpui::{
    AppContext, Axis, Context, InteractiveElement, IntoElement, Modifiers, MouseButton,
    ParentElement, Pixels, Point, Render, Styled, TestAppContext, VisualTestContext, Window, div,
    point, px,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

fn at(x: f32, y: f32) -> Point<Pixels> {
    point(px(x), px(y))
}

/// A 100 px card at the window's corner, that the pointer drags.
struct Board {
    card: DragState,
    drag: Drag,
}

impl Render for Board {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            div()
                .size(px(100.))
                .draggable(&self.card, self.drag.clone()),
        )
    }
}

fn open_board(drag: Drag, cx: &mut TestAppContext) -> (DragState, &mut VisualTestContext) {
    let card = DragState::default();
    let (_, cx) = cx.add_window_view({
        let card = card.clone();
        move |_, _| Board { card, drag }
    });
    cx.run_until_parked();
    (card, cx)
}

/// Delivers the next frame and returns how many frame requests it answered.
fn next_frame(cx: &mut VisualTestContext) -> usize {
    let requests = cx.update(Window::simulate_next_frame);
    cx.run_until_parked();
    requests
}

/// Delivers frames 16 ms apart until none is asked for, and returns how many.
fn frames_until_still(cx: &mut VisualTestContext) -> usize {
    let mut frames = 0;
    loop {
        cx.executor().advance_clock(ms(16));
        if next_frame(cx) == 0 {
            return frames;
        }
        frames += 1;
        assert!(frames < 500, "it settles");
    }
}

fn press(cx: &mut VisualTestContext, pointer: Point<Pixels>) {
    cx.simulate_mouse_down(pointer, MouseButton::Left, Modifiers::none());
}

/// Moves the pointer, held down, to `pointer` after `after`.
fn move_to(cx: &mut VisualTestContext, pointer: Point<Pixels>, after: Duration) {
    cx.executor().advance_clock(after);
    cx.simulate_mouse_move(pointer, MouseButton::Left, Modifiers::none());
}

fn release(cx: &mut VisualTestContext, pointer: Point<Pixels>) {
    cx.simulate_mouse_up(pointer, MouseButton::Left, Modifiers::none());
}

/// Drags from `from` by `step` every 10 ms, `steps` times: 100 × `step` px/s.
fn throw(cx: &mut VisualTestContext, from: Point<Pixels>, step: Point<Pixels>, steps: u8) {
    press(cx, from);
    let mut pointer = from;
    for _ in 0..steps {
        pointer = point(pointer.x + step.x, pointer.y + step.y);
        move_to(cx, pointer, ms(10));
    }
    release(cx, pointer);
}

#[gpui::test]
fn a_dragged_element_stays_exactly_under_the_pointer(cx: &mut TestAppContext) {
    let (card, cx) = open_board(Drag::new(), cx);
    press(cx, at(50., 50.));
    assert!(card.is_held());
    move_to(cx, at(80., 70.), ms(16));
    assert_eq!(card.offset(), at(30., 20.));
    move_to(cx, at(300., 260.), ms(16));
    assert_eq!(card.offset(), at(250., 210.));
    // Held, it asks for no frames of its own.
    assert_eq!(next_frame(cx), 0);
    // Let go of a still pointer, it stays where it was set down.
    cx.executor().advance_clock(ms(200));
    release(cx, at(300., 260.));
    assert!(!card.is_held());
    assert_eq!(card.rest(), at(250., 210.));
    assert_eq!(frames_until_still(cx), 0);
    // It is drawn there: a press where it now is takes it again, and a press
    // where it was does not.
    press(cx, at(20., 20.));
    assert!(!card.is_held());
    release(cx, at(20., 20.));
    press(cx, at(300., 260.));
    assert!(card.is_held());
}

#[gpui::test]
fn a_released_element_carries_the_pointers_speed_and_coasts_to_a_stop(cx: &mut TestAppContext) {
    let (card, cx) = open_board(Drag::new(), cx);
    // 1,000 px/s right and 500 px/s down: with QUICK's 0.1 s, it rests 100
    // and 50 px on from where it was let go.
    throw(cx, at(50., 50.), at(10., 5.), 5);
    assert_eq!(card.rest(), at(150., 75.));
    next_frame(cx);
    assert_eq!(
        card.offset(),
        at(50., 25.),
        "it leaves from where it was let go"
    );
    cx.executor().advance_clock(ms(100));
    next_frame(cx);
    // One time constant on: 1 − 1/e of the way, 63.2 and 31.6 px.
    let one_on = card.offset();
    assert!((f32::from(one_on.x) - 113.2).abs() < 0.1, "{one_on:?}");
    assert!((f32::from(one_on.y) - 56.6).abs() < 0.1, "{one_on:?}");
    let frames = frames_until_still(cx);
    assert!(frames > 10, "it glides there over frames: {frames}");
    assert_eq!(card.offset(), at(150., 75.));
}

#[gpui::test]
fn a_drag_along_an_axis_leaves_the_other_out(cx: &mut TestAppContext) {
    let (card, cx) = open_board(Drag::new().along(Axis::Horizontal), cx);
    press(cx, at(50., 50.));
    move_to(cx, at(90., 80.), ms(16));
    assert_eq!(card.offset(), at(40., 0.));
    // Pressed again without a release, it goes on from where it is held:
    // 40 + 50, and 100 on from there.
    throw(cx, at(90., 80.), at(10., 10.), 5);
    assert_eq!(card.rest(), at(190., 0.));
}

#[gpui::test]
fn an_element_let_go_past_a_limit_springs_back_inside_it(cx: &mut TestAppContext) {
    let track = Drag::new()
        .along(Axis::Horizontal)
        .within(at(0., 0.), at(200., 0.));
    let (card, cx) = open_board(track, cx);
    // Dragged 60 past the end and let go still.
    press(cx, at(50., 50.));
    move_to(cx, at(310., 50.), ms(16));
    assert_eq!(
        card.offset(),
        at(260., 0.),
        "past the limit, still under the pointer"
    );
    cx.executor().advance_clock(ms(200));
    release(cx, at(310., 50.));
    assert_eq!(card.rest(), at(200., 0.));
    assert!(frames_until_still(cx) > 0);
    assert_eq!(card.offset(), at(200., 0.));
    // Thrown hard toward the start, it stops there too.
    throw(cx, at(250., 50.), at(-20., 0.), 5);
    assert_eq!(card.rest(), at(0., 0.));
    frames_until_still(cx);
    assert_eq!(card.offset(), at(0., 0.));
}

#[gpui::test]
fn with_snap_points_it_comes_to_rest_on_the_nearest(cx: &mut TestAppContext) {
    let places = Drag::new().snap_to([at(0., 0.), at(240., 0.), at(480., 0.)]);
    let (card, cx) = open_board(places, cx);
    // It would coast to 150: 240 is nearer than 0.
    throw(cx, at(50., 50.), at(10., 0.), 5);
    assert_eq!(card.rest(), at(240., 0.));
    frames_until_still(cx);
    assert_eq!(card.offset(), at(240., 0.));
    // Nudged back a little and let go still, it returns to 240.
    press(cx, at(290., 50.));
    move_to(cx, at(250., 50.), ms(16));
    cx.executor().advance_clock(ms(200));
    release(cx, at(250., 50.));
    assert_eq!(card.rest(), at(240., 0.));
    frames_until_still(cx);
    assert_eq!(card.offset(), at(240., 0.));
}

#[gpui::test]
fn under_reduced_motion_a_thrown_element_is_at_rest_at_once(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (card, cx) = open_board(Drag::new(), cx);
    throw(cx, at(50., 50.), at(10., 0.), 5);
    assert_eq!(card.rest(), at(150., 0.));
    assert_eq!(next_frame(cx), 0, "no frames asked for");
    assert_eq!(card.offset(), at(150., 0.));
}

#[gpui::test]
fn a_throw_without_a_pointer_moves_as_a_pointers_would(cx: &mut TestAppContext) {
    let tray = Drag::new().within(at(0., 0.), at(300., 300.));
    let (card, cx) = open_board(tray.clone(), cx);
    cx.update(|window, cx| {
        card.throw(at(1_000., 6_000.), &tray, cx);
        window.refresh();
    });
    // 100 on, and down past the tray's floor: back to 300.
    assert_eq!(card.rest(), at(100., 300.));
    assert!(frames_until_still(cx) > 10);
    assert_eq!(card.offset(), at(100., 300.));
}

#[gpui::test]
fn a_drag_along_the_vertical_axis_leaves_x_out(cx: &mut TestAppContext) {
    let (card, cx) = open_board(Drag::new().along(Axis::Vertical), cx);
    throw(cx, at(50., 50.), at(10., 10.), 5);
    assert_eq!(card.rest(), at(0., 150.));
}

#[gpui::test]
fn under_reduced_motion_a_limit_still_decides_where_it_rests(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let walls = Drag::new().within(at(0., 0.), at(80., 80.));
    let (card, cx) = open_board(walls, cx);
    throw(cx, at(50., 50.), at(10., 10.), 5);
    assert_eq!(next_frame(cx), 0);
    assert_eq!(card.offset(), at(80., 80.));
}

#[gpui::test]
fn under_reduced_motion_a_snap_point_still_decides_where_it_rests(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let stops = Drag::new().snap_to([at(0., 0.), at(240., 0.)]);
    let (card, cx) = open_board(stops, cx);
    throw(cx, at(50., 50.), at(10., 0.), 5);
    assert_eq!(next_frame(cx), 0);
    assert_eq!(card.offset(), at(240., 0.));
}

#[gpui::test]
fn a_coasting_element_is_caught_where_it_is_drawn(cx: &mut TestAppContext) {
    let (card, cx) = open_board(Drag::new(), cx);
    throw(cx, at(50., 50.), at(10., 0.), 5);
    next_frame(cx);
    cx.executor().advance_clock(ms(50));
    next_frame(cx);
    let drawn = card.offset();
    assert!(drawn.x > px(50.) && drawn.x < px(150.), "{drawn:?}");
    // Pressed where it is now, it stops there, under the pointer.
    let pointer = point(drawn.x + px(50.), px(50.));
    press(cx, pointer);
    assert!(card.is_held());
    assert_eq!(card.offset(), drawn);
    assert_eq!(card.rest(), drawn);
}

#[gpui::test]
fn a_move_with_the_button_up_lets_the_element_go(cx: &mut TestAppContext) {
    let (card, cx) = open_board(Drag::new(), cx);
    press(cx, at(50., 50.));
    move_to(cx, at(80., 50.), ms(16));
    // The release was lost outside the window; the pointer comes back up.
    cx.executor().advance_clock(ms(200));
    cx.simulate_mouse_move(at(300., 300.), None, Modifiers::none());
    assert!(!card.is_held());
    assert_eq!(card.rest(), at(30., 0.));
}

/// Two cards, one on top of the other.
struct Stack {
    under: DragState,
    over: DragState,
}

impl Render for Stack {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let card = || div().absolute().size(px(100.));
        div()
            .size_full()
            .child(card().draggable(&self.under, Drag::new()))
            .child(card().draggable(&self.over, Drag::new()))
    }
}

#[gpui::test]
fn only_the_topmost_element_under_the_pointer_is_taken(cx: &mut TestAppContext) {
    let (under, over) = (DragState::default(), DragState::default());
    let (_, cx) = cx.add_window_view({
        let (under, over) = (under.clone(), over.clone());
        move |_, _| Stack { under, over }
    });
    cx.run_until_parked();
    press(cx, at(50., 50.));
    assert!(over.is_held());
    assert!(!under.is_held());
}

/// A card with a button in its corner that takes its own presses.
struct CardWithButton {
    card: DragState,
    pressed: Rc<Cell<u32>>,
}

impl Render for CardWithButton {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let pressed = self.pressed.clone();
        div().size_full().child(
            div()
                .size(px(100.))
                .child(
                    div()
                        .size(px(20.))
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            pressed.set(pressed.get() + 1);
                            cx.stop_propagation();
                        }),
                )
                .draggable(&self.card, Drag::new()),
        )
    }
}

#[gpui::test]
fn a_button_inside_that_takes_the_press_keeps_the_element_from_being_taken(
    cx: &mut TestAppContext,
) {
    let (card, pressed) = (DragState::default(), Rc::new(Cell::new(0)));
    let (_, cx) = cx.add_window_view({
        let (card, pressed) = (card.clone(), pressed.clone());
        move |_, _| CardWithButton { card, pressed }
    });
    cx.run_until_parked();
    press(cx, at(10., 10.));
    assert_eq!(pressed.get(), 1);
    assert!(!card.is_held(), "the button took the press");
    release(cx, at(10., 10.));
    press(cx, at(60., 60.));
    assert!(card.is_held(), "outside the button the card is taken");
}

/// A card that starts away from its place, `absolute`, and a heavy view
/// beside it that is cached and counts its renders.
struct Placed {
    card: DragState,
    heavy: gpui::Entity<Heavy>,
}

struct Heavy {
    renders: Rc<Cell<u32>>,
}

impl Render for Heavy {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        div().size(px(10.))
    }
}

impl Render for Placed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(div().absolute().size(px(100.)).draggable(
                &self.card,
                Drag::new().coast(Coast::SLOW.spring(Spring::BOUNCY)),
            ))
            .child(self.heavy.clone().cached(gpui::StyleRefinement::default()))
    }
}

#[gpui::test]
fn an_absolute_element_moves_from_its_parents_corner_and_only_its_view_is_drawn(
    cx: &mut TestAppContext,
) {
    let renders = Rc::new(Cell::new(0));
    let card = DragState::at(at(200., 100.));
    let (_, cx) = cx.add_window_view({
        let (card, renders) = (card.clone(), renders.clone());
        move |_, cx| Placed {
            card,
            heavy: cx.new(|_| Heavy { renders }),
        }
    });
    cx.run_until_parked();
    // Drawn at its offset from the corner: a press there takes it.
    press(cx, at(20., 20.));
    assert!(!card.is_held());
    release(cx, at(20., 20.));
    let before = renders.get();
    press(cx, at(250., 150.));
    assert!(card.is_held());
    for step in 1..=5u8 {
        move_to(cx, at(250. + f32::from(step) * 10., 150.), ms(10));
        next_frame(cx);
    }
    release(cx, at(300., 150.));
    frames_until_still(cx);
    assert_eq!(
        renders.get(),
        before,
        "the cached view beside it is not drawn again"
    );
}

/// A card placed by its own style, at 100, 40, that starts 20 px right of it.
struct OwnPlace {
    card: DragState,
}

impl Render for OwnPlace {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().relative().size_full().child(
            div()
                .absolute()
                .left(px(100.))
                .top(px(40.))
                .size(px(20.))
                .draggable(&self.card, Drag::new()),
        )
    }
}

#[gpui::test]
fn the_offset_adds_to_an_elements_own_place(cx: &mut TestAppContext) {
    let card = DragState::at(at(20., 0.));
    let (_, cx) = cx.add_window_view({
        let card = card.clone();
        move |_, _| OwnPlace { card }
    });
    cx.run_until_parked();
    press(cx, at(25., 45.));
    assert!(
        !card.is_held(),
        "not at its parent's corner plus the offset"
    );
    release(cx, at(25., 45.));
    press(cx, at(125., 45.));
    assert!(card.is_held(), "taken at its own place plus the offset");
}
