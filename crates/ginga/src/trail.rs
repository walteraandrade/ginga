use core::time::Duration;

use crate::Animatable;

/// How far back a trail measures a speed.
const OVER: Duration = Duration::from_millis(100);

/// How long a value may stay still before it is let go and still be thrown:
/// held still longer, its speed is zero.
const STILL_FOR: Duration = Duration::from_millis(50);

/// How far apart, at least, the places a trail keeps are: eight of them
/// then cover [`OVER`] however often places come.
const APART: Duration = Duration::from_millis(15);

/// The last places of a value someone moves by hand, such as a dragged
/// element's offset, and their times: the speed it is let go with.
///
/// The speed is measured over the last 100 ms. A value held still for 50 ms
/// before it is let go has no speed: it is set down, not thrown. A trail
/// keeps eight places in an array, so noting one never allocates.
///
/// ```
/// use std::time::Duration;
/// use ginga::Trail;
///
/// let ms = Duration::from_millis;
/// let mut trail = Trail::default();
/// for step in 0..=10u8 {
///     trail.note(ms(u64::from(step) * 10), f32::from(step) * 10.0);
/// }
/// assert_eq!(trail.speed(ms(100)), 1_000.0); // 10 px every 10 ms
/// assert_eq!(trail.speed(ms(200)), 0.0);     // still for 100 ms: set down
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Trail<T> {
    places: [Option<(Duration, T)>; 8],
    /// Where the newest place is in `places`.
    newest: usize,
}

impl<T: Copy> Default for Trail<T> {
    fn default() -> Self {
        Self {
            places: [None; 8],
            newest: 0,
        }
    }
}

impl<T: Animatable> Trail<T> {
    /// The value is at `value` at `now`. Times are noted in order.
    pub fn note(&mut self, now: Duration, value: T) {
        let close = self.places[self.newest].is_some()
            && self.places[self.before(self.newest)]
                .is_some_and(|(before, _)| now.saturating_sub(before) < APART);
        if !close && self.places[self.newest].is_some() {
            self.newest = (self.newest + 1) % self.places.len();
        }
        self.places[self.newest] = Some((now, value));
    }

    /// Forgets every place, as when the value is let go.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The value's speed at `now`, in its units per second, measured over
    /// the last 100 ms of places; zero when it has been still for 50 ms, or
    /// with fewer than two places.
    #[must_use]
    pub fn speed(&self, now: Duration) -> T {
        let still = T::Channels::default();
        let Some((last_time, last)) = self.places[self.newest] else {
            return T::from_channels(still);
        };
        if now.saturating_sub(last_time) > STILL_FOR {
            return T::from_channels(still);
        }
        let first = self
            .places
            .iter()
            .flatten()
            .filter(|(time, _)| last_time.saturating_sub(*time) <= OVER)
            .min_by_key(|(time, _)| *time);
        let Some(&(first_time, first)) = first else {
            return T::from_channels(still);
        };
        let seconds = last_time.saturating_sub(first_time).as_secs_f32();
        if seconds <= 0.0 {
            return T::from_channels(still);
        }
        let mut speed = last.to_channels();
        for (channel, from) in speed.as_mut().iter_mut().zip(first.to_channels().as_ref()) {
            *channel = (*channel - from) / seconds;
        }
        T::from_channels(speed)
    }

    fn before(&self, index: usize) -> usize {
        (index + self.places.len() - 1) % self.places.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn the_speed_is_measured_over_the_last_100_ms_only() {
        let mut trail = Trail::default();
        // Slowly for half a second, 100 px/s, then 2,000 px/s for 100 ms.
        let mut x = 0.0;
        for step in 0..50u64 {
            trail.note(ms(step * 10), x);
            x += 1.0;
        }
        for step in 50..=60u64 {
            trail.note(ms(step * 10), x);
            x += 20.0;
        }
        let speed = trail.speed(ms(600));
        assert!((speed - 2_000.0).abs() < 1.0, "{speed}");
    }

    #[test]
    fn places_that_come_often_still_cover_100_ms() {
        // A 1 kHz pointer: a place every millisecond at 1,000 px/s, and a
        // last place a little slower. Eight places one apart would measure
        // only the slow end.
        let mut trail = Trail::default();
        for step in 0..=200u64 {
            let x = if step < 200 { step as f32 } else { 199.5 };
            trail.note(ms(step), x);
        }
        let speed = trail.speed(ms(200));
        assert!(speed > 900.0 && speed < 1_000.0, "{speed}");
    }

    #[test]
    fn a_value_held_still_before_it_is_let_go_has_no_speed() {
        let mut trail = Trail::default();
        trail.note(ms(0), 0.0);
        trail.note(ms(10), 10.0);
        assert_eq!(trail.speed(ms(60)), 1_000.0, "50 ms still: still thrown");
        assert_eq!(trail.speed(ms(61)), 0.0, "more than 50 ms: set down");
    }

    #[test]
    fn one_place_or_none_has_no_speed() {
        let mut trail = Trail::<f32>::default();
        assert_eq!(trail.speed(ms(0)), 0.0);
        trail.note(ms(0), 5.0);
        assert_eq!(trail.speed(ms(0)), 0.0);
        trail.note(ms(20), 7.0);
        trail.clear();
        assert_eq!(trail.speed(ms(20)), 0.0);
    }

    #[test]
    fn each_channel_has_its_own_speed() {
        let mut trail = Trail::default();
        trail.note(ms(0), (0.0, 0.0));
        trail.note(ms(50), (50.0, -10.0));
        assert_eq!(trail.speed(ms(50)), (1_000.0, -200.0));
    }
}
