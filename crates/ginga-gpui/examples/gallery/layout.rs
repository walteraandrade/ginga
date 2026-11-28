//! Cards in a wrapping row glide to where layout puts them when one comes or goes.

use ginga_gpui::{GlideExt, Spring};
use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb};

pub struct Layout {
    /// The cards drawn, in order; Play takes the first away or puts it back.
    cards: Vec<u32>,
}

impl Layout {
    pub fn new() -> Self {
        Self {
            cards: (0..7).collect(),
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        if self.cards.first() == Some(&0) {
            self.cards.remove(0);
        } else {
            self.cards.insert(0, 0);
        }
        cx.notify();
    }
}

impl Render for Layout {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let colors = [0x00f5_a524, 0x0040_c4ff, 0x0030_a46c, 0x00e5_484d];
        div()
            .flex()
            .flex_wrap()
            .gap(px(12.))
            .w(px(528.))
            .children(self.cards.iter().map(|&card| {
                // The line to copy: the card glides when layout moves it.
                div()
                    .w(px(120.))
                    .h(px(72.))
                    .rounded(px(6.))
                    .bg(rgb(colors[card as usize % colors.len()]))
                    .child(format!("card {card}"))
                    .glides(("card", card), Spring::SNAPPY)
            }))
    }
}
