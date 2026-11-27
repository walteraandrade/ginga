//! What a presence draws, covered while it leaves: checked on GPUI's test
//! platform with clicks and the keyboard focus.

use std::{cell::Cell, rc::Rc, time::Duration};

use ginga_gpui::{CoverExt, Easing, Presence, Tween};
use gpui::{
    Context, Entity, FocusHandle, InteractiveElement, IntoElement, Modifiers, ParentElement,
    Render, StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext, Window, div,
    point, px,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

fn fade() -> Tween {
    Tween::new(ms(100), Easing::Linear)
}

/// A 50 px panel at the window's corner, shown through a presence and
/// covered while it leaves. It counts its clicks, and a button inside it
/// takes the focus.
struct Page {
    panel: Presence<()>,
    clicks: Rc<Cell<u32>>,
    button: FocusHandle,
}

impl Render for Page {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        let mut page = div().size_full();
        if let Some(shown) = self.panel.frame(window, cx) {
            let panel = div()
                .id("panel")
                .size(px(50.))
                .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
                .child(div().id("button").size(px(10.)).track_focus(&self.button));
            page = page.child(shown.cover(panel));
        }
        page
    }
}

struct Opened<'a> {
    cx: &'a mut VisualTestContext,
    page: Entity<Page>,
    clicks: Rc<Cell<u32>>,
    button: FocusHandle,
}

fn open(cx: &mut TestAppContext) -> Opened<'_> {
    let clicks = Rc::new(Cell::new(0));
    let button = cx.update(|cx| cx.focus_handle());
    let (page, cx) = cx.add_window_view({
        let (clicks, button) = (clicks.clone(), button.clone());
        move |_, _| Page {
            panel: Presence::present(()),
            clicks,
            button,
        }
    });
    cx.run_until_parked();
    Opened {
        cx,
        page,
        clicks,
        button,
    }
}

impl Opened<'_> {
    /// Clicks inside the panel and says whether the panel got it.
    fn clicked(&mut self) -> bool {
        let before = self.clicks.get();
        self.cx
            .simulate_click(point(px(30.), px(30.)), Modifiers::none());
        self.cx.run_until_parked();
        self.clicks.get() > before
    }

    fn change(&mut self, f: impl FnOnce(&mut Presence<()>, &mut Context<Page>)) {
        self.cx.update(|_, cx| {
            self.page.update(cx, |page, cx| {
                f(&mut page.panel, cx);
                cx.notify();
            });
        });
        self.cx.run_until_parked();
    }

    fn focused(&mut self) -> bool {
        let button = self.button.clone();
        self.cx.update(|window, _| button.is_focused(window))
    }
}

#[gpui::test]
fn a_click_reaches_the_panel_until_its_way_out_starts(cx: &mut TestAppContext) {
    let mut page = open(cx);
    assert!(page.clicked(), "at rest, it takes clicks");
    page.change(|panel, cx| panel.hide(fade(), cx));
    assert!(!page.clicked(), "leaving, it takes none");
    page.change(|panel, cx| panel.show((), fade(), cx));
    assert!(page.clicked(), "arriving again, it takes them");
}

#[gpui::test]
fn the_focus_leaves_the_panel_when_its_way_out_starts(cx: &mut TestAppContext) {
    let mut page = open(cx);
    let button = page.button.clone();
    page.cx.update(|window, cx| button.focus(window, cx));
    page.cx.run_until_parked();
    assert!(page.focused());
    page.change(|panel, cx| panel.hide(fade(), cx));
    assert!(!page.focused(), "blurred as it starts to leave");
}
