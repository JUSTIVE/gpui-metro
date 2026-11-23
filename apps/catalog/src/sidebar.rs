//! The component index: a search box over the pages grouped by section.

use gpui::{
    Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window, div, prelude::FluentBuilder,
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

/// The sidebar as its own view, so the catalog can embed it cached: a page
/// turning in the phone redraws every frame, and the index needn't be laid
/// out again each time. It re-renders only when told the page or query
/// changed ([`Sidebar::show`]) or the theme changes.
pub struct Sidebar {
    selected: Page,
    query: SharedString,
    search: Entity<TextInput>,
    catalog: WeakEntity<Catalog>,
    _theme: Subscription,
}

impl Sidebar {
    pub fn new(selected: Page, search: Entity<TextInput>, catalog: WeakEntity<Catalog>, cx: &mut Context<Self>) -> Self {
        let theme = cx.observe_global::<Theme>(|_, cx| cx.notify());
        Self { selected, query: SharedString::default(), search, catalog, _theme: theme }
    }

    /// Updates what the index shows; redraws only if it changed.
    pub fn show(&mut self, selected: Page, query: &SharedString, cx: &mut Context<Self>) {
        if self.selected != selected || &self.query != query {
            self.selected = selected;
            self.query = query.clone();
            cx.notify();
        }
    }
}

/// Width of the sidebar, in design px.
pub const WIDTH: f32 = 250.;

impl Render for Sidebar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Theme::palette(cx);
        let selected = self.selected;
        let groups = matches(&self.query);
        let empty = groups.is_empty();

        let sections = groups.into_iter().map(|(g, pages)| {
            div()
                .flex()
                .flex_col()
                .pb(u(14.))
                .child(div().pb(u(4.)).child(text(TextStyle::GroupHeader, g.name())))
                .children(pages.into_iter().map(|page| {
                    let on = page == selected;
                    let catalog = self.catalog.clone();
                    div()
                        .id(page.name())
                        .py(u(3.))
                        .text_size(u(size::MEDIUM_LARGE))
                        .font_weight(SEMILIGHT)
                        .text_color(if on { p.accent } else { p.foreground })
                        .cursor_pointer()
                        .hover(move |s| s.text_color(if on { p.accent } else { p.subtle }))
                        .on_click(move |_, _, cx| {
                            catalog.update(cx, |c, cx| c.go(page, cx)).ok();
                        })
                        .child(page.name())
                }))
        });

        div()
            .flex()
            .flex_col()
            .size_full()
            .border_r_1()
            .border_color(p.disabled.opacity(0.3))
            .child(div().px(u(20.)).pt(u(18.)).pb(u(12.)).child(text_box(&self.search)))
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
