//! Variants: a menu opens, and its rows follow it one after another. On the
//! left the menu goes first and its rows after it; on the right the rows go
//! first and the menu after them.

use std::time::Duration;

use ginga_gpui::{Color, Look, LookExt, Order, Spring, Variant, color};
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
    }
}

/// A menu that leads `variant`, with rows that follow it.
fn menu(id: &'static str, variant: &Variant<Menu>) -> impl IntoElement {
    let open = variant.name() == Menu::Open;
    let fill = color(rgb(0x0026_2b36));
    let panel = if open {
        Look::new().height(176.).background(fill)
    } else {
        Look::new().height(32.).background(Color::TRANSPARENT)
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
        .overflow_hidden()
        .look(id, panel, Spring::SNAPPY)
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
    }
}
