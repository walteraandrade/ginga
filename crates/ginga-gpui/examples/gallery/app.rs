//! The gallery's window and its recording, on Linux.

use std::{env, fs, path::PathBuf, process::Command, rc::Rc, time::Duration};

use gpui::{
    AnyView, App, AppContext, Bounds, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window, WindowBounds,
    WindowOptions, div, point, prelude::FluentBuilder as _, px, rgb, size,
};

/// The window's size, which is also the size of a recorded frame.
const WIDTH: f32 = 960.;
const HEIGHT: f32 = 600.;
/// How long a recording waits for the window before it presses Play.
const SETTLE: Duration = Duration::from_millis(1_500);
/// How long a recording grabs frames after it first presses Play.
const RECORDED: Duration = Duration::from_secs(2);
/// When a recording presses Play again, so the way back is recorded too.
const AGAIN: Duration = Duration::from_secs(1);
/// Frames a second in a recording.
const FPS: u32 = 30;

/// One page of the gallery: a view, and what its Play button does.
#[derive(Clone)]
struct Page {
    /// The page's name, as `--record` takes it.
    name: &'static str,
    /// What the page shows, in a line.
    about: &'static str,
    view: AnyView,
    /// Starts the page's motion.
    play: Rc<dyn Fn(&mut App)>,
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
    fn all(cx: &mut App) -> Vec<Page> {
        use crate::{island::IslandPage, list::Rows, presence::Toast, toward::Toward};
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
                "list",
                "Rows arrive and leave a list.",
                cx.new(|_| Rows::new()),
                Rows::play,
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
struct Gallery {
    pages: Vec<Page>,
    chosen: usize,
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let chosen = self.pages[self.chosen].clone();
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
    let pages = Page::all(cx);
    let chosen = match chosen {
        None => 0,
        Some(name) => pages
            .iter()
            .position(|page| page.name == name)
            .unwrap_or_else(|| {
                let names: Vec<_> = pages.iter().map(|page| page.name).collect();
                panic!("no page {name:?}: the pages are {}", names.join(", "))
            }),
    };
    let bounds = Bounds::new(point(px(0.), px(0.)), size(px(WIDTH), px(HEIGHT)));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..WindowOptions::default()
    };
    let window = cx
        .open_window(options, |_, cx| cx.new(|_| Gallery { pages, chosen }))
        .expect("a window opens; to record, run under a display such as xvfb-run");
    window
        .update(cx, |_, _, cx| cx.entity())
        .expect("the window was just opened")
}

/// Records page `name` into `folder`: frames, then a contact sheet.
fn record(name: String, folder: PathBuf) {
    gpui_platform::application().run(move |cx: &mut App| {
        let gallery = open(cx, Some(&name));
        let timer = cx.background_executor().clone();
        cx.spawn(async move |cx| {
            timer.timer(SETTLE).await;
            fs::create_dir_all(&folder).expect("the record folder can be made");
            for old in fs::read_dir(&folder).into_iter().flatten().flatten() {
                if old.file_name().to_string_lossy().starts_with("frame-") {
                    let _ = fs::remove_file(old.path());
                }
            }
            let display = env::var("DISPLAY").expect("a display: run under xvfb-run");
            let mut grab = Command::new("ffmpeg")
                .args(["-loglevel", "error", "-f", "x11grab", "-framerate"])
                .arg(FPS.to_string())
                .args(["-video_size", &format!("{WIDTH}x{HEIGHT}")])
                .args(["-i", &display, "-t"])
                .arg(format!("{:.3}", RECORDED.as_secs_f64()))
                .arg(folder.join("frame-%03d.png"))
                .spawn()
                .expect("ffmpeg grabs the screen; install it");
            // ffmpeg takes a moment to start grabbing.
            timer.timer(Duration::from_millis(200)).await;
            // ffmpeg is waited for off the main thread, which keeps drawing.
            let grabbed = timer.spawn(async move { grab.wait() });
            for pause in [Duration::ZERO, AGAIN] {
                timer.timer(pause).await;
                cx.update(|cx| {
                    let page = gallery.read(cx).pages[gallery.read(cx).chosen].clone();
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
                || PathBuf::from("target/gallery").join(&name),
                PathBuf::from,
            );
            record(name, folder);
        }
        Some("--page") => {
            let name = args.next().expect("--page takes a page's name");
            gpui_platform::application().run(move |cx: &mut App| {
                open(cx, Some(&name));
            });
        }
        Some(other) => {
            panic!("unknown argument {other:?}: use --page <name> or --record <name> [folder]")
        }
        None => gpui_platform::application().run(|cx: &mut App| {
            open(cx, None);
        }),
    }
}
