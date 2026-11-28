//! Each page, played on GPUI's test platform: it moves when Play is pressed,
//! and then it settles and asks for no more frames.

use std::time::Duration;

use gpui::{TestAppContext, WindowHandle, px, size};

use gpui::AssetSource as _;

use crate::{
    app::{Assets, CHEVRON_PATH, Gallery, Page},
    turn::Turn,
};

/// Frames a page may take to settle, at 60 a second: ten seconds.
const MOST_FRAMES: usize = 600;

/// Delivers frames 16 ms apart until one answers no frame request, and
/// returns how many frames asked for another. It sees frame requests only: a
/// loop capped with `at_most_every` asks with a timer instead, and its
/// settling is checked in `tests/keyframes.rs`.
fn frames_until_settled(window: &WindowHandle<Gallery>, cx: &mut TestAppContext) -> usize {
    for asked in 0..MOST_FRAMES {
        cx.executor().advance_clock(Duration::from_millis(16));
        let requests = window
            .update(cx, |_, window, cx| window.simulate_next_frame(cx))
            .unwrap();
        cx.run_until_parked();
        if requests == 0 {
            return asked;
        }
    }
    panic!("the page still asks for frames after {MOST_FRAMES}");
}

#[gpui::test]
fn every_page_moves_on_play_and_then_settles(cx: &mut TestAppContext) {
    let count = cx.update(|cx| Page::all(cx).len());
    for chosen in 0..count {
        let window = cx.open_window(size(px(960.), px(600.)), move |_, cx| {
            Gallery::new(cx, chosen)
        });
        cx.run_until_parked();
        let page = window.update(cx, |gallery, _, _| gallery.chosen()).unwrap();
        assert_eq!(
            frames_until_settled(&window, cx),
            0,
            "{}: still before Play",
            page.name
        );
        for press in ["first", "second"] {
            cx.update(|cx| (page.play)(cx));
            cx.run_until_parked();
            assert!(
                frames_until_settled(&window, cx) > 0,
                "{}: the {press} Play asked for no frame",
                page.name
            );
        }
    }
}

#[gpui::test]
fn the_turn_page_draws_its_chevron_and_turns_a_quarter(cx: &mut TestAppContext) {
    let chevron = Assets
        .load(CHEVRON_PATH)
        .unwrap()
        .expect("the chevron is there");
    assert!(chevron.starts_with(b"<svg"), "the chevron is an svg");
    assert!(
        Assets
            .list("icons")
            .unwrap()
            .iter()
            .any(|p| p.as_ref() == CHEVRON_PATH)
    );
    assert!(
        Assets.list("fonts").unwrap().is_empty(),
        "only icons are listed"
    );

    let window = cx.open_window(size(px(400.), px(100.)), |_, _| Turn::new());
    cx.run_until_parked();
    let turned =
        |cx: &mut TestAppContext| window.update(cx, |turn, _, cx| turn.turned(cx)).unwrap();
    assert_eq!(turned(cx), 0.0);
    for target in [0.25, 0.0] {
        window.update(cx, |turn, _, cx| turn.play(cx)).unwrap();
        cx.run_until_parked();
        for _ in 0..MOST_FRAMES {
            cx.executor().advance_clock(Duration::from_millis(16));
            let requests = window
                .update(cx, |_, window, cx| window.simulate_next_frame(cx))
                .unwrap();
            cx.run_until_parked();
            if requests == 0 {
                break;
            }
        }
        assert_eq!(turned(cx), target, "settled exactly on {target} of a turn");
    }
}
