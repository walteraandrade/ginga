//! Buttons that answer to the pointer and the keyboard: each grows a little
//! under the pointer, sinks while pressed, and shows a ring with the focus,
//! which a press gives it.
//! Play turns them from quiet to loud, so the page moves without a pointer.

use ginga_gpui::{Color, Look, LookExt, Spring, color};
use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render, Styled, Window,
    div, px, rgb,
};

pub struct Buttons {
    loud: bool,
    focus: [FocusHandle; 3],
}

impl Buttons {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            loud: false,
            focus: [cx.focus_handle(), cx.focus_handle(), cx.focus_handle()],
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.loud = !self.loud;
        cx.notify();
    }
}

impl Render for Buttons {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let fill = if self.loud {
            color(rgb(0x003b_6ef5))
        } else {
            color(rgb(0x0030_3540))
        };
        let buttons = ["Save", "Share", "Delete"]
            .into_iter()
            .zip(&self.focus)
            .enumerate()
            .map(|(index, (label, focus))| {
                // The lines to copy: a look of its own, and one for each moment.
                let rest = Look::new()
                    .size(120., 40.)
                    .radius(8.)
                    .background(fill)
                    .border_color(Color::TRANSPARENT);
                div()
                    .id(index)
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_2()
                    .cursor_pointer()
                    .track_focus(focus)
                    .child(label)
                    .look(("button", index), rest, Spring::SNAPPY)
                    .hover(
                        Look::new()
                            .size(128., 44.)
                            .background(fill.with_alpha(0.85)),
                    )
                    // It sinks by moving down, not by shrinking: a smaller
                    // button could slide out from under the pointer and lose
                    // the click.
                    .pressed(Look::new().offset(0., 2.).radius(12.))
                    .focused(focus, Look::new().border_color(color(rgb(0x00e8_eaed))))
            });
        div()
            .flex()
            .flex_col()
            .gap(px(16.))
            .child("Point at a button, press it, or click it to give it the focus.")
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .h(px(48.))
                    .children(buttons),
            )
    }
}
