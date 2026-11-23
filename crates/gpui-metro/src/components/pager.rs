//! Horizontal paging input shared by [`crate::Pivot`] and [`crate::Panorama`].
//!
//! * **Mouse**: Shift + wheel pages sideways (wheel down = next). Three
//!   notches make a page, and every further three page again.
//! * **Trackpad**: a horizontal two-finger swipe, one page per gesture.
//!
//! Plain vertical scrolling is left to the content.
//!
//! [`surface`] pages in discrete steps (pivot). [`follow_surface`] makes the
//! content track the input and then settle (panorama): it pages once the
//! drag passes a third of a section, otherwise springs back when input stops.

use std::rc::Rc;
use std::time::Duration;

use gpui::{
    App, Bounds, Canvas, Context, DispatchPhase, Entity, Pixels, ScrollDelta, ScrollWheelEvent, Styled,
    TouchPhase, Window, canvas,
};

/// Horizontal travel (px) that turns a scroll into a page step.
pub(crate) const SWIPE_THRESHOLD: f32 = 60.;
/// Pixels per wheel notch.
const LINE: f32 = 20.;

/// `+1` = next page, `-1` = previous page.
pub(crate) type StepHandler = Rc<dyn Fn(i32, &mut Window, &mut App)>;

/// Turns scroll events into discrete page steps.
pub(crate) struct Swipe {
    accum: f32,
    /// Trackpad gestures page once, then wait for the gesture to end so
    /// momentum doesn't run through several pages.
    armed: bool,
}

impl Swipe {
    pub(crate) fn new() -> Self {
        Self { accum: 0., armed: true }
    }

    /// `Some(1)` = next page, `Some(-1)` = previous.
    pub(crate) fn feed(&mut self, e: &ScrollWheelEvent) -> Option<i32> {
        if matches!(e.touch_phase, TouchPhase::Started) {
            *self = Self::new();
        }
        let (dx, dy, wheel) = match e.delta {
            ScrollDelta::Pixels(d) => (f32::from(d.x), f32::from(d.y), false),
            ScrollDelta::Lines(d) => (d.x * LINE, d.y * LINE, true),
        };
        // Shift turns a vertical wheel sideways (some platforms already do this).
        let dx = if e.modifiers.shift && dx == 0. { dy } else { dx };
        let dy = if e.modifiers.shift { 0. } else { dy };

        let mut step = None;
        if dx.abs() >= dy.abs() && dx != 0. {
            self.accum += dx;
            if self.armed && self.accum.abs() > SWIPE_THRESHOLD {
                step = Some(if self.accum < 0. { 1 } else { -1 });
                self.accum = 0.;
                // A wheel has no gesture phases: let it keep paging.
                self.armed = wheel;
            }
        }
        if matches!(e.touch_phase, TouchPhase::Ended) {
            *self = Self::new();
        }
        step
    }
}

pub(crate) struct Pager {
    swipe: Swipe,
    bounds: Option<Bounds<Pixels>>,
    /// Follow mode: px the content currently trails the input by (+ = moved right).
    pub(crate) offset: f32,
    /// Bumps on every input event, so a pending settle can tell it is stale.
    input_epoch: u64,
    /// A trackpad gesture already paged; ignore it until it ends.
    spent: bool,
    /// Offset (in sections) at the moment input paged, so the settle starts
    /// from where the content visibly was.
    pub(crate) release: Option<f32>,
    /// The selection generation last seen by the panorama.
    pub(crate) seen_generation: usize,
    /// The settle in progress: (key, from, to), in section positions.
    pub(crate) settle: Option<(u64, f32, f32)>,
    settle_key: u64,
}

impl Pager {
    pub(crate) fn new(_: &mut Context<Self>) -> Self {
        Self {
            swipe: Swipe::new(),
            bounds: None,
            offset: 0.,
            input_epoch: 0,
            spent: false,
            release: None,
            seen_generation: 0,
            settle: None,
            settle_key: 0,
        }
    }

    /// Starts a settle from `from` to `to` (section positions).
    pub(crate) fn settle_to(&mut self, from: f32, to: f32) {
        self.settle_key += 1;
        self.settle = Some((self.settle_key, from, to));
    }
}

/// Pause after the last input before an unfinished drag springs back.
const SETTLE_IDLE: Duration = Duration::from_millis(160);
/// A drag past this fraction of a section pages.
pub(crate) const FOLLOW_COMMIT: f32 = 0.3;
/// Pixels per wheel notch in follow mode.
const FOLLOW_LINE: f32 = 45.;
/// How much of the input a drag past the first/last section keeps.
const RUBBER_BAND: f32 = 0.3;

/// What one scroll event does to a following pager.
#[derive(Debug, PartialEq)]
pub(crate) enum Follow {
    Ignore,
    Moved,
    Step(i32),
}

impl Pager {
    /// Applies a scroll event in follow mode. `width` is a section's width;
    /// `can_back`/`can_next` say whether there is a section on that side.
    pub(crate) fn follow(&mut self, e: &ScrollWheelEvent, width: f32, can_back: bool, can_next: bool) -> Follow {
        let phase = &e.touch_phase;
        if matches!(phase, TouchPhase::Started) {
            self.spent = false;
        }
        let (dx, dy) = match e.delta {
            ScrollDelta::Pixels(d) => (f32::from(d.x), f32::from(d.y)),
            ScrollDelta::Lines(d) => (d.x * FOLLOW_LINE, d.y * FOLLOW_LINE),
        };
        let dx = if e.modifiers.shift && dx == 0. { dy } else { dx };
        let dy = if e.modifiers.shift { 0. } else { dy };
        if self.spent || dx == 0. || dx.abs() < dy.abs() {
            return Follow::Ignore;
        }
        self.input_epoch += 1;
        self.settle = None;
        let resisting = (self.offset + dx > 0. && !can_back) || (self.offset + dx < 0. && !can_next);
        self.offset = (self.offset + dx * if resisting { RUBBER_BAND } else { 1. }).clamp(-width, width);
        let step = if self.offset <= -width * FOLLOW_COMMIT && can_next {
            1
        } else if self.offset >= width * FOLLOW_COMMIT && can_back {
            -1
        } else {
            return Follow::Moved;
        };
        self.release = Some(self.offset / width);
        self.offset = 0.;
        self.spent = matches!(e.delta, ScrollDelta::Pixels(_));
        Follow::Step(step)
    }
}

/// An invisible layer (`absolute().inset_0()`, last child of the surface)
/// that turns scrolling over the surface into `on_step` calls.
pub(crate) fn surface(pager: Entity<Pager>, on_step: StepHandler) -> Canvas<()> {
    let measure = pager.clone();
    canvas(
        move |bounds, _, cx| measure.update(cx, |p, _| p.bounds = Some(bounds)),
        move |_, _, window, _| {
            // Capture phase: vertically scrolling content inside the surface
            // would otherwise swallow the event first.
            window.on_mouse_event(move |e: &ScrollWheelEvent, phase, window, cx| {
                let over = pager.read(cx).bounds.is_some_and(|b| b.contains(&e.position));
                if phase != DispatchPhase::Capture || !over {
                    return;
                }
                if let Some(step) = pager.update(cx, |p, _| p.swipe.feed(e)) {
                    on_step(step, window, cx);
                }
                // Shift + wheel belongs to the pager; don't let content scroll too.
                if e.modifiers.shift {
                    cx.stop_propagation();
                }
            });
        },
    )
    .absolute()
    .inset_0()
}

/// The follow-mode layer for a panorama. `width` is a section's width;
/// `position` is the selected section (for the spring-back settle).
pub(crate) fn follow_surface(
    pager: Entity<Pager>,
    width: f32,
    position: usize,
    can_back: bool,
    can_next: bool,
    on_step: StepHandler,
) -> Canvas<()> {
    let measure = pager.clone();
    canvas(
        move |bounds, _, cx| measure.update(cx, |p, _| p.bounds = Some(bounds)),
        move |_, _, window, _| {
            window.on_mouse_event(move |e: &ScrollWheelEvent, phase, window, cx| {
                let over = pager.read(cx).bounds.is_some_and(|b| b.contains(&e.position));
                if phase != DispatchPhase::Capture || !over {
                    return;
                }
                let (result, epoch) = pager.update(cx, |p, cx| {
                    let r = p.follow(e, width, can_back, can_next);
                    if r != Follow::Ignore {
                        cx.notify();
                    }
                    (r, p.input_epoch)
                });
                if e.modifiers.shift || result != Follow::Ignore {
                    cx.stop_propagation();
                }
                match result {
                    Follow::Step(step) => on_step(step, window, cx),
                    Follow::Moved => {
                        // Spring back once input pauses (or the gesture ends).
                        let pager = pager.clone();
                        let ended = matches!(e.touch_phase, TouchPhase::Ended);
                        window
                            .spawn(cx, async move |cx| {
                                if !ended {
                                    cx.background_executor().timer(SETTLE_IDLE).await;
                                }
                                cx.update(|_, cx| {
                                    pager.update(cx, |p, cx| {
                                        if p.input_epoch == epoch && p.offset != 0. {
                                            let from = position as f32 - p.offset / width;
                                            p.offset = 0.;
                                            p.settle_to(from, position as f32);
                                            cx.notify();
                                        }
                                    })
                                })
                                .ok();
                            })
                            .detach();
                    }
                    Follow::Ignore => {}
                }
            });
        },
    )
    .absolute()
    .inset_0()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Modifiers, point, px};

    fn event(delta: ScrollDelta, shift: bool, phase: TouchPhase) -> ScrollWheelEvent {
        ScrollWheelEvent {
            position: point(px(0.), px(0.)),
            delta,
            modifiers: Modifiers { shift, ..Modifiers::default() },
            touch_phase: phase,
        }
    }
    fn pixels(dx: f32, dy: f32, phase: TouchPhase) -> ScrollWheelEvent {
        event(ScrollDelta::Pixels(point(px(dx), px(dy))), false, phase)
    }
    fn notch(dy: f32, shift: bool) -> ScrollWheelEvent {
        event(ScrollDelta::Lines(point(0., dy)), shift, TouchPhase::Moved)
    }

    #[test]
    fn one_step_per_trackpad_gesture() {
        let mut s = Swipe::new();
        assert_eq!(s.feed(&pixels(-10., 0., TouchPhase::Started)), None);
        assert_eq!(s.feed(&pixels(-40., 0., TouchPhase::Moved)), None);
        assert_eq!(s.feed(&pixels(-40., 0., TouchPhase::Moved)), Some(1));
        assert_eq!(s.feed(&pixels(-200., 0., TouchPhase::Moved)), None, "momentum doesn't page twice");
        assert_eq!(s.feed(&pixels(0., 0., TouchPhase::Ended)), None);
        assert_eq!(s.feed(&pixels(90., 0., TouchPhase::Started)), Some(-1));
    }

    #[test]
    fn shift_wheel_pages_every_three_notches_both_ways() {
        let mut s = Swipe::new();
        let steps: Vec<Option<i32>> = (0..6).map(|_| s.feed(&notch(-1., true))).collect();
        assert_eq!(steps, [None, None, None, Some(1), None, None], "{steps:?}");
        assert_eq!(s.feed(&notch(-1., true)), None);
        assert_eq!(s.feed(&notch(-1., true)), Some(1), "keeps paging without a gesture end");
        let back: Vec<Option<i32>> = (0..8).map(|_| s.feed(&notch(1., true))).collect();
        assert!(back.contains(&Some(-1)), "wheel up with shift goes back: {back:?}");
    }

    fn follower() -> Pager {
        Pager {
            swipe: Swipe::new(),
            bounds: None,
            offset: 0.,
            input_epoch: 0,
            spent: false,
            release: None,
            seen_generation: 0,
            settle: None,
            settle_key: 0,
        }
    }

    #[test]
    fn follow_tracks_input_then_pages_past_a_third() {
        let mut p = follower();
        let w = 400.;
        assert_eq!(p.follow(&notch(-1., true), w, true, true), Follow::Moved);
        assert_eq!(p.offset, -45., "content trails the wheel");
        assert_eq!(p.follow(&notch(-1., true), w, true, true), Follow::Moved);
        assert_eq!(p.follow(&notch(-1., true), w, true, true), Follow::Step(1));
        assert_eq!(p.offset, 0.);
        assert!((p.release.unwrap() + 135. / 400.).abs() < 1e-6, "settle starts where the content was");
    }

    #[test]
    fn follow_resists_past_the_ends_and_never_pages_there() {
        let mut p = follower();
        for _ in 0..20 {
            assert_eq!(p.follow(&notch(1., true), 400., false, true), Follow::Moved);
        }
        assert!(p.offset > 0. && p.offset <= 400.);
        let mut q = follower();
        q.follow(&notch(1., true), 400., false, true);
        assert!((q.offset - 45. * RUBBER_BAND).abs() < 1e-4);
    }

    #[test]
    fn a_trackpad_fling_pages_only_once() {
        let mut p = follower();
        p.follow(&pixels(-10., 0., TouchPhase::Started), 400., true, true);
        let mut steps = 0;
        for _ in 0..20 {
            if let Follow::Step(_) = p.follow(&pixels(-40., 0., TouchPhase::Moved), 400., true, true) {
                steps += 1;
            }
        }
        assert_eq!(steps, 1);
        p.follow(&pixels(0., 0., TouchPhase::Ended), 400., true, true);
        p.follow(&pixels(-10., 0., TouchPhase::Started), 400., true, true);
        assert!(!p.spent, "a new gesture can page again");
    }

    #[test]
    fn plain_wheel_and_vertical_scrolling_never_page() {
        let mut s = Swipe::new();
        for _ in 0..10 {
            assert_eq!(s.feed(&notch(-1., false)), None);
            assert_eq!(s.feed(&pixels(-30., -80., TouchPhase::Moved)), None);
        }
    }
}
