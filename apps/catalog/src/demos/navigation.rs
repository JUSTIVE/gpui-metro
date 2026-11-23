//! Pivot, panorama and the application bar.

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, div, px,
};
use gpui_metro::{
    Accent, AppBarMode, Icon, PAGE_MARGIN, TILE_GAP, TextStyle, Theme, TileSize, app_bar, list_item, page_header,
    panorama, pivot, text,
    theme::{SEMILIGHT, size},
    tile, toggle_switch,
};

use super::{DemoState, caption, on, stack, tap};
use crate::app::Catalog;

const FOLDERS: [&str; 4] = ["inbox", "unread", "flagged", "urgent"];

struct Mail {
    from: &'static str,
    subject: &'static str,
    preview: &'static str,
    unread: bool,
    flagged: bool,
    urgent: bool,
}

const MAIL: [Mail; 7] = [
    Mail {
        from: "kristin",
        subject: "dinner friday?",
        preview: "there's a new thai place on 15th",
        unread: true,
        flagged: false,
        urgent: false,
    },
    Mail {
        from: "build bot",
        subject: "nightly failed",
        preview: "3 tests failed on arm64",
        unread: true,
        flagged: true,
        urgent: true,
    },
    Mail {
        from: "anna",
        subject: "photos from the trip",
        preview: "finally got them off my camera",
        unread: false,
        flagged: true,
        urgent: false,
    },
    Mail {
        from: "landlord",
        subject: "water shut-off tuesday",
        preview: "between 9 and noon",
        unread: true,
        flagged: false,
        urgent: true,
    },
    Mail {
        from: "ben",
        subject: "re: metro catalog",
        preview: "the pivot looks great",
        unread: false,
        flagged: false,
        urgent: false,
    },
    Mail {
        from: "airline",
        subject: "check in now",
        preview: "SEA → SFO, seat 14A",
        unread: false,
        flagged: true,
        urgent: false,
    },
    Mail {
        from: "newsletter",
        subject: "this week in rust",
        preview: "edition 2024 is here",
        unread: false,
        flagged: false,
        urgent: false,
    },
];

pub fn pivot_page(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let folder = s.pivot;
    let rows = MAIL
        .iter()
        .enumerate()
        .filter(|(_, m)| match folder {
            1 => m.unread,
            2 => m.flagged,
            3 => m.urgent,
            _ => true,
        })
        .map(|(ix, m)| {
            let mut item = list_item(("mail", ix), m.from).subtitle(m.subject).detail(m.preview);
            if !m.unread {
                item = item.subtle_subtitle();
            }
            item
        });
    let pane = div().id("pivot-pane").size_full().overflow_y_scroll().px(px(PAGE_MARGIN)).children(rows);
    pivot("demo-pivot", FOLDERS, folder)
        .title("outlook")
        .on_select(on(cx, |s, ix| s.pivot = ix))
        .child(pane)
        .into_any_element()
}

pub fn panorama_page(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let p = Theme::palette(cx);
    let big = |label: &'static str| {
        div().text_size(px(size::EXTRA_LARGE)).font_weight(SEMILIGHT).line_height(px(52.)).child(label)
    };
    let collection = div()
        .flex()
        .flex_col()
        .child(big("music"))
        .child(big("videos"))
        .child(big("podcasts"))
        .child(big("radio"))
        .child(div().pt(px(12.)).child(text(TextStyle::Subtle, "swipe or tap the peeking section")));

    let colors = [Accent::Orange, Accent::Magenta, Accent::Teal, Accent::Lime, Accent::Violet, Accent::Cobalt];
    let history = div().flex().flex_wrap().gap(px(TILE_GAP)).children(colors.into_iter().enumerate().map(|(ix, a)| {
        tile(("pano-tile", ix), TileSize::Medium).color(a.color()).icon(Icon::Play).metrics(85., TILE_GAP)
    }));

    let new = div().flex().flex_col().children(
        [("random access memories", "daft punk"), ("lonerism", "tame impala"), ("settle", "disclosure")]
            .into_iter()
            .enumerate()
            .map(|(ix, (album, artist))| {
                list_item(("pano-new", ix), album).subtitle(artist).leading_icon(Icon::Play).leading_color(p.accent)
            }),
    );

    panorama("demo-pano", "music+videos", s.panorama)
        .section_width(432.)
        .on_select(on(cx, |s, ix| s.panorama = ix))
        .section("collection", collection)
        .section("history", history)
        .section("new", new)
        .into_any_element()
}

pub fn app_bar_page(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let mode = if s.app_bar_minimized { AppBarMode::Minimized } else { AppBarMode::Default };
    let log = |label: &'static str| tap(cx, move |s| s.app_bar_log = label.into());
    let bar = app_bar("demo-app-bar")
        .mode(mode)
        .button(Icon::Add, "new", log("new"))
        .button(Icon::Sync, "sync", log("sync"))
        .button(Icon::Search, "search", log("search"))
        .disabled_button(Icon::Delete, "delete")
        .menu_item("settings", log("settings"))
        .menu_item("about", log("about"));

    div()
        .relative()
        .size_full()
        .flex()
        .flex_col()
        .child(page_header("app bar").app_title("metro catalog"))
        .child(
            stack()
                .px(px(PAGE_MARGIN))
                .child(
                    toggle_switch("app-bar-minimized", s.app_bar_minimized)
                        .header("minimized mode")
                        .on_change(on(cx, |s, v| s.app_bar_minimized = v)),
                )
                .child(caption("tap ··· to expand: labels and menu items appear"))
                .child(text(TextStyle::Large, format!("last action: {}", s.app_bar_log))),
        )
        // The bar overlays the page bottom so expanding never reflows it.
        .child(div().absolute().left_0().right_0().bottom_0().child(bar))
        .into_any_element()
}
