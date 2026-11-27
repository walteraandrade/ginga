//! A card to throw, that springs back inside its tray, and a slider's knob
//! that rests on one of five stops.

use ginga_gpui::{Drag, DragExt, DragState};
use gpui::{
    Axis, Context, IntoElement, ParentElement, Pixels, Point, Render, Styled, Window, div, point,
    px, rgb,
};

/// The tray's size, and the card's: the card's offset stays in what is left.
const TRAY: (f32, f32) = (640., 280.);
const CARD: (f32, f32) = (160., 100.);
/// The slider's track, and its knob.
const TRACK: f32 = 400.;
const KNOB: f32 = 28.;

fn at(x: f32, y: f32) -> Point<Pixels> {
    point(px(x), px(y))
}

pub struct Throw {
    card: DragState,
    knob: DragState,
    rightward: bool,
}

impl Throw {
    pub fn new() -> Self {
        Self {
            card: DragState::default(),
            knob: DragState::default(),
            rightward: false,
        }
    }

    // The lines to copy: how each is dragged.
    fn card_drag() -> Drag {
        Drag::new().within(at(0., 0.), at(TRAY.0 - CARD.0, TRAY.1 - CARD.1))
    }

    fn knob_drag() -> Drag {
        let end = TRACK - KNOB;
        Drag::new()
            .along(Axis::Horizontal)
            .snap_to((0..5u8).map(|stop| at(end * f32::from(stop) / 4., 0.)))
    }

    /// Throws the card at the far wall, and the knob a long way, as a
    /// pointer might; each time the other way.
    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.rightward = !self.rightward;
        let way = if self.rightward { 1. } else { -1. };
        self.card
            .throw(at(way * 6_000., way * 1_200.), &Self::card_drag(), cx);
        self.knob
            .throw(at(way * 1_500., 0.), &Self::knob_drag(), cx);
        cx.notify();
    }
}

impl Render for Throw {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let tray = div()
            .w(px(TRAY.0))
            .h(px(TRAY.1))
            .rounded(px(12.))
            .bg(rgb(0x0020_2328))
            .child(
                div()
                    .w(px(CARD.0))
                    .h(px(CARD.1))
                    .rounded(px(12.))
                    .bg(rgb(0x003b_6ef5))
                    .draggable(&self.card, Self::card_drag()),
            );
        let slider = div()
            .w(px(TRACK))
            .h(px(KNOB))
            .rounded(px(KNOB / 2.))
            .bg(rgb(0x0020_2328))
            .child(
                div()
                    .size(px(KNOB))
                    .rounded(px(KNOB / 2.))
                    .bg(rgb(0x00e5_484d))
                    .draggable(&self.knob, Self::knob_drag()),
            );
        div()
            .flex()
            .flex_col()
            .gap(px(32.))
            .child(tray)
            .child(slider)
    }
}
