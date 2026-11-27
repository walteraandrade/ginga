//! A mark breathes and three dots pulse, each through its keyframes. The mark
//! is drawn at most 30 times a second; the dots on every frame.

use std::time::Duration;

use ginga_gpui::{Easing, Keyframes, Stop};
use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

pub struct Pulse {
    breath: Keyframes<f32, 3>,
    dots: [Keyframes<f32, 5>; 3],
    played: bool,
}

impl Pulse {
    pub fn new() -> Self {
        // The lines to copy. The mark breathes in, holds, and out again, four
        // times; at most every 33 ms is enough for so slow a motion.
        let breath = Keyframes::new([
            Stop::new(ms(0), 0.4, Easing::Linear),
            Stop::new(ms(450), 1.0, Easing::InOutSine),
            Stop::new(ms(600), 1.0, Easing::Linear),
        ])
        .expect("the stops are in order from zero")
        .times(4)
        .mirrored()
        .at_most_every(ms(33));
        // Each dot waits its turn inside a round of the same length, so the
        // three stay in step round after round.
        let dot = |wait: u64| {
            Keyframes::new([
                Stop::new(ms(0), 0.3, Easing::Linear),
                Stop::new(ms(wait), 0.3, Easing::Linear),
                Stop::new(ms(wait + 250), 1.0, Easing::OutQuart),
                Stop::new(ms(wait + 500), 0.3, Easing::InOutSine),
                Stop::new(ms(800), 0.3, Easing::Linear),
            ])
            .expect("the stops are in order from zero")
            .times(2)
        };
        Self {
            breath,
            dots: [dot(0), dot(150), dot(300)],
            played: false,
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.played = true;
        self.breath.restart(cx);
        for dot in &mut self.dots {
            dot.restart(cx);
        }
        cx.notify();
    }
}

impl Render for Pulse {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Before the first Play, everything rests on its first stop.
        let breath = if self.played {
            self.breath.frame(window, cx)
        } else {
            0.4
        };
        let dots = self.dots.each_mut().map(|dot| {
            if self.played {
                dot.frame(window, cx)
            } else {
                0.3
            }
        });
        let size = px(48. + 24. * breath);
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(48.))
            .child(
                div()
                    .size(px(72.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .size(size)
                            .rounded(size / 2.)
                            .bg(rgb(0x003b_6ef5))
                            .opacity(breath),
                    ),
            )
            .child(div().flex().gap(px(12.)).children(dots.map(|opacity| {
                div()
                    .size(px(14.))
                    .rounded(px(7.))
                    .bg(rgb(0x00e8_eaed))
                    .opacity(opacity)
            })))
    }
}
