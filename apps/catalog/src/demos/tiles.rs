//! Tiles in every size and face, and a packed start screen.

use gpui::{
    AnyElement, Context, Hsla, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, div,
    px,
};
use gpui_metro::{
    Accent, Icon, PAGE_MARGIN, TILE_GAP, TILE_UNIT, TextStyle, Tile, TileSize, text,
    theme::{LIGHT, size},
    tile, tile_grid, toggle_switch,
};

use super::{DemoState, caption, on, page, stack, tap};
use crate::{app::Catalog, phone::SCREEN_W};

/// A calendar-style front face: big date, next appointment.
fn agenda(_: TileSize, _: Hsla) -> impl IntoElement {
    let ink = gpui::white();
    div()
        .relative()
        .size_full()
        .flex()
        .flex_col()
        .p(px(12.))
        .gap(px(2.))
        .text_color(ink)
        .child(div().text_size(px(size::NORMAL)).child("dentist"))
        .child(div().text_size(px(size::NORMAL)).opacity(0.8).child("3:30 pm · capitol hill"))
        .child(div().absolute().right(px(12.)).bottom(px(2.)).text_size(px(size::HUGE)).font_weight(LIGHT).child("23"))
}

/// A back face: a few lines of text over the tile colour.
fn back_text(title: &'static str, body: &'static str) -> impl FnOnce(TileSize, Hsla) -> AnyElement {
    move |_, _| {
        div()
            .size_full()
            .flex()
            .flex_col()
            .p(px(10.))
            .text_color(gpui::white())
            .child(div().text_size(px(size::MEDIUM_LARGE)).child(title))
            .child(div().text_size(px(size::SMALL)).opacity(0.85).child(body))
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
        .w(px(TILE_UNIT * 2. + TILE_GAP))
        .gap(px(TILE_GAP))
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
            .child(div().flex().gap(px(TILE_GAP)).child(mail).child(smalls))
            .child(caption("wide · custom front face"))
            .child(tile("tile-calendar", TileSize::Wide).title("calendar").front(agenda))
            .child(caption("live · back faces peek every few seconds"))
            .child(
                div()
                    .flex()
                    .gap(px(TILE_GAP))
                    .child(
                        tile("tile-people", TileSize::Medium)
                            .title("people")
                            .icon(Icon::People)
                            .back(back_text("kristin", "new photo"))
                            .period(5.),
                    )
                    .child(
                        tile("tile-music", TileSize::Medium)
                            .title("music")
                            .icon(Icon::Play)
                            .color(Accent::Orange.color())
                            .back(back_text("now playing", "daft punk · get lucky"))
                            .period(5.)
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
                    .period(7.)
                    .phase(0.25),
            ),
    )
}

/// The start screen's tiles, in pin order.
fn start_tiles(mail_count: u32) -> Vec<Tile> {
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
            .period(6.),
        tile("start-mail", TileSize::Medium).title("outlook").icon(Icon::Mail).count(mail_count),
        tile("start-calendar", TileSize::Medium)
            .title("calendar")
            .icon(Icon::Calendar)
            .back(back_text("dentist", "3:30 pm today"))
            .period(8.)
            .phase(0.4),
        tile("start-music", TileSize::Wide).title("music+videos").icon(Icon::Play).color(Accent::Orange.color()),
        tile("start-clock", TileSize::Small).icon(Icon::Clock),
        tile("start-search", TileSize::Small).icon(Icon::Search),
        tile("start-store", TileSize::Medium).title("store").icon(Icon::Download).color(Accent::Emerald.color()),
        tile("start-weather", TileSize::Large)
            .title("weather")
            .icon(Icon::Snow)
            .back(back_text("seattle −2°", "snow"))
            .period(7.)
            .phase(0.7),
        tile("start-lock", TileSize::Small).icon(Icon::Lock),
        tile("start-favorite", TileSize::Small).icon(Icon::Favorite).color(Accent::Crimson.color()),
    ]
}

pub fn grid_page(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let (columns, gap) = if s.more_tiles { (6, 8.) } else { (4, TILE_GAP) };
    let width = SCREEN_W - 2. * PAGE_MARGIN;
    let unit = (width - (columns - 1) as f32 * gap) / columns as f32;
    div()
        .id("start-screen")
        .size_full()
        .overflow_y_scroll()
        .px(px(PAGE_MARGIN))
        .pt(px(16.))
        .pb(px(40.))
        .flex()
        .flex_col()
        .gap(px(16.))
        .child(
            toggle_switch("more-tiles", s.more_tiles)
                .header("show more tiles")
                .labels("6 columns", "4 columns")
                .on_change(on(cx, |s, v| s.more_tiles = v)),
        )
        .child(tile_grid().columns(columns).metrics(unit, gap).tiles(start_tiles(s.mail_count)))
        .child(text(TextStyle::Small, format!("{columns} columns · unit {unit:.0} px · gap {gap:.0} px")))
        .into_any_element()
}
