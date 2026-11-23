//! The component index: a search box over the pages grouped by section.

use gpui::{
    Context, Entity, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled, div,
    prelude::FluentBuilder,
};
use gpui_metro::u;
use gpui_metro::{
    TextInput, TextStyle, Theme, text, text_box,
    theme::{SEMILIGHT, size},
};

use crate::{
    app::Catalog,
    pages::{Group, Page},
};

/// Pages whose name or group contains `query` (case-insensitive), by group.
fn matches(query: &str) -> Vec<(Group, Vec<Page>)> {
    let q = query.trim().to_lowercase();
    Group::ALL
        .into_iter()
        .filter_map(|g| {
            let pages: Vec<Page> = Page::ALL
                .into_iter()
                .filter(|p| p.group() == g && (q.is_empty() || p.name().contains(&q) || g.name().contains(&q)))
                .collect();
            (!pages.is_empty()).then_some((g, pages))
        })
        .collect()
}

pub fn render(
    selected: Page,
    query: &str,
    search: &Entity<TextInput>,
    cx: &mut Context<Catalog>,
) -> impl IntoElement + use<> {
    let p = Theme::palette(cx);
    let groups = matches(query);
    let empty = groups.is_empty();

    let sections = groups.into_iter().map(|(g, pages)| {
        div()
            .flex()
            .flex_col()
            .pb(u(14.))
            .child(div().pb(u(4.)).child(text(TextStyle::GroupHeader, g.name())))
            .children(pages.into_iter().map(|page| {
                let on = page == selected;
                div()
                    .id(page.name())
                    .py(u(3.))
                    .text_size(u(size::MEDIUM_LARGE))
                    .font_weight(SEMILIGHT)
                    .text_color(if on { p.accent } else { p.foreground })
                    .cursor_pointer()
                    .hover(move |s| s.text_color(if on { p.accent } else { p.subtle }))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.page = page;
                        cx.notify();
                    }))
                    .child(page.name())
            }))
    });

    div()
        .flex()
        .flex_col()
        .flex_none()
        .w(u(250.))
        .h_full()
        .border_r_1()
        .border_color(p.disabled.opacity(0.3))
        .child(div().px(u(20.)).pt(u(18.)).pb(u(12.)).child(text_box(search)))
        .child(
            div()
                .id("sidebar-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(u(20.))
                .pb(u(24.))
                .children(sections)
                .when(empty, |el| el.child(text(TextStyle::Subtle, "no components match"))),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_matches_page_and_group_names() {
        assert_eq!(matches("").iter().map(|(_, p)| p.len()).sum::<usize>(), Page::ALL.len());
        assert_eq!(matches("PIVOT"), vec![(Group::Navigation, vec![Page::Pivot])]);
        let progress = matches("progress");
        assert_eq!(progress.len(), 1);
        assert_eq!(progress[0].1.len(), 3);
        assert!(matches("zzz").is_empty());
    }
}
