//! Keyframes a view draws, with a cap on how often they ask for a frame.

use std::time::Duration;

use ginga::{KeyframesError, Sample, Stop};
use gpui::{App, EntityId, Window};

use crate::{Animate, Clock};

/// Keyframes owned by a view: a value through several stops, which can repeat.
///
/// ```ignore
/// // Built once, with the view:
/// let breath = Keyframes::new([
///     Stop::new(ms(0), 0.6, Easing::Linear),
///     Stop::new(ms(1_200), 1.0, Easing::InOutSine),
/// ])?
/// .forever()
/// .mirrored()
/// .at_most_every(Duration::from_millis(33));
///
/// // While rendering:
/// let opacity = self.breath.frame(window, cx);
/// mark().opacity(opacity)
/// ```
///
/// They start on the first frame that draws them; [`restart`](Self::restart)
/// plays them again. While they move they ask GPUI to render the view again:
/// on the next frame, or with [`at_most_every`](Self::at_most_every), no
/// sooner than a set time after the last one. When the desktop asks for
/// reduced motion they rest where they end (or, when they repeat forever, on
/// the first stop) and ask for no frames.
///
/// The builders mirror [`ginga::Keyframes`]'s rather than taking one built
/// there: the engine's keyframes hold the stand-in of each value, such as
/// [`InPixels`](crate::InPixels) for pixels, which a view should not have to
/// spell.
#[derive(Debug, Clone, Copy)]
pub struct Keyframes<T: Animate, const N: usize> {
    stops: ginga::Keyframes<T::Repr, N>,
    clock: Clock,
    every: Option<Duration>,
    /// The view a capped frame is asked for, and the time until which it
    /// is: until then, that view asks for no other.
    asked: Option<(EntityId, Duration)>,
}

impl<T: Animate, const N: usize> Keyframes<T, N> {
    /// Keyframes through `stops`, played once.
    ///
    /// # Errors
    ///
    /// As [`ginga::Keyframes::new`]: when there are no stops, when the first
    /// is not at zero, or when they are not in order of time.
    pub fn new(stops: [Stop<T>; N]) -> Result<Self, KeyframesError> {
        let stops = stops.map(|stop| Stop::new(stop.at, stop.value.to_repr(), stop.easing));
        Ok(Self {
            stops: ginga::Keyframes::new(stops)?,
            clock: Clock::default(),
            every: None,
            asked: None,
        })
    }

    /// Plays `rounds` rounds in all. See [`ginga::Keyframes::times`].
    #[must_use]
    pub fn times(mut self, rounds: u32) -> Self {
        self.stops = self.stops.times(rounds);
        self
    }

    /// Plays round after round, forever. See [`ginga::Keyframes::forever`].
    #[must_use]
    pub fn forever(mut self) -> Self {
        self.stops = self.stops.forever();
        self
    }

    /// Plays every second round backward. See [`ginga::Keyframes::mirrored`].
    #[must_use]
    pub fn mirrored(mut self) -> Self {
        self.stops = self.stops.mirrored();
        self
    }

    /// Holds still for `pause` between rounds. See [`ginga::Keyframes::pause`].
    #[must_use]
    pub fn pause(mut self, pause: Duration) -> Self {
        self.stops = self.stops.pause(pause);
        self
    }

    /// Starts `start` after the first frame that draws them instead of on
    /// it; before then they rest on the first stop. See
    /// [`ginga::Keyframes::starting_at`].
    #[must_use]
    pub fn starting_at(mut self, start: Duration) -> Self {
        self.stops = self.stops.starting_at(start);
        self
    }

    /// Plays `lag` behind the same keyframes without it, partway through a
    /// round from the first frame: copies of one loop, each a step further
    /// behind, play it one after another. See [`ginga::Keyframes::behind`].
    #[must_use]
    pub fn behind(mut self, lag: Duration) -> Self {
        self.stops = self.stops.behind(lag);
        self
    }

    /// Asks for a frame no sooner than `every` after the one being drawn, to
    /// draw a slow loop at a lower rate than the display's. The values stay
    /// those of each frame's time.
    ///
    /// The frame after the wait is drawn when the display next draws, so the
    /// rate is a little lower than one every `every`: at 60 Hz, 30 ms gives
    /// about 30 frames a second, and 33 ms about 20. It caps only the frames
    /// these keyframes ask for: another motion in the same view, or in a view
    /// below it, that asks for every frame draws the view, and these keyframes
    /// with it, on every frame. Draw a capped loop in a view of its own, such
    /// as an [`Island`](crate::Island). A cap of zero is no cap: the loop asks
    /// for every frame.
    #[must_use]
    pub fn at_most_every(mut self, every: Duration) -> Self {
        self.every = (!every.is_zero()).then_some(every);
        self
    }

    /// Plays the keyframes again from their first stop, now.
    pub fn restart(&mut self, cx: &App) {
        let now = self.clock.now(cx);
        self.stops = self.stops.starting_at(now);
    }

    /// The value to draw this frame.
    ///
    /// Call it while rendering the view that owns the keyframes. While they
    /// move it asks GPUI to render that view again.
    pub fn frame(&mut self, window: &Window, cx: &App) -> T {
        let sample = self.sample(cx);
        if !sample.settled {
            self.ask_for_a_frame(window, cx);
        }
        T::from_repr(sample.value)
    }

    /// The keyframes' own sample this frame, or where they rest under reduced
    /// motion.
    fn sample(&mut self, cx: &App) -> Sample<T::Repr> {
        if cx.reduce_motion() {
            return Sample {
                value: self.stops.rest(),
                settled: true,
            };
        }
        let now = self.clock.now(cx);
        self.stops.sample(now)
    }

    fn ask_for_a_frame(&mut self, window: &Window, cx: &App) {
        let Some(every) = self.every else {
            window.request_animation_frame();
            return;
        };
        let view = window.current_view();
        let now = self.clock.now(cx);
        if self
            .asked
            .is_some_and(|(asked, until)| asked == view && now < until)
        {
            return;
        }
        self.asked = Some((view, now.saturating_add(every)));
        let timer = cx.background_executor().timer(every);
        cx.spawn(async move |cx| {
            timer.await;
            cx.update(|cx| cx.notify(view));
        })
        .detach();
    }
}
