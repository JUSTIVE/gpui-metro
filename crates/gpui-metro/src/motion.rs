//! Metro motion. The phone's signature transitions were 3D (turnstile,
//! swivel); GPUI has no 3D transform for boxes, so they are rendered as
//! their 2D silhouettes: a short horizontal travel with a fast-out curve,
//! staggered per element ("feather"), and a fade.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, App, ElementId, IntoElement, ParentElement, SharedString, Styled, Window,
    div, px,
};

use crate::Theme;

/// Metro's curve: everything decelerates hard into place (exponential out).
pub fn ease_out_expo(t: f32) -> f32 {
    if t >= 1. { 1. } else { 1. - 2f32.powf(-10. * t) }
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 { 4. * t * t * t } else { 1. - (-2. * t + 2.).powi(3) / 2. }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The standard entrance: 350 ms, 40 px of travel.
pub const ENTER: Duration = Duration::from_millis(350);
/// Per-item stagger in a turnstile feather.
pub const FEATHER: Duration = Duration::from_millis(45);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Content arrives from the right (forward navigation).
    Forward,
    /// Content arrives from the left (back navigation).
    Backward,
    /// Content drops in from above (toasts, message boxes).
    Down,
}

fn reduced(cx: &App) -> bool {
    cx.try_global::<Theme>().map(|t| t.reduced_motion).unwrap_or(false)
}

/// Turnstile-in: slides `child` in from `direction` while fading up, after
/// `index * FEATHER` of delay. Re-plays whenever `id` changes, so key it by
/// whatever the content represents (page, pivot item, …).
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
    wrapper: gpui::Div,
    cx: &App,
) -> gpui::AnyElement {
    let wrapper = wrapper.relative();
    if reduced(cx) {
        return wrapper.into_any_element();
    }
    let delay = FEATHER * index as u32;
    let mut animations = Vec::with_capacity(2);
    if !delay.is_zero() {
        animations.push(Animation::new(delay));
    }
    let has_delay = animations.len() == 1;
    animations.push(Animation::new(ENTER).with_easing(ease_out_expo));
    wrapper
        .with_animations(id, animations, move |el, ix, t| {
            let t = if has_delay && ix == 0 { 0. } else { t };
            let travel = 40. * (1. - t);
            let el = el.opacity(t);
            match direction {
                Direction::Forward => el.left(px(travel)),
                Direction::Backward => el.left(px(-travel)),
                Direction::Down => el.top(px(-travel * 0.6)),
            }
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
    }
}
