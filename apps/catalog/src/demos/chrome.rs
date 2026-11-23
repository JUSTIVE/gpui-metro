//! System tray, page header, and the turnstile entrance.

use gpui::{AnyElement, App, Context, IntoElement, ParentElement, SharedString, Styled, Window, div};
use gpui_metro::u;
use gpui_metro::{
    Icon, Leading, TextStyle, Theme, list_item,
    motion::{self, Direction},
    page_header, round_button, slider, system_tray, text, tile, toggle_switch,
};

use super::{DemoState, caption, on, page, stack};
use crate::app::Catalog;

/// A component shown inside an outlined box, as a specimen.
fn specimen(cx: &App, child: impl IntoElement) -> impl IntoElement {
    let p = Theme::palette(cx);
    div().border_1().border_color(p.disabled).child(child)
}

pub fn tray_page(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    page(
        "system tray",
        stack()
            .child(caption("these drive the tray at the top of this phone"))
            .child(
                slider("tray-battery", s.battery)
                    .header(format!("battery · {:.0}%", s.battery * 100.))
                    .on_change(on(cx, |s, v| s.battery = v)),
            )
            .child(
                toggle_switch("tray-opaque", s.tray_opaque)
                    .header("opaque tray")
                    .labels("Opaque", "Transparent")
                    .on_change(on(cx, |s, v| s.tray_opaque = v)),
            )
            .child(caption("specimens"))
            .child(specimen(cx, system_tray("9:41").battery(0.15)))
            .child(specimen(cx, system_tray("23:59").battery(1.).opaque())),
    )
}

pub fn header_page() -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(page_header("wi-fi").app_title("settings"))
        .child(page_header("inbox").app_title("outlook"))
        .child(page_header("no app title"))
        .into_any_element()
}

const ROWS: [(&str, &str, Icon); 4] = [
    ("mail", "4 unread", Icon::Mail),
    ("calendar", "dentist at 3:30", Icon::Calendar),
    ("people", "kristin posted a photo", Icon::People),
    ("music", "get lucky", Icon::Play),
];

/// One original Metro motion: its name, what it is for, and the specimen,
/// replaying by itself every few seconds.
fn motion_row(name: &str, use_: &str, specimen: impl IntoElement, cx: &App) -> impl IntoElement {
    let p = Theme::palette(cx);
    div()
        .flex()
        .flex_col()
        .gap(u(8.))
        .child(text(TextStyle::GroupHeader, name.to_string()))
        .child(caption(use_.to_string()))
        .child(
            div()
                .relative()
                .h(u(150.))
                .overflow_hidden()
                .border_1()
                .border_color(p.disabled.opacity(0.5))
                .child(specimen),
        )
}

fn rows(prefix: &str, loop_: usize, feather: bool, cx: &App) -> gpui::Div {
    div().flex().flex_col().px(u(12.)).children(ROWS.into_iter().take(2).enumerate().map(|(ix, (title, sub, icon))| {
        let item = list_item((SharedString::from(format!("{prefix}-row")), ix), title)
            .subtitle(sub)
            .leading(Leading::Icon(icon));
        if feather {
            motion::enter(motion::generation_id(&format!("{prefix}-{ix}"), loop_), Direction::Forward, ix, item, cx)
        } else {
            item.into_any_element()
        }
    }))
}

/// Metro's own motions, and nothing else: each plays on its own, on a loop.
pub fn motion_page(s: &DemoState, window: &mut Window, cx: &mut Context<Catalog>) -> AnyElement {
    let n = s.motion_loop;
    let key = |name: &str| motion::generation_id(name, n);

    let turnstile = motion::enter_div(
        key("m-turnstile"),
        Direction::Forward,
        0,
        div().size_full().child(text(TextStyle::Title2, "inbox").lowercase()).child(rows("m-ts", n, false, cx)),
        cx,
    );
    let feather = div()
        .size_full()
        .child(motion::enter(key("m-feather-h"), Direction::Forward, 0, text(TextStyle::Title2, "inbox").lowercase(), cx))
        .child(rows("m-feather", n, true, cx));
    let p = Theme::palette(cx);
    let swivel = motion::swivel_in(
        key("m-swivel"),
        div()
            .w_full()
            .bg(p.chrome)
            .p(u(14.))
            .child(text(TextStyle::Large, "delete this message?"))
            .child(text(TextStyle::Normal, "it will be gone for good.")),
        window,
        cx,
    );
    let slide = motion::slide_in(
        key("m-slide"),
        Direction::Forward,
        div().size_full().px(u(12.)).child(rows("m-slide", n, false, cx)),
        cx,
    );
    let tilt = div()
        .flex()
        .gap(u(12.))
        .p(u(12.))
        .child(tile("m-tilt-tile", gpui_metro::TileSize::Medium).icon(Icon::Mail).title("mail").on_click(|_, _, _| {}))
        .child(round_button("m-tilt-btn", Icon::Add).on_click(|_, _, _| {}));

    page(
        "motion",
        stack()
            .child(motion_row("turnstile", "pages, on navigation: the page swings in about the left edge", turnstile, cx))
            .child(motion_row("turnstile feather", "list pages: the header, then each item, swing in", feather, cx))
            .child(motion_row("swivel", "message boxes and toasts swing in about their middle", swivel, cx))
            .child(motion_row("slide", "pivot items: the pane slides in from the side", slide, cx))
            .child(motion_row("tilt", "press and hold: the element sinks under the pointer", tilt, cx)),
    )
}
