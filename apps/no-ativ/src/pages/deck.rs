//! Deck: today's bookings, and the machines they run on.

use gpui::{App, Hsla, SharedString, prelude::*};
use gpui_metro::{Accent, Icon, Theme, list_item, pivot};

use crate::data::{DECK_TODAY, DeckBlock, DeckKind, InstrumentState};
use crate::nav::{Nav, Page, PageView};
use crate::pages::{APP, alarm, scroll, warn};
use crate::store::Store;

fn kind_color(kind: DeckKind, cx: &App) -> Hsla {
    match kind {
        DeckKind::Run => Theme::palette(cx).accent,
        DeckKind::Cip => Accent::Teal.color(),
        DeckKind::Maintenance => warn(),
        DeckKind::Booked => Accent::Steel.color(),
    }
}

fn booking(block: &DeckBlock, store: &Store, nav: &Nav, cx: &App) -> impl IntoElement {
    let machine = store
        .instrument(&block.instrument_id)
        .map(|i| i.code.clone())
        .unwrap_or_default();
    let strain = block
        .strain_id
        .as_deref()
        .and_then(|s| store.strain(s))
        .map(|s| format!(" · {}", s.name));
    let run_id = block.run_id.clone();
    list_item(SharedString::from(format!("deck-{}", block.id)), machine)
        .leading_icon(Icon::Calendar)
        .leading_color(kind_color(block.kind, cx))
        .subtitle(format!("{}{}", block.label, strain.unwrap_or_default()))
        .detail(format!(
            "{} · day {} of {} · {}",
            block.kind.as_str(),
            DECK_TODAY - block.day + 1,
            block.days,
            block.operator
        ))
        .when_some(run_id, |el, id| {
            el.on_click(nav.push(move || Page::Run { id: id.clone() }))
        })
}

pub fn render(selected: usize, store: &Store, nav: &Nav, cx: &App) -> PageView {
    let pane = |selected: usize| {
        if selected == 0 {
            scroll("deck-today").children(store.deck_today().map(|b| booking(b, store, nav, cx)))
        } else {
            scroll("deck-instruments").children(store.data.instruments.iter().map(|i| {
                let tone = match i.state {
                    InstrumentState::Down => alarm(),
                    InstrumentState::Cip => warn(),
                    InstrumentState::Running => Theme::palette(cx).accent,
                    InstrumentState::Ready => Accent::Emerald.color(),
                };

                list_item(
                    SharedString::from(format!("instrument-{}", i.id)),
                    i.code.clone(),
                )
                .leading_icon(Icon::Gauge)
                .leading_color(tone)
                .subtitle(format!("{} for {} h", i.state.as_str(), i.since_hours))
                .detail(format!("{} · {} cycles", i.kind, i.cycles))
            }))
        }
    };
    PageView::new(
        pivot("deck-pivot", ["today", "instruments"], selected)
            .title(APP)
            .on_select(nav.select())
            .all_panes((0..2).map(pane)),
    )
}
