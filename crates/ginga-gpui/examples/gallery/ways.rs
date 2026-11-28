//! Ways in and out declared on the element: a toast rises in and fades out,
//! and two titles swap, one crossing the old and the new, one waiting for
//! the old to go. Each time the toast is gone, a count goes up.

use std::time::Duration;

use ginga_gpui::{Easing, InOut, Look, Spring, Swap, Tween, present};
use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb};

const TITLES: [&str; 3] = ["Inbox", "Sent", "Drafts"];

pub struct Ways {
    saved: bool,
    /// Which title is shown: none until Play, as what is there arrives.
    title: Option<usize>,
    /// How many times the toast's way out was over.
    gone: u32,
}

impl Ways {
    pub fn new() -> Self {
        Self {
            saved: false,
            title: None,
            gone: 0,
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.saved = !self.saved;
        self.title = Some(self.title.map_or(0, |title| (title + 1) % TITLES.len()));
        cx.notify();
    }
}

fn title(swap: Swap, title: Option<&'static str>) -> impl IntoElement {
    let ways = InOut::new(
        Tween::new(Duration::from_millis(220), Easing::Out),
        Tween::new(Duration::from_millis(220), Easing::In),
    );
    let name = match swap {
        Swap::Sync => "sync",
        Swap::Wait => "wait",
    };
    div()
        .flex()
        .gap(px(16.))
        .h(px(32.))
        .child(div().w(px(48.)).text_color(rgb(0x009b_a1a6)).child(name))
        .child(
            present(name, title, ways, |title: &&'static str| {
                div().text_xl().child(*title)
            })
            .arrive_from(Look::new().offset(0., 12.).opacity(0.))
            .leave_to(Look::new().offset(0., -12.).opacity(0.))
            .swap(swap),
        )
}

impl Render for Ways {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ways = InOut::new(
            Spring::SNAPPY,
            Tween::new(Duration::from_millis(200), Easing::In),
        );
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(title(Swap::Sync, self.title.map(|title| TITLES[title])))
            .child(title(Swap::Wait, self.title.map(|title| TITLES[title])))
            .child(
                div().h(px(48.)).child(
                    // The lines to copy: say whether it is there, and how it
                    // comes and goes; it is drawn until it is gone.
                    present("toast", self.saved.then_some("Saved"), ways, |text| {
                        div()
                            .px(px(16.))
                            .py(px(10.))
                            .rounded(px(8.))
                            .bg(rgb(0x0030_a46c))
                            .child(*text)
                    })
                    .arrive_from(Look::new().offset(0., 24.).opacity(0.))
                    .leave_to(Look::new().opacity(0.))
                    // Once its way out is over: here, a count.
                    .on_gone({
                        let view = cx.entity().downgrade();
                        move |_, cx| {
                            view.update(cx, |this, cx| {
                                this.gone += 1;
                                cx.notify();
                            })
                            .ok();
                        }
                    }),
                ),
            )
            .child(
                div()
                    .text_color(rgb(0x009b_a1a6))
                    .child(format!("Gone {} times", self.gone)),
            )
    }
}
