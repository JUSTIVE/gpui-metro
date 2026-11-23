//! Panorama: one long horizontal canvas. A title far wider than the screen
//! drifts behind sections that snap one at a time, with the next section
//! peeking in from the right edge. The title travels slower than the
//! sections — the parallax that made the hubs feel deep.

use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder,
};

use crate::theme::u;

use crate::{
    Theme,
    chrome::PAGE_MARGIN,
    motion::{ease_out_exp6, lerp, track},
    pager::{self, Pager},
    pivot::SelectHandler,
    theme::{LIGHT, size},
};

pub struct PanoramaSection {
    header: SharedString,
    content: AnyElement,
}

#[derive(IntoElement)]
pub struct Panorama {
    id: ElementId,
    title: SharedString,
    sections: Vec<PanoramaSection>,
    selected: usize,
    section_width: f32,
    on_select: Option<SelectHandler>,
}

pub fn panorama(id: impl Into<ElementId>, title: impl Into<SharedString>, selected: usize) -> Panorama {
    Panorama { id: id.into(), title: title.into(), sections: Vec::new(), selected, section_width: 420., on_select: None }
}

impl Panorama {
    pub fn section(mut self, header: impl Into<SharedString>, content: impl IntoElement) -> Self {
        self.sections.push(PanoramaSection { header: header.into(), content: content.into_any_element() });
        self
    }
    /// Width of one section. Make it ~48 px narrower than the viewport so the
    /// next section peeks in.
    pub fn section_width(mut self, w: f32) -> Self {
        self.section_width = w;
        self
    }
    pub fn on_select(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

/// Title travel per section, as a fraction of the section travel.
pub const PARALLAX: f32 = 0.4;
/// Snap into a section: decelerates hard (WP Toolkit exponential, exponent 6).
const SETTLE: Duration = Duration::from_millis(450);

#[derive(Clone, Copy)]
enum Motion {
    /// The content trails the input, at this position.
    Following(f32),
    /// Settling (key, from, to).
    Settling(u64, f32, f32),
    Rest(f32),
}

/// Positions a layer (title or strip) for `motion`; `left` maps a section
/// position to the layer's left edge.
fn place(
    layer: gpui::Div,
    key: &str,
    name: &str,
    motion: Motion,
    dur: Duration,
    left: impl Fn(f32) -> gpui::Rems + 'static,
) -> AnyElement {
    match motion {
        Motion::Following(pos) | Motion::Rest(pos) => layer.left(left(pos)).into_any_element(),
        Motion::Settling(settle, from, to) => layer
            .with_animation(
                ElementId::from((SharedString::from(format!("{key}-{name}")), settle as usize)),
                Animation::new(dur).with_easing(ease_out_exp6),
                move |el, t| el.left(left(lerp(from, to, t))),
            )
            .into_any_element(),
    }
}

impl RenderOnce for Panorama {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let reduced = Theme::get(cx).reduced_motion;
        let len = self.sections.len();
        let selected = self.selected.min(len.saturating_sub(1));
        let key = format!("{}-pano", self.id);
        let (prev, generation) = track(ElementId::from(SharedString::from(key.clone())), selected, window, cx);
        let sw = self.section_width;
        let pager_state = window.use_keyed_state(
            ElementId::from(SharedString::from(format!("{key}-pager"))),
            cx,
            |_, cx| Pager::new(cx),
        );
        // Where the strip is (in sections), and how it is moving: following
        // the input, settling into a section, or at rest.
        let motion = pager_state.update(cx, |pg, _| {
            if generation != pg.seen_generation {
                pg.seen_generation = generation;
                let release = pg.release.take().unwrap_or(0.);
                pg.settle_to(prev as f32 - release, selected as f32);
            }
            if pg.offset != 0. {
                Motion::Following(selected as f32 - pg.offset / sw)
            } else {
                match pg.settle {
                    Some((key, from, to)) if !reduced => Motion::Settling(key, from, to),
                    _ => Motion::Rest(selected as f32),
                }
            }
        });
        let dur = SETTLE;

        let strip_left = move |pos: f32| u(PAGE_MARGIN - pos * sw);
        let title_left = move |pos: f32| u(PAGE_MARGIN - 6. - pos * sw * PARALLAX);

        let title = div()
            .absolute()
            .top(u(-18.))
            .whitespace_nowrap()
            .text_size(u(size::PANORAMA))
            .font_weight(LIGHT)
            .text_color(p.foreground)
            .child(SharedString::from(self.title.to_lowercase()));
        let title = place(title, &key, "t", motion, dur, title_left);

        let on_select = self.on_select.clone();
        let sections = self.sections.into_iter().enumerate().map(|(ix, s)| {
            let on_select = on_select.clone();
            div()
                .id(("pano-section", ix))
                .flex()
                .flex_col()
                .flex_none()
                .w(u(sw))
                .pr(u(PAGE_MARGIN))
                .h_full()
                .child(
                    div()
                        .text_size(u(size::EXTRA_LARGE + 4.))
                        .font_weight(LIGHT)
                        .pb(u(10.))
                        .child(SharedString::from(s.header.to_lowercase())),
                )
                .child(div().flex_1().min_h_0().child(s.content))
                // Tapping the peeking section brings it in.
                .when_some(on_select.filter(|_| ix != selected), |el, f| {
                    el.cursor_pointer().on_click(move |_, w, cx| f(ix, w, cx))
                })
        });
        let strip = div().absolute().top(u(132.)).bottom_0().flex().children(sections);
        let strip = place(strip, &key, "s", motion, dur, strip_left);
        let on_select = self.on_select;
        div()
            .id(self.id)
            .relative()
            .size_full()
            .overflow_hidden()
            .font_family(Theme::font(cx))
            .text_color(p.foreground)
            .child(title)
            .child(strip)
            .when_some(on_select, |el, f| {
                el.child(pager::follow_surface(
                    pager_state,
                    sw,
                    selected,
                    selected > 0,
                    selected + 1 < len,
                    Rc::new(move |step, w, cx| {
                        let next = if step > 0 { selected + 1 } else { selected.wrapping_sub(1) };
                        if next < len {
                            f(next, w, cx);
                        }
                    }),
                ))
            })
    }
}
