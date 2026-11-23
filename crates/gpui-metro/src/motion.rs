//! Metro motion.
//!
//! The phone's signature transitions are 3D: pages and tiles *turnstile*,
//! swinging about a vertical hinge on the screen's left edge. GPUI can't
//! transform boxes or text, so the turn is projected instead: a flat element
//! rotated by θ about the hinge, seen through a perspective camera, covers a
//! narrower span that hugs the hinge. [`turn`] computes that span; [`enter`]
//! clips the element to it at full size, so a page reads as swinging in
//! edge-first rather than sliding, fading or zooming. Small rigid things
//! (tiles) also shrink with the perspective as they turn.
//!
//! Only Metro's own motions live here, after the Windows Phone Toolkit:
//!
//! * **Turnstile** ([`enter_div`]): pages swing about the left edge;
//!   in from edge-on (the Toolkit's −80° / 50°, opened up so nothing pops),
//!   350 ms, exponential ease-out
//!   (exponent 6). Tiles turnstile out to 90° on an ease-in.
//! * **Turnstile feather** ([`enter`] with an index): items swing in one
//!   after another, top first.
//! * **Swivel** ([`swivel_in`]): pop-ups (message box, toast) swing in about
//!   a horizontal axis through their middle.
//! * **Slide** ([`slide_in`]): a pivot pane slides in from the side.
//! * **Tilt** ([`tilt`]): a pressed element sinks under the pointer.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    prelude::FluentBuilder,
    Animation, AnimationExt, App, ElementId, IntoElement, ParentElement, SharedString, Styled, Window, div,
    relative,
};

use crate::theme::u;

use crate::{Theme, chrome::PAGE_MARGIN};

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
/// Where a forward entrance starts its swing: edge-on, right edge toward
/// the viewer. (The Toolkit used −80°, but through a perspective camera that
/// is already a third of the page wide, so the page popped in; from edge-on
/// it opens smoothly. The ease-out puts it past −80° within a few frames.)
pub const FORWARD_IN_DEG: f32 = -90.;
/// Where a back entrance starts its swing: edge-on, right edge away.
pub const BACKWARD_IN_DEG: f32 = 90.;
/// Camera distance for the projection, in px (Silverlight's `PlaneProjection`).
pub const PERSPECTIVE: f32 = 1000.;
/// Width assumed for page content when projecting it (a 480 px page minus margins).
pub const PAGE_CONTENT_WIDTH: f32 = 432.;

/// Which way navigation went; decides where a turnstile swings in from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Forward navigation: swings in with its right edge toward the viewer.
    Forward,
    /// Back navigation: swings in with its right edge away from the viewer.
    Backward,
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
/// `inset` px from the hinge and `width` wide shows at `degrees`.
fn page_turn(degrees: f32, (inset, width): (f32, f32)) -> (f32, f32) {
    let t = turn(inset, width, degrees);
    ((t.width / width.max(1.)).clamp(0., 1.), t.left - inset)
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
    wrapper: gpui::Div,
    cx: &App,
) -> gpui::AnyElement {
    let from = if direction == Direction::Forward { FORWARD_IN_DEG } else { BACKWARD_IN_DEG };
    let swing = Animation::new(ENTER).with_easing(ease_out_exp6);
    turn_div(id.into(), (from, 0.), FEATHER * index as u32, swing, false, wrapper, cx)
}

/// How long a page takes to turnstile out.
pub const EXIT: Duration = Duration::from_millis(250);
/// Where a forward exit swings to: edge-on, away from the viewer, so the page
/// is gone when it ends instead of vanishing half-way open.
pub const FORWARD_OUT_DEG: f32 = 90.;
/// Where a back exit swings to: edge-on, toward the viewer.
pub const BACKWARD_OUT_DEG: f32 = -90.;

/// Turnstile-out: the page swings away about the left edge in [`EXIT`] on
/// an ease-in, and is hidden once it has gone. Render it in place of the
/// old page for `EXIT`, then swap in the new one with [`enter_div`].
pub fn exit_div(id: impl Into<ElementId>, direction: Direction, wrapper: gpui::Div, cx: &App) -> gpui::AnyElement {
    let to = if direction == Direction::Forward { FORWARD_OUT_DEG } else { BACKWARD_OUT_DEG };
    let swing = Animation::new(EXIT).with_easing(ease_in_exp6);
    turn_div(id.into(), (0., to), Duration::ZERO, swing, true, wrapper, cx)
}

/// The turnstile itself: `wrapper` turns from `degrees.0` to `degrees.1`
/// after `delay`, invisible while it waits (and at the end, if `vanish`).
fn turn_div(
    id: ElementId,
    degrees: (f32, f32),
    delay: Duration,
    swing: Animation,
    vanish: bool,
    mut wrapper: gpui::Div,
    cx: &App,
) -> gpui::AnyElement {
    if reduced(cx) {
        return wrapper.into_any_element();
    }
    let mut animations = Vec::with_capacity(2);
    if !delay.is_zero() {
        animations.push(Animation::new(delay));
    }
    let delayed = animations.len() == 1;
    animations.push(swing);
    let progress = move |phase: usize, t: f32| if delayed && phase == 0 { 0. } else { t };
    // A feathered item is invisible until its turn comes; an exit once it's gone.
    let waiting = move |phase: usize, t: f32| (delayed && phase == 0) || (vanish && t >= 0.999);

    let metrics = cx.try_global::<Theme>().map_or((PAGE_MARGIN, PAGE_CONTENT_WIDTH), |t| t.page_metrics);
    let fraction = move |phase, t| page_turn(lerp(degrees.0, degrees.1, progress(phase, t)), metrics);

    // The clip keeps the wrapper's own sizing in its parent's layout…
    let mut clip = div().relative().overflow_hidden();
    let fills_height;
    // A wrapper with its own absolute width (a grid cell) narrows from that
    // width; anything else narrows from its parent's.
    let own_width = match wrapper.style().size.width {
        Some(gpui::Length::Definite(gpui::DefiniteLength::Absolute(a))) => Some(a),
        _ => None,
    };
    {
        let (src, dst) = (wrapper.style(), clip.style());
        fills_height = src.size.height.is_some();
        dst.size.height = src.size.height;
        dst.min_size.height = src.min_size.height;
        dst.flex_grow = src.flex_grow;
        dst.flex_shrink = src.flex_shrink;
        dst.flex_basis = src.flex_basis;
    }
    // …and narrows toward the hinge, while the content inside keeps its
    // full size (1 / fraction of the clip), so it is cut by the edge as it
    // swings rather than squeezed. It is deliberately not scaled: rem
    // scaling is uniform, and a uniformly shrinking page reads as a zoom,
    // not a turn. Both layers share one timeline, so they stay in step.
    let content = div().relative().when(fills_height, |el| el.h_full()).child(wrapper.size_full());
    let content = content.with_animations(id.clone(), animations.clone(), move |el, phase, t| {
        let (f, _) = fraction(phase, t);
        match own_width {
            Some(w) => el.w(w),
            None => el.w(relative(1. / f.max(0.02))),
        }
    });
    clip.child(content)
        .with_animations(id, animations, move |el, phase, t| {
            let (f, offset) = fraction(phase, t);
            let hidden = waiting(phase, t) || f < 0.04;
            let f = f.max(0.02);
            let el = match own_width {
                Some(gpui::AbsoluteLength::Pixels(w)) => el.w(w * f),
                Some(gpui::AbsoluteLength::Rems(r)) => el.w(gpui::rems(r.0 * f)),
                None => el.w(relative(f)),
            };
            el.ml(u(offset)).opacity(if hidden { 0. } else { 1. })
        })
        .into_any_element()
}

/// A page turnstile driven by the caller's clock instead of an animation
/// element, so the page keeps one element tree (and its scroll positions
/// and component state) from the moment it swings in until it swings out.
/// Start one, render with [`Turnstile::frame`] every frame while it runs, and
/// call `window.request_animation_frame()` until [`Turnstile::done`].
#[derive(Clone, Copy, Debug)]
pub struct Turnstile {
    start: std::time::Instant,
    degrees: (f32, f32),
    duration: Duration,
    entering: bool,
}

impl Turnstile {
    /// The page swinging in (350 ms, exponential ease-out).
    pub fn enter(direction: Direction) -> Self {
        let from = if direction == Direction::Forward { FORWARD_IN_DEG } else { BACKWARD_IN_DEG };
        Self { start: std::time::Instant::now(), degrees: (from, 0.), duration: ENTER, entering: true }
    }

    /// The page swinging out (250 ms, exponential ease-in).
    pub fn exit(direction: Direction) -> Self {
        let to = if direction == Direction::Forward { FORWARD_OUT_DEG } else { BACKWARD_OUT_DEG };
        Self { start: std::time::Instant::now(), degrees: (0., to), duration: EXIT, entering: false }
    }

    fn progress(&self) -> f32 {
        (self.start.elapsed().as_secs_f32() / self.duration.as_secs_f32()).clamp(0., 1.)
    }

    pub fn done(&self) -> bool {
        self.progress() >= 1.
    }

    pub fn is_exit(&self) -> bool {
        !self.entering
    }

    /// The angle now.
    pub fn degrees(&self) -> f32 {
        let t = self.progress();
        let e = if self.entering { ease_out_exp6(t) } else { ease_in_exp6(t) };
        lerp(self.degrees.0, self.degrees.1, e)
    }

    /// `wrapper` at the current angle. A finished exit renders nothing visible.
    pub fn frame(&self, wrapper: gpui::Div, cx: &App) -> gpui::AnyElement {
        if self.is_exit() && self.done() {
            return turned(90., wrapper, cx);
        }
        turned(self.degrees(), wrapper, cx)
    }
}

/// `wrapper` (a page filling its parent) turned by `degrees` about the
/// hinge: clipped to its projected span at full size. At 0° it renders
/// exactly as itself, in the same element structure as when turned.
pub fn turned(degrees: f32, wrapper: gpui::Div, cx: &App) -> gpui::AnyElement {
    let metrics = cx.try_global::<Theme>().map_or((PAGE_MARGIN, PAGE_CONTENT_WIDTH), |t| t.page_metrics);
    let (f, offset) = if reduced(cx) { (1., 0.) } else { page_turn(degrees, metrics) };
    // Within a thousandth of rest, show it at rest: less than half a px.
    let f_clamped = if f > 0.999 { 1. } else { f.max(0.02) };
    // The page inside is the clip's width × 1/f, which in floats can land a
    // hair under the full width, and content that exactly fills a row
    // (a wrapping grid of swatches) then reflows for a frame. Err wide: a
    // fraction of a px past the page's right edge is never seen.
    let inner = if f_clamped >= 1. { 1. } else { (1. + 1e-4) / f_clamped };
    div()
        .relative()
        .size_full()
        .overflow_hidden()
        .w(relative(f_clamped))
        .ml(u(if f_clamped >= 1. { 0. } else { offset }))
        .opacity(if f < 0.04 { 0. } else { 1. })
        .child(div().relative().h_full().w(relative(inner)).child(wrapper.size_full()))
        .into_any_element()
}

/// Pivot-style entrance: the whole pane slides in from the side it came
/// from, without fading, decelerating hard (500 ms, exponent 6).
pub fn slide_in(id: impl Into<ElementId>, direction: Direction, wrapper: gpui::Div, cx: &App) -> gpui::AnyElement {
    if reduced(cx) {
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

/// Where a swivel starts: the pop-up is turned this far about its
/// horizontal axis, nearly edge-on.
pub const SWIVEL_IN_DEG: f32 = 70.;

/// Swivel-in (the WP Toolkit's `SwivelTransition`): `wrapper` swings in
/// about a horizontal axis through its middle, from nearly edge-on. Its
/// height is projected (cos θ, with perspective) and the content is cropped
/// symmetrically around the axis, so the band opens from its centre line.
/// The wrapper is measured on its first frame, during which it is hidden.
pub fn swivel_in(id: impl Into<ElementId>, wrapper: gpui::Div, window: &mut Window, cx: &mut App) -> gpui::AnyElement {
    if reduced(cx) {
        return wrapper.into_any_element();
    }
    let id = id.into();
    let height = window
        .use_keyed_state(ElementId::from((SharedString::from(format!("{id}-swivel")), 0usize)), cx, |_, _| {
            Rc::new(Cell::new(None::<f32>))
        })
        .read(cx)
        .clone();
    let measure = height.clone();
    let wrapper = wrapper.relative().child(
        gpui::canvas(move |b, _, _| measure.set(Some(f32::from(b.size.height))), |_, _, _, _| {})
            .absolute()
            .inset_0(),
    );
    let animation = Animation::new(ENTER).with_easing(ease_out_exp6);
    // Projected height of a band `h` tall turned by `deg` about its middle.
    let projected = |h: f32, deg: f32| {
        let (sin, cos) = deg.to_radians().sin_cos();
        let half = h / 2.;
        let k = PERSPECTIVE / (PERSPECTIVE + half * sin).max(1.);
        2. * half * cos * k
    };
    let (h_content, h_clip) = (height.clone(), height);
    let content = wrapper.with_animation(id.clone(), animation.clone(), move |el, t| match h_content.get() {
        Some(h) => el.top(gpui::px(-(h - projected(h, lerp(SWIVEL_IN_DEG, 0., t))) / 2.)),
        None => el,
    });
    div()
        .relative()
        .w_full()
        .overflow_hidden()
        .child(content)
        .with_animation(id, animation, move |el, t| match h_clip.get() {
            Some(h) => el.h(gpui::px(projected(h, lerp(SWIVEL_IN_DEG, 0., t)))),
            None => el.opacity(0.),
        })
        .into_any_element()
}

/// How far a pressed element sinks (the Toolkit's `TiltEffect`, flattened:
/// GPUI can't tilt, so the press reads as the element pushed back).
pub const TILT_SCALE: f32 = 0.94;

struct Press {
    down: bool,
}

/// The tilt effect. Returns a fixed-size-agnostic press surface: style its
/// size on the result (it centres `child`), add your click handler, and the
/// child sinks to [`TILT_SCALE`] while the pointer holds it down, then
/// springs back. Layout never changes; only the child's rendering scales.
pub fn tilt(id: impl Into<ElementId>, child: impl IntoElement, window: &mut Window, cx: &mut App) -> gpui::Stateful<gpui::Div> {
    use gpui::{InteractiveElement, MouseButton};
    let id = id.into();
    let key = SharedString::from(format!("{id}-tilt"));
    let press = window.use_keyed_state(ElementId::from((key.clone(), 0usize)), cx, |_, _| Press { down: false });
    let down = press.read(cx).down && !reduced(cx);
    let (was, generation) = track(ElementId::from((key.clone(), 1usize)), down, window, cx);
    let target = if down { TILT_SCALE } else { 1. };
    let from = if was { TILT_SCALE } else { 1. };
    let inner = crate::scaled::scaled(target).child(child);
    let inner = if generation > 0 && from != target {
        // Pressing is near-instant; the spring back is a touch slower.
        let ms = if down { 60 } else { 180 };
        inner
            .with_animation(
                generation_id(&key, generation),
                Animation::new(Duration::from_millis(ms)).with_easing(ease_out_exp6),
                move |el, t| el.scale(lerp(from, target, t)),
            )
            .into_any_element()
    } else {
        inner.into_any_element()
    };
    let set = |value: bool| {
        let press = press.clone();
        move |_: &gpui::MouseDownEvent, _: &mut Window, cx: &mut App| {
            press.update(cx, |p, cx| {
                p.down = value;
                cx.notify();
            })
        }
    };
    let release = {
        let press = press.clone();
        move |_: &gpui::MouseUpEvent, _: &mut Window, cx: &mut App| {
            press.update(cx, |p, cx| {
                if p.down {
                    p.down = false;
                    cx.notify();
                }
            })
        }
    };
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(MouseButton::Left, set(true))
        .on_mouse_up(MouseButton::Left, release.clone())
        .on_mouse_up_out(MouseButton::Left, release)
        .child(inner)
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
    fn turns_start_and_end_edge_on_so_nothing_pops() {
        let m = (PAGE_MARGIN, PAGE_CONTENT_WIDTH);
        for deg in [FORWARD_IN_DEG, BACKWARD_IN_DEG, FORWARD_OUT_DEG, BACKWARD_OUT_DEG] {
            assert!(page_turn(deg, m).0 < 0.01, "{deg}° should be edge-on");
        }
        assert!((page_turn(0., m).0 - 1.).abs() < 1e-4);
        // An entrance has already opened past the Toolkit's −80° early on.
        assert!(lerp(FORWARD_IN_DEG, 0., ease_out_exp6(0.05)) > -80.);
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
        let widths: Vec<f32> = (0..=10).map(|i| page_turn(lerp(FORWARD_IN_DEG, 0., i as f32 / 10.), (PAGE_MARGIN, PAGE_CONTENT_WIDTH)).0).collect();
        assert!(widths.windows(2).all(|w| w[0] <= w[1] + 1e-6), "{widths:?}");
    }
}
