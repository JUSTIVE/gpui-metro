//! Accents and palette tokens, the type ramp, the icon set.

use gpui::{
    AnyElement, App, Hsla, InteractiveElement, ParentElement, SharedString, StatefulInteractiveElement, Styled, div,
    prelude::FluentBuilder,
};
use gpui_metro::u;
use gpui_metro::{Accent, Icon, TextStyle, Theme, text, theme::size};

use super::{caption, page, stack};

/// Four squares across the 432 px content column.
const CELL: f32 = 102.;

pub fn palette(cx: &App) -> AnyElement {
    let theme = Theme::get(cx);
    let (current, p) = (theme.accent, theme.palette);

    let swatches = Accent::ALL.into_iter().map(|a| {
        div()
            .id(a.name())
            .relative()
            .size(u(CELL))
            .bg(a.color())
            .cursor_pointer()
            .active(|s| s.opacity(0.8))
            .on_click(move |_, _, cx| Theme::set_accent(a, cx))
            .when(a == current, |el| {
                el.child(div().absolute().top(u(6.)).right(u(6.)).child(Icon::Check.render(u(22.), a.ink())))
            })
            .child(
                div()
                    .absolute()
                    .left(u(8.))
                    .bottom(u(6.))
                    .text_size(u(size::SMALL))
                    .text_color(a.ink())
                    .child(a.name()),
            )
    });

    let tokens: [(&str, Hsla); 12] = [
        ("background", p.background),
        ("foreground", p.foreground),
        ("subtle", p.subtle),
        ("disabled", p.disabled),
        ("chrome", p.chrome),
        ("border", p.border),
        ("input", p.input),
        ("input focused", p.input_focused),
        ("input ink", p.input_ink),
        ("scrim", p.scrim),
        ("accent", p.accent),
        ("accent ink", p.accent_ink),
    ];
    let rows = tokens.into_iter().map(|(name, color)| {
        div()
            .flex()
            .items_center()
            .gap(u(14.))
            .child(div().size(u(32.)).bg(color).border_1().border_color(p.subtle))
            .child(text(TextStyle::Normal, name))
    });

    page(
        "accents",
        stack()
            .child(caption(format!("{} on {} — tap a colour", current.name(), theme.mode.label())))
            .child(div().flex().flex_wrap().gap(u(8.)).children(swatches))
            .child(text(TextStyle::GroupHeader, "palette"))
            .child(div().flex().flex_col().gap(u(8.)).children(rows)),
    )
}

const STYLES: [(TextStyle, &str, &str); 11] = [
    (TextStyle::Title1, "title 1", "settings"),
    (TextStyle::Title2, "title 2", "wi-fi networks"),
    (TextStyle::Title3, "title 3", "recent conversations"),
    (TextStyle::ExtraLarge, "extra large", "kristin"),
    (TextStyle::Large, "large", "mobile 425 555 0102"),
    (TextStyle::Normal, "normal", "Metro is type-first: content over chrome."),
    (TextStyle::Subtle, "subtle", "last synced two minutes ago"),
    (TextStyle::Accent, "accent", "3 new messages"),
    (TextStyle::Small, "small", "captions, dates and small print"),
    (TextStyle::AppTitle, "app title", "outlook"),
    (TextStyle::GroupHeader, "group header", "favourites"),
];

pub fn typography(cx: &App) -> AnyElement {
    let font = Theme::font(cx);
    page(
        "typography",
        stack().child(caption(format!("set in {font}"))).children(STYLES.into_iter().map(|(style, name, sample)| {
            div().flex().flex_col().child(caption(format!("{name} · {} px", style.size()))).child(text(style, sample))
        })),
    )
}

pub fn icons(cx: &App) -> AnyElement {
    let p = Theme::palette(cx);
    let cells = Icon::ALL.iter().map(|&i| {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(u(8.))
            .w(u(CELL))
            .h(u(92.))
            .child(i.render(u(36.), p.foreground))
            .child(
                div()
                    .text_size(u(size::SMALL - 1.))
                    .text_color(p.subtle)
                    .child(SharedString::from(i.name().replace('_', " "))),
            )
    });
    page(
        "icons",
        stack()
            .child(caption(format!("{} glyphs, tinted by text colour", Icon::ALL.len())))
            .child(div().flex().flex_wrap().gap(u(8.)).children(cells)),
    )
}
