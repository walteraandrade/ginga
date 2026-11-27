//! A card declares its look; clicked, it changes size and fades, and gets
//! there by itself.

use ginga_gpui::{Look, LookExt, Spring};
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Window, div, px, rgb,
};

pub struct Card {
    open: bool,
}

impl Card {
    pub fn new() -> Self {
        Self { open: false }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }
}

impl Render for Card {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The lines to copy: the look the card should have now, declared on it.
        let look = if self.open {
            Look::new().size(360., 220.).opacity(1.).radius(16.)
        } else {
            Look::new().size(180., 110.).opacity(0.5).radius(4.)
        };
        div().child(
            div()
                .id("card")
                .p(px(16.))
                .bg(rgb(0x003b_6ef5))
                .cursor_pointer()
                .child(if self.open {
                    "Click to close"
                } else {
                    "Click to open"
                })
                .on_click(cx.listener(|card, _, _, cx| card.play(cx)))
                .look("card", look, Spring::SNAPPY),
        )
    }
}
