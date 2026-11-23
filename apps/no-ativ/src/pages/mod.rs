//! One module per destination. Every page is a function of the store and its
//! own back-stack entry, returning a [`PageView`].

mod deck;
mod freezer;
mod incidents;
mod people;
mod reagents;
mod runs;
mod settings;
mod shipments;
mod start;
mod strains;
mod telemetry;

use gpui::{App, Div, Hsla, SharedString, Stateful, div, prelude::*};
use gpui_metro::u;
use gpui_metro::{Accent, PAGE_MARGIN, TextStyle, page_header, text};

use crate::nav::{Nav, Page, PageView};
use crate::store::Store;

/// Application title above every page title.
pub const APP: &str = "no ativ";

pub fn render(page: &Page, store: &Store, nav: &Nav, cx: &App) -> PageView {
    match page {
        Page::Start => start::render(store, nav, cx),
        Page::Runs { pivot } => runs::list(*pivot, store, nav, cx),
        Page::Run { id } => runs::detail(id, store, nav, cx),
        Page::Telemetry { zone } => telemetry::render(*zone, store, nav, cx),
        Page::Strains => strains::list(store, nav, cx),
        Page::Strain { id } => strains::detail(id, store, nav, cx),
        Page::Incidents { pivot } => incidents::list(*pivot, store, nav, cx),
        Page::Incident { id } => incidents::detail(id, store, nav, cx),
        Page::NewIncident(form) => incidents::form(form, store, nav, cx),
        Page::Reagents { pivot } => reagents::render(*pivot, store, nav, cx),
        Page::Shipments { pivot } => shipments::render(*pivot, store, nav, cx),
        Page::Freezer => freezer::render(store, nav, cx),
        Page::Deck { pivot } => deck::render(*pivot, store, nav, cx),
        Page::People { pivot } => people::render(*pivot, store, nav, cx),
        Page::Settings => settings::render(store, nav, cx),
    }
}

/// Warning tone: amber, whatever the accent.
pub fn warn() -> Hsla {
    Accent::Amber.color()
}

/// Alarm tone: red, whatever the accent.
pub fn alarm() -> Hsla {
    Accent::Red.color()
}

/// A vertically scrolling page body with the standard margins.
pub fn scroll(id: impl Into<SharedString>) -> Stateful<Div> {
    div()
        .id(id.into())
        .size_full()
        .overflow_y_scroll()
        .px(u(PAGE_MARGIN))
        .pb(u(32.))
        .flex()
        .flex_col()
}

/// A page with the two-line header over a scrolling body.
pub fn titled(section: &str, title: impl Into<SharedString>, body: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .size_full()
        .child(page_header(title).app_title(format!("{APP} · {section}")))
        .child(div().flex_1().min_h_0().child(body))
}

/// A labelled value: subtle caption over the value.
pub fn fact(label: &'static str, value: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .flex_col()
        .pb(u(10.))
        .child(text(TextStyle::Small, label))
        .child(text(TextStyle::Large, value))
}

/// A group heading inside a page body.
pub fn heading(label: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .pt(u(18.))
        .pb(u(6.))
        .child(TextStyle::GroupHeader.styled(cx).child(label.into()))
}

/// A subtle line of text.
pub fn subtle(s: impl Into<SharedString>, cx: &App) -> Div {
    TextStyle::Subtle.styled(cx).child(s.into())
}

/// What a detail page shows when its record has gone (e.g. a stale link).
pub fn missing(section: &str, what: &str, cx: &App) -> PageView {
    PageView::new(titled(
        section,
        "not found",
        scroll("missing").child(subtle(format!("No {what} with that id."), cx)),
    ))
}
