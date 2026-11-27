use crate::Animatable;

/// A color: red, green and blue as sRGB, the way colors are written, and an
/// alpha, each from `0.0` to `1.0`.
///
/// A color moves in linear light with its alpha premultiplied, so between two
/// colors there are only colors that sit between them: red on its way to blue
/// passes through purple, never green, and a color fading in from
/// transparent never darkens. Why this way: `brain/decisions/` in the
/// repository.
///
/// ```
/// use ginga::Color;
///
/// let blue = Color::hex(0x3b_6ef5);
/// assert_eq!(blue.alpha, 1.0);
/// assert_eq!(blue.with_alpha(0.5).alpha, 0.5);
/// assert_eq!(Color::new(1.2, 0.5, -0.1, 1.0), Color::new(1.0, 0.5, 0.0, 1.0)); // held to 0..=1
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red, as sRGB.
    pub red: f32,
    /// Green, as sRGB.
    pub green: f32,
    /// Blue, as sRGB.
    pub blue: f32,
    /// How opaque: `0.0` transparent, `1.0` opaque.
    pub alpha: f32,
}

impl Color {
    /// Nothing at all: black with no alpha. Fading from it to a color shows
    /// only that color, more and more opaque.
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    /// A color from its red, green, blue (sRGB) and alpha. Each is held to
    /// `0.0..=1.0`, so a color at rest is drawn as it is while it moves.
    #[must_use]
    pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red: red.clamp(0.0, 1.0),
            green: green.clamp(0.0, 1.0),
            blue: blue.clamp(0.0, 1.0),
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    /// An opaque color written as `0xRRGGBB`. For an alpha, add
    /// [`with_alpha`](Self::with_alpha).
    ///
    /// # Panics
    ///
    /// When `rgb` has more than six hex digits, such as `0xRRGGBBAA`; in a
    /// `const` it does not build.
    #[must_use]
    pub const fn hex(rgb: u32) -> Self {
        assert!(
            rgb <= 0x00ff_ffff,
            "Color::hex takes 0xRRGGBB; for an alpha, use .with_alpha(..)"
        );
        let [_, red, green, blue] = rgb.to_be_bytes();
        Self::new(
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
            1.0,
        )
    }

    /// The same color with `alpha`, held to `0.0..=1.0`.
    #[must_use]
    pub const fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Each value is a number.
    pub(crate) fn is_finite(self) -> bool {
        [self.red, self.green, self.blue, self.alpha]
            .iter()
            .all(|value| value.is_finite())
    }
}

/// An sRGB value in linear light.
fn linear(srgb: f32) -> f32 {
    if srgb <= 0.040_45 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

/// A value in linear light as sRGB.
fn srgb(linear: f32) -> f32 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else if linear >= 1.0 {
        // Exactly, where the formula gives 0.99999994.
        1.0
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// The channels are red, green and blue in linear light, each multiplied by
/// the alpha, and the alpha.
impl Animatable for Color {
    type Channels = [f32; 4];

    /// A ten-thousandth in linear light: near black a thousandth would already
    /// be a step of several sRGB levels.
    const PRECISION: f32 = 0.000_1;

    fn to_channels(self) -> [f32; 4] {
        let alpha = self.alpha.clamp(0.0, 1.0);
        [
            linear(self.red.clamp(0.0, 1.0)) * alpha,
            linear(self.green.clamp(0.0, 1.0)) * alpha,
            linear(self.blue.clamp(0.0, 1.0)) * alpha,
            alpha,
        ]
    }

    fn from_channels([red, green, blue, alpha]: [f32; 4]) -> Self {
        // A spring can carry a channel past its ends on the way. The colors
        // are divided by the alpha as it is, so they keep their hue when the
        // alpha passes 1; only then is each held to its range.
        if alpha <= 0.0 {
            return Self::TRANSPARENT;
        }
        let straight = |channel: f32| srgb((channel / alpha).clamp(0.0, 1.0));
        Self::new(straight(red), straight(green), straight(blue), alpha)
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::*;
    use crate::{Easing, Motion, Tween};

    fn halfway(from: Color, to: Color) -> Color {
        let ms = Duration::from_millis;
        let mut motion = Motion::at(from);
        motion.set_target(to, Tween::new(ms(100), Easing::Linear), ms(0));
        motion.sample(ms(50)).value
    }

    fn close(left: Color, right: Color) -> bool {
        let channels = |color: Color| [color.red, color.green, color.blue, color.alpha];
        channels(left)
            .iter()
            .zip(channels(right))
            .all(|(a, b)| (a - b).abs() < 1e-3)
    }

    #[test]
    fn halfway_from_red_to_blue_is_purple_and_never_green() {
        let red = Color::hex(0xff_0000);
        let blue = Color::hex(0x00_00ff);
        let middle = halfway(red, blue);
        // Half of each in linear light, 0.5, is 0.735 as sRGB.
        assert!(
            close(middle, Color::new(0.735, 0.0, 0.735, 1.0)),
            "{middle:?}"
        );
    }

    #[test]
    fn halfway_from_transparent_a_color_is_itself_half_opaque() {
        let blue = Color::hex(0x3b_6ef5);
        let middle = halfway(Color::TRANSPARENT, blue);
        assert!(close(middle, blue.with_alpha(0.5)), "{middle:?}");
        let faded = halfway(blue, Color::TRANSPARENT);
        assert!(close(faded, blue.with_alpha(0.5)), "{faded:?}");
    }

    #[test]
    fn a_color_comes_back_from_its_channels() {
        for color in [
            Color::hex(0x3b_6ef5),
            Color::hex(0x00_0001).with_alpha(0.004),
            Color::new(0.02, 0.5, 1.0, 0.25),
        ] {
            assert!(close(Color::from_channels(color.to_channels()), color));
        }
    }

    #[test]
    fn every_srgb_level_comes_back_from_its_channels_within_a_few_bits() {
        for level in 0..=255u8 {
            let value = f32::from(level) / 255.0;
            let color = Color::new(value, value, value, 1.0);
            let back = Color::from_channels(color.to_channels());
            assert!((back.red - value).abs() < 1e-6, "{level}: {}", back.red);
        }
    }

    #[test]
    fn two_partly_opaque_colors_mix_weighted_by_their_alphas() {
        // Red at 0.2 and blue at 0.8: halfway the alpha is 0.5, and the red,
        // which brings a quarter as much light, is a fifth of the color.
        let middle = halfway(
            Color::hex(0xff_0000).with_alpha(0.2),
            Color::hex(0x00_00ff).with_alpha(0.8),
        );
        assert!((middle.alpha - 0.5).abs() < 1e-6);
        assert!((middle.red - srgb(0.2)).abs() < 1e-5, "{middle:?}");
        assert!((middle.blue - srgb(0.8)).abs() < 1e-5, "{middle:?}");
        assert_eq!(middle.green, 0.0);
    }

    #[test]
    fn a_bouncy_fade_in_keeps_its_hue_all_the_way() {
        let ms = Duration::from_millis;
        let blue = Color::hex(0x3b_6ef5);
        let mut motion = Motion::at(blue.with_alpha(0.0));
        motion.set_target(blue, crate::Spring::BOUNCY, ms(0));
        let mut overshot = false;
        for t in (0..2_000).step_by(8) {
            let color = motion.sample(ms(t)).value;
            overshot |= color.alpha == 1.0 && !motion.sample(ms(t)).settled;
            if color.alpha > 0.0 {
                assert!((color.red - blue.red).abs() < 1e-4, "{t} ms: {color:?}");
                assert!((color.green - blue.green).abs() < 1e-4, "{t} ms: {color:?}");
                assert!((color.blue - blue.blue).abs() < 1e-4, "{t} ms: {color:?}");
            }
        }
        assert!(overshot, "the spring passes full alpha and is held there");
    }

    #[test]
    fn a_color_at_rest_or_settled_is_exactly_the_color_given() {
        let ms = Duration::from_millis;
        let blue = Color::hex(0x3b_6ef5);
        let clear_red = Color::hex(0xe5_484d).with_alpha(0.0);
        assert_eq!(Motion::at(blue).sample(ms(0)).value, blue);
        assert_eq!(
            Motion::at(clear_red).sample(ms(0)).value,
            clear_red,
            "its hue kept"
        );
        let mut motion = Motion::at(clear_red);
        motion.set_target(blue, Tween::new(ms(100), Easing::Linear), ms(0));
        assert_eq!(motion.target(), blue);
        let end = motion.sample(ms(100));
        assert!(end.settled);
        assert_eq!(end.value, blue);
    }

    #[test]
    fn a_channel_past_its_end_is_drawn_at_the_end() {
        let color = Color::from_channels([1.5, -0.2, 0.5, 1.2]);
        assert_eq!(color.red, 1.0);
        assert_eq!(color.green, 0.0);
        assert_eq!(color.alpha, 1.0);
        assert_eq!(
            Color::from_channels([0.3, 0.3, 0.3, -0.1]),
            Color::TRANSPARENT
        );
    }
}
