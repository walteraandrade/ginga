//! A dot moves in an island, beside a grid that is not drawn again.

use std::{cell::Cell, rc::Rc};

use ginga_gpui::{Island, Motion, Spring};
use gpui::{
    AnyElement, AppContext, Context, Entity, IntoElement, ParentElement, Pixels, Render,
    StyleRefinement, Styled, Window, div, px, rgb,
};

/// Cells in the grid.
const CELLS: u32 = 600;

/// The dot: a place and where it heads.
pub struct Dot {
    x: Motion<Pixels>,
    target: Pixels,
}

/// A view heavy enough to notice: many boxes with numbers. It counts its renders.
pub struct Grid {
    renders: Rc<Cell<u32>>,
}

impl Render for Grid {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.set(self.renders.get() + 1);
        div()
            .size_full()
            .flex()
            .flex_wrap()
            .gap(px(2.))
            .children((0..CELLS).map(|n| {
                div()
                    .size(px(22.))
                    .bg(rgb(0x0020_2328 + (n % 5) * 0x0004_0404))
                    .text_size(px(8.))
                    .text_color(rgb(0x0060_6670))
                    .child(n.to_string())
            }))
    }
}

pub struct IslandPage {
    grid: Entity<Grid>,
    dot: Entity<Island<Dot>>,
    renders: Rc<Cell<u32>>,
}

impl IslandPage {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let renders = Rc::new(Cell::new(0));
        let grid = cx.new(|_| Grid {
            renders: renders.clone(),
        });
        // The lines to copy: the island keeps the dot and draws it.
        let dot = cx.new(|_| {
            Island::new(
                Dot {
                    x: Motion::at(px(0.)),
                    target: px(0.),
                },
                |dot, window, cx| {
                    let x = dot.x.toward(dot.target, Spring::SMOOTH, window, cx);
                    div()
                        .absolute()
                        .top(px(-28.))
                        .left(x)
                        .size(px(20.))
                        .rounded_full()
                        .bg(rgb(0x00ff_5c8a))
                        .into_any_element()
                },
            )
        });
        Self { grid, dot, renders }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.dot.update(cx, |island, cx| {
            let dot = island.state_mut();
            dot.target = if dot.target == px(0.) {
                px(560.)
            } else {
                px(0.)
            };
            cx.notify();
        });
        cx.notify();
    }
}

impl Render for IslandPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let grid: AnyElement = self
            .grid
            .clone()
            .cached(StyleRefinement::default().size_full())
            .into_any_element();
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(format!(
                "The grid has rendered {} times",
                self.renders.get()
            ))
            .child(
                div()
                    .relative()
                    .mt(px(32.))
                    .flex_1()
                    .child(grid)
                    .child(self.dot.clone()),
            )
    }
}
