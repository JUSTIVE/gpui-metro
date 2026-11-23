//! Metro motion.
//!
//! The phone's signature transitions are 3D: pages and tiles *turnstile*,
//! swinging about a vertical hinge on the screen's left edge. GPUI can't
//! transform boxes or text, so the turn is projected instead: a flat element
//! rotated by θ about the hinge, seen through a perspective camera, covers a
//! narrower span that hugs the hinge. [`turn`] computes that span; [`enter`]
//! clips the element to it and really shrinks its contents (rem scaling via
//! [`crate::Scaled`], text included), so a page reads as swinging in
//! edge-first rather than sliding or fading.
//!
//! Timings follow the Windows Phone Toolkit's `TurnstileTransition`:
//! forward-in swings from −80° in 350 ms on an exponential ease-out
//! (exponent 6); back-in from 50°; turnstile-out swings to 90° on an ease-in.

use std::time::Duration;

use gpui::{
    prelude::FluentBuilder,
    Animation, AnimationExt, App, ElementId, IntoElement, ParentElement, SharedString, Styled, Window, div,
    relative,
};

use crate::theme::u;

use crate::{Theme, chrome::PAGE_MARGIN, scaled::scaled};

/// Metro's curve: everything decelerates hard into place (exponential out).
pub fn ease_out_expo(t: f32) -> f32 {
    if t >= 1. { 1. } else { 1. - 2f32.powf(-10. * t) }
}

/// Silverlight's `ExponentialEase` (EaseOut, exponent 6), the curve of every
/// WP Toolkit transition.
pub fn ease_out_exp6(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    (1. - (-6. * t).exp()) / (1. - (-6f32).exp())
}

/// The ease-in counterpart, for elements leaving.
pub fn ease_in_exp6(t: f32) -> f32 {
    1. - ease_out_exp6(1. - t)
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 { 4. * t * t * t } else { 1. - (-2. * t + 2.).powi(3) / 2. }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// A turnstile entrance.
pub const ENTER: Duration = Duration::from_millis(350);
/// Per-item stagger in a turnstile feather.
pub const FEATHER: Duration = Duration::from_millis(40);
/// Where a forward entrance starts its swing (right edge toward the viewer).
pub const FORWARD_IN_DEG: f32 = -80.;
/// Where a back entrance starts its swing (right edge away from the viewer).
pub const BACKWARD_IN_DEG: f32 = 50.;
/// Camera distance for the projection, in px (Silverlight's `PlaneProjection`).
pub const PERSPECTIVE: f32 = 1000.;
/// Width assumed for page content when projecting it (a 480 px page minus margins).
pub const PAGE_CONTENT_WIDTH: f32 = 432.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Forward navigation: swings in from −80°.
    Forward,
    /// Back navigation: swings in from 50°.
    Backward,
    /// Drops in from above (toasts, message boxes, jump grid).
    Down,
}

fn reduced(cx: &App) -> bool {
    cx.try_global::<Theme>().map(|t| t.reduced_motion).unwrap_or(false)
}

/// A flat element's projection after turning about the hinge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turn {
    /// Projected left edge, measured from the hinge.
    pub left: f32,
    /// Projected width (0 once the element is edge-on or facing away).
    pub width: f32,
    /// Perspective scale at the left and right edges (vertical size factor).
    pub scale_left: f32,
    pub scale_right: f32,
}

/// Projects an element spanning `offset..offset + width` from the hinge,
/// turned by `degrees` (positive = right edge swings away from the viewer).
pub fn turn(offset: f32, width: f32, degrees: f32) -> Turn {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let project = |d: f32| {
        let k = PERSPECTIVE / (PERSPECTIVE + d * sin).max(1.);
        (d * cos * k, k)
    };
    let (x0, k0) = project(offset);
    let (x1, k1) = project(offset + width);
    Turn { left: x0, width: (x1 - x0).max(0.), scale_left: k0, scale_right: k1 }
}

/// Fraction of its width, and px offset of its left edge, that page content
/// (sitting `PAGE_MARGIN` from the hinge) shows at `degrees`.
fn page_turn(degrees: f32) -> (f32, f32) {
    let t = turn(PAGE_MARGIN, PAGE_CONTENT_WIDTH, degrees);
    ((t.width / PAGE_CONTENT_WIDTH).clamp(0., 1.), t.left - PAGE_MARGIN)
}

/// How much a turned page's contents shrink. A true projection squeezes
/// only horizontally; rem scaling is uniform, so the page is scaled partway
/// toward its projected width and the clip does the rest.
pub fn content_scale(width_fraction: f32) -> f32 {
    0.55 + 0.45 * width_fraction.clamp(0., 1.)
}

/// Turnstile-in: `child` swings in about the left edge, after
/// `index * FEATHER` (the feather: list rows follow one another). Re-plays
/// whenever `id` changes, so key it by what the content represents.
pub fn enter(
    id: impl Into<ElementId>,
    direction: Direction,
    index: usize,
    child: impl IntoElement,
    cx: &App,
) -> gpui::AnyElement {
    enter_div(id, direction, index, div().child(child), cx)
}

/// [`enter`] with a caller-styled wrapper (e.g. `div().size_full()`), for
/// content that must fill its parent.
pub fn enter_div(
    id: impl Into<ElementId>,
    direction: Direction,
    index: usize,
    mut wrapper: gpui::Div,
    cx: &App,
) -> gpui::AnyElement {
    if reduced(cx) {
        return wrapper.into_any_element();
    }
    let id = id.into();
    let delay = FEATHER * index as u32;
    let mut animations = Vec::with_capacity(2);
    if !delay.is_zero() {
        animations.push(Animation::new(delay));
    }
    let delayed = animations.len() == 1;
    animations.push(Animation::new(ENTER).with_easing(ease_out_exp6));
    let progress = move |phase: usize, t: f32| if delayed && phase == 0 { 0. } else { t };

    if direction == Direction::Down {
        return wrapper
            .relative()
            .with_animations(id, animations, move |el, phase, t| {
                let t = progress(phase, t);
                el.top(u(-24. * (1. - t))).opacity(t)
            })
            .into_any_element();
    }

    let from = if direction == Direction::Forward { FORWARD_IN_DEG } else { BACKWARD_IN_DEG };
    let fraction = move |phase, t| page_turn(lerp(from, 0., progress(phase, t)));

    // The clip keeps the wrapper's own sizing in its parent's layout…
    let mut clip = div().relative().overflow_hidden();
    let fills_height;
    {
        let (src, dst) = (wrapper.style(), clip.style());
        fills_height = src.size.height.is_some();
        dst.size.height = src.size.height;
        dst.min_size.height = src.min_size.height;
        dst.flex_grow = src.flex_grow;
        dst.flex_shrink = src.flex_shrink;
        dst.flex_basis = src.flex_basis;
    }
    // …and narrows toward the hinge. Inside it, the content renders at
    // rem scale `s` in a box `s` × its natural width, so it shrinks as a
    // whole and nothing reflows. Both layers share one timeline, so they
    // stay in step frame for frame.
    let content = scaled(1.).relative().w_full().when(fills_height, |el| el.h_full()).child(wrapper.size_full());
    let content = content.with_animations(id.clone(), animations.clone(), move |el, phase, t| {
        let (f, _) = fraction(phase, t);
        let s = content_scale(f);
        let el = el.scale(s).w(relative(s / f.max(0.02))).top(relative((1. - s) / 2.));
        if fills_height { el.h(relative(s)) } else { el }
    });
    clip.child(content)
        .with_animations(id, animations, move |el, phase, t| {
            let (f, offset) = fraction(phase, t);
            el.w(relative(f.max(0.02))).ml(u(offset)).opacity(if f < 0.04 { 0. } else { 1. })
        })
        .into_any_element()
}

/// Pivot-style entrance: the whole pane slides in from the side it came
/// from, without fading, decelerating hard (500 ms, exponent 6).
pub fn slide_in(id: impl Into<ElementId>, direction: Direction, wrapper: gpui::Div, cx: &App) -> gpui::AnyElement {
    if reduced(cx) || direction == Direction::Down {
        return wrapper.into_any_element();
    }
    let sign = if direction == Direction::Forward { 1. } else { -1. };
    wrapper
        .relative()
        .with_animation(id, Animation::new(Duration::from_millis(500)).with_easing(ease_out_exp6), move |el, t| {
            el.left(relative(sign * 0.6 * (1. - t)))
        })
        .into_any_element()
}

/// Remembers a value across frames and reports when it changes, so a
/// component can animate *from* the old value *to* the new one.
pub struct Transition<T> {
    pub current: T,
    pub previous: T,
    /// Bumps on every change. 0 = never changed (render at rest).
    pub generation: usize,
}

/// Tracks `value` under `key`. Returns `(previous, generation)`; when
/// `generation > 0`, key the animation by it so it restarts on each change.
pub fn track<T: Clone + PartialEq + 'static>(
    key: impl Into<ElementId>,
    value: T,
    window: &mut Window,
    cx: &mut App,
) -> (T, usize) {
    let initial = value.clone();
    let state = window.use_keyed_state(key, cx, |_, _| Transition {
        current: initial.clone(),
        previous: initial,
        generation: 0,
    });
    state.update(cx, |s, _| {
        if s.current != value {
            s.previous = std::mem::replace(&mut s.current, value);
            s.generation += 1;
        }
        (s.previous.clone(), s.generation)
    })
}

/// `"{base}-{generation}"` as an element id.
pub fn generation_id(base: &str, generation: usize) -> ElementId {
    ElementId::from((SharedString::from(base.to_string()), generation))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_hit_their_ends() {
        assert_eq!(ease_out_expo(0.), 0.);
        assert_eq!(ease_out_expo(1.), 1.);
        assert!(ease_out_expo(0.3) > 0.8, "metro decelerates hard");
        assert_eq!(ease_in_out_cubic(0.), 0.);
        assert_eq!(ease_in_out_cubic(1.), 1.);
        assert!((ease_in_out_cubic(0.5) - 0.5).abs() < 1e-6);
        assert!(ease_out_exp6(0.) == 0. && (ease_out_exp6(1.) - 1.).abs() < 1e-6);
        assert!(ease_in_exp6(0.2) < 0.1, "leaving starts slow");
    }

    #[test]
    fn a_flat_element_projects_to_itself() {
        let t = turn(24., 432., 0.);
        assert!((t.left - 24.).abs() < 1e-3 && (t.width - 432.).abs() < 1e-3);
        assert!((t.scale_left - 1.).abs() < 1e-6);
    }

    #[test]
    fn turning_narrows_toward_the_hinge() {
        let away = turn(24., 432., 50.);
        assert!(away.width < 432. * 0.6, "{away:?}");
        assert!(away.scale_right < away.scale_left, "the far edge recedes");
        let toward = turn(24., 432., -80.);
        assert!(toward.width < 432. * 0.5, "{toward:?}");
        assert!(toward.scale_right > 1., "the far edge comes toward the viewer");
        assert_eq!(turn(24., 432., 90.).width.round(), 0., "edge-on at 90°");
        // Widths grow monotonically as an entrance settles.
        let widths: Vec<f32> = (0..=10).map(|i| page_turn(lerp(FORWARD_IN_DEG, 0., i as f32 / 10.)).0).collect();
        assert!(widths.windows(2).all(|w| w[0] <= w[1] + 1e-6), "{widths:?}");
    }
}
