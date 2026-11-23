//! The device the demos run in: a 480×800 screen in a bezel, with the
//! system tray on top and the three hardware buttons below.

use gpui::{App, IntoElement, ParentElement, Styled, div};
use gpui_metro::u;
use gpui_metro::{Icon, Theme, system_tray};

use crate::demos::{Demo, DemoState};

pub const SCREEN_W: f32 = 480.;
const SCREEN_H: f32 = 800.;

pub fn frame(state: &DemoState, demo: Demo, cx: &App) -> impl IntoElement {
    let p = Theme::palette(cx);
    let mut tray = system_tray("12:38").battery(state.battery);
    if state.tray_opaque {
        tray = tray.opaque();
    }

    // Overlays are children of the screen, so they cover the phone only.
    let screen = div()
        .relative()
        .overflow_hidden()
        .flex()
        .flex_col()
        .flex_none()
        .w(u(SCREEN_W))
        .h(u(SCREEN_H))
        .bg(p.background)
        .text_color(p.foreground)
        .font_family(Theme::font(cx))
        .child(tray)
        .child(div().relative().flex_1().min_h_0().child(demo.body))
        .children(demo.overlay);

    let hardware = div()
        .flex()
        .justify_around()
        .items_center()
        .h(u(56.))
        .children([Icon::Back, Icon::Home, Icon::Search].map(|i| i.render(u(22.), p.subtle)));

    div()
        .flex()
        .flex_col()
        .flex_none()
        .px(u(14.))
        .pt(u(30.))
        .rounded(u(34.))
        .bg(p.chrome)
        .border_1()
        .border_color(p.disabled.opacity(0.5))
        .child(screen)
        .child(hardware)
}
