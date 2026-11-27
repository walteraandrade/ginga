//! Motion for Rust UIs, as a pure function of time.
//!
//! A [`Motion`] is a value on its way to a target. You own it, you hand it the
//! time, and it answers with the value at that time. Nothing ticks, nothing
//! allocates while it runs, and the same times always give the same values.
//!
//! A value that moves is five calls:
//!
//! | Call | What it does |
//! | --- | --- |
//! | [`Motion::at`] | A value at rest. |
//! | [`Motion::toward`] | Each frame: head for a target, get the value now. |
//! | [`Motion::set_target`] | Change course at a moment you choose. |
//! | [`Motion::jump_to`] | Move with no motion. |
//! | [`Motion::sample`] | Read the value at any time, without changing anything. |
//!
//! How the value moves is a [`Transition`]: a [`Spring`], a [`Tween`] or
//! [`Transition::Instant`]. What can move is anything [`Animatable`].
//!
//! Something that is shown and hidden, and must stay drawn while it leaves,
//! is a [`Presence`]; a list of such things is a [`PresenceList`]. A value that
//! goes through several stops, and can repeat, is [`Keyframes`].
//!
//! To check a motion without a screen, print a [`Trace`].
//!
//! ```
//! use std::time::Duration;
//! use ginga::{Motion, Spring, Trace};
//!
//! let ms = Duration::from_millis;
//! let mut panel = Motion::at((0.0, 0.0));
//!
//! // A frame at 0 ms: the panel should be at (240, 0).
//! panel.toward((240.0, 0.0), Spring::SNAPPY, ms(0));
//!
//! // A frame at 120 ms: the target moved; the spring keeps its speed.
//! let sample = panel.toward((240.0, 80.0), Spring::SNAPPY, ms(120));
//! assert!(!sample.settled);
//!
//! // Every later frame, as text.
//! let trace = Trace::of(&panel, ms(120), ms(16));
//! assert!(trace.rows().last().unwrap().settled);
//! ```

#![forbid(unsafe_code)]

mod animatable;
mod easing;
mod keyframes;
mod look;
mod motion;
mod presence;
mod presence_list;
mod spring;
mod trace;
mod transition;

pub use animatable::Animatable;
pub use easing::Easing;
pub use keyframes::{Keyframes, KeyframesError, Stop};
pub use look::{Look, LookMotion};
pub use motion::{Motion, Sample};
pub use presence::{InOut, Phase, Presence, Shown};
pub use presence_list::{PresenceList, ShownItem};
pub use spring::{Spring, SpringError};
pub use trace::{Row, Trace, Traceable};
pub use transition::{Transition, Tween};

/// The README's examples, run as tests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct Readme;
