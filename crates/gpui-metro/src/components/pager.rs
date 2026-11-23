//! Horizontal paging input shared by [`crate::Pivot`] and [`crate::Panorama`].
//!
//! * **Mouse**: Shift + wheel pages sideways (wheel down = next). Three
//!   notches make a page, and every further three page again.
//! * **Trackpad**: a horizontal two-finger swipe, one page per gesture.
//!
//! Plain vertical scrolling is left to the content.

use std::rc::Rc;

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
}

impl Pager {
    pub(crate) fn new(_: &mut Context<Self>) -> Self {
        Self { swipe: Swipe::new(), bounds: None }
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

    #[test]
    fn plain_wheel_and_vertical_scrolling_never_page() {
        let mut s = Swipe::new();
        for _ in 0..10 {
            assert_eq!(s.feed(&notch(-1., false)), None);
            assert_eq!(s.feed(&pixels(-30., -80., TouchPhase::Moved)), None);
        }
    }
}
