//! Tiles in every size and face, and a packed start screen.

use gpui::{
    AnyElement, Context, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div,
};
use gpui_metro::u;
use gpui_metro::{
    Accent, Icon, PAGE_MARGIN, TILE_GAP, TILE_UNIT, TextStyle, Tile, TileSize,
    motion::{Direction, enter_div},
    round_button, text,
    theme::{LIGHT, size},
    tile, tile_grid, toggle_switch,
};

use super::{DemoState, caption, navigation, on, page, stack, tap};
use crate::{app::Catalog, phone::SCREEN_W};

/// A calendar-style front face: big date, next appointment.
fn agenda(_: TileSize, _: Hsla) -> impl IntoElement {
    let ink = gpui::white();
    div()
        .relative()
        .size_full()
        .flex()
        .flex_col()
        .p(u(12.))
        .gap(u(2.))
        .text_color(ink)
        .child(div().text_size(u(size::NORMAL)).child("dentist"))
        .child(div().text_size(u(size::NORMAL)).opacity(0.8).child("3:30 pm · capitol hill"))
        .child(div().absolute().right(u(12.)).bottom(u(2.)).text_size(u(size::HUGE)).font_weight(LIGHT).child("23"))
}

/// A back face: a few lines of text over the tile colour.
fn back_text(title: &'static str, body: &'static str) -> impl FnOnce(TileSize, Hsla) -> AnyElement {
    move |_, _| {
        div()
            .size_full()
            .flex()
            .flex_col()
            .p(u(10.))
            .text_color(gpui::white())
            .child(div().text_size(u(size::MEDIUM_LARGE)).child(title))
            .child(div().text_size(u(size::SMALL)).opacity(0.85).child(body))
            .into_any_element()
    }
}

pub fn tile_page(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let mail = tile("tile-mail", TileSize::Medium)
        .title("mail")
        .icon(Icon::Mail)
        .count(s.mail_count)
        .on_click(tap(cx, |s| s.mail_count += 1));
    let smalls = div()
        .flex()
        .flex_wrap()
        .w(u(TILE_UNIT * 2. + TILE_GAP))
        .gap(u(TILE_GAP))
        .child(tile("tile-phone", TileSize::Small).icon(Icon::Phone).count(2))
        .child(tile("tile-camera", TileSize::Small).icon(Icon::Camera).color(Accent::Magenta.color()))
        .child(tile("tile-wifi", TileSize::Small).icon(Icon::Wifi).color(Accent::Teal.color()))
        .child(
            tile("tile-reset", TileSize::Small)
                .icon(Icon::Refresh)
                .color(Accent::Steel.color())
                .on_click(tap(cx, |s| s.mail_count = 0)),
        );

    page(
        "tile",
        stack()
            .child(caption("medium · small — tap mail to add, refresh to clear"))
            .child(div().flex().gap(u(TILE_GAP)).child(mail).child(smalls))
            .child(caption("wide · custom front face"))
            .child(tile("tile-calendar", TileSize::Wide).title("calendar").front(agenda))
            .child(caption("live · back faces peek every few seconds"))
            .child(
                div()
                    .flex()
                    .gap(u(TILE_GAP))
                    .child(
                        tile("tile-people", TileSize::Medium)
                            .title("people")
                            .icon(Icon::People)
                            .back(back_text("kristin", "new photo"))
                            .period(10.),
                    )
                    .child(
                        tile("tile-music", TileSize::Medium)
                            .title("music")
                            .icon(Icon::Play)
                            .color(Accent::Orange.color())
                            .back(back_text("now playing", "daft punk · get lucky"))
                            .period(10.)
                            .phase(0.5),
                    ),
            )
            .child(caption("large"))
            .child(
                tile("tile-weather", TileSize::Large)
                    .title("weather")
                    .icon(Icon::Snow)
                    .color(Accent::Cobalt.color())
                    .back(back_text("seattle −2°", "snow through the evening, clearing overnight"))
                    .period(12.)
                    .phase(0.25),
            ),
    )
}

/// An app opened from the start-screen demo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Launched {
    /// outlook: a pivot of mail folders.
    Pivot,
    /// people: a panorama hub.
    Panorama,
}

fn launch(cx: &Context<Catalog>, app: Launched) -> impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static {
    tap(cx, move |s| {
        s.launched = Some(app);
        s.launches += 1;
    })
}

/// The start screen's tiles, in pin order. people and outlook launch apps.
fn start_tiles(mail_count: u32, cx: &Context<Catalog>) -> Vec<Tile> {
    vec![
        tile("start-phone", TileSize::Medium).title("phone").icon(Icon::Phone).count(1),
        tile("start-sms", TileSize::Small).icon(Icon::Mail).count(3),
        tile("start-wifi", TileSize::Small).icon(Icon::Wifi),
        tile("start-camera", TileSize::Small).icon(Icon::Camera),
        tile("start-settings", TileSize::Small).icon(Icon::Settings),
        tile("start-people", TileSize::Wide)
            .title("people")
            .icon(Icon::People)
            .back(back_text("ben, kristin, anna", "3 new posts"))
            .period(11.)
            .on_click(launch(cx, Launched::Panorama)),
        tile("start-mail", TileSize::Medium)
            .title("outlook")
            .icon(Icon::Mail)
            .count(mail_count)
            .on_click(launch(cx, Launched::Pivot)),
        tile("start-calendar", TileSize::Medium)
            .title("calendar")
            .icon(Icon::Calendar)
            .back(back_text("dentist", "3:30 pm today"))
            .period(13.)
            .phase(0.4),
        tile("start-music", TileSize::Wide).title("music+videos").icon(Icon::Play).color(Accent::Orange.color()),
        tile("start-clock", TileSize::Small).icon(Icon::Clock),
        tile("start-search", TileSize::Small).icon(Icon::Search),
        tile("start-store", TileSize::Medium).title("store").icon(Icon::Download).color(Accent::Emerald.color()),
        tile("start-weather", TileSize::Large)
            .title("weather")
            .icon(Icon::Snow)
            .back(back_text("seattle −2°", "snow"))
            .period(12.)
            .phase(0.7),
        tile("start-lock", TileSize::Small).icon(Icon::Lock),
        tile("start-favorite", TileSize::Small).icon(Icon::Favorite).color(Accent::Crimson.color()),
    ]
}

pub fn grid_page(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let key = |side: &str| ElementId::from((SharedString::from(format!("start-{side}")), s.launches));
    if let Some(app) = s.launched {
        let body = match app {
            Launched::Pivot => navigation::pivot_page(s, cx),
            Launched::Panorama => navigation::panorama_page(s, cx),
        };
        let back = tap(cx, |s| {
            s.launched = None;
            s.launches += 1;
        });
        return div()
            .size_full()
            .flex()
            .flex_col()
            .child(enter_div(key("app"), Direction::Forward, 0, div().flex_1().min_h_0().child(body), cx))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(u(12.))
                    .px(u(PAGE_MARGIN))
                    .py(u(10.))
                    .child(round_button("start-back", Icon::Back).diameter(40.).on_click(back))
                    .child(caption("back to start")),
            )
            .into_any_element();
    }
    let start = start_screen(s, cx);
    if s.launches == 0 {
        return start;
    }
    // Coming back from an app: the start screen swings back in from the left.
    enter_div(key("return"), Direction::Backward, 0, div().size_full().child(start), cx)
}

fn start_screen(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let (columns, gap) = if s.more_tiles { (6, 8.) } else { (4, TILE_GAP) };
    let width = SCREEN_W - 2. * PAGE_MARGIN;
    let unit = (width - (columns - 1) as f32 * gap) / columns as f32;
    div()
        .id("start-screen")
        .size_full()
        .overflow_y_scroll()
        .px(u(PAGE_MARGIN))
        .pt(u(16.))
        .pb(u(40.))
        .flex()
        .flex_col()
        .gap(u(16.))
        .child(
            toggle_switch("more-tiles", s.more_tiles)
                .header("show more tiles")
                .labels("6 columns", "4 columns")
                .on_change(on(cx, |s, v| s.more_tiles = v)),
        )
        .child(caption("tap people (panorama) or outlook (pivot) to launch"))
        .child(
            tile_grid()
                .turnstile(true)
                .columns(columns)
                .metrics(unit, gap)
                .tiles(start_tiles(s.mail_count, cx)),
        )
        .child(text(TextStyle::Small, format!("{columns} columns · unit {unit:.0} px · gap {gap:.0} px")))
        .into_any_element()
}
