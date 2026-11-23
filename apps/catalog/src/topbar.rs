//! The catalog's top bar: name, ground toggle, accent swatches, reduced motion.

use gpui::{
    App, Context, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled,
    Subscription, Window, div, prelude::FluentBuilder,
};
use gpui_metro::u;
use gpui_metro::{
    Accent, Icon, Mode, TextStyle, Theme, check_box, text,
    theme::{LIGHT, SEMIBOLD, size},
};

/// The top bar as its own view, embedded cached: it changes only with the
/// theme, so it isn't laid out again on every animation frame.
pub struct TopBar {
    _theme: Subscription,
}

impl TopBar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self { _theme: cx.observe_global::<Theme>(|_, cx| cx.notify()) }
    }
}

/// Height of the bar, in design px.
pub const HEIGHT: f32 = 64.;

impl Render for TopBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render(cx)
    }
}

fn render(cx: &mut App) -> impl IntoElement + use<> {
    let theme = Theme::get(cx);
    let (mode, accent, reduced) = (theme.mode, theme.accent, theme.reduced_motion);
    let p = theme.palette;

    let ground = |m: Mode| {
        let on = m == mode;
        div()
            .id(m.label())
            .px(u(6.))
            .text_size(u(size::MEDIUM))
            .text_color(if on { p.foreground } else { p.subtle })
            .when(on, |el| el.font_weight(SEMIBOLD))
            .cursor_pointer()
            .hover(move |s| s.text_color(p.accent))
            .on_click(move |_, _, cx: &mut App| Theme::set_mode(m, cx))
            .child(m.label())
    };

    let swatches = Accent::ALL.into_iter().map(|a| {
        let on = a == accent;
        div()
            .id(a.name())
            .size(u(22.))
            .flex()
            .items_center()
            .justify_center()
            .bg(a.color())
            .cursor_pointer()
            .when(on, |el| el.border_2().border_color(p.foreground))
            .hover(|s| s.opacity(0.8))
            .on_click(move |_, _, cx: &mut App| Theme::set_accent(a, cx))
            .when(on, |el| el.child(Icon::Check.render(u(14.), a.ink())))
    });

    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(u(28.))
        .size_full()
        .px(u(24.))
        .border_b_1()
        .border_color(p.disabled.opacity(0.3))
        .child(
            div()
                .flex()
                .items_baseline()
                .gap(u(10.))
                .child(div().text_size(u(size::EXTRA_LARGE)).font_weight(LIGHT).child("metro"))
                .child(text(TextStyle::Subtle, "gpui component catalog")),
        )
        .child(div().flex_1())
        .child(
            div()
                .flex()
                .items_center()
                .child(ground(Mode::Dark))
                .child(div().text_color(p.disabled).child("/"))
                .child(ground(Mode::Light)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(u(10.))
                .child(text(TextStyle::Small, accent.name()))
                .child(div().flex().gap(u(3.)).children(swatches)),
        )
        .child(
            check_box("reduced-motion", reduced)
                .label("reduced motion")
                .on_change(|on, _, cx| Theme::set_reduced_motion(on, cx)),
        )
}
