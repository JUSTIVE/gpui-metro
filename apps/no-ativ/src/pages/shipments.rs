//! Shipments: cryo boxes leaving the floor, each with its stage progress.

use gpui::{App, SharedString, div, prelude::*};
use gpui_metro::u;
use gpui_metro::{TextStyle, Theme, pivot, progress_bar, text};

use crate::data::{Shipment, ShipmentStage};
use crate::nav::{Nav, PageView};
use crate::pages::{APP, alarm, scroll, subtle};
use crate::store::{Store, stage_progress};

const STAGES: [ShipmentStage; 5] = [
    ShipmentStage::Packed,
    ShipmentStage::PickedUp,
    ShipmentStage::InTransit,
    ShipmentStage::Customs,
    ShipmentStage::Delivered,
];

fn shipment_row(s: &Shipment, cx: &App) -> impl IntoElement {
    let p = Theme::palette(cx);
    let stages = div()
        .flex()
        .justify_between()
        .children(STAGES.iter().map(|&stage| {
            let reached = stage_progress(stage) <= stage_progress(s.stage);
            div()
                .text_size(u(11.))
                .text_color(if stage == s.stage {
                    p.accent
                } else if reached {
                    p.foreground
                } else {
                    p.disabled
                })
                .child(stage.as_str())
        }));
    div()
        .flex()
        .flex_col()
        .gap(u(4.))
        .py(u(10.))
        .child(text(TextStyle::Large, s.destination.to_lowercase()))
        .child(text(TextStyle::Accent, format!("{} · {}", s.code, s.eta)))
        .child(div().pt(u(6.)).child(progress_bar(
            SharedString::from(format!("ship-{}", s.id)),
            stage_progress(s.stage),
        )))
        .child(stages)
        .child(
            div()
                .flex()
                .gap(u(8.))
                .child(subtle(
                    format!(
                        "{} · {} vials · {} kg dry ice",
                        s.courier, s.vials, s.dry_ice_kg
                    ),
                    cx,
                ))
                .children((s.excursions > 0).then(|| {
                    subtle(
                        format!(
                            "{} excursion{}",
                            s.excursions,
                            if s.excursions == 1 { "" } else { "s" }
                        ),
                        cx,
                    )
                    .text_color(alarm())
                })),
        )
}

pub fn render(selected: usize, store: &Store, nav: &Nav, cx: &App) -> PageView {
    let delivered = selected == 1;
    let mut shipments: Vec<&Shipment> = store
        .data
        .shipments
        .iter()
        .filter(|s| (s.stage == ShipmentStage::Delivered) == delivered)
        .collect();
    shipments.sort_by_key(|s| if delivered { -s.eta_days } else { s.eta_days });
    let pane = scroll(format!("shipments-{selected}")).children(
        shipments
            .into_iter()
            .enumerate()
            .map(|(ix, s)| nav.feather("shipments", ix, shipment_row(s, cx), cx)),
    );
    PageView::new(
        pivot("shipments-pivot", ["en route", "delivered"], selected)
            .title(APP)
            .on_select(nav.select())
            .child(pane),
    )
}
