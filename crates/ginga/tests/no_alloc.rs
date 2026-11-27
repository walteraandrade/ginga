//! Running a motion never touches the heap: not when it is read, and not when
//! it changes course.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    time::Duration,
};

use ginga::{
    Color, Easing, InOut, Look, LookMotion, Motion, PresenceList, Spring, Transition, Tween,
};

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
        let sample = card.toward(look, Spring::BOUNCY, now);
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
