//! One live demo per catalog page, grouped like the sidebar. Demos read
//! [`DemoState`] and write it back through the catalog entity, so every
//! toggle, slider and selection persists while browsing.

mod chrome;
mod controls;
mod foundations;
mod lists;
mod navigation;
mod overlays;
mod progress;
mod tiles;

use gpui::{
    AnyElement, App, AppContext, ClickEvent, Context, ElementId, Entity, InteractiveElement, IntoElement,
    ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div,
};
use gpui_metro::u;
use gpui_metro::{PAGE_MARGIN, TextInput, TextStyle, motion::Direction, page_header, text};

use crate::{app::Catalog, pages::Page};

pub use overlays::ToastState;
pub use tiles::Launched;

/// What a demo puts on the phone screen.
pub struct Demo {
    pub body: AnyElement,
    /// Drawn over the whole screen, tray included (message box, toast).
    pub overlay: Option<AnyElement>,
}

impl Demo {
    fn new(body: impl IntoElement) -> Self {
        Self { body: body.into_any_element(), overlay: None }
    }
}

/// Every value the demos let you change.
pub struct DemoState {
    // tiles
    pub mail_count: u32,
    pub more_tiles: bool,
    /// The app opened from the start-screen demo, if any.
    pub launched: Option<Launched>,
    /// Bumps on every launch and return, so entrances replay.
    pub launches: usize,
    // navigation
    pub pivot: usize,
    pub panorama: usize,
    pub app_bar_minimized: bool,
    pub app_bar_log: SharedString,
    // controls
    pub button_taps: u32,
    pub last_button: SharedString,
    pub favorite: bool,
    pub playing: bool,
    pub last_link: Option<SharedString>,
    pub wifi: bool,
    pub bluetooth: bool,
    pub airplane: bool,
    pub checks: [bool; 3],
    pub route: usize,
    pub volume: f32,
    pub brightness: f32,
    pub ringtone: usize,
    pub name_input: Entity<TextInput>,
    pub message_input: Entity<TextInput>,
    pub submitted: Option<SharedString>,
    // progress
    pub progress: f32,
    pub loading: bool,
    // lists
    pub selected_row: Option<usize>,
    // overlays
    pub message_open: bool,
    pub dont_ask: bool,
    pub message_result: SharedString,
    pub toast: Option<ToastState>,
    pub toasts_sent: usize,
    pub toast_log: SharedString,
    // chrome
    pub battery: f32,
    pub tray_opaque: bool,
    pub motion_direction: Direction,
    pub motion_replays: usize,
}

impl DemoState {
    pub fn new(cx: &mut Context<Catalog>) -> Self {
        Self {
            mail_count: 4,
            more_tiles: false,
            launched: None,
            launches: 0,
            pivot: 0,
            panorama: 0,
            app_bar_minimized: false,
            app_bar_log: "nothing yet".into(),
            button_taps: 0,
            last_button: "none".into(),
            favorite: true,
            playing: false,
            last_link: None,
            wifi: true,
            bluetooth: false,
            airplane: false,
            checks: [true, false, true],
            route: 0,
            volume: 0.6,
            brightness: 0.5,
            ringtone: 2,
            name_input: cx.new(|cx| TextInput::new("your name", cx)),
            message_input: cx.new(|cx| TextInput::new("type and press enter", cx)),
            submitted: None,
            progress: 0.42,
            loading: true,
            selected_row: Some(1),
            message_open: false,
            dont_ask: false,
            message_result: "not asked yet".into(),
            toast: None,
            toasts_sent: 0,
            toast_log: "no toasts yet".into(),
            battery: 0.8,
            tray_opaque: false,
            motion_direction: Direction::Forward,
            motion_replays: 0,
        }
    }
}

pub fn render(page: Page, s: &DemoState, window: &mut Window, cx: &mut Context<Catalog>) -> Demo {
    match page {
        Page::Palette => Demo::new(foundations::palette(cx)),
        Page::Typography => Demo::new(foundations::typography(cx)),
        Page::Icons => Demo::new(foundations::icons(cx)),
        Page::Tile => Demo::new(tiles::tile_page(s, cx)),
        Page::TileGrid => Demo::new(tiles::grid_page(s, cx)),
        Page::Pivot => Demo::new(navigation::pivot_page(s, cx)),
        Page::Panorama => Demo::new(navigation::panorama_page(s, cx)),
        Page::AppBar => Demo::new(navigation::app_bar_page(s, cx)),
        Page::Button => Demo::new(controls::buttons(s, cx)),
        Page::RoundButton => Demo::new(controls::round_buttons(s, cx)),
        Page::Hyperlink => Demo::new(controls::hyperlinks(s, cx)),
        Page::ToggleSwitch => Demo::new(controls::toggle_switches(s, cx)),
        Page::CheckBox => Demo::new(controls::check_boxes(s, cx)),
        Page::RadioButton => Demo::new(controls::radio_buttons(s, cx)),
        Page::Slider => Demo::new(controls::sliders(s, cx)),
        Page::ListPicker => Demo::new(controls::list_pickers(s, cx)),
        Page::TextBox => Demo::new(controls::text_boxes(s, window, cx)),
        Page::ProgressBar => Demo::new(progress::bar(s, cx)),
        Page::ProgressDots => Demo::new(progress::dots(s, cx)),
        Page::ProgressRing => Demo::new(progress::ring()),
        Page::ListItem => Demo::new(lists::list_items(s, cx)),
        Page::JumpList => Demo::new(lists::contacts()),
        Page::MessageBox => overlays::message_box_page(s, cx),
        Page::Toast => overlays::toast_page(s, cx),
        Page::SystemTray => Demo::new(chrome::tray_page(s, cx)),
        Page::Header => Demo::new(chrome::header_page()),
        Page::Motion => Demo::new(chrome::motion_page(s, cx)),
    }
}

/// The usual page: `CATALOG` over a lower-case title, then a scrolling body.
fn page(title: &'static str, body: impl IntoElement) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(page_header(title).app_title("metro catalog"))
        .child(
            div()
                .id(ElementId::Name(SharedString::new_static(title)))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(u(PAGE_MARGIN))
                .pb(u(28.))
                .child(body),
        )
        .into_any_element()
}

/// A vertical stack with the standard gap between demo rows.
fn stack() -> gpui::Div {
    div().flex().flex_col().gap(u(18.))
}

/// A small subtle label over a demo row.
fn caption(label: impl Into<SharedString>) -> impl IntoElement {
    text(TextStyle::Small, label)
}

/// A click handler that edits demo state and re-renders.
fn tap(
    cx: &Context<Catalog>,
    f: impl Fn(&mut DemoState) + 'static,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    cx.listener(move |c: &mut Catalog, _: &ClickEvent, _, cx| {
        f(&mut c.demo);
        cx.notify();
    })
}

/// A value callback (`Fn(V, &mut Window, &mut App)`, as toggles, sliders and
/// pickers take) that edits demo state and re-renders.
fn on<V: 'static>(
    cx: &Context<Catalog>,
    f: impl Fn(&mut DemoState, V) + 'static,
) -> impl Fn(V, &mut Window, &mut App) + 'static {
    let this = cx.weak_entity();
    move |v, _, cx| {
        this.update(cx, |c, cx| {
            f(&mut c.demo, v);
            cx.notify();
        })
        .ok();
    }
}
