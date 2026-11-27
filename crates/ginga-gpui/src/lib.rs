//! [`ginga`] motion for GPUI views.
//!
//! Keep a [`Motion`] in your view, next to the state it animates, and call
//! [`Motion::toward`] while rendering:
//!
//! ```ignore
//! struct Panel {
//!     open: bool,
//!     width: ginga_gpui::Motion<Pixels>,
//! }
//!
//! impl Render for Panel {
//!     fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
//!         let target = if self.open { px(320.) } else { px(0.) };
//!         let width = self.width.toward(target, Spring::SNAPPY, window, cx);
//!         div().w(width)
//!     }
//! }
//! ```
//!
//! `toward` reads the frame's clock, asks GPUI for another frame while the
//! value is moving, and moves at once when the desktop asks for reduced motion.

#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

use ginga::Animatable;
pub use ginga::{
    Color, Easing, InOut, KeyframesError, Look, LookMotion, Phase, PresenceList, Sample, Shown,
    ShownItem, Spring, SpringError, Stop, Trace, Transition, Tween,
};
pub use keyframes::Keyframes;
pub use look::{LookExt, LookState, Looked, color};

mod keyframes;
mod look;
use gpui::{
    AnyElement, App, Context, Hsla, IntoElement, Pixels, Point, Render, Rgba, Size, Window, point,
    px, size,
};

/// A GPUI value that ginga can move.
///
/// The orphan rule keeps this crate from implementing [`Animatable`] for
/// GPUI's own types, so each one names a stand-in that the engine moves.
pub trait Animate: Copy {
    /// The value as the engine sees it.
    type Repr: Animatable;

    /// The value for the engine.
    fn to_repr(self) -> Self::Repr;

    /// The value back from the engine.
    fn from_repr(repr: Self::Repr) -> Self;
}

/// Pixels as the engine sees them: they stop a tenth of a pixel from the target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InPixels<const N: usize>([f32; N]);

impl<const N: usize> Animatable for InPixels<N>
where
    [f32; N]: Default,
{
    type Channels = [f32; N];

    const PRECISION: f32 = 0.1;

    fn to_channels(self) -> [f32; N] {
        self.0
    }

    fn from_channels(channels: [f32; N]) -> Self {
        Self(channels)
    }
}

impl Animate for f32 {
    type Repr = f32;

    fn to_repr(self) -> f32 {
        self
    }

    fn from_repr(repr: f32) -> Self {
        repr
    }
}

impl Animate for Pixels {
    type Repr = InPixels<1>;

    fn to_repr(self) -> InPixels<1> {
        InPixels([f32::from(self)])
    }

    fn from_repr(InPixels([value]): InPixels<1>) -> Self {
        px(value)
    }
}

/// A color moves as the engine's [`Color`]: in linear light, its alpha
/// premultiplied, so between two colors there are only colors between them.
impl Animate for Rgba {
    type Repr = Color;

    fn to_repr(self) -> Color {
        Color::new(self.r, self.g, self.b, self.a)
    }

    fn from_repr(color: Color) -> Self {
        Rgba {
            r: color.red,
            g: color.green,
            b: color.blue,
            a: color.alpha,
        }
    }
}

/// As [`Rgba`]: the hue is not a channel, so red on its way to blue passes
/// through purple, not green.
impl Animate for Hsla {
    type Repr = Color;

    fn to_repr(self) -> Color {
        Rgba::from(self).to_repr()
    }

    fn from_repr(color: Color) -> Self {
        Rgba::from_repr(color).into()
    }
}

impl Animate for Point<Pixels> {
    type Repr = InPixels<2>;

    fn to_repr(self) -> InPixels<2> {
        InPixels([f32::from(self.x), f32::from(self.y)])
    }

    fn from_repr(InPixels([x, y]): InPixels<2>) -> Self {
        point(px(x), px(y))
    }
}

impl Animate for Size<Pixels> {
    type Repr = InPixels<2>;

    fn to_repr(self) -> InPixels<2> {
        InPixels([f32::from(self.width), f32::from(self.height)])
    }

    fn from_repr(InPixels([width, height]): InPixels<2>) -> Self {
        size(px(width), px(height))
    }
}

/// The frame's time, as the [`Duration`] that [`ginga::Motion`] counts in.
///
/// Use it when the motion lives in code that knows nothing about GPUI: that
/// code takes the time as an argument, and the view supplies it.
///
/// ```ignore
/// let now = self.clock.now(cx);
/// let frame = self.caret.step(place, transition, now);
/// if frame.moving {
///     window.request_animation_frame();
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Clock {
    /// The first frame this clock saw; time is counted from there.
    epoch: Option<Instant>,
}

impl Clock {
    /// The time of the frame being rendered, counted from this clock's first frame.
    pub fn now(&mut self, cx: &App) -> Duration {
        let now = cx.background_executor().now();
        now.saturating_duration_since(*self.epoch.get_or_insert(now))
    }
}

/// The transition to use this frame: `transition`, or none at all when the
/// desktop asks for reduced motion.
pub fn unless_reduced(transition: impl Into<Transition>, cx: &App) -> Transition {
    if cx.reduce_motion() {
        Transition::Instant
    } else {
        transition.into()
    }
}

/// A value on its way to a target, owned by a view.
#[derive(Debug, Clone, Copy)]
pub struct Motion<T: Animate> {
    motion: ginga::Motion<T::Repr>,
    clock: Clock,
}

impl<T: Animate> Motion<T> {
    /// A value at rest.
    #[must_use]
    pub fn at(value: T) -> Self {
        Self {
            motion: ginga::Motion::at(value.to_repr()),
            clock: Clock::default(),
        }
    }

    /// Heads for `target` and returns the value for this frame.
    ///
    /// Call it while rendering the view that owns the motion. While the value
    /// is moving it asks GPUI to render that view again on the next frame.
    /// When the desktop asks for reduced motion the value is at `target` at once.
    pub fn toward(
        &mut self,
        target: T,
        transition: impl Into<Transition>,
        window: &mut Window,
        cx: &mut App,
    ) -> T {
        let transition = unless_reduced(transition, cx);
        let now = self.clock.now(cx);
        let sample = self.motion.toward(target.to_repr(), transition, now);
        if !sample.settled {
            window.request_animation_frame();
        }
        T::from_repr(sample.value)
    }

    /// Puts the value at `value`, at rest, with no motion.
    pub fn jump_to(&mut self, value: T) {
        self.motion.jump_to(value.to_repr());
    }

    /// Where the motion is heading.
    #[must_use]
    pub fn target(&self) -> T {
        T::from_repr(self.motion.target())
    }

    /// The engine's own motion and this frame's time on its clock, to read it
    /// with [`ginga::Motion::sample`] or write it down with [`Trace::of`].
    pub fn inspect(&mut self, cx: &App) -> (&ginga::Motion<T::Repr>, Duration) {
        let now = self.clock.now(cx);
        (&self.motion, now)
    }
}

/// Something a view shows and hides, and keeps to draw while it leaves.
///
/// ```ignore
/// // When something happens:
/// self.toast.show(Toast::new("Saved"), enter, cx);
/// // Later:
/// self.toast.hide(exit, cx);
///
/// // While rendering:
/// if let Some(shown) = self.toast.frame(window, cx) {
///     let leaving = shown.phase == Phase::Exiting;
///     draw(shown.content).opacity(shown.progress)
/// }
/// ```
///
/// `show` and `hide` move at once when the desktop asks for reduced motion.
#[derive(Debug, Clone)]
pub struct Presence<T> {
    presence: ginga::Presence<T>,
    clock: Clock,
}

impl<T> Default for Presence<T> {
    fn default() -> Self {
        Self::absent()
    }
}

impl<T> Presence<T> {
    /// Nothing is shown.
    #[must_use]
    pub fn absent() -> Self {
        Self {
            presence: ginga::Presence::absent(),
            clock: Clock::default(),
        }
    }

    /// `content` is shown, and has been: it does not arrive.
    #[must_use]
    pub fn present(content: T) -> Self {
        Self {
            presence: ginga::Presence::present(content),
            clock: Clock::default(),
        }
    }

    /// Shows `content`. What was hidden or leaving arrives with `enter`; what
    /// was already shown only changes content.
    pub fn show(&mut self, content: T, enter: impl Into<Transition>, cx: &App) {
        let now = self.clock.now(cx);
        self.presence.show(content, unless_reduced(enter, cx), now);
    }

    /// Starts the way out with `exit`. The content stays, to be drawn, until
    /// the way out is over.
    pub fn hide(&mut self, exit: impl Into<Transition>, cx: &App) {
        let now = self.clock.now(cx);
        self.presence.hide(unless_reduced(exit, cx), now);
    }

    /// What to draw this frame, or `None` when nothing is here.
    ///
    /// Call it while rendering the view that owns the presence. While the
    /// content is arriving or leaving it asks GPUI to render that view again
    /// on the next frame.
    pub fn frame(&mut self, window: &Window, cx: &App) -> Option<Shown<'_, T>> {
        let now = self.clock.now(cx);
        if self.presence.frame(now).is_some_and(|shown| shown.moving()) {
            window.request_animation_frame();
        }
        self.presence.sample(now)
    }

    /// The content should be on screen: it was shown and not hidden since.
    #[must_use]
    pub fn is_wanted(&self) -> bool {
        self.presence.is_wanted()
    }
}

/// A view that draws on its own, so that its motion redraws little.
///
/// GPUI renders again the view that asks for an animation frame, and every
/// view above it, on each frame of a motion. Views beside them are left alone
/// only when they are cached. So a motion is cheap when three things hold:
///
/// - what moves is drawn by a small view of its own, such as this one;
/// - the heavy views are beside it, not above it, and are embedded with
///   `.cached(style)`;
/// - the views above it are cheap to render.
///
/// An island keeps a state and a function that draws it. Motions that the
/// function drives ask for frames of the island, so the island and the views
/// above it render again on each frame, and the cached views beside it don't.
///
/// ```ignore
/// struct Dot {
///     x: ginga_gpui::Motion<Pixels>,
///     target: Pixels,
/// }
///
/// let dot = cx.new(|_| {
///     Island::new(Dot { x: Motion::at(px(0.)), target: px(0.) }, |dot, window, cx| {
///         let x = dot.x.toward(dot.target, Spring::SNAPPY, window, cx);
///         div().absolute().left(x).size_2().into_any_element()
///     })
/// });
///
/// // In the parent: the heavy view is cached, the island sits beside it.
/// div()
///     .relative()
///     .child(self.document.clone().cached(StyleRefinement::default().size_full()))
///     .child(dot.clone())
/// ```
pub struct Island<S> {
    state: S,
    draw: Draw<S>,
}

type Draw<S> = Box<dyn Fn(&mut S, &mut Window, &mut Context<Island<S>>) -> AnyElement>;

impl<S: 'static> Island<S> {
    /// An island that keeps `state` and draws it with `draw`.
    pub fn new(
        state: S,
        draw: impl Fn(&mut S, &mut Window, &mut Context<Self>) -> AnyElement + 'static,
    ) -> Self {
        Self {
            state,
            draw: Box::new(draw),
        }
    }

    /// The island's state.
    pub fn state(&self) -> &S {
        &self.state
    }

    /// The island's state, to change it. Call `cx.notify()` after a change
    /// that should be drawn.
    pub fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }
}

impl<S: 'static> Render for Island<S> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        (self.draw)(&mut self.state, window, cx)
    }
}
