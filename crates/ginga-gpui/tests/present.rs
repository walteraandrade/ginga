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
    /// Without the arriving and leaving looks, both ways are instant.
    looks: bool,
    drawn: Drawn,
    clicks: Rc<Cell<u32>>,
    focus: FocusHandle,
}

impl Render for Stage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let fade = Tween::new(ms(100), Easing::Linear);
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
        .swap(self.swap);
        let shown = if self.looks {
            shown
                .arrive_from(Look::new().offset(0., 40.))
                .leave_to(Look::new().offset(0., -40.))
        } else {
            shown
        };
        div().size_full().pt(px(100.)).child(shown)
    }
}

struct Opened<'a> {
    cx: &'a mut VisualTestContext,
    wanted: Rc<Cell<Option<&'static str>>>,
    drawn: Drawn,
    clicks: Rc<Cell<u32>>,
    focus: FocusHandle,
}

fn open<'a>(cx: &'a mut TestAppContext, first: Option<&'static str>, swap: Swap) -> Opened<'a> {
    open_with(cx, first, swap, true)
}

fn open_with<'a>(
    cx: &'a mut TestAppContext,
    first: Option<&'static str>,
    swap: Swap,
    looks: bool,
) -> Opened<'a> {
    let wanted = Rc::new(Cell::new(first));
    let (drawn, clicks) = (Drawn::default(), Rc::new(Cell::new(0)));
    let focus = cx.update(|cx| cx.focus_handle());
    let (_, cx) = cx.add_window_view({
        let (wanted, drawn, clicks, focus) =
            (wanted.clone(), drawn.clone(), clicks.clone(), focus.clone());
        move |_, _| Stage {
            wanted,
            swap,
            looks,
            drawn,
            clicks,
            focus,
        }
    });
    cx.run_until_parked();
    Opened {
        cx,
        wanted,
        drawn,
        clicks,
        focus,
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
    let mut stage = open_with(cx, Some("toast"), Swap::Sync, false);
    assert_eq!(stage.last(), [("toast", 0.)], "there at once");
    assert_eq!(stage.cx.update(Window::simulate_next_frame), 0);
    stage.want(None);
    assert_eq!(stage.last(), [], "gone at once");
}

#[gpui::test]
fn without_looks_a_swap_that_waits_is_instant_too(cx: &mut TestAppContext) {
    let mut stage = open_with(cx, Some("old"), Swap::Wait, false);
    stage.want(Some("new"));
    assert_eq!(stage.last(), [("new", 0.)], "the old one left at once");
}

#[gpui::test]
fn under_reduced_motion_it_is_there_and_gone_at_once(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let mut stage = open(cx, Some("toast"), Swap::Sync);
    assert_eq!(stage.last(), [("toast", 0.)]);
    stage.want(None);
    assert_eq!(stage.last(), []);
}
