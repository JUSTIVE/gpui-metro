//! Freezer: occupancy per rack and box.

use gpui::{App, SharedString, div, prelude::*};
use gpui_metro::u;
use gpui_metro::{TextStyle, progress_bar, text};

use crate::nav::{Nav, PageView};
use crate::pages::{heading, scroll, subtle, titled};
use crate::store::Store;

pub fn render(store: &Store, nav: &Nav, cx: &App) -> PageView {
    let (filled, slots) = store.freezer_occupancy();
    let summary = div()
        .flex()
        .flex_col()
        .child(text(
            TextStyle::ExtraLarge,
            format!("{} %", filled * 100 / slots.max(1)),
        ))
        .child(subtle(
            format!(
                "{filled} of {slots} slots across {} racks",
                store.data.racks.len()
            ),
            cx,
        ));

    let racks = store.data.racks.iter().enumerate().map(|(ix, rack)| {
        let boxes = rack.boxes.iter().map(|b| {
            let used = b.slots.iter().filter(|s| s.is_some()).count();
            div()
                .flex()
                .items_center()
                .gap(u(12.))
                .py(u(5.))
                .child(
                    div()
                        .w(u(36.))
                        .child(text(TextStyle::Normal, b.label.clone())),
                )
                .child(div().flex_1().child(progress_bar(
                    SharedString::from(format!("box-{}-{}", rack.id, b.id)),
                    used as f32 / b.slots.len().max(1) as f32,
                )))
                .child(
                    div()
                        .w(u(56.))
                        .flex()
                        .justify_end()
                        .child(subtle(format!("{used}/{}", b.slots.len()), cx)),
                )
        });
        nav.feather(
            "racks",
            ix + 1,
            div()
                .flex()
                .flex_col()
                .child(heading(
                    format!("{} · {}", rack.name.to_lowercase(), rack.temp),
                    cx,
                ))
                .children(boxes),
            cx,
        )
    });

    PageView::new(titled(
        "freezer",
        "freezer",
        scroll("freezer")
            .child(nav.feather("racks", 0, summary, cx))
            .children(racks),
    ))
}
