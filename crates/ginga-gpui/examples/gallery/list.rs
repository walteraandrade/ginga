//! Rows arrive in a list and leave it.

use std::time::Duration;

use ginga_gpui::{Clock, Easing, InOut, Phase, PresenceList, Tween, unless_reduced};
use gpui::{
    Context, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px, rgb,
};

/// A row's full height.
const ROW: f32 = 40.;

pub struct Rows {
    wanted: Vec<(u32, SharedString)>,
    next: u32,
    list: PresenceList<u32, SharedString>,
    clock: Clock,
}

impl Rows {
    pub fn new() -> Self {
        let wanted: Vec<_> = (1..=4).map(|n| (n, format!("Row {n}").into())).collect();
        let mut rows = PresenceList::new();
        rows.sync(wanted.iter().cloned(), InOut::INSTANT, Duration::ZERO);
        Self {
            wanted,
            next: 5,
            list: rows,
            clock: Clock::default(),
        }
    }

    /// Takes out the second row and adds one at the end.
    pub fn play(&mut self, cx: &mut Context<Self>) {
        if self.wanted.len() > 1 {
            self.wanted.remove(1);
        }
        self.wanted
            .push((self.next, format!("Row {}", self.next).into()));
        self.next += 1;
        // A list presence is the engine's own, so it does not know the
        // desktop's setting: `unless_reduced` makes the fade instant under
        // reduced motion, as the adapter's `Motion` and `Presence` do.
        let fade = unless_reduced(Tween::new(Duration::from_millis(300), Easing::InOut), cx);
        let now = self.clock.now(cx);
        self.list
            .sync(self.wanted.iter().cloned(), InOut::new(fade, fade), now);
        cx.notify();
    }
}

impl Render for Rows {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The lines to copy: draw each row as far in as its progress says,
        // and ask for a frame while any of them moves.
        let now = self.clock.now(cx);
        let mut moving = false;
        let rows: Vec<_> = self
            .list
            .frame(now)
            .map(|row| {
                moving |= row.moving();
                let color = if row.phase == Phase::Exiting {
                    0x0055_3333
                } else {
                    0x0028_2c34
                };
                div()
                    .h(px(ROW * row.progress))
                    .overflow_hidden()
                    .opacity(row.progress)
                    .child(
                        div()
                            .h(px(ROW - 4.))
                            .px(px(12.))
                            .flex()
                            .items_center()
                            .rounded(px(4.))
                            .bg(rgb(color))
                            .child(row.content.clone()),
                    )
            })
            .collect();
        if moving {
            window.request_animation_frame();
        }
        div().w(px(320.)).flex().flex_col().children(rows)
    }
}
