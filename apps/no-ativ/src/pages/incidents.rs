//! Incidents: a pivot by status, the incident detail, and the form that files
//! a new one.

use gpui::{App, Hsla, SharedString, Window, div, prelude::*, relative};
use gpui_metro::u;
use gpui_metro::{
    Icon, TextStyle, Theme, app_bar, button, check_box, list_item, list_picker, pivot, text,
    text_box, toggle_switch,
};

use crate::app::{Dialog, NoAtiv};
use crate::data::{Incident, IncidentSeverity, IncidentStatus};
use crate::nav::{AREAS, IncidentForm, Nav, Page, PageView, SEVERITIES};
use crate::pages::{
    APP, alarm, fact, heading, missing, runs::run_item, scroll, subtle, titled, warn,
};
use crate::store::{IncidentDraft, Store, StoreError};

const STATUSES: [IncidentStatus; 4] = [
    IncidentStatus::Open,
    IncidentStatus::Investigating,
    IncidentStatus::Capa,
    IncidentStatus::Closed,
];

pub fn severity_color(severity: IncidentSeverity, cx: &App) -> Hsla {
    match severity {
        IncidentSeverity::Critical => alarm(),
        IncidentSeverity::Major => warn(),
        IncidentSeverity::Minor => Theme::palette(cx).accent,
    }
}

fn incident_item(incident: &Incident, nav: &Nav, cx: &App) -> impl IntoElement {
    let id = incident.id.clone();
    list_item(
        SharedString::from(format!("incident-{}", incident.id)),
        incident.title.clone(),
    )
    .leading_icon(Icon::Alert)
    .leading_color(severity_color(incident.severity, cx))
    .subtitle(format!(
        "{} · {} · {}",
        incident.code,
        incident.severity.as_str(),
        incident.area.as_str()
    ))
    .detail(format!("opened {} · {}", incident.opened, incident.owner))
    .on_click(nav.push(move || Page::Incident { id: id.clone() }))
}

pub fn list(selected: usize, store: &Store, nav: &Nav, cx: &App) -> PageView {
    let status = STATUSES[selected.min(STATUSES.len() - 1)];
    let incidents: Vec<&Incident> = store
        .data
        .incidents
        .iter()
        .filter(|i| i.status == status)
        .collect();
    let pane = scroll(format!("incidents-{selected}"))
        .when(incidents.is_empty(), |el| {
            el.child(subtle("nothing in this state", cx))
        })
        .children(
            incidents
                .iter()
                
                .map(|i| incident_item(i, nav, cx)),
        );
    let body = pivot("incidents-pivot", STATUSES.map(|s| s.as_str()), selected)
        .title(APP)
        .on_select(nav.select())
        .child(pane);
    PageView::new(body).app_bar(app_bar("incidents-bar").button(
        Icon::Add,
        "add",
        nav.click(|app, window, cx| app.new_incident(None, window, cx)),
    ))
}

pub fn detail(id: &str, store: &Store, nav: &Nav, cx: &App) -> PageView {
    let Some(incident) = store.incident(id) else {
        return missing("incidents", "incident", cx);
    };
    let p = Theme::palette(cx);
    let half = |el: gpui::Div| el.w(relative(0.5));

    let summary = div()
        .flex()
        .flex_col()
        .gap(u(4.))
        .child(text(TextStyle::Large, incident.title.clone()))
        .child(
            text(
                TextStyle::Accent,
                format!(
                    "{} · {}",
                    incident.severity.as_str(),
                    incident.status.as_str()
                ),
            )
            .color(severity_color(incident.severity, cx)),
        );

    let facts = div()
        .flex()
        .flex_wrap()
        .pt(u(14.))
        .child(half(fact("area", incident.area.as_str())))
        .child(half(fact("owner", incident.owner.clone())))
        .child(half(fact("opened", incident.opened.clone())))
        .child(half(fact("age", format!("{} days", incident.age))))
        .children(
            incident
                .instrument_id
                .as_deref()
                .and_then(|i| store.instrument(i))
                .map(|i| {
                    half(fact(
                        "instrument",
                        format!("{} · {}", i.code, i.state.as_str()),
                    ))
                }),
        )
        .children(
            incident
                .lot_id
                .as_deref()
                .and_then(|l| store.lot(l))
                .map(|l| half(fact("lot", l.code.clone()))),
        );

    let linked = incident
        .run_id
        .as_deref()
        .and_then(|r| store.run(r))
        .map(|run| {
            div()
                .flex()
                .flex_col()
                .child(heading("run", cx))
                .child(run_item("incident-run", run, store, nav, cx))
        });

    let timeline = div()
        .flex()
        .flex_col()
        .child(heading("timeline", cx))
        .children(incident.timeline.iter().map(|e| {
            div()
                .flex()
                .gap(u(12.))
                .py(u(6.))
                .child(div().w(u(3.)).flex_none().bg(p.accent))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(text(TextStyle::Normal, e.text.clone()))
                        .child(subtle(e.at.clone(), cx)),
                )
        }));

    let body = titled(
        "incidents",
        incident.code.clone(),
        scroll(format!("incident-{}", incident.id))
            .child(summary)
            .child(facts)
            .children(linked)
            .child(timeline),
    );

    let bar = app_bar("incident-bar");
    let bar = if incident.status == IncidentStatus::Closed {
        bar.disabled_button(Icon::Next, "advance")
    } else {
        let id = incident.id.clone();
        bar.button(
            Icon::Next,
            "advance",
            nav.click(move |app, _, cx| {
                let moved = app.store.update(cx, |s, cx| {
                    cx.notify();
                    s.advance_incident(&id)
                        .map(|status| (s.incident(&id).map(|i| i.code.clone()), status))
                });
                if let Some((Some(code), status)) = moved {
                    app.toast(
                        "incident-moved",
                        Icon::Next,
                        code,
                        format!("moved to {}", status.as_str()),
                        cx,
                    );
                }
            }),
        )
    };
    PageView::new(body).app_bar(bar)
}

/// A form callback: edits the form on top of the stack, then re-renders.
fn edit<A: 'static>(
    nav: &Nav,
    f: fn(&mut IncidentForm, A),
) -> impl Fn(A, &mut Window, &mut App) + 'static {
    nav.on(move |app, a, _, cx| {
        app.update_page(|page| {
            if let Some(form) = page.form_mut() {
                f(form, a);
            }
        });
        cx.notify();
    })
}

fn save(app: &mut NoAtiv, window: &mut Window, cx: &mut Context<NoAtiv>) {
    let Page::NewIncident(form) = app.top() else {
        return;
    };
    let draft = IncidentDraft {
        title: form.title.read(cx).text().to_string(),
        area: AREAS[form.area],
        severity: SEVERITIES[form.severity],
        run_id: if form.link_run {
            form.run_choices.get(form.run).cloned()
        } else {
            None
        },
        quarantine: form.quarantine,
        page_lead: form.page_lead,
    };
    match app.store.update(cx, |s, cx| {
        cx.notify();
        s.file_incident(draft)
    }) {
        Ok(code) => {
            app.pop(window, cx);
            app.update_page(|page| {
                if let Page::Incidents { pivot } = page {
                    *pivot = 0;
                }
            });
            app.toast("incident-filed", Icon::Alert, code, "filed · open", cx);
        }
        Err(e) => {
            let title = if e == StoreError::EmptyTitle {
                "missing title"
            } else {
                "can't file this"
            };
            app.show(
                Dialog::Notice {
                    title: title.into(),
                    message: e.to_string().into(),
                },
                cx,
            );
        }
    }
}

pub fn form(form: &IncidentForm, store: &Store, nav: &Nav) -> PageView {
    let runs: Vec<String> = form
        .run_choices
        .iter()
        .filter_map(|id| store.run(id))
        .map(|r| format!("{} · {}", r.code, r.phase.as_str()))
        .collect();

    let fields = scroll("incident-form")
        .gap(u(18.))
        .child(text_box(&form.title).header("title"))
        .child(
            list_picker("area", AREAS.map(|a| a.as_str()), form.area)
                    .header("area")
                    .on_select(edit(nav, |f, ix| f.area = ix)),
        )
        .child(
            list_picker("severity", SEVERITIES.map(|s| s.as_str()), form.severity)
                    .header("severity")
                    .on_select(edit(nav, |f, ix| f.severity = ix)),
        )
        .child(
            div()
                    .flex()
                    .flex_col()
                    .gap(u(8.))
                    .child(
                        toggle_switch("link-run", form.link_run)
                            .header("link to a run")
                            .disabled(runs.is_empty())
                            .on_change(edit(nav, |f, on| f.link_run = on)),
                    )
                    .when(form.link_run && !runs.is_empty(), |el| {
                        el.child(
                            list_picker("run", runs, form.run)
                                .on_select(edit(nav, |f, ix| f.run = ix)),
                        )
                    }),
        )
        .child(
            div()
                    .flex()
                    .flex_col()
                    .child(
                        check_box("quarantine", form.quarantine)
                            .label("quarantine the area")
                            .on_change(edit(nav, |f, on| f.quarantine = on)),
                    )
                    .child(
                        check_box("page-lead", form.page_lead)
                            .label("page the floor lead")
                            .on_change(edit(nav, |f, on| f.page_lead = on)),
                    ),
        )
        .child(
            button("save", "save")
                    .accent()
                    .full_width()
                    .on_click(nav.click(save)),
        );

    PageView::new(titled("incidents", "new incident", fields)).app_bar(
        app_bar("form-bar")
            .button(Icon::Save, "save", nav.click(save))
            .button(
                Icon::Close,
                "cancel",
                nav.click(|app, window, cx| app.pop(window, cx)),
            ),
    )
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;
    use crate::app::tests::{boot, depth};

    #[gpui::test]
    fn save_validates_then_files_and_returns(cx: &mut TestAppContext) {
        let (app, cx) = boot(Some("new-incident"), cx);
        let save_now = |cx: &mut gpui::VisualTestContext| {
            let app = app.clone();
            cx.update(move |window, cx| app.update(cx, |a, cx| save(a, window, cx)));
            cx.run_until_parked();
        };
        let open = |cx: &mut gpui::VisualTestContext| {
            app.read_with(cx, |a, cx| a.store.read(cx).open_incidents().count())
        };
        let before = open(cx);

        save_now(cx);
        assert_eq!(depth(&app, cx), 2, "an empty title keeps the form open");
        assert!(
            app.read_with(cx, |a, _| a.has_dialog()),
            "…and explains why"
        );
        cx.simulate_keystrokes("escape");
        assert!(
            !app.read_with(cx, |a, _| a.has_dialog()),
            "back closes the message box first"
        );
        assert_eq!(depth(&app, cx), 2);

        let title = app.read_with(cx, |a, _| match a.top() {
            Page::NewIncident(form) => form.title.clone(),
            _ => unreachable!(),
        });
        cx.update(|_, cx| title.update(cx, |t, cx| t.set_text("chiller alarm on rack B", cx)));
        save_now(cx);
        assert_eq!(depth(&app, cx), 1, "filed, back to where we came from");
        assert_eq!(open(cx), before + 1);
        let top = app.read_with(cx, |a, cx| a.store.read(cx).data.incidents[0].title.clone());
        assert_eq!(top, "chiller alarm on rack B");
    }
}
