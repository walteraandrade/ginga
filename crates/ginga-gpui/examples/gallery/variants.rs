//! Variants: a menu opens, and its rows follow it one after another. On the
//! left the menu goes first and its rows after it; in the middle the rows go
//! first and the menu after them; on the right the menu opens first and
//! closes last, with the order given to each change.

use std::time::Duration;

use ginga_gpui::{Color, Easing, Look, LookExt, Order, Spring, Tween, Variant, color};
use gpui::{Context, IntoElement, ParentElement, Render, Styled, Window, div, px, rgb};

const ROWS: [&str; 4] = ["New file", "Open", "Save", "Close"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Menu {
    Closed,
    Open,
}

pub struct Menus {
    parent_first: Variant<Menu>,
    children_first: Variant<Menu>,
    each_way: Variant<Menu>,
}

impl Menus {
    pub fn new() -> Self {
        let menu = |order| {
            Variant::new(Menu::Closed)
                .stagger(Duration::from_millis(60))
                .order(order)
        };
        Self {
            parent_first: menu(Order::ParentFirst),
            children_first: menu(Order::ChildrenFirst),
            each_way: menu(Order::Together),
        }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        for menu in [&self.parent_first, &self.children_first] {
            let next = match menu.name() {
                Menu::Closed => Menu::Open,
                Menu::Open => Menu::Closed,
            };
            // Renders again the views that drew the menu; no notify needed.
            menu.set(next, cx);
        }
        // The lines to copy: open with the panel first, close with the rows first.
        let (next, order) = match self.each_way.name() {
            Menu::Closed => (Menu::Open, Order::ParentFirst),
            Menu::Open => (Menu::Closed, Order::ChildrenFirst),
        };
        self.each_way.set_in_order(next, order, cx);
    }
}

/// A menu that leads `variant`, with rows that follow it.
fn menu(id: &'static str, variant: &Variant<Menu>) -> impl IntoElement {
    let open = variant.name() == Menu::Open;
    let fill = color(rgb(0x0026_2b36));
    // The panel only fades its fill, so the rows stay in sight both ways.
    let panel = if open {
        Look::new().background(fill)
    } else {
        Look::new().background(Color::TRANSPARENT)
    };
    let row = if open {
        Look::new().offset(0., 0.).opacity(1.)
    } else {
        Look::new().offset(-12., 0.).opacity(0.)
    };
    div()
        .w(px(200.))
        .rounded(px(8.))
        .p(px(8.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .look(
            id,
            panel,
            Tween::new(Duration::from_millis(200), Easing::Out),
        )
        .leads(variant)
        .child(div().h(px(16.)).text_color(rgb(0x009b_a1a6)).child(id))
        .children((0..).zip(ROWS).map(|(n, text)| {
            div()
                .h(px(32.))
                .px(px(8.))
                .rounded(px(6.))
                .bg(rgb(0x0033_3a48))
                .child(text)
                // The lines to copy: the row's look for the name, and its place.
                .look((id, n), row, Spring::SNAPPY)
                .follows(variant, n)
        }))
}

impl Render for Menus {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .gap(px(32.))
            .child(menu("menu first", &self.parent_first))
            .child(menu("rows first", &self.children_first))
            .child(menu("each way", &self.each_way))
    }
}
