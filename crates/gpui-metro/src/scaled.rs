//! Scaling a subtree. GPUI has no transform for boxes or text, but every
//! rem-based length (and gpui-metro sizes everything in rems, see
//! [`crate::u`]) resolves against the window's rem size. [`Scaled`] renders
//! its children under a different rem size, so the whole subtree, text
//! included, really shrinks or grows. Motion uses it for perspective.
//!
//! The pattern follows Zed's `ui::WithRemSize`.

use gpui::{
    AnyElement, App, Bounds, Div, DivFrameState, Element, ElementId, GlobalElementId, Hitbox, IntoElement,
    LayoutId, ParentElement, Pixels, StyleRefinement, Styled, Window, div,
};

use crate::theme::REM_PX;

/// A `div` whose children (and its own rem lengths) resolve at `scale` × the
/// normal rem size.
pub struct Scaled {
    div: Div,
    scale: f32,
}

/// Wraps children so they render at `scale` (1.0 = unchanged).
pub fn scaled(scale: f32) -> Scaled {
    Scaled { div: div(), scale }
}

impl Scaled {
    /// Changes the scale (for use inside an animator).
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    fn rem(&self, window: &Window) -> Option<Pixels> {
        // Relative to whatever rem size is in force here (scales nest).
        let base = f32::from(window.rem_size());
        let base = if base > 0. { base } else { REM_PX };
        ((self.scale - 1.).abs() > 1e-4).then(|| gpui::px(base * self.scale.max(0.01)))
    }
}

impl Styled for Scaled {
    fn style(&mut self) -> &mut StyleRefinement {
        self.div.style()
    }
}

impl ParentElement for Scaled {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.div.extend(elements)
    }
}

impl Element for Scaled {
    type RequestLayoutState = DivFrameState;
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        Element::id(&self.div)
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        Element::source_location(&self.div)
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let rem = self.rem(window);
        window.with_rem_size(rem, |window| self.div.request_layout(id, inspector_id, window, cx))
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let rem = self.rem(window);
        window.with_rem_size(rem, |window| self.div.prepaint(id, inspector_id, bounds, request_layout, window, cx))
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let rem = self.rem(window);
        window.with_rem_size(rem, |window| {
            self.div.paint(id, inspector_id, bounds, request_layout, prepaint, window, cx)
        })
    }
}

impl IntoElement for Scaled {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, canvas, div};

    use super::scaled;
    use crate::u;

    /// Renders a 100×40 design-px box under `scale` and reports its laid-out width and height.
    struct Probe {
        scale: f32,
        seen: Rc<Cell<(f32, f32)>>,
    }

    impl Render for Probe {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let seen = self.seen.clone();
            div().size_full().child(
                scaled(self.scale).child(
                    div().w(u(100.)).h(u(40.)).child(
                        canvas(
                            move |b, _, _| seen.set((f32::from(b.size.width), f32::from(b.size.height))),
                            |_, _, _, _| {},
                        )
                        .size_full(),
                    ),
                ),
            )
        }
    }

    fn measure(scale: f32, cx: &mut TestAppContext) -> (f32, f32) {
        let seen = Rc::new(Cell::new((0., 0.)));
        let probe_seen = seen.clone();
        let (_, vcx) = cx.add_window_view(move |_, _| Probe { scale, seen: probe_seen });
        vcx.run_until_parked();
        vcx.update(|window, _| window.refresh());
        vcx.run_until_parked();
        seen.get()
    }

    #[gpui::test]
    fn rem_lengths_follow_the_scale(cx: &mut TestAppContext) {
        assert_eq!(measure(1., cx), (100., 40.));
        assert_eq!(measure(0.5, cx), (50., 20.));
    }
}
