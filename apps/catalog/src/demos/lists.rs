//! List item variants and the a–z contacts jump list.

use gpui::{AnyElement, Context, IntoElement, ParentElement, SharedString, Styled, div};
use gpui_metro::u;
use gpui_metro::{ALPHABET, Accent, Icon, Leading, ListGroup, Theme, jump_list, list_item, page_header, progress_ring};

use super::{DemoState, caption, page, stack, tap};
use crate::app::Catalog;

pub fn list_items(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let p = Theme::palette(cx);
    let chevron = || Icon::ChevronRight.render(u(22.), p.subtle);
    let choices = ["cellular data", "wi-fi", "bluetooth"];
    let selectable = choices.into_iter().enumerate().map(|(ix, title)| {
        let on = s.selected_row == Some(ix);
        let mut item = list_item(("li-pick", ix), title)
            .subtitle(if on { "selected" } else { "tap to select" })
            .on_click(tap(cx, move |s| s.selected_row = Some(ix)));
        if on {
            item = item.trailing(Icon::Check.render(u(24.), p.accent));
        } else {
            item = item.subtle_subtitle();
        }
        item
    });

    page(
        "list item",
        stack()
            .gap(u(4.))
            .child(caption("title only"))
            .child(list_item("li-title", "ringtones + sounds"))
            .child(caption("accent subtitle · subtle subtitle"))
            .child(list_item("li-accent", "kristin").subtitle("mobile"))
            .child(list_item("li-subtle", "lock screen").subtitle("notifications, background").subtle_subtitle())
            .child(caption("third line"))
            .child(list_item("li-detail", "anna").subtitle("photos from the trip").detail("yesterday, 6:12 pm"))
            .child(caption("leading glyph · letters · element · colour"))
            .child(list_item("li-icon", "email + accounts").subtitle("3 accounts").leading_icon(Icon::Mail))
            .child(
                list_item("li-glyph", "ben thompson")
                    .subtitle("work")
                    .leading(Leading::Glyph("bt".into()))
                    .leading_color(Accent::Teal.color()),
            )
            .child(
                list_item("li-element", "backup")
                    .subtitle("in progress")
                    .leading(Leading::Element(progress_ring("li-ring").diameter(32.).into_any_element()))
                    .leading_color(p.chrome),
            )
            .child(
                list_item("li-color", "battery saver")
                    .subtitle("off")
                    .subtle_subtitle()
                    .leading_icon(Icon::Battery)
                    .leading_color(Accent::Emerald.color()),
            )
            .child(caption("trailing element"))
            .child(list_item("li-trailing", "about").subtitle("phone info").subtle_subtitle().trailing(chevron()))
            .child(caption("clickable"))
            .children(selectable),
    )
}

const CONTACTS: [&str; 34] = [
    "Aaron Burr",
    "Ada Lovelace",
    "Alan Turing",
    "Anna Karenina",
    "Barbara Liskov",
    "Ben Thompson",
    "Brian Kernighan",
    "Charles Babbage",
    "Claude Shannon",
    "Dennis Ritchie",
    "Donald Knuth",
    "Edsger Dijkstra",
    "Frances Allen",
    "Grace Hopper",
    "Guido van Rossum",
    "Hedy Lamarr",
    "Ivan Sutherland",
    "John McCarthy",
    "Ken Thompson",
    "Kristin Larsen",
    "Leslie Lamport",
    "Linus Torvalds",
    "Margaret Hamilton",
    "Niklaus Wirth",
    "Radia Perlman",
    "Rob Pike",
    "Robin Milner",
    "Sophie Wilson",
    "Tim Berners-Lee",
    "Tony Hoare",
    "Vint Cerf",
    "Whitfield Diffie",
    "Yukihiro Matsumoto",
    "8 Ball Pool Club",
];

/// Lower-case first letter, or `#` for anything that isn't a letter.
fn key_of(name: &str) -> String {
    match name.chars().next() {
        Some(c) if c.is_alphabetic() => c.to_lowercase().to_string(),
        _ => "#".into(),
    }
}

fn initials(name: &str) -> SharedString {
    name.split_whitespace().filter_map(|w| w.chars().next()).take(2).collect::<String>().to_lowercase().into()
}

pub fn contacts() -> AnyElement {
    let groups = ALPHABET.into_iter().filter_map(|key| {
        let rows: Vec<AnyElement> = CONTACTS
            .iter()
            .enumerate()
            .filter(|(_, name)| key_of(name) == key)
            .map(|(ix, name)| {
                list_item(("contact", ix), *name)
                    .leading(Leading::Glyph(initials(name)))
                    .leading_color(Accent::ALL[ix % Accent::ALL.len()].color())
                    .into_any_element()
            })
            .collect();
        (!rows.is_empty()).then(|| ListGroup::new(key, rows))
    });
    div()
        .size_full()
        .flex()
        .flex_col()
        .child(page_header("people").app_title("contacts"))
        .child(div().flex_1().min_h_0().child(jump_list("contacts").keys(ALPHABET).groups(groups)))
        .into_any_element()
}
