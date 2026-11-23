//! Progress: the flat accent bar, the five flowing dots of the indeterminate
//! bar, and the orbiting dots of the progress ring.

use std::f32::consts::TAU;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
    relative,
};

use crate::theme::u;

use crate::{Theme, motion::ease_in_out_cubic};

/// `None` = indeterminate.
#[derive(IntoElement)]
pub struct ProgressBar {
    id: ElementId,
    value: Option<f32>,
}

/// Determinate bar at `value` (0–1).
pub fn progress_bar(id: impl Into<ElementId>, value: f32) -> ProgressBar {
    ProgressBar { id: id.into(), value: Some(value.clamp(0., 1.)) }
}

/// The indeterminate dots.
pub fn progress_dots(id: impl Into<ElementId>) -> ProgressBar {
    ProgressBar { id: id.into(), value: None }
}

const DOTS: usize = 5;
/// One pass of the dots across the bar.
const CYCLE: Duration = Duration::from_millis(3200);

/// Where dot `i` sits (0–1 across, may be outside) at cycle time `t`. The dots
/// rush in, crowd and slow through the middle, then rush out.
pub fn dot_position(i: usize, t: f32) -> Option<f32> {
    let local = t - i as f32 * 0.055;
    if !(0. ..=0.72).contains(&local) {
        return None;
    }
    let u = local / 0.72;
    // A cubic with a flat middle: fast–slow–fast.
    let x = if u < 0.3 {
        0.38 * ease_out(u / 0.3)
    } else if u < 0.7 {
        0.38 + 0.24 * ((u - 0.3) / 0.4)
    } else {
        0.62 + 0.45 * ease_in((u - 0.7) / 0.3)
    };
    Some(x)
}

fn ease_out(t: f32) -> f32 {
    1. - (1. - t).powi(2)
}
fn ease_in(t: f32) -> f32 {
    t * t
}

impl RenderOnce for ProgressBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        match self.value {
            Some(v) => div()
                .relative()
                .w_full()
                .h(u(4.))
                .bg(p.accent.opacity(0.25))
                .child(div().absolute().left_0().top_0().bottom_0().w(relative(v)).bg(p.accent))
                .into_any_element(),
            None => {
                let accent = p.accent;
                div()
                    .relative()
                    .w_full()
                    .h(u(4.))
                    .overflow_hidden()
                    .with_animation(self.id, Animation::new(CYCLE).repeat(), move |el, t| {
                        let mut el = el;
                        for i in 0..DOTS {
                            if let Some(x) = dot_position(i, t) {
                                el = el.child(
                                    div().absolute().top_0().left(relative(x)).size(u(4.)).bg(accent),
                                );
                            }
                        }
                        el
                    })
                    .into_any_element()
            }
        }
    }
}

/// Dots chasing each other round a circle.
#[derive(IntoElement)]
pub struct ProgressRing {
    id: ElementId,
    diameter: f32,
}

pub fn progress_ring(id: impl Into<ElementId>) -> ProgressRing {
    ProgressRing { id: id.into(), diameter: 48. }
}

impl ProgressRing {
    pub fn diameter(mut self, d: f32) -> Self {
        self.diameter = d;
        self
    }
}

impl RenderOnce for ProgressRing {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let accent = Theme::palette(cx).accent;
        let d = self.diameter;
        let dot = (d / 10.).max(3.);
        let r = (d - dot) / 2.;
        div()
            .relative()
            .flex_none()
            .size(u(d))
            .with_animation(self.id, Animation::new(Duration::from_millis(2400)).repeat(), move |el, t| {
                let mut el = el;
                for i in 0..6 {
                    let local = (t - i as f32 * 0.04).rem_euclid(1.);
                    // Two laps per cycle, each eased so the dots bunch at the top.
                    let lap = (local * 2.).fract();
                    let angle = ease_in_out_cubic(lap) * TAU - TAU / 4.;
                    let (x, y) = (r + r * angle.cos(), r + r * angle.sin());
                    el = el.child(
                        div().absolute().left(u(x)).top(u(y)).size(u(dot)).rounded_full().bg(accent),
                    );
                }
                el
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dots_cross_left_to_right_in_order() {
        // At any instant, an earlier dot is never behind a later one.
        for step in 0..100 {
            let t = step as f32 / 100.;
            let xs: Vec<f32> = (0..DOTS).filter_map(|i| dot_position(i, t)).collect();
            assert!(xs.windows(2).all(|w| w[0] >= w[1]), "t={t}: {xs:?}");
        }
        assert_eq!(dot_position(0, 0.), Some(0.));
        assert!(dot_position(0, 0.72).unwrap() > 1.0, "dots leave the bar");
        assert_eq!(dot_position(0, 0.9), None);
    }
}
