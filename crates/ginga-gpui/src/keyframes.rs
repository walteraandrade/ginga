//! Keyframes a view draws, with a cap on how often they ask for a frame.

use std::{cell::Cell, rc::Rc, time::Duration};

use ginga::{KeyframesError, Sample, Stop};
use gpui::{App, Window};

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
#[derive(Debug, Clone)]
pub struct Keyframes<T: Animate, const N: usize> {
    stops: ginga::Keyframes<T::Repr, N>,
    clock: Clock,
    every: Option<Duration>,
    /// A frame is already asked for, after `every`.
    asked: Rc<Cell<bool>>,
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
            asked: Rc::default(),
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

    /// Asks for a frame no sooner than `every` after the one being drawn, to
    /// draw a slow loop at a lower rate than the display's: 33 ms is about 30
    /// frames a second. The values stay those of each frame's time.
    #[must_use]
    pub fn at_most_every(mut self, every: Duration) -> Self {
        self.every = Some(every);
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
            let rest = self.stops.settles_at().unwrap_or(Duration::ZERO);
            let value = self.stops.sample(rest).value;
            return Sample {
                value,
                settled: true,
            };
        }
        let now = self.clock.now(cx);
        self.stops.sample(now)
    }

    fn ask_for_a_frame(&self, window: &Window, cx: &App) {
        let Some(every) = self.every else {
            window.request_animation_frame();
            return;
        };
        if self.asked.replace(true) {
            return;
        }
        let view = window.current_view();
        let asked = self.asked.clone();
        let timer = cx.background_executor().timer(every);
        cx.spawn(async move |cx| {
            timer.await;
            asked.set(false);
            cx.update(|cx| cx.notify(view));
        })
        .detach();
    }
}
