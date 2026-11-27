//! A look's offset moves what is painted and where the pointer finds it,
//! from wherever the element is laid out: checked on GPUI's test platform.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use ginga_gpui::{Easing, Look, LookExt, Tween};
use gpui::{
    Bounds, Context, Entity, InteractiveElement, IntoElement, Modifiers, ParentElement, Pixels,
    Render, StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext, Window, canvas,
    div, point, px,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Where the card was painted, one entry per frame drawn.
type Painted = Rc<RefCell<Vec<Bounds<Pixels>>>>;

/// How the card is placed by its own style.
#[derive(Clone, Copy)]
enum Place {
    /// `absolute`, at 100, 40 in its parent.
    Absolute,
    /// In the flow, moved by its own `relative` `left` and `top` of 5, 7.
    Relative,
}

/// A 20 px card that slides in from an offset of 8, 0 to `offset` on a
/// linear 100 ms tween, counting the clicks it gets.
struct Card {
    place: Place,
    offset: (f32, f32),
    painted: Painted,
    clicks: Rc<Cell<u32>>,
}

impl Render for Card {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (painted, clicks) = (self.painted.clone(), self.clicks.clone());
        let card = div().id("card").size(px(20.));
        let card = match self.place {
            Place::Absolute => card.absolute().left(px(100.)).top(px(40.)),
            Place::Relative => card.relative().left(px(5.)).top(px(7.)),
        };
        let (x, y) = self.offset;
        let card = card
            .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
            .child(
                canvas(
                    move |bounds, _, _| painted.borrow_mut().push(bounds),
                    |_, (), _, _| {},
                )
                .size_full(),
            )
            .look(
                "card",
                Look::new().offset(x, y),
                Tween::new(ms(100), Easing::Linear),
            )
            .from(Look::new().offset(8., 0.));
        div().relative().size_full().child(card)
    }
}

struct Opened<'a> {
    cx: &'a mut VisualTestContext,
    view: Entity<Card>,
    painted: Painted,
    clicks: Rc<Cell<u32>>,
}

fn open(cx: &mut TestAppContext, place: Place) -> Opened<'_> {
    let (painted, clicks) = (Painted::default(), Rc::new(Cell::new(0)));
    let (view, cx) = cx.add_window_view({
        let (painted, clicks) = (painted.clone(), clicks.clone());
        move |_, _| Card {
            place,
            offset: (0., 0.),
            painted,
            clicks,
        }
    });
    cx.run_until_parked();
    Opened {
        cx,
        view,
        painted,
        clicks,
    }
}

impl Opened<'_> {
    /// Where the card was painted after the clock moves `millis` and a
    /// frame is drawn.
    fn after(&mut self, millis: u64) -> gpui::Point<Pixels> {
        self.cx.executor().advance_clock(ms(millis));
        self.cx.update(Window::simulate_next_frame);
        self.cx.run_until_parked();
        self.painted.borrow().last().unwrap().origin
    }

    /// Clicks at `x`, `y` and says whether the card got the click.
    fn clicked_at(&mut self, x: f32, y: f32) -> bool {
        let before = self.clicks.get();
        self.cx
            .simulate_click(point(px(x), px(y)), Modifiers::none());
        self.cx.run_until_parked();
        self.clicks.get() > before
    }
}

#[gpui::test]
fn an_absolute_card_slides_in_from_its_own_place(cx: &mut TestAppContext) {
    let mut card = open(cx, Place::Absolute);
    assert_eq!(
        card.painted.borrow()[0].origin,
        point(px(108.), px(40.)),
        "8 px right of its own place, not its parent's corner"
    );
    assert_eq!(card.after(50), point(px(104.), px(40.)));
    assert_eq!(
        card.after(50),
        point(px(100.), px(40.)),
        "at rest on its place"
    );
}

#[gpui::test]
fn the_offset_adds_to_a_cards_own_left_and_top(cx: &mut TestAppContext) {
    let mut card = open(cx, Place::Relative);
    assert_eq!(card.painted.borrow()[0].origin, point(px(13.), px(7.)));
    assert_eq!(card.after(100), point(px(5.), px(7.)));
}

#[gpui::test]
fn the_pointer_finds_the_card_where_it_is_painted(cx: &mut TestAppContext) {
    let mut card = open(cx, Place::Absolute);
    card.after(100);
    card.cx.update(|_, cx| {
        card.view.update(cx, |card, cx| {
            card.offset = (50., 0.);
            cx.notify();
        });
    });
    assert_eq!(card.after(100), point(px(150.), px(40.)));
    assert!(card.clicked_at(160., 50.), "a click where it is painted");
    assert!(!card.clicked_at(110., 50.), "none where it is laid out");
}
