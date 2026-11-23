//! Application bar: up to four round glyph buttons along the bottom edge,
//! the `···` that expands it, and lower-case menu items revealed above.
//! Expansion is the bar's own state; buttons and items collapse it.

use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};

use crate::{
    Icon, Theme,
    button::{ClickHandler, round_button},
    motion::ease_out_expo,
    theme::size,
};

struct BarButton {
    icon: Icon,
    label: SharedString,
    disabled: bool,
    on_click: Option<ClickHandler>,
}

struct MenuItem {
    label: SharedString,
    on_click: Option<ClickHandler>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppBarMode {
    /// Buttons visible.
    Default,
    /// Only the `···` strip until expanded.
    Minimized,
}

#[derive(IntoElement)]
pub struct AppBar {
    id: ElementId,
    buttons: Vec<BarButton>,
    items: Vec<MenuItem>,
    mode: AppBarMode,
    opacity: f32,
}

pub fn app_bar(id: impl Into<ElementId>) -> AppBar {
    AppBar { id: id.into(), buttons: Vec::new(), items: Vec::new(), mode: AppBarMode::Default, opacity: 1. }
}

impl AppBar {
    /// A glyph button. The phone allowed four; extra ones are dropped.
    pub fn button(
        mut self,
        icon: Icon,
        label: impl Into<SharedString>,
        f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.buttons.push(BarButton { icon, label: label.into(), disabled: false, on_click: Some(Rc::new(f)) });
        self
    }
    pub fn disabled_button(mut self, icon: Icon, label: impl Into<SharedString>) -> Self {
        self.buttons.push(BarButton { icon, label: label.into(), disabled: true, on_click: None });
        self
    }
    pub fn menu_item(
        mut self,
        label: impl Into<SharedString>,
        f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(MenuItem { label: label.into(), on_click: Some(Rc::new(f)) });
        self
    }
    pub fn mode(mut self, mode: AppBarMode) -> Self {
        self.mode = mode;
        self
    }
    /// Bar background opacity (WP apps often used 0.7 over imagery).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0., 1.);
        self
    }
}

pub const MAX_BUTTONS: usize = 4;

impl RenderOnce for AppBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let reduced = Theme::get(cx).reduced_motion;
        let open = window.use_keyed_state(self.id.clone(), cx, |_, _| false);
        let expanded = *open.read(cx);
        let minimized = self.mode == AppBarMode::Minimized && !expanded;

        let collapse = {
            let open = open.clone();
            move |cx: &mut App| open.update(cx, |o, cx| {
                if *o {
                    *o = false;
                    cx.notify();
                }
            })
        };

        let buttons = self.buttons.into_iter().take(MAX_BUTTONS).enumerate().map(|(ix, b)| {
            let collapse = collapse.clone();
            let mut rb = round_button(("appbar-btn", ix), b.icon).diameter(44.).disabled(b.disabled);
            if let Some(f) = b.on_click.clone() {
                rb = rb.on_click(move |e, w, cx| {
                    collapse(cx);
                    f(e, w, cx)
                });
            }
            div()
                .flex()
                .flex_col()
                .items_center()
                .w(px(76.))
                .gap(px(4.))
                .child(rb)
                .when(expanded, |el| {
                    el.child(
                        div()
                            .text_size(px(size::SMALL - 2.))
                            .text_color(if b.disabled { p.disabled } else { p.foreground })
                            .child(SharedString::from(b.label.to_lowercase())),
                    )
                })
        });

        let toggle = {
            let open = open.clone();
            div()
                .id("appbar-more")
                .absolute()
                .top_0()
                .right_0()
                .w(px(56.))
                .h(px(if minimized { 30. } else { 36. }))
                .flex()
                .justify_center()
                .pt(px(2.))
                .cursor_pointer()
                .child(Icon::More.render(px(26.), p.foreground))
                .on_click(move |_, _, cx| open.update(cx, |o, cx| {
                    *o = !*o;
                    cx.notify();
                }))
        };

        let has_items = !self.items.is_empty();
        let items = div().flex().flex_col().pt(px(10.)).pb(px(12.)).px(px(24.)).gap(px(14.)).children(
            self.items.into_iter().enumerate().map(|(ix, item)| {
                let collapse = collapse.clone();
                let accent = p.accent;
                div()
                    .id(("appbar-item", ix))
                    .text_size(px(size::MEDIUM_LARGE))
                    .text_color(p.foreground)
                    .cursor_pointer()
                    .active(move |s| s.text_color(accent))
                    .child(SharedString::from(item.label.to_lowercase()))
                    .when_some(item.on_click, |el, f| {
                        el.on_click(move |e, w, cx| {
                            collapse(cx);
                            f(e, w, cx)
                        })
                    })
            }),
        );

        let body = div()
            .flex()
            .flex_col()
            .when(!minimized, |el| {
                el.child(div().flex().justify_center().pt(px(8.)).pb(px(if expanded { 4. } else { 10. })).children(buttons))
            })
            .when(minimized, |el| el.h(px(30.)))
            .when(expanded && has_items, |el| el.child(items));
        let body = if expanded && !reduced {
            body.with_animation(
                "appbar-open",
                Animation::new(Duration::from_millis(260)).with_easing(ease_out_expo),
                |el, t| el.opacity(0.4 + 0.6 * t).mt(px(24. * (1. - t))),
            )
            .into_any_element()
        } else {
            body.into_any_element()
        };

        let close_out = collapse.clone();
        div()
            .id(self.id)
            .relative()
            .w_full()
            .flex_none()
            .bg(p.chrome.opacity(self.opacity))
            .font_family(Theme::font(cx))
            .occlude()
            .child(body)
            .child(toggle)
            .when(expanded, |el| el.on_mouse_down_out(move |_, _, cx| close_out(cx)))
    }
}
