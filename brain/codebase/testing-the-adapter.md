# Testing the adapter on GPUI's test platform

`crates/ginga-gpui/tests/adapter.rs` checks the adapter with no display. It uses GPUI's **test platform**: `gpui` with the `test-support` feature, as a dev-dependency only.

**What it needs.** Nothing from the system: no platform crate, no X11 or Wayland packages, no GPU. `test-support` does not reach a normal build of the adapter or of an app that depends on it (`cargo tree -p ginga-gpui -e normal,features` shows none of it). A test is `#[gpui::test] fn name(cx: &mut TestAppContext)`.

**The clock.** The test moves it: `cx.executor().advance_clock(duration)`. It does not move on its own, not even while the executor parks, so a `Tween` with `Easing::Linear` gives exact values and the tests assert with `==`. `ITERATIONS=50 cargo test -p ginga-gpui` runs each test over 50 seeds.

**Frames.** `window.simulate_next_frame(cx)` delivers the next frame, as the platform would, and returns how many frame requests it answered; `cx.run_until_parked()` after it lets the views render. A settled motion makes no request, so the next frame answers `0`.

**What a frame request renders.** `window.request_animation_frame()` notifies the view that asked, so that view and every view above it render again on the next frame; a view beside them renders again only if it isn't cached. The island test counts all three.

**Reduced motion.** `cx.update(|cx| cx.set_reduce_motion(true))` before opening the window.

**Reading a declared look.** A look kept in a `LookState` is read with `state.inspect(cx)`: its engine motion and the frame's time, for `Trace::of` or a sample, with no `canvas` probe of the laid-out bounds. A look kept with an id cannot be read; a test that needs its values keeps it in a `LookState`.
