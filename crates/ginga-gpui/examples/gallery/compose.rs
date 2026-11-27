//! Motions composed: a bar whose color follows its width, and an entrance
//! of three steps, one after another, the last overlapping the one before.

use std::time::Duration;

use ginga_gpui::{
    Animate, Clock, Color, Easing, Map, Sequence, SettledOnce, Spring, Tween, unless_reduced,
};
use gpui::{Context, IntoElement, ParentElement, Render, Rgba, Styled, Window, div, px, rgb};

pub struct Compose {
    shown: bool,
    clock: Clock,
    /// The bar's progress, from 0 to 1.
    progress: ginga::Motion<f32>,
    /// The three steps of the entrance, each from 0 (away) to 1 (here).
    steps: [ginga::Motion<f32>; 3],
    /// The last entrance, once Play is pressed.
    entrance: Option<Sequence>,
    ended: SettledOnce,
    /// How many times the entrance has settled, to show that it is said once.
    endings: u32,
}

impl Compose {
    pub fn new() -> Self {
        Self {
            shown: false,
            clock: Clock::default(),
            progress: ginga::Motion::at(0.0),
            steps: [ginga::Motion::at(0.0); 3],
            entrance: None,
            ended: SettledOnce::default(),
            endings: 0,
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.shown = !self.shown;
        let now = self.clock.now(cx);
        let to = if self.shown { 1.0 } else { 0.0 };
        self.progress
            .set_target(to, unless_reduced(Spring::BOUNCY, cx), now);
        // The lines to copy: three steps, the third starting 120 ms before
        // the second ends.
        let fade = unless_reduced(Tween::new(Duration::from_millis(300), Easing::Out), cx);
        let [title, body, button] = &mut self.steps;
        let mut entrance = Sequence::starting_at(now);
        entrance.then(title, to, fade);
        let second = entrance.then(body, to, fade);
        entrance.at(
            second.ends.saturating_sub(Duration::from_millis(120)),
            button,
            to,
            unless_reduced(Spring::SNAPPY, cx),
        );
        self.entrance = Some(entrance);
        cx.notify();
    }
}

impl Render for Compose {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = self.clock.now(cx);
        // The lines to copy: a width derived from the progress, and a color
        // derived from the width, held to its ends.
        let width = Map::new(0.0, 1.0, 40.0, 480.0).expect("two different ends");
        let fill = Map::new(40.0, 480.0, Color::hex(0xe5_484d), Color::hex(0x30_a46c))
            .expect("two different ends")
            .clamped();
        let bar = width.of(&self.progress).sample(now);
        let color = fill.at(bar.value);
        let steps = self.steps.map(|step| step.sample(now));
        let ends = self.entrance.map(|entrance| entrance.settles_at());
        if self.ended.check(ends, now) {
            self.endings += 1;
        }
        if !bar.settled || steps.iter().any(|step| !step.settled) {
            window.request_animation_frame();
        }
        let line = |text: &'static str, step: ginga::Sample<f32>| {
            div()
                .relative()
                .top(px((1.0 - step.value) * 24.))
                .opacity(step.value.clamp(0., 1.))
                .child(text)
        };
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child("A color that follows a width")
                    .child(
                        div()
                            .h(px(24.))
                            .w(px(bar.value.max(0.)))
                            .rounded(px(6.))
                            .bg(Rgba::from_repr(color)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .p(px(16.))
                    .w(px(320.))
                    .rounded(px(12.))
                    .bg(rgb(0x0022_252b))
                    .child(line("1. A title", steps[0]))
                    .child(line("2. A body, after it", steps[1]))
                    .child(line("3. A button, 120 ms early", steps[2])),
            )
            .child(format!("The steps have ended {} times", self.endings))
    }
}
