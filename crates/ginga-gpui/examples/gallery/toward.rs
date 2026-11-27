//! A value heads for a target: with a spring, and with a tween.

use std::time::Duration;

use ginga_gpui::{Easing, Motion, Spring, Tween};
use gpui::{Context, IntoElement, ParentElement, Pixels, Render, Styled, Window, div, px, rgb};

pub struct Toward {
    right: bool,
    spring: Motion<Pixels>,
    tween: Motion<Pixels>,
}

impl Toward {
    pub fn new() -> Self {
        Self {
            right: false,
            spring: Motion::at(px(0.)),
            tween: Motion::at(px(0.)),
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.right = !self.right;
        cx.notify();
    }
}

impl Render for Toward {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The lines to copy: a target, and `toward` it on every render.
        let target = if self.right { px(480.) } else { px(0.) };
        let sprung = self.spring.toward(target, Spring::BOUNCY, window, cx);
        let eased = self.tween.toward(
            target,
            Tween::new(Duration::from_millis(400), Easing::InOut),
            window,
            cx,
        );

        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(lane("Spring::BOUNCY", sprung, 0x00f5_a524))
            .child(lane("Tween, 400 ms, InOut", eased, 0x0040_c4ff))
    }
}

/// A labelled lane with a box at `x`.
fn lane(label: &'static str, x: Pixels, color: u32) -> impl IntoElement {
    div().flex().flex_col().gap(px(6.)).child(label).child(
        div()
            .relative()
            .h(px(48.))
            .w(px(528.))
            .rounded(px(6.))
            .bg(rgb(0x0022_252b))
            .child(
                div()
                    .absolute()
                    .left(x)
                    .size(px(48.))
                    .rounded(px(6.))
                    .bg(rgb(color)),
            ),
    )
}
