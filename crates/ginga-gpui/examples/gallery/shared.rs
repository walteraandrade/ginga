//! One thing in two places: the mark under the selected tab slides to the
//! next tab, and a row opens into a card further down, growing as it goes.

use std::time::Duration;

use ginga_gpui::{Easing, SharedExt, Tween};
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb,
};

const TABS: [&str; 3] = ["Inbox", "Sent", "Drafts"];

pub struct Shared {
    tab: usize,
    open: bool,
}

impl Shared {
    pub fn new() -> Self {
        Self {
            tab: 0,
            open: false,
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.tab = (self.tab + 1) % TABS.len();
        self.open = !self.open;
        cx.notify();
    }
}

fn glide() -> Tween {
    Tween::new(Duration::from_millis(320), Easing::InOut)
}

impl Render for Shared {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let tabs = div()
            .flex()
            .children(TABS.iter().enumerate().map(|(tab, name)| {
                div()
                    .id(tab)
                    .w(px(96.))
                    .h(px(36.))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .child(name.to_string())
                    .children((tab == self.tab).then(|| {
                        // The line to copy: the same key in both places.
                        div()
                            .h(px(3.))
                            .w_full()
                            .bg(rgb(0x0061_afef))
                            .shared("mark", glide())
                    }))
            }));
        let item = |w: f32, h: f32| {
            div()
                .w(px(w))
                .h(px(h))
                .rounded(px(8.))
                .bg(rgb(0x0030_a46c))
                .shared("item", glide())
        };
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(tabs)
            .child(
                div()
                    .id("row")
                    .h(px(40.))
                    .w(px(288.))
                    .children((!self.open).then(|| item(288., 40.))),
            )
            .child(
                div()
                    .id("card")
                    .h(px(160.))
                    .children(self.open.then(|| item(192., 160.))),
            )
    }
}
