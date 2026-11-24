//! Progress: the flat accent bar, the five flowing dots of the indeterminate
//! bar, and the orbiting dots of the progress ring.

use std::time::{Duration, Instant};

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, relative};

use crate::theme::u;

use crate::{
    Theme,
    motion::request_ambient_frame,
};

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

/// Seconds since this indicator first appeared, on its own clock, with the
/// next frame booked on the shared ambient clock: an indicator spins for as
/// long as it is shown, so it runs at [`crate::motion::AMBIENT_FPS`], not every refresh.
fn spin_clock(id: &ElementId, window: &mut Window, cx: &mut App) -> f32 {
    let key = ElementId::from((SharedString::from(format!("{id}-spin")), 0usize));
    let started = *window.use_keyed_state(key, cx, |_, _| Instant::now()).read(cx);
    request_ambient_frame(window, cx);
    started.elapsed().as_secs_f32()
}

impl RenderOnce for ProgressBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
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
                let t = (spin_clock(&self.id, window, cx) / CYCLE.as_secs_f32()).fract();
                div()
                    .relative()
                    .w_full()
                    .h(u(4.))
                    .overflow_hidden()
                    .children((0..DOTS).filter_map(|i| dot_position(i, t)).map(|x| {
                        div().absolute().top_0().left(relative(x)).size(u(4.)).bg(accent)
                    }))
                    .into_any_element()
            }
        }
    }
}

/// The Windows 8 progress ring: five dots that set off one after another
/// from the lower left, rush round, crowd together near the top, lap again
/// and vanish, then start over.
#[derive(IntoElement)]
pub struct ProgressRing {
    id: ElementId,
    diameter: f32,
}

pub fn progress_ring(id: impl Into<ElementId>) -> ProgressRing {
    ProgressRing { id: id.into(), diameter: 40. }
}

impl ProgressRing {
    pub fn diameter(mut self, d: f32) -> Self {
        self.diameter = d;
        self
    }
}

/// Dots in the ring.
pub const RING_DOTS: usize = 5;
/// One full cycle of a dot.
const RING_CYCLE: f32 = 5.5;
/// Each dot sets off this long after the one before it.
const RING_STAGGER: f32 = 0.24;

/// The ring's keyframes (WinJS `win-progress-ring`): (cycle fraction,
/// clockwise degrees from twelve o'clock, easing into the next keyframe).
const RING_KEYS: [(f32, f32, Ease); 6] = [
    (0.00, 225., Ease::Out),
    (0.07, 345., Ease::Linear),
    (0.30, 455., Ease::InOut),
    (0.39, 690., Ease::Linear),
    (0.70, 815., Ease::Out),
    (0.75, 945., Ease::Linear),
];

#[derive(Clone, Copy)]
enum Ease {
    Linear,
    Out,
    InOut,
}

impl Ease {
    fn apply(self, t: f32) -> f32 {
        match self {
            Ease::Linear => t,
            Ease::Out => 1. - (1. - t) * (1. - t),
            Ease::InOut => t * t * (3. - 2. * t),
        }
    }
}

/// Where one dot is at cycle fraction `t` (0–1, its own clock): its angle
/// in degrees clockwise from the top, or `None` while it is hidden.
pub fn ring_dot(t: f32) -> Option<f32> {
    let t = t.rem_euclid(1.);
    if t >= 0.76 {
        return None;
    }
    let (end, end_deg, _) = RING_KEYS[RING_KEYS.len() - 1];
    if t >= end {
        return Some(end_deg);
    }
    let i = RING_KEYS.iter().rposition(|k| k.0 <= t).unwrap_or(0);
    let (t0, a0, ease) = RING_KEYS[i];
    let (t1, a1, _) = RING_KEYS[i + 1];
    Some(a0 + (a1 - a0) * ease.apply((t - t0) / (t1 - t0)))
}

impl RenderOnce for ProgressRing {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let accent = Theme::palette(cx).accent;
        let d = self.diameter;
        let dot = (d / 8.).max(3.);
        let r = (d - dot) / 2.;
        // One animation over the whole staggered cycle; each dot reads its own clock.
        let total = RING_CYCLE + RING_STAGGER * (RING_DOTS - 1) as f32;
        let now = spin_clock(&self.id, window, cx) % total;
        div().relative().flex_none().size(u(d)).children((0..RING_DOTS).filter_map(|i| {
            let local = now - i as f32 * RING_STAGGER;
            if !(0. ..RING_CYCLE).contains(&local) {
                return None;
            }
            let a = ring_dot(local / RING_CYCLE)?.to_radians();
            let (x, y) = (r + r * a.sin(), r - r * a.cos());
            Some(div().absolute().left(u(x)).top(u(y)).size(u(dot)).rounded_full().bg(accent))
        }))
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

    #[test]
    fn ring_dots_only_ever_go_clockwise_then_vanish() {
        let angles: Vec<f32> = (0..76).filter_map(|i| ring_dot(i as f32 / 100.)).collect();
        assert_eq!(angles.len(), 76);
        assert!(angles.windows(2).all(|w| w[1] >= w[0] - 1e-3), "{angles:?}");
        assert_eq!(ring_dot(0.), Some(225.), "sets off from the lower left");
        assert!((ring_dot(0.755).unwrap() - 945.).abs() < 1e-3, "two laps and a bit");
        assert_eq!(ring_dot(0.8), None, "hidden until the next cycle");
    }
}
