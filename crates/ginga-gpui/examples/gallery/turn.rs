//! An icon that turns: a chevron that points down when its section opens.
//! A `div` has no rotate in GPUI, so the turn is a `Motion<f32>` given to the
//! icon's own transformation.

use std::f32::consts::TAU;

use ginga_gpui::{Motion, Spring};

use crate::app::CHEVRON_PATH;
use gpui::{
    Context, IntoElement, ParentElement, Render, Styled, Transformation, Window, div, px, radians,
    rgb, svg,
};

pub struct Turn {
    open: bool,
    /// How much of a whole turn the chevron has made.
    turned: Motion<f32>,
}

impl Turn {
    pub fn new() -> Self {
        Self {
            open: false,
            turned: Motion::at(0.),
        }
    }

    /// How much of a whole turn the chevron has made this frame.
    #[cfg(test)]
    pub fn turned(&mut self, cx: &gpui::App) -> f32 {
        let (motion, now) = self.turned.inspect(cx);
        motion.sample(now).value
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }
}

impl Render for Turn {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The lines to copy: a quarter turn when open, drawn by the icon.
        let target = if self.open { 0.25 } else { 0. };
        let turned = self.turned.toward(target, Spring::SNAPPY, window, cx);
        div()
            .flex()
            .items_center()
            .gap(px(12.))
            .child(
                svg()
                    .path(CHEVRON_PATH)
                    .size(px(24.))
                    .text_color(rgb(0x00e0_e2e6))
                    .with_transformation(Transformation::rotate(radians(turned * TAU))),
            )
            .child(if self.open {
                "Details, open"
            } else {
                "Details"
            })
    }
}
