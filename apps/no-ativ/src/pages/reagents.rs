//! Reagents: every lot with its shelf life, expiring and expired stock toned.

use gpui::{App, Hsla, div, prelude::*, px};
use gpui_metro::{TextStyle, Theme, pivot, progress_bar, text};

use crate::data::{LotState, ReagentLot, to_fixed};
use crate::nav::{Nav, PageView};
use crate::pages::{APP, alarm, scroll, subtle, warn};
use crate::store::Store;

fn tone(state: LotState, cx: &App) -> Hsla {
    match state {
        LotState::Expired => alarm(),
        LotState::Expiring | LotState::Low => warn(),
        LotState::Ok => Theme::palette(cx).subtle,
    }
}

fn lot_row(lot: &ReagentLot, state: LotState, cx: &App) -> impl IntoElement {
    let shelf = (lot.shelf_days.max(0) as f32 / lot.shelf_total.max(1) as f32).clamp(0., 1.);
    let expiry = match state {
        LotState::Expired => lot.expires.clone(),
        LotState::Ok => format!("expires {}", lot.expires),
        _ => format!("{} · expires {}", state.as_str(), lot.expires),
    };
    div()
        .flex()
        .flex_col()
        .gap(px(4.))
        .py(px(10.))
        .child(text(TextStyle::Large, lot.name.clone()))
        .child(subtle(
            format!(
                "{} · {} · {} · {}",
                lot.code, lot.vendor, lot.grade, lot.location
            ),
            cx,
        ))
        .child(div().pt(px(4.)).child(progress_bar(
            gpui::SharedString::from(format!("shelf-{}", lot.id)),
            shelf,
        )))
        .child(
            div()
                .flex()
                .justify_between()
                .child(text(TextStyle::Normal, expiry).color(tone(state, cx)))
                .child(
                    text(
                        TextStyle::Normal,
                        format!("{} / {} {}", to_fixed(lot.qty, 2), lot.capacity, lot.unit),
                    )
                    .color(if state == LotState::Low {
                        warn()
                    } else {
                        Theme::palette(cx).subtle
                    }),
                ),
        )
        .when_some(lot.reserved_for.clone(), |el, run| {
            el.child(subtle(format!("reserved for {run}"), cx))
        })
}

pub fn render(selected: usize, store: &Store, nav: &Nav, cx: &App) -> PageView {
    let mut lots: Vec<(&ReagentLot, LotState)> = store
        .data
        .lots
        .iter()
        .map(|l| (l, store.lot_state(l)))
        .collect();
    if selected == 0 {
        lots.retain(|(_, s)| *s != LotState::Ok);
        lots.sort_by_key(|(l, _)| l.shelf_days);
    }
    let pane = scroll(format!("reagents-{selected}"))
        .child(subtle(
            format!(
                "warning {} days before expiry · change it in settings",
                store.expiry_window
            ),
            cx,
        ))
        .children(
            lots.into_iter()
                .enumerate()
                .map(|(ix, (lot, state))| nav.feather("lots", ix, lot_row(lot, state, cx), cx)),
        );
    PageView::new(
        pivot("reagents-pivot", ["attention", "all lots"], selected)
            .title(APP)
            .on_select(nav.select())
            .child(pane),
    )
}
