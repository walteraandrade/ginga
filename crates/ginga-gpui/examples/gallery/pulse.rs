//! A mark breathes and three dots pulse, each through its keyframes. The mark
//! is an island of its own, so it is drawn at most about 30 times a second
//! while the dots, beside it, are drawn on every frame.

use std::time::Duration;

use ginga_gpui::{Easing, Island, Keyframes, Stop};
use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

pub struct Pulse {
    mark: Entity<Island<Keyframes<f32, 3>>>,
    dots: [Keyframes<f32, 5>; 3],
}

impl Pulse {
    pub fn new(cx: &mut Context<Self>) -> Self {
        // The lines to copy. The mark breathes in, holds, and out again; at
        // most every 30 ms is enough for so slow a motion. Zero rounds until
        // Play: it rests on its first stop and asks for nothing.
        let breath = Keyframes::new([
            Stop::new(ms(0), 0.4, Easing::Linear),
            Stop::new(ms(450), 1.0, Easing::InOutSine),
            Stop::new(ms(600), 1.0, Easing::Linear),
        ])
        .expect("the stops are in order from zero")
        .times(0)
        .mirrored()
        .at_most_every(ms(30));
        let mark = cx.new(|_| {
            Island::new(breath, |breath, window, cx| {
                let breath = breath.frame(window, cx);
                let size = px(48. + 24. * breath);
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
                    )
                    .into_any_element()
            })
        });
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
            .times(0)
        };
        Self {
            mark,
            dots: [dot(0), dot(150), dot(300)],
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.mark.update(cx, |island, cx| {
            let breath = island.state_mut();
            *breath = breath.times(4);
            breath.restart(cx);
            cx.notify();
        });
        for dot in &mut self.dots {
            *dot = dot.times(2);
            dot.restart(cx);
        }
        cx.notify();
    }
}

impl Render for Pulse {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dots = self.dots.each_mut().map(|dot| dot.frame(window, cx));
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(48.))
            .child(self.mark.clone())
            .child(div().flex().gap(px(12.)).children(dots.map(|opacity| {
                div()
                    .size(px(14.))
                    .rounded(px(7.))
                    .bg(rgb(0x00e8_eaed))
                    .opacity(opacity)
            })))
    }
}
