//! A look applied as it is, with no motion: checked on GPUI's test platform.

use std::{cell::RefCell, rc::Rc};

use ginga_gpui::{Look, LookExt, Transition};
use gpui::{
    Bounds, Context, IntoElement, ParentElement, Pixels, Render, Styled, TestAppContext, Window,
    canvas, div, point, px, size,
};

/// Where each card was painted in the last frame, and how many frames drew.
/// Opacity cannot be read back from a frame (GPUI keeps it crate-private);
/// `styled`'s unit tests check it, for `look` and `with_look` alike.
#[derive(Default)]
struct Painted {
    cards: Vec<Bounds<Pixels>>,
    frames: usize,
}

/// Two cards with the same look, a 30 px high one in the flow and an
/// absolute one at 100, 40, neither with an id; and an absolute one at
/// 200, 40 whose offset is not finite. Declared with `look` instead, at
/// once, when `declared`.
struct Cards {
    painted: Rc<RefCell<Painted>>,
    declared: bool,
}

impl Render for Cards {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let look = Look::new().offset(4., 0.).opacity(0.5).width(80.);
        self.painted.borrow_mut().frames += 1;
        self.painted.borrow_mut().cards.clear();
        let card = |painted: &Rc<RefCell<Painted>>| {
            let painted = painted.clone();
            div().w(px(10.)).h(px(30.)).child(
                canvas(
                    move |bounds, _, _| painted.borrow_mut().cards.push(bounds),
                    |_, (), _, _| {},
                )
                .size_full(),
            )
        };
        let nowhere = Look::new().offset(f32::NAN, 0.).width(80.);
        let styled = |card: gpui::Div, id: &'static str, look: Look| {
            if self.declared {
                card.look(id, look, Transition::Instant).into_any_element()
            } else {
                card.with_look(look).into_any_element()
            }
        };
        div()
            .relative()
            .size_full()
            .child(styled(card(&self.painted), "flow", look))
            .child(styled(
                card(&self.painted).absolute().left(px(100.)).top(px(40.)),
                "absolute",
                look,
            ))
            .child(styled(
                card(&self.painted).absolute().left(px(200.)).top(px(40.)),
                "nowhere",
                nowhere,
            ))
    }
}

#[gpui::test]
fn a_look_is_applied_as_it_is_at_rest(cx: &mut TestAppContext) {
    let painted = Rc::new(RefCell::new(Painted::default()));
    let window = cx.open_window(size(px(400.), px(300.)), {
        let painted = painted.clone();
        move |_, _| Cards {
            painted,
            declared: false,
        }
    });
    cx.run_until_parked();
    let cards = painted.borrow().cards.clone();
    assert_eq!(
        cards,
        [
            Bounds::new(point(px(4.), px(0.)), size(px(80.), px(30.))),
            Bounds::new(point(px(104.), px(40.)), size(px(80.), px(30.))),
            Bounds::new(point(px(200.), px(40.)), size(px(80.), px(30.))),
        ],
        "the look's width, and the offset added to each card's own place; \
         an offset that is not finite moves nothing"
    );
    let frames = window
        .update(cx, |_, window, cx| window.simulate_next_frame(cx))
        .unwrap();
    assert_eq!(frames, 0, "nothing moves, so no frame is asked for");
    assert_eq!(painted.borrow().frames, 1, "and nothing is drawn again");
}

#[gpui::test]
fn it_is_drawn_as_a_declared_look_at_rest_is(cx: &mut TestAppContext) {
    let draw = |declared: bool, cx: &mut TestAppContext| {
        let painted = Rc::new(RefCell::new(Painted::default()));
        cx.open_window(size(px(400.), px(300.)), {
            let painted = painted.clone();
            move |_, _| Cards { painted, declared }
        });
        cx.run_until_parked();
        painted.borrow().cards.clone()
    };
    let declared = draw(true, cx);
    assert_eq!(draw(false, cx), declared);
}
