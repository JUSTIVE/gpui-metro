//! Determinate bar, indeterminate dots, and the ring.

use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use gpui_metro::u;
use gpui_metro::{TextStyle, progress_bar, progress_dots, progress_ring, slider, text, toggle_switch};

use super::{DemoState, caption, on, page, stack};
use crate::app::Catalog;

pub fn bar(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let done = (s.progress * 7.).round() as u32;
    page(
        "progress bar",
        stack()
            .child(text(TextStyle::Normal, format!("downloading updates · {done} of 7")))
            .child(progress_bar("pb-download", s.progress))
            .child(text(TextStyle::Subtle, format!("{:.0}%", s.progress * 100.)))
            .child(slider("pb-drive", s.progress).header("drive the bar").on_change(on(cx, |s, v| s.progress = v)))
            .child(caption("empty and full"))
            .child(progress_bar("pb-empty", 0.))
            .child(progress_bar("pb-full", 1.)),
    )
}

pub fn dots(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let feed = div()
        .flex()
        .flex_col()
        .gap(u(6.))
        .child(text(TextStyle::Large, "what's new"))
        .child(text(TextStyle::Subtle, if s.loading { "fetching your feed…" } else { "up to date" }));
    page(
        "progress dots",
        stack()
            .child(if s.loading {
                progress_dots("pd-feed").into_any_element()
            } else {
                div().h(u(4.)).into_any_element()
            })
            .child(feed)
            .child(toggle_switch("pd-loading", s.loading).header("loading").on_change(on(cx, |s, v| s.loading = v)))
            .child(caption("the caller decides when the dots show")),
    )
}

pub fn ring() -> AnyElement {
    let sizes = [24., 48., 96.];
    page(
        "progress ring",
        stack()
            .child(caption("24 · 48 · 96 px"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(u(40.))
                    .children(sizes.into_iter().enumerate().map(|(ix, d)| progress_ring(("ring", ix)).diameter(d))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(u(16.))
                    .pt(u(20.))
                    .child(progress_ring("ring-inline").diameter(32.))
                    .child(text(TextStyle::Normal, "signing in…")),
            ),
    )
}
