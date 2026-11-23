//! System tray, page header, and the turnstile entrance.

use gpui::{AnyElement, App, Context, IntoElement, ParentElement, Styled, div};
use gpui_metro::u;
use gpui_metro::{
    Icon, Leading, TextStyle, Theme, button, list_item,
    motion::{self, Direction},
    page_header, radio_button, slider, system_tray, text, toggle_switch,
};

use super::{DemoState, caption, on, page, stack, tap};
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

const DIRECTIONS: [(Direction, &str); 3] =
    [(Direction::Forward, "forward"), (Direction::Backward, "backward"), (Direction::Down, "down")];

const ROWS: [(&str, &str, Icon); 6] = [
    ("mail", "4 unread", Icon::Mail),
    ("calendar", "dentist at 3:30", Icon::Calendar),
    ("people", "kristin posted a photo", Icon::People),
    ("music", "get lucky", Icon::Play),
    ("weather", "snow, −2°", Icon::Snow),
    ("settings", "wi-fi on", Icon::Settings),
];

pub fn motion_page(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let reduced = Theme::get(cx).reduced_motion;
    let direction = s.motion_direction;
    let radios = DIRECTIONS.into_iter().enumerate().map(|(ix, (d, label))| {
        let select = on(cx, move |s, ()| {
            s.motion_direction = d;
            s.motion_replays += 1;
        });
        radio_button(("motion-dir", ix), d == direction).label(label).on_select(move |w, cx| select((), w, cx))
    });
    // Each row's id carries the replay count, so a replay restarts them all;
    // the index staggers them by `motion::FEATHER`.
    let rows = ROWS.into_iter().enumerate().map(|(ix, (title, subtitle, icon))| {
        motion::enter(
            motion::generation_id(&format!("motion-row-{ix}"), s.motion_replays),
            direction,
            ix,
            list_item(("motion-item", ix), title).subtitle(subtitle).leading(Leading::Icon(icon)),
            cx,
        )
    });
    page(
        "motion",
        stack()
            .child(div().flex().children(radios).gap(u(12.)))
            .child(div().flex().child(
                button("motion-replay", "replay").icon(Icon::Refresh).on_click(tap(cx, |s| s.motion_replays += 1)),
            ))
            .child(caption(if reduced {
                "reduced motion is on: rows appear without moving"
            } else {
                "turnstile feather: 350 ms ease-out, 45 ms per row"
            }))
            .child(div().flex().flex_col().children(rows))
            .child(text(TextStyle::Subtle, format!("played {} times", s.motion_replays + 1))),
    )
}
