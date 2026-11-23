//! Phone chrome: the system tray along the top edge and the two-line page
//! header (APPLICATION NAME over a huge lower-case page title).

use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, prelude::FluentBuilder};

use crate::theme::u;

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
            .pt(u(12.))
            .pb(u(20.))
            .px(u(PAGE_MARGIN))
            .when_some(self.app_title, |el, t| el.child(text(TextStyle::AppTitle, t)))
            .child(div().ml(u(-3.)).child(text(TextStyle::Title1, self.title).lowercase()))
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
            .h(u(32.))
            .px(u(12.))
            .when(!self.transparent, |el| el.bg(p.chrome))
            .child(
                div()
                    .flex()
                    .gap(u(6.))
                    .items_center()
                    .child(Icon::Signal.render(u(16.), fg))
                    .child(Icon::Wifi.render(u(16.), fg)),
            )
            .child(
                div()
                    .flex()
                    .gap(u(10.))
                    .items_center()
                    .child(
                        // Battery glyph drawn as boxes so the fill tracks the level.
                        div()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .w(u(22.))
                                    .h(u(11.))
                                    .border_1()
                                    .border_color(fg)
                                    .p(u(1.5))
                                    .child(div().h_full().w(gpui::relative(self.battery)).bg(fg)),
                            )
                            .child(div().w(u(2.)).h(u(5.)).bg(fg)),
                    )
                    .child(text(TextStyle::Small, self.time).color(fg)),
            )
    }
}
