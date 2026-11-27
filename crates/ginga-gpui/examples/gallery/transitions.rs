//! A card comes in and goes out: it springs to its place while it fades
//! with a tween, each value on its own clock.

use std::time::Duration;

use ginga_gpui::{Easing, Look, LookExt, Spring, Transitions, Tween};
use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb};

pub struct Arrive {
    shown: bool,
}

impl Arrive {
    pub fn new() -> Self {
        Self { shown: false }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.shown = !self.shown;
        cx.notify();
    }
}

impl Render for Arrive {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // The lines to copy: the place springs, bouncy, and the opacity
        // fades in a fixed 250 ms.
        let look = if self.shown {
            Look::new().offset(0., 0.).opacity(1.)
        } else {
            Look::new().offset(0., 120.).opacity(0.)
        };
        let transitions = Transitions::new()
            .offset(Spring::BOUNCY)
            .opacity(Tween::new(Duration::from_millis(250), Easing::Linear));
        div().child(
            div()
                .w(px(280.))
                .p(px(16.))
                .rounded(px(12.))
                .bg(rgb(0x003b_6ef5))
                .child("Springs to its place, fades on its own clock")
                .look("card", look, transitions),
        )
    }
}
