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

/// Ambient motion — live tiles peeking, progress indicators — runs at this
/// rate whatever the display does. On a 120 Hz screen it would otherwise ask
/// for every refresh: ambient motion is slow and always somewhere on screen,
/// so those frames are most of what a resting app spends. Windows Phone drew
/// it at 60. Transitions the user starts (turnstiles, pivots) keep the
/// display's rate.
pub const AMBIENT_FPS: f32 = 60.;

/// The shared ambient clock: the views waiting for its next tick, and the
/// one timer that will wake them, so every ambient animation on screen lands
/// in the same frames instead of each asking on its own.
struct Ambient {
    epoch: std::time::Instant,
    views: Vec<gpui::EntityId>,
    tick: Option<gpui::Task<()>>,
}

impl gpui::Global for Ambient {}

/// How long from `since_epoch` to the next tick of a clock ticking every
/// `period` from its epoch.
pub fn until_next_tick(since_epoch: Duration, period: Duration) -> Duration {
    let p = period.as_secs_f64();
    let k = (since_epoch.as_secs_f64() / p).floor() + 1.;
    Duration::from_secs_f64(k * p) - since_epoch
}

/// Asks for the next ambient frame for the view being drawn: it is
/// notified on the shared [`AMBIENT_FPS`] clock's next tick. Use it in
/// place of `window.request_animation_frame()` for motion nobody started.
pub fn request_ambient_frame(window: &Window, cx: &mut App) {
    let view = window.current_view();
    ensure_ambient(cx);
    let ambient = cx.global_mut::<Ambient>();
    if !ambient.views.contains(&view) {
        ambient.views.push(view);
    }
    if ambient.tick.is_some() {
        return;
    }
    let period = Duration::from_secs_f32(1. / AMBIENT_FPS);
    let wait = until_next_tick(ambient.epoch.elapsed(), period);
    let tick = cx.spawn(async move |cx| {
        cx.background_executor().timer(wait).await;
        cx.update(|cx| {
            let ambient = cx.global_mut::<Ambient>();
            ambient.tick = None;
            let views = std::mem::take(&mut ambient.views);
            for view in views {
                cx.notify(view);
            }
        });
    });
    cx.global_mut::<Ambient>().tick = Some(tick);
}

fn ensure_ambient(cx: &mut App) {
    if !cx.has_global::<Ambient>() {
        cx.set_global(Ambient { epoch: std::time::Instant::now(), views: Vec::new(), tick: None });
    }
}

/// The beat live tiles slide on, if any; see [`set_live_tile_beat`].
struct LiveTileBeat(Option<f32>);

impl gpui::Global for LiveTileBeat {}

/// Puts every live tile's slides on a shared beat, `seconds` apart (`None`,
/// the default: each tile keeps its own time). Left alone, tiles with
/// different periods and phases nearly always have one mid-slide somewhere,
/// so a start screen draws frames all the time. On a beat they slide
/// together, in waves, and between waves nothing is drawn at all. Each
/// tile's period becomes the nearest whole number of beat pairs and its
/// phase the nearest beat, so tiles still take turns.
pub fn set_live_tile_beat(seconds: Option<f32>, cx: &mut App) {
    cx.set_global(LiveTileBeat(seconds.map(|s| s.max(1.))));
    cx.refresh_windows();
}

/// The beat set by [`set_live_tile_beat`].
pub fn live_tile_beat(cx: &App) -> Option<f32> {
    cx.try_global::<LiveTileBeat>().and_then(|b| b.0)
}

/// When the ambient clock started: what beats are counted from.
pub fn ambient_epoch(cx: &mut App) -> std::time::Instant {
    ensure_ambient(cx);
    cx.global::<Ambient>().epoch
}

/// A cycle of `period` seconds at `phase` (0–1), moved onto a beat: the
/// period and phase to use, counting from the ambient epoch, so that both
/// of the cycle's slides (each `slide` seconds long, ending at the half and
/// at the end of the cycle) start on a beat.
pub fn on_beat(period: f32, phase: f32, slide: f32, beat: f32) -> (f32, f32) {
    let period = (period / (2. * beat)).round().max(1.) * 2. * beat;
    let beats = (phase * period / beat).round();
    let phase = 0.5 - slide / period - beats * beat / period;
    (period, phase.rem_euclid(1.))
}

/// `after`, pushed back to the ambient clock's next tick: for a timer that
/// wakes ambient motion later (a live tile's next slide), so the frame it
/// causes is one the clock draws anyway instead of an extra one between.
pub fn ambient_delay(after: Duration, cx: &mut App) -> Duration {
    ensure_ambient(cx);
    let period = Duration::from_secs_f32(1. / AMBIENT_FPS);
    let at = cx.global::<Ambient>().epoch.elapsed() + after;
    after + until_next_tick(at, period)
}

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
///
/// A page turned toward the viewer is drawn as if turned as far away. Toward
/// the viewer the far edge magnifies past the screen, which a clip at full
/// size can't show: the span was already full width two frames into a
/// 350 ms entrance from −90°, so the page popped in. Mirrored, it opens from
/// a sliver at the hinge, the way the start screen's tiles do.
fn page_turn(degrees: f32, (inset, width): (f32, f32)) -> (f32, f32) {
    let t = turn(inset, width, degrees.abs());
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
        // Sized by height, or grown by flex (`flex_1`): either way the
        // content must fill the clip, or a `size_full` page inside it
        // collapses to nothing.
        fills_height = src.size.height.is_some() || src.flex_grow.is_some_and(|g| g > 0.);
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

/// How long a pivot pane takes to slide in.
pub const PANE_IN: Duration = Duration::from_millis(400);
/// How far (design px) a pivot pane starts from where it settles.
pub const PANE_SHIFT: f32 = 110.;
/// Extra distance per line for the pivot's line-by-line slide (the WP
/// Toolkit's `SlideInEffect`): line 1 starts this much further out than
/// line 0, line 2 twice as far, and so on. All arrive together on the same
/// ease, so the further lines trail and the pane fans in.
pub const LINE_LEAD: f32 = 70.;
/// How long the outgoing pivot pane takes to slip away.
pub const PANE_OUT: Duration = Duration::from_millis(130);

/// A pane slide in progress, read by [`slide_line`] while the pane lays out.
#[derive(Clone)]
pub struct LineSlide {
    key: SharedString,
    /// +1: from the right (forward); −1: from the left.
    sign: f32,
    delay: Duration,
}

thread_local! {
    static LINE_SLIDES: std::cell::RefCell<Vec<LineSlide>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Pivot-style entrance: the pane slides in from the side it came from,
/// without fading, decelerating hard; lines inside it marked with
/// [`slide_line`] (list items mark theirs) trail it by line, so the pane fans
/// in. `delay` holds it hidden first (while the old pane leaves).
pub fn slide_in_after(
    id: impl Into<ElementId>,
    direction: Direction,
    delay: Duration,
    wrapper: gpui::Div,
    cx: &App,
) -> gpui::AnyElement {
    if reduced(cx) {
        return wrapper.into_any_element();
    }
    let id = id.into();
    let sign = if direction == Direction::Forward { 1. } else { -1. };
    let slide = LineSlide { key: SharedString::from(format!("{id}")), sign, delay };
    let mut animations = Vec::with_capacity(2);
    if !delay.is_zero() {
        animations.push(Animation::new(delay));
    }
    let delayed = animations.len() == 1;
    animations.push(Animation::new(PANE_IN).with_easing(ease_out_exp6));
    LineScope { div: div().size_full(), slide: Some(slide) }
        .child(wrapper)
        .relative()
        .with_animations(id, animations, move |el, phase, t| {
            if delayed && phase == 0 {
                el.left(u(sign * PANE_SHIFT)).opacity(0.)
            } else {
                el.left(u(sign * PANE_SHIFT * (1. - t))).opacity(1.)
            }
        })
        .into_any_element()
}

/// [`slide_in_after`] with no wait.
pub fn slide_in(id: impl Into<ElementId>, direction: Direction, wrapper: gpui::Div, cx: &App) -> gpui::AnyElement {
    slide_in_after(id, direction, Duration::ZERO, wrapper, cx)
}

/// The pane leaving as another slides in: it slips a little the other way
/// and fades, quickly, on an ease-in.
pub fn slide_out(id: impl Into<ElementId>, direction: Direction, wrapper: gpui::Div, cx: &App) -> gpui::AnyElement {
    if reduced(cx) {
        return div().into_any_element();
    }
    let sign = if direction == Direction::Forward { -1. } else { 1. };
    wrapper
        .relative()
        // Linear fade, gently accelerating slip: an exponential ease-in sat
        // still for most of so short a move, then the pane vanished.
        .with_animation(id, Animation::new(PANE_OUT), move |el, t| el.left(u(sign * PANE_SHIFT * 0.5 * t * t)).opacity(1. - t))
        .into_any_element()
}

/// One line of content (`line` 0 = the first) that trails a pane sliding in
/// around it by [`LINE_LEAD`] per line. Outside a sliding pane it renders
/// as is. `id` must be unique among its siblings' lines.
pub fn slide_line(id: impl Into<ElementId>, line: usize, el: gpui::Div) -> gpui::AnyElement {
    let Some(slide) = LINE_SLIDES.with(|s| s.borrow().last().cloned()) else {
        return el.into_any_element();
    };
    if line == 0 {
        return el.into_any_element();
    }
    let lead = slide.sign * LINE_LEAD * line as f32;
    let mut animations = Vec::with_capacity(2);
    if !slide.delay.is_zero() {
        animations.push(Animation::new(slide.delay));
    }
    let delayed = animations.len() == 1;
    animations.push(Animation::new(PANE_IN).with_easing(ease_out_exp6));
    let id: ElementId = id.into();
    el.relative()
        .with_animations(
            ElementId::from((SharedString::from(format!("{}-{id}", slide.key)), line)),
            animations,
            move |el, phase, t| el.left(u(lead * if delayed && phase == 0 { 1. } else { 1. - t })),
        )
        .into_any_element()
}

/// A div that makes a [`LineSlide`] current while its children lay out.
struct LineScope {
    div: gpui::Div,
    slide: Option<LineSlide>,
}

impl LineScope {
    fn scoped<R>(&self, f: impl FnOnce() -> R) -> R {
        let Some(slide) = self.slide.clone() else { return f() };
        LINE_SLIDES.with(|s| s.borrow_mut().push(slide));
        let r = f();
        LINE_SLIDES.with(|s| s.borrow_mut().pop());
        r
    }
}

impl Styled for LineScope {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.div.style()
    }
}

impl ParentElement for LineScope {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.div.extend(elements)
    }
}

impl gpui::Element for LineScope {
    type RequestLayoutState = gpui::DivFrameState;
    type PrepaintState = Option<gpui::Hitbox>;

    fn id(&self) -> Option<ElementId> {
        gpui::Element::id(&self.div)
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        gpui::Element::source_location(&self.div)
    }

    fn request_layout(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        // Children render (and their animations sample) during layout.
        let slide = self.slide.take();
        let scope = LineScope { div: div(), slide };
        let r = scope.scoped(|| self.div.request_layout(id, inspector_id, window, cx));
        self.slide = scope.slide;
        r
    }

    fn prepaint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.div.prepaint(id, inspector_id, bounds, request_layout, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.div.paint(id, inspector_id, bounds, request_layout, prepaint, window, cx)
    }
}

impl IntoElement for LineScope {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
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
    fn a_cycle_on_a_beat_starts_both_slides_on_it() {
        let (beat, slide) = (3., 0.9);
        for (period, phase) in [(9., 0.), (12., 0.37), (15., 0.9), (7., 0.5)] {
            let (p, ph) = on_beat(period, phase, slide, beat);
            assert_eq!((p / (2. * beat)).fract(), 0., "a whole number of beat pairs ({p})");
            // Slides start where the cycle reaches 0.5 - slide/p and 1 - slide/p.
            for at in [0.5 - slide / p, 1. - slide / p] {
                let secs = (at - ph).rem_euclid(1.) * p;
                let off = (secs / beat).round() * beat - secs;
                assert!(off.abs() < 1e-3, "period {period} phase {phase}: slide {off} s off the beat");
            }
        }
    }

    #[test]
    fn ambient_ticks_land_on_one_grid() {
        let p = Duration::from_millis(16);
        assert_eq!(until_next_tick(Duration::from_millis(0), p), p);
        assert_eq!(until_next_tick(Duration::from_millis(5), p), Duration::from_millis(11));
        assert_eq!(until_next_tick(Duration::from_millis(16), p), p, "on a tick: wait for the next");
        // Two asks a few ms apart wake on the same tick.
        let a = Duration::from_millis(33) + until_next_tick(Duration::from_millis(33), p);
        let b = Duration::from_millis(40) + until_next_tick(Duration::from_millis(40), p);
        assert_eq!(a, b);
    }

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
    fn a_page_entrance_opens_over_many_frames() {
        let m = (PAGE_MARGIN, PAGE_CONTENT_WIDTH);
        let frame = 1. / 60. / ENTER.as_secs_f32();
        for from in [FORWARD_IN_DEG, BACKWARD_IN_DEG] {
            // Two frames in, still well short of full width (it used to be full).
            let f = page_turn(lerp(from, 0., ease_out_exp6(2. * frame)), m).0;
            assert!(f < 0.6, "{from}°: {f}");
        }
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
