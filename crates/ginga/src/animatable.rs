use core::fmt::Debug;

/// A value the engine can move: a small, fixed set of `f32` channels.
///
/// The engine moves each channel on its own and keeps a velocity for each, so
/// a spring retargeted mid-flight keeps its momentum on every channel.
///
/// ```
/// use ginga::Animatable;
///
/// #[derive(Clone, Copy)]
/// struct Size {
///     width: f32,
///     height: f32,
/// }
///
/// impl Animatable for Size {
///     type Channels = [f32; 2];
///
///     fn to_channels(self) -> [f32; 2] {
///         [self.width, self.height]
///     }
///
///     fn from_channels([width, height]: [f32; 2]) -> Self {
///         Self { width, height }
///     }
/// }
/// ```
pub trait Animatable: Copy {
    /// The value as numbers, normally `[f32; N]`.
    type Channels: Copy + Debug + Default + PartialEq + AsRef<[f32]> + AsMut<[f32]>;

    /// How close to its target a channel must be for the motion to stop.
    ///
    /// It is in the value's own units. The default suits values in `0.0..=1.0`;
    /// a value in pixels stops sooner, and asks for fewer frames, with `0.1`.
    const PRECISION: f32 = 0.001;

    /// Splits the value into its channels.
    fn to_channels(self) -> Self::Channels;

    /// Builds the value back from its channels.
    fn from_channels(channels: Self::Channels) -> Self;
}

impl Animatable for f32 {
    type Channels = [f32; 1];

    fn to_channels(self) -> [f32; 1] {
        [self]
    }

    fn from_channels([value]: [f32; 1]) -> Self {
        value
    }
}

impl Animatable for (f32, f32) {
    type Channels = [f32; 2];

    fn to_channels(self) -> [f32; 2] {
        [self.0, self.1]
    }

    fn from_channels([a, b]: [f32; 2]) -> Self {
        (a, b)
    }
}

impl Animatable for (f32, f32, f32) {
    type Channels = [f32; 3];

    fn to_channels(self) -> [f32; 3] {
        [self.0, self.1, self.2]
    }

    fn from_channels([a, b, c]: [f32; 3]) -> Self {
        (a, b, c)
    }
}

impl Animatable for (f32, f32, f32, f32) {
    type Channels = [f32; 4];

    fn to_channels(self) -> [f32; 4] {
        [self.0, self.1, self.2, self.3]
    }

    fn from_channels([a, b, c, d]: [f32; 4]) -> Self {
        (a, b, c, d)
    }
}

impl<const N: usize> Animatable for [f32; N]
where
    [f32; N]: Default,
{
    type Channels = [f32; N];

    fn to_channels(self) -> [f32; N] {
        self
    }

    fn from_channels(channels: [f32; N]) -> Self {
        channels
    }
}
