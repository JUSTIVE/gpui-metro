//! The start screen: one live tile per destination, each showing a real
//! reading from the store.

use gpui::{App, Hsla, ScrollWheelEvent, div, prelude::*};
use gpui_metro::u;
use gpui_metro::{Accent, Icon, PAGE_MARGIN, TextStyle, TileSize, text, theme::size, tile, tile_grid};

use crate::data::{LotState, SensorState};
use crate::nav::{Nav, Page, PageView};
use crate::store::{Store, reading};

/// A live tile's back face: a headline over a couple of detail lines.
fn back(
    headline: String,
    lines: Vec<String>,
) -> impl FnOnce(TileSize, Hsla) -> gpui::Div + 'static {
    move |tile_size, _| {
        let big = if tile_size == TileSize::Small {
            size::MEDIUM
        } else {
            size::LARGE
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .p(u(10.))
            .pb(u(26.))
            .gap(u(2.))
            .text_color(gpui::white())
            .child(
                div()
                    .text_size(u(big))
                    .line_height(u(big + 4.))
                    .child(headline),
            )
            .children(lines.into_iter().map(|l| {
                div()
                    .text_size(u(size::SMALL))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(l)
            }))
    }
}

pub fn render(store: &Store, nav: &Nav, _cx: &App) -> PageView {
    let live: Vec<_> = store.live_runs().collect();
    let open: Vec<_> = store.open_incidents().collect();
    let alarms = store.sensors_in(SensorState::Alarm);
    let (filled, slots) = store.freezer_occupancy();
    let expiring: Vec<_> = {
        let mut lots: Vec<_> = store.lots_in(LotState::Expiring).collect();
        lots.sort_by_key(|l| l.shelf_days);
        lots
    };
    let en_route = store
        .data
        .shipments
        .iter()
        .filter(|s| s.eta_days > 0)
        .count();

    let runs = {
        let t = tile("tile-runs", TileSize::Medium)
            .title("runs")
            .icon(Icon::Flask)
            .count(live.len() as u32);
        match live.first() {
            Some(r) => t
                .back(back(
                    r.code.clone(),
                    vec![
                        format!("{} · OD {}", r.phase.as_str(), r.od600),
                        r.vessel.clone(),
                    ],
                ))
                .phase(0.05),
            None => t,
        }
    };

    let incidents = {
        let t = tile("tile-incidents", TileSize::Medium)
            .title("incidents")
            .icon(Icon::Alert)
            .count(open.len() as u32)
            .color(Accent::Crimson.color());
        match open.first() {
            Some(i) => t
                .back(back(
                    i.code.clone(),
                    vec![i.title.clone(), i.severity.as_str().into()],
                ))
                .phase(0.35),
            None => t,
        }
    };

    let shipments = {
        let t = tile("tile-shipments", TileSize::Wide)
            .title("shipments")
            .icon(Icon::Truck)
            .count(en_route as u32);
        match store.next_shipment() {
            Some(s) => t
                .back(back(
                    format!("{} {}", s.code, s.eta),
                    vec![
                        format!("→ {}", s.destination),
                        format!("{} · {} · {} vials", s.stage.as_str(), s.courier, s.vials),
                    ],
                ))
                .phase(0.6)
                .period(15.),
            None => t,
        }
    };

    let telemetry = {
        let t = tile("tile-telemetry", TileSize::Medium)
            .title("telemetry")
            .icon(Icon::Gauge)
            .count(alarms as u32)
            .color(if alarms > 0 {
                Accent::Red.color()
            } else {
                Accent::Teal.color()
            });
        // Ticks every two seconds, so this face is always current.
        match store.worst_sensor() {
            Some(s) => t
                .back(back(
                    format!("{} {}", reading(s), s.unit),
                    vec![s.code.clone(), s.zone.to_lowercase()],
                ))
                .phase(0.18)
                .period(12.),
            None => t,
        }
    };

    let reagents = {
        let t = tile("tile-reagents", TileSize::Medium)
            .title("reagents")
            .icon(Icon::Clock)
            .count(expiring.len() as u32);
        match expiring.first() {
            Some(l) => t
                .back(back(
                    l.expires.clone(),
                    vec![l.name.clone(), l.code.clone()],
                ))
                .phase(0.8),
            None => t,
        }
    };

    let small = |id: &'static str, title: &'static str, icon: Icon, count: usize| {
        tile(id, TileSize::Small)
            .title(title)
            .icon(icon)
            .count(count as u32)
    };
    let freezer = small("tile-freezer", "freezer", Icon::Snow, 0)
        .back(back(
            format!("{} %", filled * 100 / slots.max(1)),
            vec![format!("{filled} vials")],
        ))
        .phase(0.45)
        .period(10.);

    let to = |page: fn() -> Page| nav.push(page);
    let strains = small("tile-strains", "strains", Icon::Dna, store.active_strains());
    let deck = small("tile-deck", "deck", Icon::Calendar, store.deck_today().count());
    let people = small("tile-people", "people", Icon::People, store.active_members());
    let settings = small("tile-settings", "settings", Icon::Settings, 0);

    // The Windows 8 start screen: named groups side by side, like the
    // React console's sidebar sections, scrolling sideways.
    let scroll = nav.start_scroll.clone();
    let hinge = START_INSET + f32::from(scroll.offset().x);
    let grid = tile_grid()
        .turnstile(true)
        .hinge(hinge)
        .group(
            "floor",
            [
                runs.on_click(to(|| Page::Runs { pivot: 1 })),
                telemetry.on_click(to(|| Page::Telemetry { zone: 0 })),
                incidents.on_click(to(|| Page::Incidents { pivot: 0 })),
                deck.on_click(to(|| Page::Deck { pivot: 0 })),
            ],
        )
        .group("biology", [strains.on_click(to(|| Page::Strains))])
        .group(
            "inventory",
            [
                shipments.on_click(to(|| Page::Shipments { pivot: 0 })),
                freezer.on_click(to(|| Page::Freezer)),
                reagents.on_click(to(|| Page::Reagents { pivot: 0 })),
            ],
        )
        .group(
            "org",
            [
                people.on_click(to(|| Page::People { pivot: 0 })),
                settings.on_click(to(|| Page::Settings)),
            ],
        );

    let wheel = scroll.clone();
    PageView::new(
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .px(u(PAGE_MARGIN))
                    .pb(u(28.))
                    .child(text(TextStyle::Title1, "start").lowercase()),
            )
            .child(
                div()
                    .id("start")
                    .flex_1()
                    .min_h_0()
                    .overflow_x_scroll()
                    .track_scroll(&scroll)
                    // A mouse wheel scrolls the start screen sideways.
                    .on_scroll_wheel(move |e: &ScrollWheelEvent, window, _| {
                        let delta = e.delta.pixel_delta(window.line_height());
                        let step = if delta.x.abs() > delta.y.abs() { delta.x } else { delta.y };
                        let max = wheel.max_offset().x;
                        let x = (wheel.offset().x + step).clamp(-max, gpui::px(0.));
                        wheel.set_offset(gpui::point(x, wheel.offset().y));
                        window.refresh();
                    })
                    .child(div().flex().px(u(PAGE_MARGIN)).pb(u(48.)).child(grid).child(div().w(u(96.)))),
            ),
    )
}

/// Distance from the window edge to the first group (the shell's gutter
/// plus the page margin), for the turnstile hinge.
const START_INSET: f32 = crate::app::LEFT_GUTTER + PAGE_MARGIN;
