//! Runs: a pivot over the ledger, and the run detail with harvest.

use gpui::{App, Hsla, div, prelude::*, relative};
use gpui_metro::u;
use gpui_metro::{
    Accent, Icon, Leading, TextStyle, Theme, app_bar, list_item, pivot, progress_bar, text,
};

use crate::app::Dialog;
use crate::data::{Run, RunPhase, Verdict, od_series, run_log, to_fixed};
use crate::nav::{Nav, Page, PageView};
use crate::pages::{APP, alarm, fact, heading, missing, scroll, subtle, titled};
use crate::store::{PEAK_OD, Store, growth, is_live};

const FILTERS: [&str; 4] = ["all", "live", "harvested", "failed"];

fn matches(filter: usize, phase: RunPhase) -> bool {
    match filter {
        1 => is_live(phase),
        2 => phase == RunPhase::Harvested,
        3 => phase == RunPhase::Failed,
        _ => true,
    }
}

pub fn phase_color(phase: RunPhase, cx: &App) -> Hsla {
    match phase {
        RunPhase::Lag => Accent::Steel.color(),
        RunPhase::Log => Theme::palette(cx).accent,
        RunPhase::Stationary => Accent::Teal.color(),
        RunPhase::Harvested => Accent::Emerald.color(),
        RunPhase::Failed => alarm(),
    }
}

/// A run as a list row, for any page that lists runs.
pub fn run_item(
    id_prefix: &str,
    run: &Run,
    store: &Store,
    nav: &Nav,
    cx: &App,
) -> impl IntoElement {
    let strain = store
        .strain(&run.strain_id)
        .map(|s| format!("{} {}", s.code, s.name))
        .unwrap_or_default();
    let initial = run.phase.as_str()[..1].to_string();
    let id = run.id.clone();
    list_item(
        gpui::SharedString::from(format!("{id_prefix}-{}", run.id)),
        run.code.clone(),
    )
    .leading(Leading::Glyph(initial.into()))
    .leading_color(phase_color(run.phase, cx))
    .subtitle(format!("{} · {}", run.phase.as_str(), strain))
    .detail(format!(
        "{} · OD {} · {}",
        run.vessel,
        to_fixed(run.od600, 1),
        run.started
    ))
    .on_click(nav.push(move || Page::Run { id: id.clone() }))
}

pub fn list(selected: usize, store: &Store, nav: &Nav, cx: &App) -> PageView {
    let pane = |selected: usize| {
        let runs: Vec<&Run> = store
            .data
            .runs
            .iter()
            .filter(|r| matches(selected, r.phase))
            .collect();
        scroll(format!("runs-{selected}"))
            .when(runs.is_empty(), |el| {
                el.child(subtle("nothing here right now", cx))
            })
            .children(runs.iter().map(|r| run_item("run", r, store, nav, cx)))
    };
    let body = pivot("runs-pivot", FILTERS, selected)
        .title(APP)
        .on_select(nav.select())
        .all_panes((0..FILTERS.len()).map(pane));
    PageView::new(body).app_bar(
        app_bar("runs-bar")
            .button(
                Icon::Alert,
                "incident",
                nav.click(|app, window, cx| app.new_incident(None, window, cx)),
            )
            .button(
                Icon::Thermometer,
                "telemetry",
                nav.push(|| Page::Telemetry { zone: 0 }),
            )
            .menu_item("freezer", nav.push(|| Page::Freezer))
            .menu_item("deck bookings", nav.push(|| Page::Deck { pivot: 0 })),
    )
}

/// Twenty-four bars of OD600 history.
fn growth_curve(run: &Run, cx: &App) -> impl IntoElement {
    let color = phase_color(run.phase, cx);
    let series = od_series(run);
    let max = series.iter().copied().fold(PEAK_OD * 0.25, f64::max);
    div()
        .flex()
        .items_end()
        .gap(u(3.))
        .h(u(56.))
        .pt(u(8.))
        .children(
            series
                .into_iter()
                .map(move |v| div().flex_1().h(relative((v / max) as f32)).bg(color)),
        )
}

pub fn detail(id: &str, store: &Store, nav: &Nav, cx: &App) -> PageView {
    let Some(run) = store.run(id) else {
        return missing("runs", "run", cx);
    };
    let p = Theme::palette(cx);
    let strain = store.strain(&run.strain_id);
    let instrument = run
        .instrument_id
        .as_deref()
        .and_then(|i| store.instrument(i));
    let lots: Vec<String> = run
        .lot_ids
        .iter()
        .filter_map(|l| store.lot(l))
        .map(|l| l.name.clone())
        .collect();

    let summary = div()
        .flex()
        .flex_col()
        .child(text(TextStyle::ExtraLarge, run.phase.as_str()).color(phase_color(run.phase, cx)))
        .children(strain.map(|s| subtle(format!("{} · {}", s.code, s.name), cx)));

    let growth_block = div()
        .flex()
        .flex_col()
        .gap(u(6.))
        .child(heading("growth", cx))
        .child(progress_bar("growth", growth(run)))
        .child(subtle(
            format!(
                "OD600 {} of {PEAK_OD:.0} · titer {}",
                to_fixed(run.od600, 2),
                run.titer
            ),
            cx,
        ))
        .child(growth_curve(run, cx));

    let half = |el: gpui::Div| el.w(relative(0.5));
    let facts = div()
        .flex()
        .flex_wrap()
        .pt(u(12.))
        .child(half(fact("vessel", run.vessel.clone())))
        .child(half(fact(
            "instrument",
            instrument
                .map(|i| i.code.clone())
                .unwrap_or_else(|| "—".into()),
        )))
        .child(half(fact("operator", run.operator.clone())))
        .child(half(fact("started", run.started.clone())))
        .child(half(fact("duration", run.duration.clone())))
        .child(half(fact(
            "charge",
            run.charge_l
                .map(|l| format!("{} L", to_fixed(l, 2)))
                .unwrap_or_else(|| "—".into()),
        )))
        .child(fact(
            "draws",
            if lots.is_empty() {
                "—".into()
            } else {
                lots.join(", ")
            },
        ));

    let assays: Vec<_> = store.assays_for(&run.id).collect();
    let assay_block = div()
        .flex()
        .flex_col()
        .when(!assays.is_empty(), |el| el.child(heading("assays", cx)))
        .children(assays.into_iter().map(|a| {
            let flagged = a.verdict == Verdict::Flag;
            div()
                .flex()
                .justify_between()
                .py(u(4.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(text(TextStyle::Normal, a.metric.clone()))
                        .child(subtle(
                            format!("{} · {}", a.kind.as_str(), a.verdict.as_str()),
                            cx,
                        )),
                )
                .child(text(TextStyle::Large, a.value.clone()).color(if flagged {
                    alarm()
                } else {
                    p.foreground
                }))
        }));

    let log = div().flex().flex_col().child(heading("log", cx)).child(
        div()
            .flex()
            .flex_col()
            .p(u(10.))
            .bg(p.chrome)
            .font_family("Menlo")
            .text_size(u(12.))
            .text_color(p.subtle)
            .children(run_log(run).into_iter().map(|l| div().child(l))),
    );

    let body = titled(
        "runs",
        run.code.clone(),
        scroll(format!("run-{}", run.id))
            .child(summary)
            .child(growth_block)
            .child(facts)
            .child(assay_block)
            .child(log),
    );

    let (run_id, code) = (run.id.clone(), run.code.clone());
    let strain_id = run.strain_id.clone();
    let bar = app_bar("run-bar");
    let bar = if matches!(run.phase, RunPhase::Log | RunPhase::Stationary) {
        let id = run_id.clone();
        bar.button(
            Icon::Check,
            "harvest",
            nav.click(move |app, _, cx| app.show(Dialog::Harvest { run_id: id.clone() }, cx)),
        )
    } else {
        bar.disabled_button(Icon::Check, "harvest")
    };
    let bar = bar
        .button(
            Icon::Alert,
            "flag",
            nav.click(move |app, window, cx| app.new_incident(Some(&run_id), window, cx)),
        )
        .button(
            Icon::Refresh,
            "refresh",
            nav.click(|app, _, cx| {
                app.store.update(cx, |s, cx| {
                    s.tick();
                    cx.notify();
                });
                app.toast(
                    "refreshed",
                    Icon::Refresh,
                    "refreshed",
                    "latest readings pulled from the floor",
                    cx,
                );
            }),
        )
        .menu_item(
            "open strain",
            nav.push(move || Page::Strain {
                id: strain_id.clone(),
            }),
        )
        .menu_item(
            "copy run code",
            nav.click(move |app, _, cx| app.copy(code.clone(), cx)),
        );
    PageView::new(body).app_bar(bar)
}
