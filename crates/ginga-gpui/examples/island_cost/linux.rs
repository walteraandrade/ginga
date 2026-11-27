//! The measurement itself: it reads Linux's `/proc` and opens an X11 window.

use std::{
    cell::Cell,
    env, fs,
    process::Command,
    rc::Rc,
    time::{Duration, Instant},
};

use ginga_gpui::{Island, Motion, Spring};
use gpui::{
    AnyElement, App, AppContext, Bounds, Context, Entity, IntoElement, ParentElement, Pixels,
    Render, StyleRefinement, Styled, Window, WindowBounds, WindowOptions, div, point, px, rgb,
    size,
};

/// How long a run waits for its first frame.
const FIRST_FRAME: Duration = Duration::from_secs(30);
/// How often it looks for that frame.
const POLL: Duration = Duration::from_millis(10);
/// How long a run waits after its first frame before it measures.
const WARM_UP: Duration = Duration::from_secs(1);
/// How long a run measures.
const MEASURED: Duration = Duration::from_secs(5);
/// Runs of each mode.
const RUNS: usize = 5;
/// What starts the line that carries a run's result.
const RESULT: &str = "island_cost result:";
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

/// All the process's threads' CPU time so far: the main thread's, and the
/// threads of the GPU driver, which in software draws the pixels.
fn process_cpu() -> Duration {
    fs::read_dir("/proc/self/task")
        .expect("this example reads its threads from Linux's /proc")
        .flatten()
        .filter_map(|task| fs::read_to_string(task.path().join("schedstat")).ok())
        .filter_map(|stat| stat.split_whitespace().next()?.parse().ok())
        .map(Duration::from_nanos)
        .sum()
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

/// Where the dot is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// By the heavy view itself.
    Inside,
    /// By an island beside the heavy view, which is cached.
    Island,
}

impl Mode {
    const ALL: [Mode; 2] = [Mode::Inside, Mode::Island];

    /// The mode's name, as the command line takes it.
    fn name(self) -> &'static str {
        match self {
            Mode::Inside => "inside",
            Mode::Island => "island",
        }
    }
}

/// Runs one mode and prints its [`RESULT`] line:
/// `frames heavy_renders cpu_ms process_cpu_ms wall_ms`.
fn run(mode: Mode) {
    let island = mode == Mode::Island;
    gpui_platform::application().run(move |cx: &mut App| {
        // A desktop that asks for reduced motion would stop the dot: this
        // measures motion, so it turns that off for its own process.
        cx.set_reduce_motion(false);
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
            // Measure from the first frame on, not from when the window opened.
            let mut waited = Duration::ZERO;
            while counts.dot.get() == 0 || counts.heavy.get() == 0 {
                assert!(
                    waited < FIRST_FRAME,
                    "no frame in {FIRST_FRAME:?}: is a Vulkan driver installed (mesa-vulkan-drivers)?"
                );
                timer.timer(POLL).await;
                waited += POLL;
            }
            timer.timer(WARM_UP).await;
            let (dot, heavy) = (counts.dot.get(), counts.heavy.get());
            let (cpu, process, started) = (thread_cpu(), process_cpu(), Instant::now());
            timer.timer(MEASURED).await;
            let cpu = thread_cpu().saturating_sub(cpu);
            let process = process_cpu().saturating_sub(process);
            let wall = started.elapsed();
            let frames = counts.dot.get() - dot;
            let heavy = counts.heavy.get() - heavy;
            println!(
                "{RESULT} {frames} {heavy} {:.3} {:.3} {:.3}",
                cpu.as_secs_f64() * 1e3,
                process.as_secs_f64() * 1e3,
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
    process_ms: f64,
    wall_ms: f64,
}

impl Run {
    /// A run's [`RESULT`] line. Anything else, or a count or time that cannot
    /// be measured from, is `None`.
    fn parse(line: &str) -> Option<Run> {
        let mut fields = line.strip_prefix(RESULT)?.split_whitespace();
        let run = Run {
            frames: fields.next()?.parse().ok()?,
            heavy: fields.next()?.parse().ok()?,
            cpu_ms: fields.next()?.parse().ok()?,
            process_ms: fields.next()?.parse().ok()?,
            wall_ms: fields.next()?.parse().ok()?,
        };
        let times = [run.cpu_ms, run.process_ms, run.wall_ms];
        let measurable = run.frames > 0 && times.iter().all(|ms| ms.is_finite() && *ms > 0.0);
        (fields.next().is_none() && measurable).then_some(run)
    }

    /// The main thread's time per frame.
    fn per_frame(&self) -> f64 {
        self.cpu_ms / f64::from(self.frames)
    }

    /// All threads' time per frame.
    fn process_per_frame(&self) -> f64 {
        self.process_ms / f64::from(self.frames)
    }

    fn fps(&self) -> f64 {
        f64::from(self.frames) / self.wall_ms * 1e3
    }
}

/// Runs `mode` in a process of its own.
fn measure(mode: Mode) -> Run {
    let output = Command::new(env::current_exe().expect("the example knows where it is"))
        .arg(mode.name())
        .output()
        .expect("the example runs itself");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let failed = |why: &str| -> ! {
        panic!(
            "the {} run {why}:\n{stdout}\n{}",
            mode.name(),
            String::from_utf8_lossy(&output.stderr)
        )
    };
    if !output.status.success() {
        failed(&format!("failed ({})", output.status));
    }
    let run = stdout
        .lines()
        .find(|line| line.starts_with(RESULT))
        .unwrap_or_else(|| failed("printed no result"));
    let run =
        Run::parse(run).unwrap_or_else(|| failed("printed a result that cannot be measured from"));
    assert!(
        mode != Mode::Island || run.heavy == 0,
        "the heavy view rendered {} times beside the island: the island no longer saves it",
        run.heavy
    );
    run
}

pub fn main() {
    match env::args().nth(1).as_deref() {
        None => {}
        Some(name) => {
            let mode = Mode::ALL
                .into_iter()
                .find(|mode| mode.name() == name)
                .unwrap_or_else(|| {
                    panic!("unknown mode {name:?}: use inside, island, or nothing for both")
                });
            return run(mode);
        }
    }
    println!(
        "{:<4} {:<7} {:>7} {:>6} {:>13} {:>9} {:>11}",
        "run", "mode", "frames", "fps", "heavy renders", "ms/frame", "all threads"
    );
    let mut per_frame = Mode::ALL.map(|_| Vec::new());
    let mut process_per_frame = Mode::ALL.map(|_| Vec::new());
    for run in 1..=RUNS {
        for (index, mode) in Mode::ALL.into_iter().enumerate() {
            let result = measure(mode);
            println!(
                "{run:<4} {:<7} {:>7} {:>6.1} {:>13} {:>9.3} {:>11.3}",
                mode.name(),
                result.frames,
                result.fps(),
                result.heavy,
                result.per_frame(),
                result.process_per_frame()
            );
            per_frame[index].push(result.per_frame());
            process_per_frame[index].push(result.process_per_frame());
        }
    }
    println!();
    for ((mode, mut per_frame), mut process) in
        Mode::ALL.into_iter().zip(per_frame).zip(process_per_frame)
    {
        per_frame.sort_by(f64::total_cmp);
        process.sort_by(f64::total_cmp);
        println!(
            "{:<7} median {:.3} ms/frame on the main thread, runs from {:.3} to {:.3}; {:.3} ms/frame on all threads",
            mode.name(),
            per_frame[RUNS / 2],
            per_frame[0],
            per_frame[RUNS - 1],
            process[RUNS / 2]
        );
    }
}
