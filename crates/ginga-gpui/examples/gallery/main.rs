//! The gallery: a page for each thing ginga does, to open and watch.
//!
//! ```sh
//! cargo run -p ginga-gpui --example gallery
//! ```
//!
//! It runs on Linux, where the adapter has GPUI's platform as a dev-dependency.
//!
//! Pick a page on the left and press Play.
//!
//! To record a page in a virtual display, so a pull request can show a motion
//! as frames beside its trace:
//!
//! ```sh
//! crates/ginga-gpui/examples/gallery/record presence
//! ```
//!
//! That leaves the frames and a contact sheet of them in
//! `target/gallery/presence/`. It runs the gallery as
//! `gallery --record <page> <folder>` under `xvfb-run`, which opens the page,
//! presses Play, presses it again a second later, and grabs the screen with
//! `ffmpeg`. It needs `xvfb-run`, `ffmpeg`, a Vulkan driver and `montage`.

#[cfg(target_os = "linux")]
mod app;
#[cfg(target_os = "linux")]
mod island;
#[cfg(target_os = "linux")]
mod list;
#[cfg(target_os = "linux")]
mod look;
#[cfg(target_os = "linux")]
mod presence;
#[cfg(target_os = "linux")]
mod pulse;
#[cfg(all(test, target_os = "linux"))]
mod tests;
#[cfg(target_os = "linux")]
mod toward;

fn main() {
    #[cfg(target_os = "linux")]
    app::main();
    #[cfg(not(target_os = "linux"))]
    eprintln!(
        "the gallery runs on Linux only for now: GPUI's platform is a Linux-only dev-dependency here"
    );
}
