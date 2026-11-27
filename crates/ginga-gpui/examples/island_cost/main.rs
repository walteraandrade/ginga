//! What an island saves: the main thread's time per frame of a moving dot,
//! drawn inside a heavy view or in an island beside it.
//!
//! ```sh
//! xvfb-run -a -s "-screen 0 1280x1024x24" cargo run --release -p ginga-gpui --example island_cost
//! ```
//!
//! With no argument it runs each mode five times, alternating, each in a
//! process of its own, and prints a table. With `inside` or `island` it runs
//! that mode once and prints one line.
//!
//! - `inside`: the dot is drawn by the heavy view, so every frame of the dot's
//!   motion renders the heavy view again.
//! - `island`: the dot is a [`ginga_gpui::Island`] beside the heavy view, which
//!   is embedded with `.cached(..)`, so a frame renders the island and the
//!   small view above it.
//!
//! The main thread's time comes from `/proc/thread-self/schedstat`: the CPU
//! time the thread ran, whatever it did with it (render, layout, paint, and
//! handing the frame to the GPU driver). The all-threads column adds the
//! process's other threads, where a software GPU driver draws the pixels.
//!
//! It runs on Linux only.

#[cfg(target_os = "linux")]
mod linux;

fn main() {
    #[cfg(target_os = "linux")]
    linux::main();
    #[cfg(not(target_os = "linux"))]
    eprintln!("island_cost measures on Linux only: it reads /proc and opens an X11 window");
}
