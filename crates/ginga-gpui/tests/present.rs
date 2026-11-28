//! Ways in and out declared on an element, checked on GPUI's test platform.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use ginga_gpui::{Easing, InOut, Look, Swap, Tween, present};
use gpui::{
    Bounds, Context, FocusHandle, InteractiveElement, IntoElement, Modifiers, ParentElement,
    Pixels, Render, StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext, Window,
    canvas, div, point, px,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// What each content drew in the last frame: its name and where.
type Drawn = Rc<RefCell<Vec<(&'static str, Bounds<Pixels>)>>>;

/// A 50 px square for each content, which counts its clicks and tracks a
/// focus handle. It arrives from 40 px down and leaves to 40 px up, on
/// 100 ms linear tweens so the places are exact.
struct Stage {
    wanted: Rc<Cell<Option<&'static str>>>,
    swap: Swap,
    /// What the arriving and leaving looks do.
    ways: Ways,
    /// How long each way waits before it starts.
    delay: Duration,
    drawn: Drawn,
    clicks: Rc<Cell<u32>>,
    focus: FocusHandle,
    /// The contents found gone, in the order they went.
    gone: Gone,
    /// How many times the view rendered.
    renders: Rc<Cell<u32>>,
    /// Draws no `present` at all.
    hidden: bool,
    /// Renders again when content is gone, as a view that changes then does.
    renders_on_gone: bool,
}

/// What a stage's ways in and out do.
#[derive(Clone, Copy, PartialEq)]
enum Ways {
    /// No arriving or leaving look: both ways are instant.
    Instant,
    /// Arrive from 40 px down, leave to 40 px up.
    Move,
    /// Move, and fade from and to unseen too.
    MoveAndFade,
}

/// The contents a stage found gone.
type Gone = Rc<RefCell<Vec<&'static str>>>;

impl Render for Stage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        let fade = Tween::new(ms(100), Easing::Linear).delay(self.delay);
        let (drawn, clicks, focus) = (self.drawn.clone(), self.clicks.clone(), self.focus.clone());
        let shown = present(
            "stage",
            self.wanted.get(),
            InOut::new(fade, fade),
            move |name: &&'static str| {
                let (name, drawn, clicks) = (*name, drawn.clone(), clicks.clone());
                div()
                    .id(name)
                    .size(px(50.))
                    .track_focus(&focus)
                    .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
                    .child(
                        canvas(
                            move |bounds, _, _| drawn.borrow_mut().push((name, bounds)),
                            |_, (), _, _| {},
                        )
                        .size_full(),
                    )
            },
        )
        .swap(self.swap)
        .on_gone({
            let view = cx.entity().downgrade();
            move |name, _, cx| {
                view.update(cx, |this, cx| {
                    this.gone.borrow_mut().push(name);
                    if this.renders_on_gone {
                        cx.notify();
                    }
                })
                .ok();
            }
        });
        let unseen = if self.ways == Ways::MoveAndFade {
            Look::new().opacity(0.)
        } else {
            Look::new()
        };
        let shown = if self.ways == Ways::Instant {
            shown
        } else {
            shown
                .arrive_from(Look::new().offset(0., 40.).or(unseen))
                .leave_to(Look::new().offset(0., -40.).or(unseen))
        };
        div()
            .size_full()
            .pt(px(100.))
            .children((!self.hidden).then_some(shown))
    }
}

struct Opened<'a> {
    cx: &'a mut VisualTestContext,
    view: gpui::Entity<Stage>,
    wanted: Rc<Cell<Option<&'static str>>>,
    drawn: Drawn,
    clicks: Rc<Cell<u32>>,
    focus: FocusHandle,
    gone: Gone,
    renders: Rc<Cell<u32>>,
}

fn open<'a>(cx: &'a mut TestAppContext, first: Option<&'static str>, swap: Swap) -> Opened<'a> {
    open_with(cx, first, swap, Ways::Move, Duration::ZERO)
}

fn open_with<'a>(
    cx: &'a mut TestAppContext,
    first: Option<&'static str>,
    swap: Swap,
    ways: Ways,
    delay: Duration,
) -> Opened<'a> {
    let wanted = Rc::new(Cell::new(first));
    let (drawn, clicks, gone, renders) = (
        Drawn::default(),
        Rc::new(Cell::new(0)),
        Gone::default(),
        Rc::new(Cell::new(0)),
    );
    let focus = cx.update(|cx| cx.focus_handle());
    let (view, cx) = cx.add_window_view({
        let (wanted, drawn, clicks, focus, gone, renders) = (
            wanted.clone(),
            drawn.clone(),
            clicks.clone(),
            focus.clone(),
            gone.clone(),
            renders.clone(),
        );
        move |_, _| Stage {
            wanted,
            swap,
            ways,
            delay,
            drawn,
            clicks,
            focus,
            gone,
            renders,
            hidden: false,
            renders_on_gone: false,
        }
    });
    cx.run_until_parked();
    Opened {
        cx,
        view,
        wanted,
        drawn,
        clicks,
        focus,
        gone,
        renders,
    }
}

impl Opened<'_> {
    /// Each content drawn, with how far its top is from its place, after the
    /// clock moves `millis` and a frame is drawn.
    fn after(&mut self, millis: u64) -> Vec<(&'static str, f32)> {
        self.drawn.borrow_mut().clear();
        self.cx.executor().advance_clock(ms(millis));
        self.cx.update(Window::simulate_next_frame);
        self.cx.run_until_parked();
        self.last()
    }

    /// What the last frame drew, with how far each top is from its place.
    fn last(&self) -> Vec<(&'static str, f32)> {
        self.drawn
            .borrow()
            .iter()
            .map(|(name, bounds)| (*name, f32::from(bounds.origin.y) - 100.))
            .collect()
    }

    /// The contents found gone so far, in the order they went.
    fn gone(&self) -> Vec<&'static str> {
        self.gone.borrow().clone()
    }

    /// Wants `content` from the next frame on.
    fn want(&mut self, content: Option<&'static str>) {
        self.wanted.set(content);
        self.drawn.borrow_mut().clear();
        self.cx.update(|window, _| window.refresh());
        self.cx.run_until_parked();
    }
}

#[gpui::test]
fn content_with_an_arriving_look_animates_in_when_it_first_appears(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("toast"), Swap::Sync);
    assert_eq!(stage.after(0), [("toast", 40.)]);
    assert_eq!(stage.after(50), [("toast", 20.)]);
    assert_eq!(stage.after(50), [("toast", 0.)]);
    assert_eq!(stage.cx.update(Window::simulate_next_frame), 0, "settled");
}

#[gpui::test]
fn no_longer_wanted_it_plays_its_way_out_and_is_gone(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("toast"), Swap::Sync);
    stage.after(100);
    stage.want(None);
    assert_eq!(stage.after(0), [("toast", 0.)]);
    assert_eq!(stage.after(50), [("toast", -20.)]);
    assert_eq!(stage.after(50), []);
    assert_eq!(stage.after(100), [], "and stays gone");
}

#[gpui::test]
fn wanted_again_on_its_way_out_it_turns_around_from_where_it_is(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("toast"), Swap::Sync);
    stage.after(100);
    stage.want(None);
    assert_eq!(stage.after(50), [("toast", -20.)]);
    stage.want(Some("toast"));
    // It comes back the way it was leaving, from where it is: no jump.
    assert_eq!(stage.after(0), [("toast", -20.)]);
    assert_eq!(stage.after(50), [("toast", -10.)]);
    assert_eq!(stage.after(50), [("toast", 0.)]);
}

#[gpui::test]
fn on_its_way_out_it_takes_no_clicks_and_no_focus(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("button"), Swap::Sync);
    stage.after(100);
    let inside = point(px(25.), px(125.));
    stage.cx.simulate_click(inside, Modifiers::none());
    assert_eq!(stage.clicks.get(), 1, "while here it takes a click");
    let focus = stage.focus.clone();
    stage.cx.update(Window::blur);
    stage.want(None);
    stage.after(10);
    stage
        .cx
        .simulate_click(point(px(25.), px(121.)), Modifiers::none());
    assert_eq!(stage.clicks.get(), 1, "leaving, it takes none");
    assert!(!stage.cx.update(|window, _| focus.is_focused(window)));
}

#[gpui::test]
fn in_sync_both_are_drawn_and_waiting_the_new_one_holds_back(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("old"), Swap::Sync);
    stage.after(100);
    stage.want(Some("new"));
    assert_eq!(stage.after(50), [("old", -20.), ("new", 20.)]);

    let mut stage = open(cx, Some("old"), Swap::Wait);
    stage.after(100);
    stage.want(Some("new"));
    assert_eq!(stage.after(50), [("old", -20.)], "new waits, unseen");
    assert_eq!(stage.after(100), [("new", 20.)]);
}

#[gpui::test]
fn content_that_has_the_focus_loses_it_when_its_way_out_starts(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("field"), Swap::Sync);
    stage.after(100);
    let focus = stage.focus.clone();
    stage.cx.update(|window, cx| window.focus(&focus, cx));
    assert!(stage.cx.update(|window, _| focus.is_focused(window)));
    stage.want(None);
    stage.after(10);
    stage.after(10);
    assert!(
        !stage.cx.update(|window, _| focus.is_focused(window)),
        "the focus left with it"
    );
}

#[gpui::test]
fn without_looks_both_ways_are_instant(cx: &mut TestAppContext) {
    let mut stage = open_with(cx, Some("toast"), Swap::Sync, Ways::Instant, Duration::ZERO);
    assert_eq!(stage.last(), [("toast", 0.)], "there at once");
    assert_eq!(stage.cx.update(Window::simulate_next_frame), 0);
    stage.want(None);
    assert_eq!(stage.last(), [], "gone at once");
}

#[gpui::test]
fn without_looks_a_swap_that_waits_is_instant_too(cx: &mut TestAppContext) {
    let mut stage = open_with(cx, Some("old"), Swap::Wait, Ways::Instant, Duration::ZERO);
    stage.want(Some("new"));
    assert_eq!(stage.last(), [("new", 0.)], "the old one left at once");
}

#[gpui::test]
fn under_reduced_motion_each_way_keeps_its_delay(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let mut stage = open_with(cx, Some("toast"), Swap::Sync, Ways::Move, ms(60));
    assert_eq!(
        stage.after(0),
        [("toast", 40.)],
        "it waits where it arrives from"
    );
    assert_eq!(stage.after(50), [("toast", 40.)]);
    assert_eq!(stage.after(10), [("toast", 0.)], "then is in place at once");
    stage.want(None);
    assert_eq!(
        stage.after(50),
        [("toast", 0.)],
        "still there while its way out waits"
    );
    assert_eq!(stage.after(10), [], "then gone at once");
}

#[gpui::test]
fn under_reduced_motion_it_is_there_and_gone_at_once(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let mut stage = open(cx, Some("toast"), Swap::Sync);
    assert_eq!(stage.last(), [("toast", 0.)]);
    stage.want(None);
    assert_eq!(stage.last(), []);
}

#[gpui::test]
fn when_its_way_out_is_over_it_says_so_once(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("toast"), Swap::Sync);
    stage.after(100);
    stage.want(None);
    stage.after(50);
    assert_eq!(stage.gone().len(), 0, "still leaving");
    assert_eq!(stage.after(49).len(), 1, "drawn at 99 ms");
    assert_eq!(stage.gone().len(), 0);
    assert_eq!(stage.after(1), []);
    assert_eq!(stage.gone(), ["toast"], "the frame at 100 ms");
    stage.after(100);
    stage.want(None);
    stage.after(16);
    assert_eq!(stage.gone().len(), 1, "and never again");
}

#[gpui::test]
fn wanted_again_on_its_way_out_it_is_not_gone(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("toast"), Swap::Sync);
    stage.after(100);
    stage.want(None);
    stage.after(50);
    stage.want(Some("toast"));
    stage.after(50);
    stage.after(100);
    assert_eq!(stage.gone().len(), 0);
}

#[gpui::test]
fn an_instant_way_out_is_over_on_the_frame_that_starts_it(cx: &mut TestAppContext) {
    let mut stage = open_with(cx, Some("toast"), Swap::Sync, Ways::Instant, Duration::ZERO);
    stage.after(16);
    stage.want(None);
    assert_eq!(stage.last(), []);
    assert_eq!(stage.gone().len(), 1);
}

#[gpui::test]
fn under_reduced_motion_a_way_out_is_over_at_once(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let mut stage = open(cx, Some("toast"), Swap::Sync);
    stage.after(16);
    stage.want(None);
    assert_eq!(stage.gone().len(), 1);
}

#[gpui::test]
fn each_content_a_swap_sends_away_is_gone_once(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("one"), Swap::Sync);
    stage.after(100);
    stage.want(Some("two"));
    stage.after(30);
    stage.want(Some("three"));
    stage.after(30);
    assert_eq!(stage.gone().len(), 0, "two leave, one arrives");
    stage.after(40);
    assert_eq!(stage.gone(), ["one"], "the first is gone at 100 ms");
    stage.after(30);
    assert_eq!(
        stage.gone(),
        ["one", "two"],
        "the second 100 ms after it left"
    );
    // Drawn again, still wanted: what is here is not gone.
    stage.want(Some("three"));
    stage.after(100);
    assert_eq!(stage.gone().len(), 2, "the one here is not gone");
}

#[gpui::test]
fn content_held_back_by_a_swap_that_waits_is_not_gone(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("one"), Swap::Wait);
    stage.after(100);
    stage.want(Some("two"));
    stage.after(50);
    stage.after(60);
    assert_eq!(stage.gone().len(), 1, "only the one that left");
    stage.after(200);
    assert_eq!(stage.gone().len(), 1);
}

#[gpui::test]
fn two_contents_gone_by_one_frame_are_each_gone(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("one"), Swap::Sync);
    stage.after(100);
    stage.want(Some("two"));
    stage.after(30);
    stage.want(Some("three"));
    stage.after(200);
    assert_eq!(stage.gone(), ["one", "two"]);
}

#[gpui::test]
fn content_a_wait_drops_unseen_is_not_gone(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("one"), Swap::Wait);
    stage.after(100);
    stage.want(Some("two"));
    stage.after(30);
    stage.want(Some("three"));
    stage.after(30);
    stage.want(None);
    stage.after(200);
    assert_eq!(stage.gone(), ["one"], "two and three were never seen");
}

#[gpui::test]
fn a_fourth_swap_while_two_leave_cuts_the_oldest_short_and_it_is_gone(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("one"), Swap::Sync);
    stage.after(100);
    for name in ["two", "three", "four"] {
        stage.want(Some(name));
        stage.after(10);
    }
    assert_eq!(stage.gone(), ["one"], "cut short at 20 ms");
}

#[gpui::test]
fn content_there_when_the_element_stops_being_drawn_is_not_gone(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("toast"), Swap::Sync);
    stage.after(100);
    stage.want(None);
    stage.after(50);
    stage.view.update(stage.cx, |this, cx| {
        this.hidden = true;
        cx.notify();
    });
    stage.after(200);
    assert_eq!(stage.gone(), Vec::<&str>::new());
}

#[gpui::test]
fn what_it_changes_when_content_is_gone_draws_the_next_frame(cx: &mut TestAppContext) {
    let mut stage = open(cx, Some("toast"), Swap::Sync);
    stage
        .view
        .update(stage.cx, |this, _| this.renders_on_gone = true);
    stage.after(100);
    stage.want(None);
    stage.after(50);
    let renders = stage.renders.get();
    stage.after(50);
    assert_eq!(stage.gone(), ["toast"]);
    // The frame that finds it gone, and the one the view's change asks for:
    // a change made while the frame is drawn would ask for none.
    assert_eq!(stage.renders.get(), renders + 2);
}

#[gpui::test]
fn under_reduced_motion_a_way_that_fades_keeps_its_fade_where_it_is(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let mut stage = open(cx, None, Swap::Sync);
    let view = stage.view.clone();
    stage
        .cx
        .update(|_, cx| view.update(cx, |stage, _| stage.ways = Ways::MoveAndFade));
    stage.want(Some("toast"));
    assert_eq!(
        stage.after(0),
        [("toast", 0.)],
        "in its place, not 40 px down"
    );
    assert_eq!(stage.after(50), [("toast", 0.)]);
    assert!(
        stage.cx.update(Window::simulate_next_frame) > 0,
        "the fade in asks for frames"
    );
    stage.after(50);
    assert_eq!(
        stage.cx.update(Window::simulate_next_frame),
        0,
        "settled at 100 ms"
    );
    stage.want(None);
    assert_eq!(stage.after(50), [("toast", 0.)], "fading out where it is");
    assert_eq!(stage.gone().len(), 0);
    assert_eq!(stage.after(50), [], "gone when the fade is over");
    assert_eq!(stage.gone(), ["toast"]);
}
