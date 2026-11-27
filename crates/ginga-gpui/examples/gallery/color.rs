//! A button changes color: its fill, its text and its border, each through the
//! colors between, never through another hue.

use ginga_gpui::{Color, Look, LookExt, Spring, color};
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Window, div, px, rgb,
};

pub struct Button {
    on: bool,
}

impl Button {
    pub fn new() -> Self {
        Self { on: false }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.on = !self.on;
        cx.notify();
    }
}

impl Render for Button {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The lines to copy: the button's colors now, declared on it. Off, it
        // is a red outline with no fill; on, it is filled blue.
        let look = if self.on {
            Look::new()
                .background(color(rgb(0x003b_6ef5)))
                .text_color(color(rgb(0x00ff_ffff)))
                .border_color(color(rgb(0x003b_6ef5)))
        } else {
            Look::new()
                .background(Color::hex(0xe5_484d).with_alpha(0.))
                .text_color(Color::hex(0xe5_484d))
                .border_color(Color::hex(0xe5_484d))
        };
        // A row, so the button is as wide as its label.
        div().flex().child(
            div()
                .id("button")
                .px(px(24.))
                .py(px(12.))
                .border_2()
                .rounded(px(8.))
                .cursor_pointer()
                .child(if self.on { "On" } else { "Off" })
                .on_click(cx.listener(|button, _, _, cx| button.play(cx)))
                .look("button", look, Spring::SMOOTH),
        )
    }
}
