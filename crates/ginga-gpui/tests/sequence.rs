//! A sequence in a view, on the view's clock, checked on GPUI's test platform.

use std::time::Duration;

use ginga_gpui::{Clock, Easing, Look, LookMotion, Transitions, Tween, ginga::Motion, sequence};
use gpui::TestAppContext;

fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

fn linear(millis: u64) -> Tween {
    Tween::new(ms(millis), Easing::Linear)
}

/// A card that fades in after 20 ms, then a count to 10 after 40 ms more,
/// started 300 ms after the
/// clock's first frame. Returns the card's opacity and the count at each of
/// `times`, counted from the start.
fn entrance(cx: &mut TestAppContext, times: &[u64]) -> Vec<(f32, f32)> {
    let mut clock = Clock::default();
    cx.update(|cx| clock.now(cx));
    cx.executor().advance_clock(ms(300));
    let mut card = LookMotion::at(Look::new().opacity(0.0));
    let mut count = Motion::at(0.0_f32);
    let start = cx.update(|cx| {
        let now = clock.now(cx);
        let mut entrance = sequence(now, cx);
        let fade = Transitions::new().opacity(linear(100).delay(ms(20)));
        entrance.then(&mut card, Look::new().opacity(1.0), fade);
        entrance.then(&mut count, 10.0, linear(200).delay(ms(40)));
        now
    });
    assert_eq!(start, ms(300));
    let mut at = 0;
    times
        .iter()
        .map(|&t| {
            cx.executor().advance_clock(ms(t - at));
            at = t;
            let now = cx.update(|cx| clock.now(cx));
            (
                card.sample(now).value.opacity.unwrap(),
                count.sample(now).value,
            )
        })
        .collect()
}

#[gpui::test]
fn a_look_then_a_value_follow_each_other_on_the_views_clock(cx: &mut TestAppContext) {
    assert_eq!(
        entrance(cx, &[0, 20, 70, 120, 160, 260, 360]),
        [
            (0.0, 0.0),
            (0.0, 0.0),
            (0.5, 0.0),
            (1.0, 0.0),
            (1.0, 0.0),
            (1.0, 5.0),
            (1.0, 10.0)
        ]
    );
}

#[gpui::test]
fn under_reduced_motion_each_step_is_there_after_its_own_delay(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    assert_eq!(
        entrance(cx, &[0, 19, 20, 39, 40, 340]),
        [
            (0.0, 0.0),
            (0.0, 0.0),
            (1.0, 0.0),
            (1.0, 0.0),
            (1.0, 10.0),
            (1.0, 10.0)
        ]
    );
}
