//! Running a motion never touches the heap: not when it is read, and not when
//! it changes course.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    time::Duration,
};

use ginga::{
    Coast, Color, Easing, InOut, Keyframes, Look, LookMotion, Map, Motion, Presence, PresenceList,
    Sequence, SettledOnce, Slot, Spring, Stop, Swap, Trail, Transition, Transitions, Tween,
};
use ginga::{Order, Variant};

struct Counting;

thread_local! {
    /// Allocations made by this thread. The test harness allocates on its own
    /// threads while a test runs, and those are not the motion's.
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn allocations() -> usize {
    ALLOCATIONS.with(Cell::get)
}

#[expect(
    unsafe_code,
    reason = "a counting global allocator can only be written with unsafe"
)]
// SAFETY: every call is forwarded unchanged to the system allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // `try_with`, not `with`: an allocator must never panic.
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the caller upholds `GlobalAlloc::dealloc`'s contract.
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

#[test]
fn the_counter_sees_this_threads_allocations() {
    let before = allocations();
    drop(std::hint::black_box(Box::new(1_u8)));
    assert!(
        allocations() > before,
        "the counter must count this thread's allocations, or the test below checks nothing"
    );
}

#[test]
fn reading_and_retargeting_a_motion_allocates_nothing() {
    let ms = Duration::from_millis;
    let transitions: [Transition; 5] = [
        Transition::Instant,
        Spring::SNAPPY.into(),
        Spring::BOUNCY.into(),
        Tween::new(Duration::from_millis(300), Easing::OutExpo).into(),
        Tween::new(
            Duration::from_millis(300),
            Easing::Bezier(0.16, 1.0, 0.3, 1.0),
        )
        .into(),
    ];
    let mut motion = Motion::at((0.0, 0.0, 0.0, 1.0));
    let mut checksum = 0.0;
    let before = allocations();
    for frame in 0..10_000u64 {
        let now = Duration::from_micros(frame * 8_333);
        let side = if (frame / 37).is_multiple_of(2) {
            0.0
        } else {
            400.0
        };
        // Staggered as a list would be: a delay that grows, up to a cap.
        let delay = (ms(20) * u32::try_from(frame / 37 % 4).unwrap()).min(ms(40));
        let transition = transitions[(frame / 500) as usize % transitions.len()].delay(delay);
        let sample = motion.toward((side, 120.0, side / 2.0, 1.0), transition, now);
        checksum += sample.value.0;
    }
    let after = allocations();
    assert!(checksum > 0.0);
    assert_eq!(after - before, 0);
}

#[test]
fn a_presence_that_only_leaves_allocates_nothing() {
    let fade = Tween::new(Duration::from_millis(500), Easing::Linear);
    let mut checksum = 0.0;
    let before = allocations();
    for round in 0..100u64 {
        let start = Duration::from_millis(round * 1_000);
        let mut ghost = Presence::leaving(round, fade, start);
        for frame in 0..40u64 {
            let now = start + Duration::from_millis(frame * 16);
            checksum += ghost.wanted().map_or(0.0, |_| 1.0);
            checksum += ghost.frame(now).map_or(0.0, |shown| shown.progress);
        }
    }
    let after = allocations();
    assert!(checksum > 0.0);
    assert_eq!(after - before, 0);
}

#[test]
fn moving_list_items_to_new_places_allocates_nothing() {
    let mut rows = PresenceList::new();
    rows.sync(
        (0..8u32).map(|key| (key, ())),
        InOut::INSTANT,
        Duration::ZERO,
    );
    let mut checksum = 0.0;
    let before = allocations();
    for frame in 0..10_000u32 {
        let now = Duration::from_micros(u64::from(frame) * 8_333);
        // Every 50 frames the list turns around: the first item is placed last.
        let turned = !(frame / 50).is_multiple_of(2);
        rows.place_all(
            |index, _| {
                let index = if turned { 7 - index } else { index };
                f32::from(u8::try_from(index).unwrap()) * 40.0
            },
            Spring::SNAPPY,
            now,
        );
        checksum += rows.frame(now).map(|row| row.offset.abs()).sum::<f32>();
    }
    let after = allocations();
    assert!(checksum > 0.0);
    assert_eq!(after - before, 0);
}

#[test]
fn running_a_look_allocates_nothing() {
    let mut card = LookMotion::at(Look::new().offset(0.0, 0.0).opacity(0.0));
    let mut checksum = 0.0;
    let before = allocations();
    for frame in 0..10_000u64 {
        let now = Duration::from_micros(frame * 8_333);
        // Every 37 frames the card changes look, and names a value more or less.
        let look = if (frame / 37).is_multiple_of(2) {
            Look::new().offset(0.0, 0.0).opacity(0.0)
        } else {
            Look::new()
                .offset(120.0, 40.0)
                .opacity(1.0)
                .size(320.0, 200.0)
        };
        // A transition for each value, over defaults, as an app gives them.
        let transitions = Transitions::new()
            .opacity(Tween::new(Duration::from_millis(150), Easing::Linear))
            .or(Transitions::new().all(Spring::BOUNCY));
        let sample = card.toward(look, transitions, now);
        checksum += sample.value.opacity.unwrap_or_default();
    }
    let after = allocations();
    assert!(checksum > 0.0);
    assert_eq!(after - before, 0);
}

#[test]
fn running_a_look_with_colors_allocates_nothing() {
    let rest = Look::new()
        .background(Color::hex(0x1e_2126))
        .text_color(Color::TRANSPARENT)
        .border_color(Color::hex(0x30_3540));
    let hover = Look::new()
        .background(Color::hex(0x3b_6ef5))
        .text_color(Color::hex(0xe8_eaed))
        .border_color(Color::hex(0x3b_6ef5).with_alpha(0.5));
    let mut button = LookMotion::at(rest);
    let mut checksum = 0.0;
    let before = allocations();
    for frame in 0..10_000u64 {
        let now = Duration::from_micros(frame * 8_333);
        let look = if (frame / 37).is_multiple_of(2) {
            rest
        } else {
            hover
        };
        let sample = button.toward(look, Spring::SNAPPY, now);
        checksum += sample.value.background.map_or(0.0, |color| color.blue);
    }
    let after = allocations();
    assert!(checksum > 0.0);
    assert_eq!(after - before, 0);
}

#[test]
fn building_and_reading_keyframes_allocates_nothing() {
    let ms = Duration::from_millis;
    let mut checksum = 0.0;
    let before = allocations();
    // A breath: in, hold, out, mirrored forever with a pause; and pulsing
    // dots, each started a little later, three times.
    let breath = Keyframes::new([
        Stop::new(ms(0), (0.6, 0.9), Easing::Linear),
        Stop::new(ms(900), (1.0, 1.0), Easing::InOut),
        Stop::new(ms(1_200), (1.0, 1.0), Easing::Linear),
    ])
    .unwrap()
    .forever()
    .mirrored()
    .pause(ms(300))
    // Partway through a round from its start.
    .behind(ms(700));
    let dots = [0u32, 1, 2].map(|n| {
        Keyframes::new([
            Stop::new(ms(0), 0.3, Easing::Linear),
            Stop::new(ms(300), 1.0, Easing::OutExpo),
            Stop::new(ms(600), 0.3, Easing::InOut),
        ])
        .unwrap()
        .times(3)
        .starting_at(ms(150) * n)
    });
    for frame in 0..10_000u64 {
        let now = Duration::from_micros(frame * 8_333);
        checksum += breath.sample(now).value.0;
        for dot in &dots {
            checksum += dot.sample(now).value;
        }
    }
    let after = allocations();
    assert!(checksum > 0.0);
    assert_eq!(after - before, 0);
}

#[test]
fn letting_go_and_coasting_allocates_nothing() {
    let ms = Duration::from_millis;
    let snaps = [(0.0, 0.0), (240.0, 0.0), (480.0, 0.0)];
    let mut checksum = 0.0;
    let before = allocations();
    // A card dragged, thrown, caught again and thrown at a snap point; a knob
    // thrown past its end.
    let mut card = Motion::at((0.0, 0.0));
    let mut knob = Motion::at(0.0);
    let mut trail = Trail::default();
    for throw in 0..200u64 {
        let now = ms(throw * 300);
        let speed = (throw % 7) as f32 * 400.0 - 1_200.0;
        card.jump_to(card.sample(now).value);
        card.coast((speed, -speed / 2.0), Coast::QUICK, now);
        checksum += card.sample(now + ms(16)).value.0;
        card.coast_to_nearest((speed, 0.0), Coast::SLOW, &snaps, now + ms(32));
        for place in 0..20u64 {
            trail.note(now + ms(place), (place as f32, 0.0));
        }
        checksum += trail.speed(now + ms(20)).0;
        knob.coast_within(speed, Coast::QUICK, 0.0, 200.0, now);
        for frame in 0..18 {
            let at = now + ms(frame * 16);
            checksum += card.sample(at).value.1 + knob.sample(at).value;
            checksum += card.velocity(at)[0];
        }
    }
    let after = allocations();
    assert!(checksum.is_finite());
    assert_eq!(after - before, 0);
}

#[test]
fn reading_derived_values_and_running_a_sequence_allocates_nothing() {
    let ms = Duration::from_millis;
    let width = Map::new(0.0, 1.0, 40.0, 240.0).unwrap();
    let fill = Map::new(40.0, 240.0, Color::hex(0xe5_484d), Color::hex(0x30_a46c))
        .unwrap()
        .clamped();
    let mut progress = Motion::at(0.0);
    let (mut a, mut b, mut c) = (Motion::at(0.0), Motion::at(0.0), Motion::at(0.0));
    let mut ended = SettledOnce::default();
    let mut checksum = 0.0;
    let before = allocations();
    for frame in 0..10_000u64 {
        let now = Duration::from_micros(frame * 8_333);
        if frame % 120 == 0 {
            let side = if (frame / 120).is_multiple_of(2) {
                1.0
            } else {
                0.0
            };
            progress.set_target(side, Spring::BOUNCY, now);
            let mut entrance = Sequence::starting_at(now);
            let first = entrance.then(&mut a, side, Spring::SNAPPY);
            entrance.with(&mut b, side, Tween::new(ms(200), Easing::Out));
            entrance.at(
                first.ends.saturating_sub(ms(80)),
                &mut c,
                side,
                Spring::SMOOTH,
            );
            checksum += entrance.settles_at().as_secs_f32();
        }
        let px = width.of(&progress).sample(now).value;
        checksum += px + fill.at(px).green + c.sample(now).value;
        if ended.check(c.settles_at(), now) {
            checksum += 1.0;
        }
    }
    let after = allocations();
    assert!(checksum > 0.0);
    assert_eq!(after - before, 0);
}

#[test]
fn swapping_a_slot_and_mixing_its_looks_allocates_nothing() {
    let fade = Tween::new(Duration::from_millis(120), Easing::Linear);
    let ways = InOut::new(fade, Spring::SNAPPY);
    let from = Look::new().offset(0.0, 24.0).opacity(0.0);
    let mut slot = Slot::new(Swap::Wait);
    let mut checksum = 0.0;
    let mut gone_count = 0;
    let before = allocations();
    for frame in 0..10_000u64 {
        let now = Duration::from_micros(frame * 8_333);
        // Every 29 frames the title changes; every 61 it is hidden.
        if frame % 61 == 0 {
            slot.hide(ways.exit, now);
        } else {
            slot.show(frame / 29 % 3, ways, now);
        }
        for shown in slot.frame(now).into_iter().flatten() {
            let look = from.mix(Look::new(), shown.progress);
            checksum += look.opacity.unwrap_or_default();
        }
        while let Some(gone) = slot.take_gone() {
            gone_count += gone + 1;
        }
    }
    let after = allocations();
    assert!(checksum > 0.0);
    assert!(gone_count > 0, "contents were gone and handed back");
    assert_eq!(after - before, 0);
}

#[test]
fn switching_a_variant_and_following_it_allocates_nothing() {
    let shown = Look::new().offset(0.0, 0.0).opacity(1.0);
    let hidden = Look::new().offset(0.0, 12.0).opacity(0.0);
    let transitions = Transitions::new()
        .opacity(Tween::new(Duration::from_millis(120), Easing::Linear))
        .or(Transitions::new().all(Spring::SNAPPY));
    let mut checksum = 0.0;
    let before = allocations();
    for order in [Order::ParentFirst, Order::ChildrenFirst, Order::Together] {
        let mut menu = Variant::new(false)
            .stagger(Duration::from_millis(40))
            .order(order);
        let mut panel = LookMotion::at(hidden);
        let mut rows = [LookMotion::at(hidden); 4];
        for frame in 0..5_000u64 {
            let now = Duration::from_micros(frame * 8_333);
            // Every 53 frames the menu switches, and the schedule starts again;
            // every other round each change gives its own order.
            let open = (frame / 53).is_multiple_of(2);
            let switched = if (frame / 106).is_multiple_of(2) {
                menu.set(open, now)
            } else {
                let order = if open {
                    Order::ParentFirst
                } else {
                    Order::ChildrenFirst
                };
                menu.set_in_order(open, order, now)
            };
            if switched {
                let look = if menu.name() { shown } else { hidden };
                let set_panel = |menu: &mut Variant<bool>, panel: &mut LookMotion| {
                    panel.set_target(look, transitions.delay(menu.parent_delay(now)), now);
                    menu.parent_ends(panel.settles_at().unwrap_or(now));
                };
                if !menu.parent_waits_for_children() {
                    set_panel(&mut menu, &mut panel);
                }
                for (n, row) in (0..).zip(&mut rows) {
                    row.set_target(look, transitions.delay(menu.child_delay(n, now)), now);
                    menu.child_ends(row.settles_at().unwrap_or(now));
                }
                if menu.parent_waits_for_children() {
                    set_panel(&mut menu, &mut panel);
                }
            }
            checksum += panel.sample(now).value.opacity.unwrap_or_default();
            for row in &rows {
                checksum += row.sample(now).value.opacity.unwrap_or_default();
            }
        }
    }
    let after = allocations();
    assert!(checksum > 0.0);
    assert_eq!(after - before, 0);
}
