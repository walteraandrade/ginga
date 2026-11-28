//! Questions open and close as an accordion: each answer's height goes from
//! nothing to its own, by its presence's progress.

use std::time::Duration;

use ginga_gpui::{Easing, InOut, OpenExt, Presence, Tween};
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Window, div, px, rgb,
};

const QUESTIONS: [(&str, &str); 3] = [
    (
        "What moves?",
        "Any value that can become a row of numbers: a length, a color, a place.",
    ),
    (
        "What does it cost at rest?",
        "Nothing: a settled motion asks for no frame, and the view is not drawn again for it.",
    ),
    (
        "How is it checked?",
        "With a trace: the value at each time, written down, and asserted in a test.",
    ),
];

pub struct Accordion {
    answers: [Presence<&'static str>; 3],
    /// The question Play opens next.
    next: usize,
}

impl Accordion {
    pub fn new() -> Self {
        Self {
            answers: [Presence::absent(), Presence::absent(), Presence::absent()],
            next: 0,
        }
    }

    fn way() -> InOut {
        InOut::new(
            Tween::new(Duration::from_millis(260), Easing::Out),
            Tween::new(Duration::from_millis(200), Easing::In),
        )
    }

    /// Opens one question and closes the one open before.
    fn toggle(&mut self, chosen: usize, cx: &mut Context<Self>) {
        let way = Self::way();
        for (index, answer) in self.answers.iter_mut().enumerate() {
            if index == chosen && !answer.is_wanted() {
                answer.show(QUESTIONS[index].1, way.enter, cx);
            } else if answer.is_wanted() {
                answer.hide(way.exit, cx);
            }
        }
        cx.notify();
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        let chosen = self.next;
        self.next = (self.next + 1) % QUESTIONS.len();
        self.toggle(chosen, cx);
    }
}

impl Render for Accordion {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div().w(px(360.)).flex().flex_col().gap(px(6.));
        for (index, (question, _)) in QUESTIONS.iter().enumerate() {
            // The lines to copy: the presence's progress opens the answer.
            let answer = self.answers[index].frame(window, cx).map(|shown| {
                div()
                    .px(px(12.))
                    .py(px(8.))
                    .child(*shown.content)
                    .opened(("answer", index), shown.progress)
            });
            list = list.child(
                div()
                    .rounded(px(8.))
                    .bg(rgb(0x0020_2830))
                    .child(
                        div()
                            .id(("question", index))
                            .px(px(12.))
                            .py(px(10.))
                            .on_click(cx.listener(move |this, _, _, cx| this.toggle(index, cx)))
                            .child(*question),
                    )
                    .children(answer),
            );
        }
        div().p(px(40.)).child(list)
    }
}
