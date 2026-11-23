//! Settings: the classic Windows Phone theme page, plus the one foundry
//! setting the phone owns (how early reagents start warning).

use gpui::{App, div, prelude::*};
use gpui_metro::u;
use gpui_metro::{Accent, Mode, TextStyle, Theme, list_picker, slider, text, toggle_switch};

use crate::nav::{Nav, PageView};
use crate::pages::{heading, scroll, subtle, titled};
use crate::store::{EXPIRY_WINDOW, Store};

const SWATCH: f32 = 78.;

fn accent_grid(nav: &Nav, cx: &App) -> impl IntoElement {
    let theme = Theme::get(cx);
    let (current, ring) = (theme.accent, theme.palette.foreground);
    div()
        .flex()
        .flex_wrap()
        .gap(u(10.))
        .children(Accent::ALL.iter().map(|&accent| {
            div()
                .id(accent.name())
                .size(u(SWATCH))
                .bg(accent.color())
                .cursor_pointer()
                .when(accent == current, |el| el.border(u(3.)).border_color(ring))
                .active(|s| s.opacity(0.7))
                .on_click(nav.click(move |_, _, cx| Theme::set_accent(accent, cx)))
        }))
}

pub fn render(store: &Store, nav: &Nav, cx: &App) -> PageView {
    let theme = Theme::get(cx);
    let (first, last) = (*EXPIRY_WINDOW.start(), *EXPIRY_WINDOW.end());
    let span = (last - first) as f32;
    let window = (store.expiry_window - first) as f32 / span;
    let facility = &store.data;

    let body = scroll("settings")
        .gap(u(14.))
        .child(
            list_picker(
                    "background",
                    [Mode::Dark.label(), Mode::Light.label()],
                    usize::from(theme.mode == Mode::Light),
                )
                .header("background")
                .on_select(nav.on(|_, ix: usize, _, cx| {
                    Theme::set_mode(if ix == 0 { Mode::Dark } else { Mode::Light }, cx)
                })),
        )
        .child(
            div()
                    .flex()
                    .flex_col()
                    .gap(u(8.))
                    .child(subtle("accent colour", cx))
                    .child(accent_grid(nav, cx))
                    .child(text(TextStyle::Accent, theme.accent.name())),
        )
        .child(
            toggle_switch("reduced-motion", theme.reduced_motion)
                    .header("reduce motion")
                    .on_change(nav.on(|_, on: bool, _, cx| Theme::set_reduced_motion(on, cx))),
        )
        .child(
            div()
                    .flex()
                    .flex_col()
                    .child(
                        slider("expiry-window", window)
                            .header("warn about reagents expiring within")
                            .step(1. / span)
                            .on_change(nav.on(move |app, v: f32, _, cx| {
                                let days = first + (v * span).round() as i64;
                                app.store.update(cx, |s, cx| {
                                    if s.expiry_window != days {
                                        s.set_expiry_window(days);
                                        cx.notify();
                                    }
                                });
                            })),
                    )
                    .child(text(
                        TextStyle::Large,
                        format!("{} days", store.expiry_window),
                    )),
        )
        .child(
            div()
                    .flex()
                    .flex_col()
                    .child(heading("about", cx))
                    .child(text(
                        TextStyle::Normal,
                        format!("Seoul Foundry · {}", facility.biosafety.as_str()),
                    ))
                    .child(subtle(
                        format!(
                            "logging runs since {} · {} years",
                            facility.opened, facility.years
                        ),
                        cx,
                    )),
        );
    PageView::new(titled("settings", "settings", body))
}
