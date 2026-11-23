//! The WP8 slider: a thick track, accent fill, and a thin upright thumb.
//! Click or drag anywhere on it; the drag keeps tracking outside the bounds.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    App, Bounds, DispatchPhase, ElementId, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, RenderOnce, SharedString, Styled, Window, canvas, div,
    prelude::FluentBuilder, px, relative,
};

use crate::{Theme, theme::size};

pub type ValueHandler = Rc<dyn Fn(f32, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Slider {
    id: ElementId,
    value: f32,
    header: Option<SharedString>,
    step: Option<f32>,
    disabled: bool,
    on_change: Option<ValueHandler>,
}

/// `value` in 0–1.
pub fn slider(id: impl Into<ElementId>, value: f32) -> Slider {
    Slider { id: id.into(), value: value.clamp(0., 1.), header: None, step: None, disabled: false, on_change: None }
}

impl Slider {
    pub fn header(mut self, header: impl Into<SharedString>) -> Self {
        self.header = Some(header.into());
        self
    }
    /// Snap to multiples of `step` (e.g. 0.1).
    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn on_change(mut self, f: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

/// Maps a pointer x onto the track, snapped.
pub fn value_at(x: Pixels, bounds: Bounds<Pixels>, step: Option<f32>) -> f32 {
    let w = f32::from(bounds.size.width).max(1.);
    let v = (f32::from(x - bounds.origin.x) / w).clamp(0., 1.);
    match step {
        Some(s) if s > 0. => ((v / s).round() * s).clamp(0., 1.),
        _ => v,
    }
}

struct DragState {
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    dragging: Rc<Cell<bool>>,
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| DragState {
            bounds: Rc::new(Cell::new(None)),
            dragging: Rc::new(Cell::new(false)),
        });
        let (bounds, dragging) = {
            let s = state.read(cx);
            (s.bounds.clone(), s.dragging.clone())
        };
        let fill = if self.disabled { p.disabled } else { p.accent };
        let track = if self.disabled { p.disabled.opacity(0.4) } else { p.input.opacity(0.5) };
        let thumb = if self.disabled { p.disabled } else { p.foreground };
        let step = self.step;
        let on_change = self.on_change.clone().filter(|_| !self.disabled);

        let measure = {
            let bounds = bounds.clone();
            let dragging = dragging.clone();
            let on_change = on_change.clone();
            let bounds_p = bounds.clone();
            canvas(
                move |b, _, _| bounds_p.set(Some(b)),
                move |_, _, window, _| {
                    // While dragging, follow the pointer anywhere in the window.
                    let Some(on_change) = on_change else { return };
                    let (bounds_m, dragging_m) = (bounds.clone(), dragging.clone());
                    let f = on_change.clone();
                    window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, cx| {
                        if phase == DispatchPhase::Bubble
                            && dragging_m.get()
                            && let Some(b) = bounds_m.get()
                        {
                            f(value_at(e.position.x, b, step), window, cx);
                        }
                    });
                    let dragging_u = dragging.clone();
                    window.on_mouse_event(move |_: &MouseUpEvent, _, _, _| dragging_u.set(false));
                },
            )
            .absolute()
            .inset_0()
        };

        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(6.))
            .font_family(Theme::font(cx))
            .when_some(self.header, |el, h| el.child(div().text_size(px(size::NORMAL)).text_color(p.subtle).child(h)))
            .child(
                div()
                    .relative()
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(12.))
                            .bg(track)
                            .child(div().absolute().top_0().bottom_0().left_0().w(relative(self.value)).bg(fill)),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(relative(self.value))
                            .ml(px(-6.))
                            .w(px(12.))
                            .bg(thumb),
                    )
                    .child(measure)
                    .when_some(on_change, |el, f| {
                        el.cursor_pointer().on_mouse_down(MouseButton::Left, move |e: &MouseDownEvent, window, cx| {
                            dragging.set(true);
                            if let Some(b) = bounds.get() {
                                f(value_at(e.position.x, b, step), window, cx);
                            }
                        })
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, size};

    #[test]
    fn value_maps_and_snaps() {
        let b = Bounds::new(point(px(100.), px(0.)), size(px(200.), px(10.)));
        assert_eq!(value_at(px(100.), b, None), 0.);
        assert_eq!(value_at(px(200.), b, None), 0.5);
        assert_eq!(value_at(px(400.), b, None), 1.);
        assert_eq!(value_at(px(0.), b, None), 0.);
        assert!((value_at(px(161.), b, Some(0.25)) - 0.25).abs() < 1e-6);
    }
}
