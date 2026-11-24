//! The performance incubator: stress scenes for gpui-metro, each measured by
//! the process's own CPU load, so a change to a component can be judged by a
//! number rather than by eye.
//!
//! ```text
//! cargo run --release -p perf-incubator                 # the whole suite
//! cargo run --release -p perf-incubator -- live-tiles-60 # one scene
//! cargo run --release -p perf-incubator -- --show live-tiles-60 # just look
//! ```
//!
//! Options: `--secs N` (measure for N seconds, default 20), `--warmup N`
//! (default 3), `--out FILE` (also write the table there).
//!
//! Each scene is measured in the same window, one after another: CPU % is
//! user + system time of this process over the window (100 % = one core);
//! frames/s counts renders of the root view (every animation frame, every
//! notify). Live tiles keep their own phases, so measure long enough for the
//! slides to average out (20 s covers more than one peek cycle).

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, AppContext, Bounds, Context, Hsla, IntoElement, ParentElement, Render, SharedString, Styled,
    Task, TitlebarOptions, Window, WindowBounds, WindowOptions, div, px, size,
};
use gpui_metro::{
    Icon, TextStyle, Theme, TileSize, list_item, panorama, pivot, progress_bar, progress_dots, progress_ring, text,
    theme::size as type_size, tile, tile_grid, u,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scene {
    Idle,
    StaticTiles(usize),
    LiveTiles(usize),
    /// Live tiles without a content key: every slide redraws every tile.
    LiveTilesUnkeyed(usize),
    Progress,
    PivotPaging,
    PanoramaPaging,
    List(usize),
}

impl Scene {
    const SUITE: [Scene; 10] = [
        Scene::Idle,
        Scene::StaticTiles(60),
        Scene::LiveTiles(12),
        Scene::LiveTiles(60),
        Scene::LiveTiles(120),
        Scene::LiveTilesUnkeyed(60),
        Scene::Progress,
        Scene::PivotPaging,
        Scene::PanoramaPaging,
        Scene::List(200),
    ];

    fn name(self) -> String {
        match self {
            Scene::Idle => "idle".into(),
            Scene::StaticTiles(n) => format!("static-tiles-{n}"),
            Scene::LiveTiles(n) => format!("live-tiles-{n}"),
            Scene::LiveTilesUnkeyed(n) => format!("live-tiles-{n}-unkeyed"),
            Scene::Progress => "progress".into(),
            Scene::PivotPaging => "pivot-paging".into(),
            Scene::PanoramaPaging => "panorama-paging".into(),
            Scene::List(n) => format!("list-{n}"),
        }
    }

    fn find(name: &str) -> Option<Scene> {
        let all = Scene::SUITE.into_iter().chain([
            Scene::StaticTiles(120),
            Scene::LiveTiles(30),
            Scene::LiveTiles(240),
            Scene::List(1000),
        ]);
        all.into_iter().find(|s| s.name() == name).or_else(|| {
            let (head, n) = name.rsplit_once('-')?;
            let n: usize = n.parse().ok()?;
            match head {
                "live-tiles" => Some(Scene::LiveTiles(n)),
                "static-tiles" => Some(Scene::StaticTiles(n)),
                "list" => Some(Scene::List(n)),
                _ => None,
            }
        })
    }

    /// Scenes that page on their own every second.
    fn pages(self) -> bool {
        matches!(self, Scene::PivotPaging | Scene::PanoramaPaging)
    }
}

struct Row {
    scene: String,
    cpu: f64,
    fps: f64,
}

/// User + system CPU seconds this process has used.
fn cpu_seconds() -> f64 {
    // SAFETY: getrusage only writes into the struct we hand it.
    let mut u: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut u) };
    let t = |v: libc::timeval| v.tv_sec as f64 + v.tv_usec as f64 / 1e6;
    t(u.ru_utime) + t(u.ru_stime)
}

struct Options {
    scenes: Vec<Scene>,
    secs: u64,
    warmup: u64,
    out: Option<String>,
    show_only: bool,
}

fn options() -> Options {
    let mut o = Options { scenes: Vec::new(), secs: 20, warmup: 3, out: None, show_only: false };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--secs" => o.secs = args.next().and_then(|v| v.parse().ok()).unwrap_or(o.secs),
            "--warmup" => o.warmup = args.next().and_then(|v| v.parse().ok()).unwrap_or(o.warmup),
            "--out" => o.out = args.next(),
            "--show" => o.show_only = true,
            name => match Scene::find(name) {
                Some(s) => o.scenes.push(s),
                None => eprintln!("unknown scene {name:?}; try one of {:?}", Scene::SUITE.map(|s| s.name())),
            },
        }
    }
    if o.scenes.is_empty() {
        o.scenes = Scene::SUITE.to_vec();
    }
    o
}

struct Lab {
    scene: Scene,
    frames: Rc<Cell<u64>>,
    pivot: usize,
    pano: usize,
    _driver: Task<()>,
    _pager: Task<()>,
}

impl Lab {
    fn new(o: Options, cx: &mut Context<Self>) -> Self {
        let frames = Rc::new(Cell::new(0));
        let first = o.scenes[0];
        let counter = frames.clone();
        let driver = cx.spawn(async move |this, cx| {
            if o.show_only {
                return;
            }
            let mut rows = Vec::new();
            for scene in o.scenes {
                let switched = this.update(cx, |lab, cx| {
                    lab.scene = scene;
                    cx.notify();
                });
                if switched.is_err() {
                    return;
                }
                cx.background_executor().timer(Duration::from_secs(o.warmup)).await;
                let (c0, f0, t0) = (cpu_seconds(), counter.get(), Instant::now());
                cx.background_executor().timer(Duration::from_secs(o.secs)).await;
                let (c1, f1, dt) = (cpu_seconds(), counter.get(), t0.elapsed().as_secs_f64());
                let row = Row { scene: scene.name(), cpu: (c1 - c0) / dt * 100., fps: (f1 - f0) as f64 / dt };
                eprintln!("{:<20} {:>6.1} % CPU {:>6.1} frames/s", row.scene, row.cpu, row.fps);
                rows.push(row);
            }
            let mut table = String::from("| scene | CPU % | frames/s |\n|---|---:|---:|\n");
            for r in &rows {
                table += &format!("| {} | {:.1} | {:.1} |\n", r.scene, r.cpu, r.fps);
            }
            println!("{table}");
            if let Some(path) = o.out {
                std::fs::write(&path, &table).ok();
            }
            cx.update(|cx| cx.quit());
        });
        let pager = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let alive = this.update(cx, |lab, cx| {
                    if lab.scene.pages() {
                        lab.pivot = (lab.pivot + 1) % 4;
                        lab.pano = (lab.pano + 1) % 4;
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        });
        Self { scene: first, frames, pivot: 0, pano: 0, _driver: driver, _pager: pager }
    }
}

const SIZES: [TileSize; 10] = [
    TileSize::Medium,
    TileSize::Small,
    TileSize::Small,
    TileSize::Small,
    TileSize::Small,
    TileSize::Wide,
    TileSize::Medium,
    TileSize::Small,
    TileSize::Small,
    TileSize::Medium,
];

const ICONS: [Icon; 6] = [Icon::Mail, Icon::Phone, Icon::Camera, Icon::Calendar, Icon::People, Icon::Clock];

/// A live tile's back face: a headline over two lines, like the hubs had.
fn back_face(i: usize) -> impl FnOnce(TileSize, Hsla) -> gpui::Div + 'static {
    move |s, _| {
        let big = if s == TileSize::Small { type_size::MEDIUM } else { type_size::LARGE };
        div()
            .size_full()
            .flex()
            .flex_col()
            .p(u(8.))
            .text_color(gpui::white())
            .child(div().text_size(u(big)).child(format!("R-{}", 4700 + i)))
            .child(div().text_size(u(type_size::SMALL)).child(format!("OD {}.{} · log", i % 12, i % 10)))
            .child(div().text_size(u(type_size::SMALL)).child("BR-2K · 2 L"))
    }
}

fn tiles(n: usize, live: bool, keyed: bool) -> AnyElement {
    let grid = tile_grid().columns(16).metrics(64., 8.).tiles((0..n).map(|i| {
        let t = tile(SharedString::from(format!("t{i}")), SIZES[i % SIZES.len()])
            .title(format!("tile {i}"))
            .icon(ICONS[i % ICONS.len()])
            .count((i % 7) as u32);
        if live {
            // Spread the peeks: different periods and phases, as a real
            // start screen of many apps would have.
            let t = t.period(9. + (i % 7) as f32).phase((i as f32 * 0.37).fract());
            if keyed {
                // The recommended path: a text back face, which the tile can key itself.
                t.back_text(format!("R-{}", 4700 + i), [format!("OD {}.{} · log", i % 12, i % 10), "BR-2K · 2 L".into()])
            } else {
                // A closure face with no key: the tile can't tell when it changes.
                t.back(back_face(i))
            }
        } else {
            t
        }
    }));
    div().size_full().overflow_hidden().p(u(24.)).child(grid).into_any_element()
}

fn mail_pane(folder: usize) -> AnyElement {
    div()
        .size_full()
        .px(u(24.))
        .children((0..12).map(|i| {
            list_item(("mail", folder * 100 + i), format!("sender {i}"))
                .subtitle(format!("subject line {i} in folder {folder}"))
                .detail("a preview of the message body that runs on")
        }))
        .into_any_element()
}

fn scene(lab: &Lab, cx: &mut Context<Lab>) -> AnyElement {
    let _ = cx;
    match lab.scene {
        Scene::Idle => div().size_full().p(u(24.)).child(text(TextStyle::Title1, "idle")).into_any_element(),
        Scene::StaticTiles(n) => tiles(n, false, false),
        Scene::LiveTiles(n) => tiles(n, true, true),
        Scene::LiveTilesUnkeyed(n) => tiles(n, true, false),
        Scene::Progress => div()
            .size_full()
            .p(u(24.))
            .flex()
            .flex_col()
            .gap(u(32.))
            .child(progress_dots("dots"))
            .child(progress_bar("bar", 0.4))
            .child(progress_ring("ring").diameter(64.))
            .into_any_element(),
        Scene::PivotPaging => {
            pivot("pv", ["inbox", "unread", "flagged", "urgent"], lab.pivot)
                .title("perf")
                .all_panes((0..4).map(mail_pane))
                .into_any_element()
        }
        Scene::PanoramaPaging => {
            let mut p = panorama("pn", "incubator", lab.pano).section_width(560.);
            for i in 0..4 {
                p = p.section(format!("section {i}"), mail_pane(i));
            }
            p.into_any_element()
        }
        Scene::List(n) => div()
            .id("list")
            .size_full()
            .overflow_hidden()
            .px(u(24.))
            .children((0..n).map(|i| list_item(("row", i), format!("row {i}")).subtitle("subtitle").detail("detail")))
            .into_any_element(),
    }
}

impl Render for Lab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.frames.set(self.frames.get() + 1);
        let p = Theme::palette(cx);
        div()
            .size_full()
            .bg(p.background)
            .text_color(p.foreground)
            .font_family(Theme::font(cx))
            .child(scene(self, cx))
    }
}

use gpui::InteractiveElement;

fn main() {
    let o = options();
    gpui_platform::application().with_assets(gpui_metro::Assets::new()).run(move |cx: &mut App| {
        gpui_metro::init(cx);
        let bounds = Bounds::centered(None, size(px(1280.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions { title: Some("gpui-metro perf incubator".into()), ..Default::default() }),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| Lab::new(o, cx)),
        )
        .expect("failed to open the incubator window");
        cx.activate(true);
    });
}
