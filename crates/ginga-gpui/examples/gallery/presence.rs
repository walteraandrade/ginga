//! A toast is shown and hidden, and stays drawn while it leaves.

use std::time::Duration;

use ginga_gpui::{CoverExt, Easing, InOut, Look, LookExt, Phase, Presence, Tween};
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Window, div, px, rgb,
};

pub struct Toast {
    presence: Presence<&'static str>,
    saves: u32,
    /// Clicks on the toast: they count only while it is not leaving.
    undos: u32,
}

impl Toast {
    pub fn new() -> Self {
        Self {
            presence: Presence::absent(),
            saves: 0,
            undos: 0,
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        let way = InOut::new(
            Tween::new(Duration::from_millis(240), Easing::Out),
            Tween::new(Duration::from_millis(400), Easing::In),
        );
        if self.presence.is_wanted() {
            self.presence.hide(way.exit, cx);
        } else {
            self.saves += 1;
            self.presence.show("Saved", way.enter, cx);
        }
        cx.notify();
    }
}

impl Render for Toast {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut page = div().relative().size_full();
        // The lines to copy: `frame` gives what to draw, and how far in it
        // is; `with_look` draws it with the look mixed that far.
        if let Some(shown) = self.presence.frame(window, cx) {
            let from = Look::new().offset(0., 24.).opacity(0.);
            let label = if shown.phase == Phase::Exiting {
                format!("{} (leaving)", shown.content)
            } else {
                format!("{}, click to undo", shown.content)
            };
            // While it leaves, the toast takes no clicks and loses the focus.
            page = page.child(
                shown
                    .cover(
                        div()
                            .id("toast")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.undos += 1;
                                cx.notify();
                            }))
                            .absolute()
                            .top(px(40.))
                            .left(px(40.))
                            .px(px(16.))
                            .py(px(10.))
                            .rounded(px(8.))
                            .bg(rgb(0x0030_a46c))
                            .child(label),
                    )
                    .with_look(from.mix(Look::new(), shown.progress)),
            );
        }
        page.child(div().absolute().bottom(px(0.)).child(format!(
            "Shown {} times, undone {} times",
            self.saves, self.undos
        )))
    }
}
