//! A parent's named look that its children follow, checked on GPUI's test
//! platform: each element's laid-out width, frame by frame.

use std::{cell::RefCell, fmt::Write, rc::Rc, time::Duration};

use ginga_gpui::{Easing, Look, LookExt, Order, Tween, Variant};
use gpui::{
    AnyView, AppContext, Bounds, Context, Entity, IntoElement, ParentElement, Pixels, Render,
    StyleRefinement, Styled, TestAppContext, Window, WindowHandle, canvas, div, px, size,
};

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Menu {
    Closed,
    Open,
}

/// A linear tween, so the widths are exact.
fn glide() -> Tween {
    Tween::new(ms(100), Easing::Linear)
}

fn width(menu: Menu) -> Look {
    match menu {
        Menu::Closed => Look::new().width(0.),
        Menu::Open => Look::new().width(100.),
    }
}

/// The width each element was laid out at in the last frame: the parent,
/// then the children.
type Widths = Rc<RefCell<[Pixels; 4]>>;

/// An element of `widths` at `slot`, with its look from the variant.
fn measured(slot: usize, widths: &Widths) -> gpui::Div {
    let widths = widths.clone();
    div().h(px(10.)).child(
        canvas(
            move |bounds: Bounds<Pixels>, _, _| widths.borrow_mut()[slot] = bounds.size.width,
            |_, (), _, _| {},
        )
        .size_full(),
    )
}

/// The children, in a view of their own.
struct Rows {
    menu: Variant<Menu>,
    widths: Widths,
}

impl Render for Rows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let name = self.menu.name();
        div().children((0..3u32).map(|n| {
            measured(n as usize + 1, &self.widths)
                .look(("row", n), width(name), glide())
                .follows(&self.menu, n)
        }))
    }
}

/// A parent and three children. The children are drawn by the parent's view,
/// or by a cached view of their own.
struct Panel {
    menu: Variant<Menu>,
    widths: Widths,
    rows: Option<Entity<Rows>>,
}

impl Render for Panel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let name = self.menu.name();
        let rows = match &self.rows {
            Some(rows) => AnyView::from(rows.clone())
                .cached(StyleRefinement::default())
                .into_any_element(),
            None => div()
                .children((0..3u32).map(|n| {
                    measured(n as usize + 1, &self.widths)
                        .look(("row", n), width(name), glide())
                        .follows(&self.menu, n)
                }))
                .into_any_element(),
        };
        // The rows are laid out inside the panel, so a panel that goes after
        // them hears when they end before it changes course.
        measured(0, &self.widths)
            .look("panel", width(name), glide())
            .leads(&self.menu)
            .child(rows)
    }
}

fn open(
    cx: &mut TestAppContext,
    order: Order,
    own_view: bool,
) -> (WindowHandle<Panel>, Variant<Menu>, Widths) {
    let menu = Variant::new(Menu::Closed).stagger(ms(50)).order(order);
    let widths = Widths::default();
    let window = cx.open_window(size(px(400.), px(300.)), {
        let (menu, widths) = (menu.clone(), widths.clone());
        move |_, cx: &mut Context<Panel>| Panel {
            rows: own_view.then(|| {
                cx.new(|_| Rows {
                    menu: menu.clone(),
                    widths: widths.clone(),
                })
            }),
            menu,
            widths,
        }
    });
    cx.run_until_parked();
    (window, menu, widths)
}

/// Each element's width every 50 ms after the menu opens, as a table.
fn schedule(
    window: &WindowHandle<Panel>,
    menu: &Variant<Menu>,
    widths: &Widths,
    cx: &mut TestAppContext,
) -> String {
    cx.update(|cx| menu.set(Menu::Open, cx));
    cx.run_until_parked();
    let mut table = String::from("  ms  panel  row0  row1  row2\n");
    for t in (0..=300).step_by(50) {
        if t > 0 {
            cx.executor().advance_clock(ms(50));
            window
                .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                .unwrap();
            cx.run_until_parked();
        }
        write!(table, "{t:>4}").unwrap();
        for w in *widths.borrow() {
            write!(table, "  {:>4}", f32::from(w)).unwrap();
        }
        table.push('\n');
    }
    table
}

#[gpui::test]
fn the_rows_follow_the_panel_one_step_apart(cx: &mut TestAppContext) {
    let (window, menu, widths) = open(cx, Order::ParentFirst, false);
    assert_eq!(
        schedule(&window, &menu, &widths, cx),
        "  ms  panel  row0  row1  row2
   0     0     0     0     0
  50    50     0     0     0
 100   100     0     0     0
 150   100    50     0     0
 200   100   100    50     0
 250   100   100   100    50
 300   100   100   100   100
"
    );
}

#[gpui::test]
fn the_panel_goes_after_its_rows(cx: &mut TestAppContext) {
    let (window, menu, widths) = open(cx, Order::ChildrenFirst, false);
    assert_eq!(
        schedule(&window, &menu, &widths, cx),
        "  ms  panel  row0  row1  row2
   0     0     0     0     0
  50     0    50     0     0
 100     0   100    50     0
 150     0   100   100    50
 200     0   100   100   100
 250    50   100   100   100
 300   100   100   100   100
"
    );
}

#[gpui::test]
fn rows_in_a_cached_view_of_their_own_follow_too(cx: &mut TestAppContext) {
    let (window, menu, widths) = open(cx, Order::ParentFirst, true);
    let table = schedule(&window, &menu, &widths, cx);
    assert!(
        table.ends_with(" 300   100   100   100   100\n"),
        "the cached view is told when the name changes:\n{table}"
    );
    assert!(
        table.contains(" 150   100    50     0     0\n"),
        "and keeps the schedule:\n{table}"
    );
}

#[gpui::test]
fn reduced_motion_changes_every_element_at_once(cx: &mut TestAppContext) {
    let (window, menu, widths) = open(cx, Order::ParentFirst, false);
    cx.update(|cx| cx.set_reduce_motion(true));
    let table = schedule(&window, &menu, &widths, cx);
    assert!(
        table.contains("   0   100   100   100   100\n"),
        "no schedule under reduced motion:\n{table}"
    );
}
