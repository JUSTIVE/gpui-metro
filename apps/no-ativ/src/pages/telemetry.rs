//! Telemetry: a panorama with one section per zone. Values tick live.

use gpui::{App, Div, div, prelude::*, px, relative};
use gpui_metro::{TextStyle, Theme, panorama, text, theme::size};

use crate::data::{Sensor, SensorState};
use crate::nav::{Nav, PageView};
use crate::pages::{alarm, subtle, warn};
use crate::store::{Store, reading};

fn sensor_card(sensor: &Sensor, cx: &App) -> Div {
    let p = Theme::palette(cx);
    let tone = match sensor.state {
        SensorState::Ok => p.foreground,
        SensorState::Warn => warn(),
        SensorState::Alarm => alarm(),
    };
    div()
        .w(relative(0.5))
        .flex()
        .flex_col()
        .pb(px(18.))
        .child(text(TextStyle::Small, sensor.kind.as_str()))
        .child(
            div()
                .flex()
                .items_baseline()
                .gap(px(6.))
                .child(
                    div()
                        .text_size(px(size::EXTRA_LARGE + 6.))
                        .font_weight(gpui_metro::theme::LIGHT)
                        .text_color(tone)
                        .child(reading(sensor)),
                )
                .child(subtle(sensor.unit.clone(), cx)),
        )
        .child(
            subtle(
                format!("{}–{} · {}", sensor.low, sensor.high, sensor.state.as_str()),
                cx,
            )
            .text_color(if sensor.state == SensorState::Ok {
                p.subtle
            } else {
                tone
            }),
        )
        .child(
            div()
                .text_size(px(size::SMALL - 2.))
                .text_color(p.disabled)
                .child(sensor.code.clone()),
        )
}

pub fn render(zone: usize, store: &Store, nav: &Nav, cx: &App) -> PageView {
    let mut zones: Vec<&str> = Vec::new();
    for s in &store.data.sensors {
        if !zones.contains(&s.zone.as_str()) {
            zones.push(&s.zone);
        }
    }
    let mut pano = panorama("telemetry", "telemetry", zone).on_select(nav.select());
    for (ix, name) in zones.iter().enumerate() {
        let sensors = store.data.sensors.iter().filter(|s| s.zone == *name);
        let content = div()
            .id(("zone", ix))
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_wrap()
            .content_start()
            .children(sensors.map(|s| sensor_card(s, cx)));
        pano = pano.section(name.to_lowercase(), content);
    }
    PageView::new(pano)
}
