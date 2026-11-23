//! Strains: an a–z jump list, and the strain detail with its lineage.

use gpui::{App, SharedString, div, prelude::*, px, relative};
use gpui_metro::{ALPHABET, Leading, ListGroup, jump_list, list_item, page_header, progress_bar};

use crate::data::{Strain, StrainStatus};
use crate::nav::{Nav, Page, PageView};
use crate::pages::{APP, fact, heading, missing, runs::run_item, scroll, subtle, titled};
use crate::store::Store;

fn group_key(name: &str) -> String {
    match name.chars().next() {
        Some(c) if c.is_ascii_alphabetic() => c.to_ascii_lowercase().to_string(),
        _ => "#".into(),
    }
}

fn strain_item(prefix: &str, strain: &Strain, nav: &Nav) -> impl IntoElement {
    let id = strain.id.clone();
    list_item(
        SharedString::from(format!("{prefix}-{}", strain.id)),
        strain.name.clone(),
    )
    .subtitle(format!("{} · {}", strain.code, strain.chassis.as_str()))
    .detail(format!(
        "gen {} · fitness {} · {}",
        strain.generation,
        strain.fitness,
        strain.status.as_str()
    ))
    .when(strain.status == StrainStatus::Archived, |el| {
        el.subtle_subtitle()
    })
    .on_click(nav.push(move || Page::Strain { id: id.clone() }))
}

pub fn list(store: &Store, nav: &Nav, cx: &App) -> PageView {
    let mut strains: Vec<&Strain> = store.data.strains.iter().collect();
    strains.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.code.cmp(&b.code))
    });
    let mut groups: Vec<ListGroup> = Vec::new();
    for (ix, strain) in strains.into_iter().enumerate() {
        let key = group_key(&strain.name);
        let item = nav.feather("strains", ix, strain_item("strain", strain, nav), cx);
        match groups.last_mut() {
            Some(g) if g.key.as_ref() == key.as_str() => g.items.push(item),
            _ => groups.push(ListGroup::new(key, [item])),
        }
    }
    PageView::new(
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(page_header("strains").app_title(APP))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(jump_list("strains-jump").keys(ALPHABET).groups(groups)),
            ),
    )
}

pub fn detail(id: &str, store: &Store, nav: &Nav, cx: &App) -> PageView {
    let Some(strain) = store.strain(id) else {
        return missing("strains", "strain", cx);
    };
    let half = |el: gpui::Div| el.w(relative(0.5));

    let facts = div()
        .flex()
        .flex_wrap()
        .child(half(fact("code", strain.code.clone())))
        .child(half(fact("chassis", strain.chassis.as_str())))
        .child(half(fact("status", strain.status.as_str())))
        .child(half(fact("generation", strain.generation.to_string())))
        .child(half(fact("owner", strain.owner.clone())))
        .child(half(fact("updated", strain.updated.clone())));

    let fitness = div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(heading("fitness", cx))
        .child(progress_bar("fitness", strain.fitness as f32 / 100.))
        .child(subtle(
            format!("{} of 100, growth-normalized", strain.fitness),
            cx,
        ));

    let lineage = {
        let parent = strain.parent_id.as_deref().and_then(|p| store.strain(p));
        let children: Vec<&Strain> = store.children_of(&strain.id).collect();
        div()
            .flex()
            .flex_col()
            .child(heading("lineage", cx))
            .child(match parent {
                Some(p) => strain_item("parent", p, nav).into_any_element(),
                None => subtle("founder strain — no parent on record", cx).into_any_element(),
            })
            .when(!children.is_empty(), |el| {
                el.child(subtle(format!("passaged into {}", children.len()), cx).pt(px(6.)))
                    .children(children.into_iter().map(|c| strain_item("child", c, nav)))
            })
    };

    let constructs = div()
        .flex()
        .flex_col()
        .child(heading("constructs", cx))
        .children(
            strain
                .construct_ids
                .iter()
                .filter_map(|c| store.construct(c))
                .map(|c| {
                    list_item(
                        SharedString::from(format!("construct-{}", c.id)),
                        c.name.clone(),
                    )
                    .leading(Leading::Glyph("p".into()))
                    .subtitle(format!(
                        "{} · {} bp · {} % GC · {}",
                        c.code,
                        c.bp,
                        c.gc,
                        c.status.as_str()
                    ))
                    .detail(c.parts.join(" → "))
                }),
        );

    let runs: Vec<_> = store.runs_of(&strain.id).take(6).collect();
    let samples: Vec<_> = store.samples_of(&strain.id).take(8).collect();
    let history = div()
        .flex()
        .flex_col()
        .when(!runs.is_empty(), |el| {
            el.child(heading("runs", cx)).children(
                runs.iter()
                    .map(|r| run_item("strain-run", r, store, nav, cx)),
            )
        })
        .when(!samples.is_empty(), |el| {
            el.child(heading("banked", cx))
                .children(samples.iter().map(|s| {
                    subtle(
                        format!("{} · {} · frozen {}", s.code, s.form.as_str(), s.frozen_at),
                        cx,
                    )
                    .py(px(2.))
                }))
        });

    let body = titled(
        "strains",
        strain.name.clone(),
        scroll(format!("strain-{}", strain.id))
            .child(nav.feather("strain", 0, facts, cx))
            .child(nav.feather("strain", 1, fitness, cx))
            .child(nav.feather("strain", 2, lineage, cx))
            .child(nav.feather("strain", 3, constructs, cx))
            .child(nav.feather("strain", 4, history, cx)),
    );
    PageView::new(body)
}
