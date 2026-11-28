//! The gallery's window and its recording, on Linux.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    AnyView, App, AppContext, Bounds, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window, WindowBounds,
    WindowOptions, div, point, prelude::FluentBuilder as _, px, rgb, size,
};

/// The window's size, which is also the size of a recorded frame. The
/// `record` script's screen must be at least as large.
const WIDTH: f32 = 960.;
const HEIGHT: f32 = 600.;
/// How long a recording waits for the window's first frame, and then for
/// ffmpeg's first frame, before it gives up.
const FIRST_FRAME: Duration = Duration::from_secs(30);
/// How long ffmpeg grabs frames. Play is pressed once its first frame is
/// written, so the page at rest is the first frame.
const RECORDED: Duration = Duration::from_secs(2);
/// When a recording presses Play again, so the way back is recorded too.
const AGAIN: Duration = Duration::from_secs(1);
/// How often a recording checks what it waits for.
const POLL: Duration = Duration::from_millis(10);
/// Frames a second in a recording.
const FPS: u32 = 30;

/// One page of the gallery: a view, and what its Play button does.
#[derive(Clone)]
pub struct Page {
    /// The page's name, as `--record` takes it.
    pub name: &'static str,
    /// What the page shows, in a line.
    about: &'static str,
    view: AnyView,
    /// Starts the page's motion.
    pub play: Rc<dyn Fn(&mut App)>,
}

impl Page {
    fn new<V: Render>(
        name: &'static str,
        about: &'static str,
        view: Entity<V>,
        play: fn(&mut V, &mut Context<V>),
    ) -> Self {
        Self {
            name,
            about,
            view: view.clone().into(),
            play: Rc::new(move |cx| view.update(cx, play)),
        }
    }

    /// Every page, in the order of the list on the left. A new page is one
    /// more line here, and a module in `main.rs`.
    pub fn all(cx: &mut App) -> Vec<Page> {
        use crate::{
            accordion::Accordion, color::Button, compose::Compose, drag::Throw, gestures::Buttons,
            island::IslandPage, layout::Layout, list::Rows, look::Card, presence::Toast,
            pulse::Pulse, shared::Shared, toward::Toward, transitions::Arrive, variants::Menus,
            ways::Ways,
        };
        vec![
            Page::new(
                "toward",
                "A value heads for a target, with a spring and with a tween.",
                cx.new(|_| Toward::new()),
                Toward::play,
            ),
            Page::new(
                "presence",
                "A toast is shown and hidden, and drawn while it leaves.",
                cx.new(|_| Toast::new()),
                Toast::play,
            ),
            Page::new(
                "ways",
                "A toast rises in and fades out; titles swap, crossing or one after the other.",
                cx.new(|_| Ways::new()),
                Ways::play,
            ),
            Page::new(
                "shared",
                "One thing in two places: a tab's mark slides over, a row opens into a card.",
                cx.new(|_| Shared::new()),
                Shared::play,
            ),
            Page::new(
                "list",
                "Rows arrive and leave a list.",
                cx.new(|cx| Rows::new(cx)),
                Rows::play,
            ),
            Page::new(
                "layout",
                "Cards in a wrapping row glide to their new places when the first one goes or comes back.",
                cx.new(|_| Layout::new()),
                Layout::play,
            ),
            Page::new(
                "look",
                "A card declares its look; clicked, it changes size and fades.",
                cx.new(|_| Card::new()),
                Card::play,
            ),
            Page::new(
                "accordion",
                "Questions open and close: each answer's height goes from nothing to its own.",
                cx.new(|_| Accordion::new()),
                Accordion::play,
            ),
            Page::new(
                "variants",
                "A menu opens and its rows follow one after another, after it or before it.",
                cx.new(|_| Menus::new()),
                Menus::play,
            ),
            Page::new(
                "transitions",
                "A card springs to its place while it fades with a tween.",
                cx.new(|_| Arrive::new()),
                Arrive::play,
            ),
            Page::new(
                "color",
                "A button changes color; red to blue passes through purple, never green.",
                cx.new(|_| Button::new()),
                Button::play,
            ),
            Page::new(
                "gestures",
                "Buttons answer to the pointer over them, a press and the keyboard focus.",
                cx.new(Buttons::new),
                Buttons::play,
            ),
            Page::new(
                "drag",
                "A card is thrown and springs back inside its tray; a knob rests on a stop.",
                cx.new(|_| Throw::new()),
                Throw::play,
            ),
            Page::new(
                "pulse",
                "A mark breathes and three dots pulse, through keyframes that repeat.",
                cx.new(Pulse::new),
                Pulse::play,
            ),
            Page::new(
                "compose",
                "A bar's color follows its width; an entrance of three steps.",
                cx.new(|_| Compose::new()),
                Compose::play,
            ),
            Page::new(
                "island",
                "A dot moves in an island; the grid beside it is not drawn again.",
                cx.new(IslandPage::new),
                IslandPage::play,
            ),
        ]
    }
}

/// The window: the list of pages on the left, the chosen page on the right.
pub struct Gallery {
    pages: Vec<Page>,
    chosen: usize,
    /// Frames drawn, so a recording knows when the window is on screen.
    drawn: u32,
}

impl Gallery {
    /// The gallery, on page `chosen`.
    pub fn new(cx: &mut App, chosen: usize) -> Self {
        Self {
            pages: Page::all(cx),
            chosen,
            drawn: 0,
        }
    }

    /// The page on screen.
    pub fn chosen(&self) -> Page {
        self.pages[self.chosen].clone()
    }
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.drawn += 1;
        let chosen = self.chosen();
        let list = self.pages.iter().enumerate().map(|(index, page)| {
            let here = index == self.chosen;
            div()
                .id(index)
                .px(px(12.))
                .py(px(6.))
                .rounded(px(4.))
                .cursor_pointer()
                .when(here, |item| item.bg(rgb(0x0030_3540)))
                .text_color(rgb(if here { 0x00ff_ffff } else { 0x00a0_a8b0 }))
                .child(page.name)
                .on_click(cx.listener(move |gallery, _, _, cx| {
                    gallery.chosen = index;
                    cx.notify();
                }))
        });
        let play = chosen.clone();
        div()
            .size_full()
            .flex()
            .bg(rgb(0x0016_181c))
            .text_color(rgb(0x00e8_eaed))
            .text_size(px(14.))
            .child(
                div()
                    .w(px(160.))
                    .h_full()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .bg(rgb(0x001e_2126))
                    .children(list),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .p(px(20.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(12.))
                            .child(
                                div()
                                    .id("play")
                                    .px(px(12.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .bg(rgb(0x003b_6ef5))
                                    .cursor_pointer()
                                    .child("Play")
                                    .on_click(move |_, _, cx| (play.play)(cx)),
                            )
                            .child(SharedString::from(chosen.about)),
                    )
                    .child(div().flex_1().relative().child(chosen.view.clone())),
            )
    }
}

/// Opens the gallery on page `chosen`.
fn open(cx: &mut App, chosen: Option<&str>) -> Entity<Gallery> {
    let mut gallery = Gallery::new(cx, 0);
    if let Some(name) = chosen {
        let pages = &gallery.pages;
        gallery.chosen = pages
            .iter()
            .position(|page| page.name == name)
            .unwrap_or_else(|| {
                let names: Vec<_> = pages.iter().map(|page| page.name).collect();
                panic!("no page {name:?}: the pages are {}", names.join(", "))
            });
    }
    let bounds = Bounds::new(point(px(0.), px(0.)), size(px(WIDTH), px(HEIGHT)));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..WindowOptions::default()
    };
    let window = cx
        .open_window(options, |_, cx| cx.new(|_| gallery))
        .expect("a window opens; to record, run under a display such as xvfb-run");
    window
        .update(cx, |_, _, cx| cx.entity())
        .expect("the window was just opened")
}

/// Records page `name` into `folder`: frames, then a contact sheet.
fn record(name: String, folder: PathBuf) {
    gpui_platform::application().run(move |cx: &mut App| {
        // A recording shows the motion whatever the desktop's setting, which
        // xvfb-run passes on from the caller's session.
        cx.set_reduce_motion(false);
        let gallery = open(cx, Some(&name));
        let timer = cx.background_executor().clone();
        cx.spawn(async move |cx| {
            let waited = Instant::now();
            while cx.update(|cx| gallery.read(cx).drawn) == 0 {
                assert!(waited.elapsed() < FIRST_FRAME, "the window drew no frame");
                timer.timer(POLL).await;
            }
            fs::create_dir_all(&folder).expect("the record folder can be made");
            for old in fs::read_dir(&folder).into_iter().flatten().flatten() {
                // An old contact sheet goes too, so a failed run leaves none behind.
                let name = old.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("frame-") || name == "contact.png" {
                    let _ = fs::remove_file(old.path());
                }
            }
            let display = env::var("DISPLAY").expect("a display: run under xvfb-run");
            let mut grab = Command::new("ffmpeg")
                .args([
                    "-loglevel",
                    "error",
                    "-f",
                    "x11grab",
                    "-draw_mouse",
                    "0",
                    "-framerate",
                ])
                .arg(FPS.to_string())
                .args(["-video_size", &format!("{WIDTH}x{HEIGHT}")])
                .args(["-i", &display, "-t"])
                .arg(format!("{:.3}", RECORDED.as_secs_f64()))
                .arg(folder.join("frame-%03d.png"))
                .spawn()
                .expect("ffmpeg grabs the screen; install it");
            let first = folder.join("frame-001.png");
            let waited = Instant::now();
            while !first.exists() {
                assert!(waited.elapsed() < FIRST_FRAME, "ffmpeg grabbed no frame");
                timer.timer(POLL).await;
            }
            // ffmpeg is waited for off the main thread, which keeps drawing.
            let grabbed = timer.spawn(async move { grab.wait() });
            for pause in [Duration::ZERO, AGAIN] {
                timer.timer(pause).await;
                cx.update(|cx| {
                    let page = gallery.read(cx).chosen();
                    (page.play)(cx);
                });
            }
            let grabbed = grabbed.await.expect("ffmpeg runs");
            assert!(grabbed.success(), "ffmpeg failed to grab the screen");
            let sheet = folder.join("contact.png");
            let mut frames: Vec<_> = fs::read_dir(&folder)
                .expect("the record folder exists")
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .is_some_and(|f| f.to_string_lossy().starts_with("frame-"))
                })
                .collect();
            frames.sort();
            // A recording is evidence only if something moved in it.
            let first = fs::read(&frames[0]).expect("a frame can be read");
            assert!(
                frames[1..]
                    .iter()
                    .any(|frame| fs::read(frame).is_ok_and(|bytes| bytes != first)),
                "no frame changed: is the page's motion running, and is nothing blocking the main thread?"
            );
            let made = Command::new("montage")
                .args(frames.iter().step_by(3))
                .args([
                    "-tile",
                    "5x",
                    "-geometry",
                    "320x200+2+2",
                    "-background",
                    "#000",
                ])
                .arg(&sheet)
                .status();
            match made {
                Ok(status) if status.success() => println!("{}", sheet.display()),
                _ => println!(
                    "{} frames in {} (no contact sheet: install ImageMagick)",
                    frames.len(),
                    folder.display()
                ),
            }
            cx.update(|cx| cx.quit());
        })
        .detach();
    });
}

pub fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--record") => {
            let name = args.next().expect("--record takes a page's name");
            let folder = args.next().map_or_else(
                // The workspace's target/, wherever the command is run from.
                || {
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../target/gallery")
                        .join(&name)
                },
                PathBuf::from,
            );
            record(name, folder);
        }
        Some(other) => {
            panic!("unknown argument {other:?}: use --record <page> [folder], or no argument")
        }
        None => gpui_platform::application().run(|cx: &mut App| {
            open(cx, None);
        }),
    }
}
