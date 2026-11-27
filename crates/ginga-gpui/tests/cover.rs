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

/// How the panel is placed.
#[derive(Clone, Copy)]
enum Place {
    /// 50 px, in the flow at the window's corner.
    Flow,
    /// 50 px, `absolute` at (5, 5).
    Absolute,
    /// As big as a 50 px box around it, from `size_full`.
    Full,
}

/// A panel shown through a presence and covered while it leaves. It counts
/// its clicks, and a button inside it takes the focus. Beside it, outside,
/// is another button.
struct Page {
    panel: Presence<()>,
    place: Place,
    clicks: Rc<Cell<u32>>,
    button: FocusHandle,
    outside: FocusHandle,
}

impl Render for Page {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        let mut page = div().size(px(50.)).flex();
        if let Some(shown) = self.panel.frame(window, cx) {
            let panel = div()
                .id("panel")
                .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
                .child(div().id("button").size(px(10.)).track_focus(&self.button));
            let panel = match self.place {
                Place::Flow => panel.size(px(50.)),
                Place::Absolute => panel.absolute().left(px(5.)).top(px(5.)).size(px(50.)),
                Place::Full => panel.size_full(),
            };
            page = page.child(shown.cover(panel));
        }
        div()
            .size_full()
            .child(page)
            .child(div().id("outside").size(px(10.)).track_focus(&self.outside))
    }
}

struct Opened<'a> {
    cx: &'a mut VisualTestContext,
    page: Entity<Page>,
    clicks: Rc<Cell<u32>>,
    button: FocusHandle,
    outside: FocusHandle,
}

fn open(cx: &mut TestAppContext, place: Place) -> Opened<'_> {
    let clicks = Rc::new(Cell::new(0));
    let (button, outside) = cx.update(|cx| (cx.focus_handle(), cx.focus_handle()));
    let (page, cx) = cx.add_window_view({
        let (clicks, button, outside) = (clicks.clone(), button.clone(), outside.clone());
        move |_, _| Page {
            panel: Presence::present(()),
            place,
            clicks,
            button,
            outside,
        }
    });
    cx.run_until_parked();
    Opened {
        cx,
        page,
        clicks,
        button,
        outside,
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

    fn focus(&mut self, handle: &FocusHandle) {
        let handle = handle.clone();
        self.cx.update(|window, cx| handle.focus(window, cx));
        self.cx.run_until_parked();
    }

    fn is_focused(&mut self, handle: &FocusHandle) -> bool {
        let handle = handle.clone();
        self.cx.update(|window, _| handle.is_focused(window))
    }
}

#[gpui::test]
fn a_click_reaches_the_panel_until_its_way_out_starts(cx: &mut TestAppContext) {
    for place in [Place::Flow, Place::Absolute, Place::Full] {
        let mut page = open(cx, place);
        assert!(page.clicked(), "at rest, it takes clicks");
        page.change(|panel, cx| panel.hide(fade(), cx));
        assert!(!page.clicked(), "leaving, it takes none");
        page.change(|panel, cx| panel.show((), fade(), cx));
        assert!(page.clicked(), "arriving again, it takes them");
    }
}

#[gpui::test]
fn the_focus_leaves_the_panel_when_its_way_out_starts(cx: &mut TestAppContext) {
    let mut page = open(cx, Place::Flow);
    let button = page.button.clone();
    page.focus(&button);
    assert!(page.is_focused(&button));
    page.change(|panel, cx| panel.hide(fade(), cx));
    assert!(!page.is_focused(&button), "blurred as it starts to leave");
}

#[gpui::test]
fn a_focus_outside_the_panel_stays_while_it_leaves(cx: &mut TestAppContext) {
    let mut page = open(cx, Place::Flow);
    let outside = page.outside.clone();
    page.focus(&outside);
    page.change(|panel, cx| panel.hide(fade(), cx));
    page.cx.executor().advance_clock(ms(50));
    page.cx.run_until_parked();
    assert!(page.is_focused(&outside));
}
