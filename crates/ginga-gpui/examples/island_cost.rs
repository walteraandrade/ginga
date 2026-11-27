//! What an island saves: the main thread's time per frame of a moving dot,
//! drawn inside a heavy view or in an island beside it.
//!
//! ```sh
//! xvfb-run -a cargo run --release -p ginga-gpui --example island_cost
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
//! handing the frame to the GPU driver).

use std::{
    cell::Cell,
    env, fs,
    process::Command,
    rc::Rc,
    time::{Duration, Instant},
};

use ginga_gpui::{Island, Motion, Spring};
use gpui::{
    AnyElement, App, AppContext, Bounds, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, StyleRefinement, Styled, Window, WindowBounds, WindowOptions,
    div, point, px, rgb, size,
};

/// How long a run waits before it measures, for the window to settle.
const WARM_UP: Duration = Duration::from_secs(1);
/// How long a run measures.
const MEASURED: Duration = Duration::from_secs(5);
/// Runs of each mode.
const RUNS: usize = 5;
/// Cells in the heavy view: each is a box with a number in it.
const CELLS: usize = 2_400;

/// The main thread's CPU time so far.
fn thread_cpu() -> Duration {
    let stat = fs::read_to_string("/proc/thread-self/schedstat")
        .expect("this example reads the thread's CPU time from Linux's /proc");
    let nanos = stat
        .split_whitespace()
        .next()
        .and_then(|field| field.parse().ok())
        .expect("schedstat starts with the time the thread ran, in nanoseconds");
    Duration::from_nanos(nanos)
}

/// The dot's state: a place that goes back and forth across the window.
struct Dot {
    x: Motion<Pixels>,
    target: Pixels,
}

impl Dot {
    fn new() -> Self {
        Self {
            x: Motion::at(px(0.)),
            target: px(600.),
        }
    }

    /// The dot for this frame. Once it arrives it turns around, so it never stops.
    fn draw(&mut self, window: &mut Window, cx: &mut App) -> AnyElement {
        let mut x = self.x.toward(self.target, Spring::SMOOTH, window, cx);
        if x == self.target {
            self.target = if self.target == px(0.) {
                px(600.)
            } else {
                px(0.)
            };
            x = self.x.toward(self.target, Spring::SMOOTH, window, cx);
        }
        div()
            .absolute()
            .top(px(8.))
            .left(x)
            .size(px(12.))
            .rounded_full()
            .bg(rgb(0x00d0_6040))
            .into_any_element()
    }
}

/// Counts of what a run drew.
#[derive(Default)]
struct Counts {
    dot: Cell<u32>,
    heavy: Cell<u32>,
}

/// A view that is heavy to render on purpose: many boxes, each with text.
struct Heavy {
    counts: Rc<Counts>,
    /// `inside` mode: the dot is drawn here.
    dot: Option<Dot>,
}

impl Render for Heavy {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.counts.heavy.set(self.counts.heavy.get() + 1);
        let cells = (0..CELLS).map(|n| {
            div()
                .id(n)
                .size(px(15.))
                .bg(rgb(0x0020_2020 + (n as u32 % 7) * 0x0010_1010))
                .text_size(px(6.))
                .child(format!("{n}"))
        });
        let mut page = div()
            .relative()
            .size_full()
            .flex()
            .flex_wrap()
            .children(cells);
        if let Some(dot) = &mut self.dot {
            self.counts.dot.set(self.counts.dot.get() + 1);
            page = page.child(dot.draw(window, cx));
        }
        page
    }
}

/// The window's root: the heavy view and, in `island` mode, the island.
struct Root {
    heavy: Entity<Heavy>,
    island: Option<Entity<Island<Dot>>>,
}

impl Render for Root {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let root = div().relative().size_full().bg(rgb(0x0010_1010));
        match &self.island {
            Some(island) => root
                .child(
                    self.heavy
                        .clone()
                        .cached(StyleRefinement::default().size_full()),
                )
                .child(island.clone()),
            None => root.child(self.heavy.clone()),
        }
    }
}

/// Runs one mode and prints `frames heavy_renders cpu_ms wall_ms`.
fn run(island: bool) {
    gpui_platform::application().run(move |cx: &mut App| {
        let counts = Rc::new(Counts::default());
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(800.), px(600.)));
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..WindowOptions::default()
        };
        let root_counts = counts.clone();
        cx.open_window(options, move |_, cx| {
            cx.new(|cx| {
                let heavy = cx.new(|_| Heavy {
                    counts: root_counts.clone(),
                    dot: (!island).then(Dot::new),
                });
                let island = island.then(|| {
                    let counts = root_counts.clone();
                    cx.new(|_| {
                        Island::new(Dot::new(), move |dot, window, cx| {
                            counts.dot.set(counts.dot.get() + 1);
                            dot.draw(window, cx)
                        })
                    })
                });
                Root { heavy, island }
            })
        })
        .expect("a window opens; run this under a display, such as xvfb-run");
        let timer = cx.background_executor().clone();
        cx.spawn(async move |cx| {
            timer.timer(WARM_UP).await;
            let (dot, heavy) = (counts.dot.get(), counts.heavy.get());
            let (cpu, started) = (thread_cpu(), Instant::now());
            timer.timer(MEASURED).await;
            let cpu = thread_cpu().saturating_sub(cpu);
            let wall = started.elapsed();
            let frames = counts.dot.get() - dot;
            let heavy = counts.heavy.get() - heavy;
            println!(
                "{frames} {heavy} {:.3} {:.3}",
                cpu.as_secs_f64() * 1e3,
                wall.as_secs_f64() * 1e3
            );
            cx.update(|cx| cx.quit());
        })
        .detach();
    });
}

/// One run's result.
struct Run {
    frames: u32,
    heavy: u32,
    cpu_ms: f64,
    wall_ms: f64,
}

impl Run {
    fn per_frame(&self) -> f64 {
        self.cpu_ms / f64::from(self.frames)
    }

    fn fps(&self) -> f64 {
        f64::from(self.frames) / self.wall_ms * 1e3
    }
}

/// Runs `mode` in a process of its own.
fn measure(mode: &str) -> Run {
    let output = Command::new(env::current_exe().expect("the example knows where it is"))
        .arg(mode)
        .output()
        .expect("the example runs itself");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().last().unwrap_or_default();
    let fields: Vec<&str> = line.split_whitespace().collect();
    let parsed = match fields[..] {
        [frames, heavy, cpu_ms, wall_ms] => (|| {
            Some(Run {
                frames: frames.parse().ok()?,
                heavy: heavy.parse().ok()?,
                cpu_ms: cpu_ms.parse().ok()?,
                wall_ms: wall_ms.parse().ok()?,
            })
        })(),
        _ => None,
    };
    parsed.unwrap_or_else(|| {
        panic!(
            "the {mode} run printed no result:\n{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn main() {
    match env::args().nth(1).as_deref() {
        Some("inside") => return run(false),
        Some("island") => return run(true),
        Some(other) => panic!("unknown mode {other:?}: use inside, island, or nothing for both"),
        None => {}
    }
    println!(
        "{:<4} {:<7} {:>7} {:>6} {:>13} {:>9}",
        "run", "mode", "frames", "fps", "heavy renders", "ms/frame"
    );
    let mut per_frame = [Vec::new(), Vec::new()];
    for run in 1..=RUNS {
        for (mode, per_frame) in ["inside", "island"].into_iter().zip(&mut per_frame) {
            let result = measure(mode);
            println!(
                "{run:<4} {mode:<7} {:>7} {:>6.1} {:>13} {:>9.3}",
                result.frames,
                result.fps(),
                result.heavy,
                result.per_frame()
            );
            per_frame.push(result.per_frame());
        }
    }
    println!();
    for (mode, per_frame) in ["inside", "island"].into_iter().zip(per_frame) {
        let low = per_frame.iter().copied().fold(f64::MAX, f64::min);
        let high = per_frame.iter().copied().fold(f64::MIN, f64::max);
        println!(
            "{mode:<7} median {:.3} ms/frame, runs from {low:.3} to {high:.3}",
            median(per_frame)
        );
    }
}
