# Colors mix in linear light, premultiplied

**Chosen.** `ginga::Color` is written as sRGB with a straight alpha, the way colors are written everywhere, and moves as four channels: red, green and blue in linear light, each multiplied by the alpha, and the alpha. Issue #26.

**Options.**

- sRGB channels as written, straight alpha.
- HSL or HSV, with the hue as a channel.
- Oklab or Oklch, a space made for even steps in lightness.
- Linear light with premultiplied alpha.

**Why.**

- The issue sets two tests. Red on its way to blue must not turn green: a hue channel fails it, since the short way from 0° to 240° passes 120°, green. Any mix of red, green and blue passes it, because the green channel stays at zero.
- A color fading in from transparent must not flash dark. With a straight alpha, transparent is black with no alpha, so halfway is a half-opaque dark grey. Premultiplied, transparent is all zeros, and halfway to a color is that color at half alpha. This is the test that rules out plain sRGB and plain linear light.
- Linear light over sRGB: the middle of two colors is as bright as the light they give, not darker. Halfway from red to blue is `(0.735, 0, 0.735)` as sRGB, not the duller `(0.5, 0, 0.5)`.
- Oklab gives even steps in lightness, which matters for long gradients and little for a button that changes color in 200 ms. It needs a cube root and a matrix both ways each frame, and a spring that overshoots leaves its gamut. Linear light needs one `powf` per channel and clamps simply.

**What follows from it.**

- The precision is a ten-thousandth: near black, a thousandth in linear light is several sRGB levels, a visible snap when the motion settles.
- A settled motion returns the color it was given, not one rebuilt from channels, since the round trip through `powf` is off in the last bit for 70 of the 256 sRGB levels: `Motion` keeps the target as given and hands it back once settled and from `target()`. Any `Animatable` whose round trip is not exact gains from it.
- `Color::new` holds each value to `0.0..=1.0`, so a color at rest is drawn as it is while it moves. A spring can carry a channel past either end; `from_channels` divides by the alpha as it is, then clamps, so the hue holds when the alpha passes 1.
