//! Buttons. The Metro button is a hard 3 px outline with no fill; pressing it
//! floods it with the accent. The round button is the app-bar glyph in a ring.

use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::{Icon, Theme, theme::size};

pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The standard button.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    icon: Option<Icon>,
    on_click: Option<ClickHandler>,
    disabled: bool,
    accent: bool,
    full_width: bool,
}

pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    Button {
        id: id.into(),
        label: label.into(),
        icon: None,
        on_click: None,
        disabled: false,
        accent: false,
        full_width: false,
    }
}

impl Button {
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    /// Filled with the accent at rest — the default action of a form.
    pub fn accent(mut self) -> Self {
        self.accent = true;
        self
    }
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let (fg, border) = if self.disabled { (p.disabled, p.disabled) } else { (p.foreground, p.foreground) };
        let disabled = self.disabled;
        let accent = self.accent && !disabled;
        div()
            .id(self.id)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .min_w(px(108.))
            .h(px(52.))
            .px(px(20.))
            .border(px(3.))
            .border_color(if accent { p.accent } else { border })
            .when(accent, |el| el.bg(p.accent))
            .font_family(Theme::font(cx))
            .text_size(px(size::MEDIUM))
            .text_color(if accent { p.accent_ink } else { fg })
            .when(self.full_width, |el| el.w_full())
            .when(!disabled, |el| {
                el.cursor_pointer()
                    .active(move |s| s.bg(p.accent).border_color(p.accent).text_color(p.accent_ink))
            })
            .when_some(self.icon, |el, icon| {
                el.child(icon.render(px(20.), if accent { p.accent_ink } else { fg }))
            })
            .child(self.label)
            .when_some(self.on_click.filter(|_| !disabled), |el, f| el.on_click(move |e, w, cx| f(e, w, cx)))
    }
}

/// A glyph in a 2 px ring — the app bar button, also used standalone.
#[derive(IntoElement)]
pub struct RoundButton {
    id: ElementId,
    icon: Icon,
    diameter: f32,
    on_click: Option<ClickHandler>,
    disabled: bool,
    filled: bool,
}

pub fn round_button(id: impl Into<ElementId>, icon: Icon) -> RoundButton {
    RoundButton { id: id.into(), icon, diameter: 48., on_click: None, disabled: false, filled: false }
}

impl RoundButton {
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
    pub fn diameter(mut self, d: f32) -> Self {
        self.diameter = d;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    /// A filled (foreground) disc with the glyph knocked out — a toggled-on state.
    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }
}

impl RenderOnce for RoundButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let d = self.diameter;
        let color = if self.disabled { p.disabled } else { p.foreground };
        let (bg, glyph) = if self.filled { (Some(color), p.background) } else { (None, color) };
        let disabled = self.disabled;
        let accent = p.accent;
        div()
            .id(self.id)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(d))
            .rounded_full()
            .border_2()
            .border_color(color)
            .when_some(bg, |el, bg| el.bg(bg))
            .when(!disabled, |el| el.cursor_pointer().active(move |s| s.bg(accent).border_color(accent)))
            .child(self.icon.render(px(d * 0.5), glyph))
            .when_some(self.on_click.filter(|_| !disabled), |el, f| el.on_click(move |e, w, cx| f(e, w, cx)))
    }
}

/// Accent-free underlined-on-press text link.
#[derive(IntoElement)]
pub struct HyperlinkButton {
    id: ElementId,
    label: SharedString,
    on_click: Option<ClickHandler>,
}

pub fn hyperlink(id: impl Into<ElementId>, label: impl Into<SharedString>) -> HyperlinkButton {
    HyperlinkButton { id: id.into(), label: label.into(), on_click: None }
}

impl HyperlinkButton {
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for HyperlinkButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        div()
            .id(self.id)
            .font_family(Theme::font(cx))
            .text_size(px(size::MEDIUM))
            .text_color(p.foreground)
            .underline()
            .cursor_pointer()
            .active(move |s| s.text_color(p.accent))
            .child(self.label)
            .when_some(self.on_click, |el, f| el.on_click(move |e, w, cx| f(e, w, cx)))
    }
}
