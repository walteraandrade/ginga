//! A parent's named look that its children follow, checked on GPUI's test
//! platform: each element's laid-out width, frame by frame.

use std::{cell::RefCell, fmt::Write, rc::Rc, time::Duration};

use ginga_gpui::{Easing, Look, LookExt, LookState, Order, Tween, Variant};
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

/// Each element's look for the name, widened by `extra` for a change of
/// look that has nothing to do with the name.
fn width(menu: Menu, extra: f32) -> Look {
    match menu {
        Menu::Closed => Look::new().width(extra),
        Menu::Open => Look::new().width(100. + extra),
    }
}

/// The width each element was laid out at in the last frame: the parent,
/// then the children.
type Widths = Rc<RefCell<[Pixels; 4]>>;

/// An element of `widths` at `slot`.
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

/// The three rows, each following `menu`, with its motion kept with its id
/// or in `states`.
fn rows(
    menu: &Variant<Menu>,
    widths: &Widths,
    extra: f32,
    states: Option<&[LookState; 3]>,
) -> gpui::Div {
    let name = menu.name();
    div().children((0..3u32).map(|n| {
        let row = measured(n as usize + 1, widths);
        let row = match states {
            Some(states) => row.look_with(&states[n as usize], width(name, extra), glide()),
            None => row.look(("row", n), width(name, extra), glide()),
        };
        row.follows(menu, n)
    }))
}

/// The children, in a view of their own.
struct Rows {
    menu: Variant<Menu>,
    widths: Widths,
}

impl Render for Rows {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        rows(&self.menu, &self.widths, 0., None)
    }
}

/// How the rows are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drawn {
    /// By the panel's view, their motion kept with their ids.
    Inside,
    /// By the panel's view, their motion kept in a `LookState` each.
    Owned,
    /// By a cached view of their own.
    Cached,
}

/// A parent and three children inside it.
struct Panel {
    menu: Variant<Menu>,
    widths: Widths,
    cached: Option<Entity<Rows>>,
    states: Option<[LookState; 3]>,
    /// Widens every element, for a change of look that is not a change of name.
    extra: f32,
}

impl Render for Panel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let children = match &self.cached {
            Some(view) => AnyView::from(view.clone())
                .cached(StyleRefinement::default())
                .into_any_element(),
            None => {
                rows(&self.menu, &self.widths, self.extra, self.states.as_ref()).into_any_element()
            }
        };
        // The rows are laid out inside the panel, so a panel that goes after
        // them hears when they end before it changes course.
        measured(0, &self.widths)
            .look("panel", width(self.menu.name(), self.extra), glide())
            .leads(&self.menu)
            .child(children)
    }
}

struct Opened {
    window: WindowHandle<Panel>,
    menu: Variant<Menu>,
    widths: Widths,
}

fn open(cx: &mut TestAppContext, order: Order, drawn: Drawn) -> Opened {
    let menu = Variant::new(Menu::Closed).stagger(ms(50)).order(order);
    let widths = Widths::default();
    let window = cx.open_window(size(px(400.), px(300.)), {
        let (menu, widths) = (menu.clone(), widths.clone());
        move |_, cx: &mut Context<Panel>| Panel {
            cached: (drawn == Drawn::Cached).then(|| {
                cx.new(|_| Rows {
                    menu: menu.clone(),
                    widths: widths.clone(),
                })
            }),
            states: (drawn == Drawn::Owned).then(Default::default),
            menu,
            widths,
            extra: 0.,
        }
    });
    cx.run_until_parked();
    Opened {
        window,
        menu,
        widths,
    }
}

impl Opened {
    /// Moves the clock by `millis` and delivers the next frame; returns how
    /// many frame requests it answered.
    fn after(&self, millis: u64, cx: &mut TestAppContext) -> usize {
        cx.executor().advance_clock(ms(millis));
        let requests = self
            .window
            .update(cx, |_, window, cx| window.simulate_next_frame(cx))
            .unwrap();
        cx.run_until_parked();
        requests
    }

    fn set(&self, name: Menu, cx: &mut TestAppContext) {
        cx.update(|cx| self.menu.set(name, cx));
        cx.run_until_parked();
    }

    fn row(&self, t: u64) -> String {
        let mut row = format!("{t:>4}");
        for w in *self.widths.borrow() {
            write!(row, "  {:>4}", f32::from(w)).unwrap();
        }
        row + "\n"
    }

    /// Each element's width now and every 50 ms after, up to `until`.
    fn table(&self, until: u64, cx: &mut TestAppContext) -> String {
        let mut table = String::from("  ms  panel  row0  row1  row2\n") + &self.row(0);
        for t in (50..=until).step_by(50) {
            self.after(50, cx);
            table += &self.row(t);
        }
        table
    }

    /// Opens the menu and writes down the next 300 ms.
    fn schedule(&self, cx: &mut TestAppContext) -> String {
        self.set(Menu::Open, cx);
        self.table(300, cx)
    }
}

const ROWS_AFTER_THE_PANEL: &str = "  ms  panel  row0  row1  row2
   0     0     0     0     0
  50    50     0     0     0
 100   100     0     0     0
 150   100    50     0     0
 200   100   100    50     0
 250   100   100   100    50
 300   100   100   100   100
";

const THE_PANEL_AFTER_ITS_ROWS: &str = "  ms  panel  row0  row1  row2
   0     0     0     0     0
  50     0    50     0     0
 100     0   100    50     0
 150     0   100   100    50
 200     0   100   100   100
 250    50   100   100   100
 300   100   100   100   100
";

#[gpui::test]
fn the_rows_follow_the_panel_one_step_apart(cx: &mut TestAppContext) {
    let menu = open(cx, Order::ParentFirst, Drawn::Inside);
    assert_eq!(menu.schedule(cx), ROWS_AFTER_THE_PANEL);
    assert_eq!(menu.after(16, cx), 0, "settled: no more frames");
}

#[gpui::test]
fn the_panel_goes_after_its_rows(cx: &mut TestAppContext) {
    let menu = open(cx, Order::ChildrenFirst, Drawn::Inside);
    assert_eq!(menu.schedule(cx), THE_PANEL_AFTER_ITS_ROWS);
    assert_eq!(menu.after(16, cx), 0, "settled: no more frames");
}

#[gpui::test]
fn together_the_panel_moves_with_the_first_row(cx: &mut TestAppContext) {
    let menu = open(cx, Order::Together, Drawn::Inside);
    assert_eq!(
        menu.schedule(cx),
        "  ms  panel  row0  row1  row2
   0     0     0     0     0
  50    50    50     0     0
 100   100   100    50     0
 150   100   100   100    50
 200   100   100   100   100
 250   100   100   100   100
 300   100   100   100   100
"
    );
}

#[gpui::test]
fn rows_kept_in_look_states_follow_the_same_way(cx: &mut TestAppContext) {
    let menu = open(cx, Order::ParentFirst, Drawn::Owned);
    assert_eq!(menu.schedule(cx), ROWS_AFTER_THE_PANEL);
}

#[gpui::test]
fn rows_in_a_cached_view_of_their_own_follow_too(cx: &mut TestAppContext) {
    let menu = open(cx, Order::ParentFirst, Drawn::Cached);
    assert_eq!(menu.schedule(cx), ROWS_AFTER_THE_PANEL);
}

#[gpui::test]
fn a_panel_waits_for_rows_in_a_cached_view(cx: &mut TestAppContext) {
    let menu = open(cx, Order::ChildrenFirst, Drawn::Cached);
    assert_eq!(menu.schedule(cx), THE_PANEL_AFTER_ITS_ROWS);
}

#[gpui::test]
fn a_change_of_look_without_a_new_name_goes_at_once(cx: &mut TestAppContext) {
    let menu = open(cx, Order::ParentFirst, Drawn::Inside);
    menu.schedule(cx);
    menu.window
        .update(cx, |panel, _, cx| {
            panel.extra = 50.;
            cx.notify();
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(
        menu.table(100, cx),
        "  ms  panel  row0  row1  row2
   0   100   100   100   100
  50   125   125   125   125
 100   150   150   150   150
"
    );
}

#[gpui::test]
fn a_name_set_back_midway_starts_the_schedule_again(cx: &mut TestAppContext) {
    let menu = open(cx, Order::ParentFirst, Drawn::Inside);
    menu.set(Menu::Open, cx);
    menu.after(50, cx);
    // Halfway through the panel, the menu closes: the panel turns back at
    // once, and the rows, which had not moved, are told to stay closed.
    menu.set(Menu::Closed, cx);
    assert_eq!(
        menu.table(250, cx),
        "  ms  panel  row0  row1  row2
   0    50     0     0     0
  50    25     0     0     0
 100     0     0     0     0
 150     0     0     0     0
 200     0     0     0     0
 250     0     0     0     0
"
    );
}

#[gpui::test]
fn reduced_motion_changes_every_element_at_once(cx: &mut TestAppContext) {
    let menu = open(cx, Order::ParentFirst, Drawn::Inside);
    cx.update(|cx| cx.set_reduce_motion(true));
    let table = menu.schedule(cx);
    assert!(
        table.contains("   0   100   100   100   100\n"),
        "no schedule under reduced motion:\n{table}"
    );
}
