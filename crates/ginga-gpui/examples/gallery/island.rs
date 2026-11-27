//! A dot moves in an island, beside a grid that is not drawn again.

use std::{cell::Cell, rc::Rc};

use ginga_gpui::{Island, Motion, Spring};
use gpui::{
    AnyElement, AppContext, Context, Entity, IntoElement, ParentElement, Pixels, Render,
    StyleRefinement, Styled, Window, div, px, rgb,
};

/// Cells in the grid.
const CELLS: u32 = 600;

/// The dot: a place and where it heads, and the grid's count of renders,
/// with what it was at the last Play.
pub struct Dot {
    x: Motion<Pixels>,
    target: Pixels,
    grid_renders: Rc<Cell<u32>>,
    at_play: Option<u32>,
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
                    grid_renders: renders,
                    at_play: None,
                },
                |dot, window, cx| {
                    let x = dot.x.toward(dot.target, Spring::SMOOTH, window, cx);
                    // The island renders before the cached grid in a frame,
                    // so it counts the grid's renders since Play, not in all.
                    let label = match dot.at_play {
                        None => "Press Play: the grid does not render while the dot moves".into(),
                        Some(at_play) => format!(
                            "The grid has rendered {} times since Play",
                            dot.grid_renders.get() - at_play
                        ),
                    };
                    div()
                        .absolute()
                        .top(px(-56.))
                        .left_0()
                        .right_0()
                        .child(label)
                        .child(
                            div()
                                .absolute()
                                .top(px(28.))
                                .left(x)
                                .size(px(20.))
                                .rounded_full()
                                .bg(rgb(0x00ff_5c8a)),
                        )
                        .into_any_element()
                },
            )
        });
        Self { grid, dot }
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.dot.update(cx, |island, cx| {
            let dot = island.state_mut();
            dot.at_play = Some(dot.grid_renders.get());
            dot.target = if dot.target == px(0.) {
                px(560.)
            } else {
                px(0.)
            };
            cx.notify();
        });
    }
}

impl Render for IslandPage {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let grid: AnyElement = self
            .grid
            .clone()
            .cached(StyleRefinement::default().size_full())
            .into_any_element();
        div().size_full().flex().flex_col().child(
            div()
                .relative()
                .mt(px(56.))
                .flex_1()
                .child(grid)
                .child(self.dot.clone()),
        )
    }
}
