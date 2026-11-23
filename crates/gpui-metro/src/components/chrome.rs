//! Phone chrome: the system tray along the top edge and the two-line page
//! header (APPLICATION NAME over a huge lower-case page title).

use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, prelude::FluentBuilder, px};

use crate::{Icon, Theme, TextStyle, text};

/// Left/right page margin of every Metro page.
pub const PAGE_MARGIN: f32 = 24.;

/// `APP NAME` over `page title`.
#[derive(IntoElement)]
pub struct PageHeader {
    app_title: Option<SharedString>,
    title: SharedString,
}

pub fn page_header(title: impl Into<SharedString>) -> PageHeader {
    PageHeader { app_title: None, title: title.into() }
}

impl PageHeader {
    pub fn app_title(mut self, app: impl Into<SharedString>) -> Self {
        self.app_title = Some(app.into());
        self
    }
}

impl RenderOnce for PageHeader {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .pt(px(12.))
            .pb(px(20.))
            .px(px(PAGE_MARGIN))
            .when_some(self.app_title, |el, t| el.child(text(TextStyle::AppTitle, t)))
            .child(div().ml(px(-3.)).child(text(TextStyle::Title1, self.title).lowercase()))
    }
}

/// The status strip: signal, wifi, battery and the clock.
#[derive(IntoElement)]
pub struct SystemTray {
    time: SharedString,
    battery: f32,
    transparent: bool,
}

pub fn system_tray(time: impl Into<SharedString>) -> SystemTray {
    SystemTray { time: time.into(), battery: 0.8, transparent: true }
}

impl SystemTray {
    /// Battery charge, 0–1.
    pub fn battery(mut self, level: f32) -> Self {
        self.battery = level.clamp(0., 1.);
        self
    }

    /// Paint the tray in the chrome colour instead of letting the page show through.
    pub fn opaque(mut self) -> Self {
        self.transparent = false;
        self
    }
}

impl RenderOnce for SystemTray {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let fg = p.foreground;
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .h(px(32.))
            .px(px(12.))
            .when(!self.transparent, |el| el.bg(p.chrome))
            .child(
                div()
                    .flex()
                    .gap(px(6.))
                    .items_center()
                    .child(Icon::Signal.render(px(16.), fg))
                    .child(Icon::Wifi.render(px(16.), fg)),
            )
            .child(
                div()
                    .flex()
                    .gap(px(10.))
                    .items_center()
                    .child(
                        // Battery glyph drawn as boxes so the fill tracks the level.
                        div()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .w(px(22.))
                                    .h(px(11.))
                                    .border_1()
                                    .border_color(fg)
                                    .p(px(1.5))
                                    .child(div().h_full().w(gpui::relative(self.battery)).bg(fg)),
                            )
                            .child(div().w(px(2.)).h(px(5.)).bg(fg)),
                    )
                    .child(text(TextStyle::Small, self.time).color(fg)),
            )
    }
}
