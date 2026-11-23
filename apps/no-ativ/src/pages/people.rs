//! People: the roster, the alumni, and what each role may do.

use gpui::{App, SharedString, div, prelude::*};
use gpui_metro::u;
use gpui_metro::{Leading, TextStyle, list_item, pivot, text};

use crate::data::{Member, ROLES};
use crate::nav::{Nav, PageView};
use crate::pages::{APP, scroll, subtle};
use crate::store::Store;

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_lowercase()
}

fn role_name(member: &Member) -> &'static str {
    ROLES
        .iter()
        .find(|r| r.id == member.role)
        .map(|r| r.name)
        .unwrap_or("")
}

fn member_item(member: &Member) -> impl IntoElement {
    let since = match &member.left {
        Some(left) => format!("{} – {}", member.joined, left),
        None => format!("since {}", member.joined),
    };
    list_item(
        SharedString::from(format!("member-{}", member.id)),
        member.name.clone(),
    )
    .leading(Leading::Glyph(initials(&member.name).into()))
    .subtitle(format!("{} · {}", role_name(member).to_lowercase(), since))
    .detail(member.email.clone())
}

pub fn render(selected: usize, store: &Store, nav: &Nav, cx: &App) -> PageView {
    let pane = match selected {
        0 | 1 => {
            let alumni = selected == 1;
            scroll(format!("people-{selected}")).children(
                store
                    .data
                    .members
                    .iter()
                    .filter(|m| m.left.is_some() == alumni)
                    
                    .map(member_item),
            )
        }
        _ => scroll("roles").children(ROLES.iter().map(|role| {
            let count = store
                .data
                .members
                .iter()
                .filter(|m| m.role == role.id && m.left.is_none())
                .count();
            div()
                    .flex()
                    .flex_col()
                    .py(u(8.))
                    .child(text(
                        TextStyle::Large,
                        format!("{} · {count}", role.name.to_lowercase()),
                    ))
                    .child(subtle(role.description, cx))
        })),
    };
    PageView::new(
        pivot("people-pivot", ["roster", "alumni", "roles"], selected)
            .title(APP)
            .on_select(nav.select())
            .child(pane),
    )
}
