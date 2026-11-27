# The GPUI pin and the orphan rule

**One GPUI.** The workspace pins `gpui = { package = "gpui-pre", version = "=0.3.12", default-features = false }` in `[workspace.dependencies]`, and the adapter uses it for its code and, with `test-support`, for its tests: the GPUI snapshot that gpui-kit 0.9.1 builds on. An app must use the same one. Two versions of GPUI in one build give two `Pixels` types, two `Window` types and so on, and the compiler reports that a type does not match itself. To move the pin, move it in the app and here together, and try the minimum Rust version again ([[decisions/2026-11-27-minimum-rust-versions]]).

`default-features = false` keeps the adapter from linking a platform. Tests use GPUI's test platform, which needs nothing from the system ([[codebase/testing-the-adapter]]). An example that opens a real window adds GPUI's platform as a dev-dependency, which needs system packages on Linux (`libxkbcommon-x11-dev libwayland-dev libfontconfig-dev libfreetype-dev`).

**Why `Animate` and stand-ins.** The engine moves anything that implements `ginga::Animatable`. The adapter cannot implement `Animatable` for GPUI's `Pixels`, `Point<Pixels>` or `Size<Pixels>`: the orphan rule forbids implementing a foreign trait for a foreign type, and both are foreign to `ginga-gpui`. So the adapter has its own trait, `Animate`, implemented for GPUI's types, and each one names a stand-in that the engine moves instead. `InPixels` is the stand-in for pixels, with a precision of a tenth of a pixel.

**For a new GPUI type that moves** (a color, an edge set), add an `Animate` impl and a stand-in in the adapter. The motion logic stays in the engine ([[principles/the-engine-knows-no-framework]]).
