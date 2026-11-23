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
/// The hub opening: the sections fly in from the right, the title (further
/// back) less far, both decelerating into place.
const INTRO: Duration = Duration::from_millis(650);
const INTRO_SECTIONS: f32 = 360.;
const INTRO_TITLE: f32 = INTRO_SECTIONS * PARALLAX;

#[derive(Clone, Copy)]
enum Motion {
    /// The content trails the input, at this position.
    Following(f32),
    /// Settling (key, from, to), in section positions. `to` may lie past
    /// either end when the panorama wraps around.
    Settling(u64, f32, f32),
    Rest(f32),
}

/// Where section `ix` sits on the endless strip when the view is at `pos`:
/// with wrapping, the copy of it nearest the screen (the screen shows `pos`
/// and the peek of `pos + 1`), so the last section is followed by the first.
pub fn section_slot(ix: usize, pos: f32, len: usize, wraps: bool) -> f32 {
    if !wraps || len == 0 {
        return ix as f32;
    }
    let l = len as f32;
    // Centred a little left of the peek, so the slots never tie: the strip
    // a section width either side of `pos` (its left margin shows a sliver
    // of the one before) through the peek is always covered.
    ix as f32 + l * ((pos + 0.45 - ix as f32) / l).round()
}

/// The position to settle at for `selected`, coming from `from`: the copy
/// of it nearest `from`, so a wrap continues the way it was going.
pub fn settle_target(from: f32, selected: usize, len: usize, wraps: bool) -> f32 {
    if !wraps || len == 0 {
        return selected as f32;
    }
    let l = len as f32;
    selected as f32 + l * ((from - selected as f32) / l).round()
}

/// Positions a layer for `motion`; `left` maps a view position to the
/// layer's left edge. A settle runs `from → to` (`to` overridden by `to`).
fn place(
    layer: gpui::AnyElement,
    id: ElementId,
    motion: Motion,
    to: Option<f32>,
    left: impl Fn(f32) -> gpui::Rems + 'static,
) -> AnyElement {
    let wrap = div().absolute().top_0().bottom_0().child(layer);
    match motion {
        Motion::Following(pos) | Motion::Rest(pos) => wrap.left(left(pos)).into_any_element(),
        Motion::Settling(_, from, end) => {
            let end = to.unwrap_or(end);
            wrap.with_animation(id, Animation::new(SETTLE).with_easing(ease_out_exp6), move |el, t| {
                el.left(left(lerp(from, end, t)))
            })
            .into_any_element()
        }
    }
}

/// A layer that flies in from `distance` px to the right when the panorama
/// first appears.
fn intro(layer: gpui::Div, id: ElementId, distance: f32, reduced: bool) -> AnyElement {
    if reduced {
        return layer.into_any_element();
    }
    layer
        .with_animation(id, Animation::new(INTRO).with_easing(ease_out_exp6), move |el, t| {
            el.left(u(distance * (1. - t)))
        })
        .into_any_element()
}

impl RenderOnce for Panorama {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let reduced = Theme::get(cx).reduced_motion;
        let len = self.sections.len();
        // Hubs wrap around: past the last section comes the first. (With
        // two, both neighbours would be the same section, so they don't.)
        let wraps = len >= 3;
        let selected = self.selected.min(len.saturating_sub(1));
        let key = format!("{}-pano", self.id);
        let (prev, generation) = track(ElementId::from(SharedString::from(key.clone())), selected, window, cx);
        let sw = self.section_width;
        let pager_state = window.use_keyed_state(
            ElementId::from(SharedString::from(format!("{key}-pager"))),
            cx,
            |_, cx| Pager::new(cx),
        );
        // Where the view is (in sections), and how it is moving: following
        // the input, settling into a section, or at rest.
        let motion = pager_state.update(cx, |pg, _| {
            if generation != pg.seen_generation {
                pg.seen_generation = generation;
                let release = pg.release.take().unwrap_or(0.);
                let from = prev as f32 - release;
                pg.settle_to(from, settle_target(from, selected, len, wraps));
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
        let settle_key = match motion {
            Motion::Settling(k, ..) => k as usize,
            _ => 0,
        };

        let title_left = move |pos: f32| u(PAGE_MARGIN - 6. - pos * sw * PARALLAX);
        let title = div()
            .whitespace_nowrap()
            .text_size(u(size::PANORAMA))
            .font_weight(LIGHT)
            .text_color(p.foreground)
            .child(SharedString::from(self.title.to_lowercase()))
            .into_any_element();
        // On a wrap the title doesn't go round: it runs back to its start
        // (to the selected section itself, not its copy past the end).
        let title = place(
            title,
            ElementId::from((SharedString::from(format!("{key}-t")), settle_key)),
            motion,
            Some(selected as f32),
            title_left,
        );
        let title = intro(
            div().absolute().top(u(-18.)).left_0().right_0().h(u(140.)).child(title),
            ElementId::from(SharedString::from(format!("{key}-intro-t"))),
            INTRO_TITLE,
            reduced,
        );

        let on_select = self.on_select.clone();
        let sections: Vec<AnyElement> = self
            .sections
            .into_iter()
            .enumerate()
            .map(|(ix, s)| {
                let on_select = on_select.clone();
                let section = div()
                    .id(("pano-section", ix))
                    .flex()
                    .flex_col()
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
                    .into_any_element();
                let left = move |pos: f32| u(PAGE_MARGIN + (section_slot(ix, pos, len, wraps) - pos) * sw);
                // At rest, a section wrapped round to the left of the
                // current one is only a sliver in the margin; hide it, or
                // its overflow shows there and the opening slides it in.
                let behind = matches!(motion, Motion::Rest(_))
                    && section_slot(ix, selected as f32, len, wraps) < selected as f32 - 0.5;
                let section = if behind {
                    div().invisible().child(section).into_any_element()
                } else {
                    section
                };
                place(
                    section,
                    ElementId::from((SharedString::from(format!("{key}-s{ix}")), settle_key)),
                    motion,
                    None,
                    left,
                )
            })
            .collect();
        let strip = intro(
            div().absolute().top(u(132.)).bottom_0().left_0().right_0().children(sections),
            ElementId::from(SharedString::from(format!("{key}-intro-s"))),
            INTRO_SECTIONS,
            reduced,
        );
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
            .when_some(on_select.filter(|_| len > 1), |el, f| {
                el.child(pager::follow_surface(
                    pager_state,
                    sw,
                    selected,
                    wraps || selected > 0,
                    wraps || selected + 1 < len,
                    Rc::new(move |step, w, cx| {
                        let next = if step > 0 { selected + 1 } else { selected + len - 1 };
                        if wraps || (step > 0 && next < len) || (step < 0 && selected > 0) {
                            f(next % len, w, cx);
                        }
                    }),
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_section_follows_the_last() {
        // At the last of three, the first sits just past it (peeking in).
        assert_eq!(section_slot(0, 2., 3, true), 3.);
        assert_eq!(section_slot(1, 2., 3, true), 1.);
        // At the first, the last waits to its left.
        assert_eq!(section_slot(2, 0., 3, true), -1.);
        assert_eq!(section_slot(1, 0., 3, true), 1.);
        // Without wrapping, sections stay put.
        assert_eq!(section_slot(0, 2., 3, false), 0.);
    }

    #[test]
    fn a_wrap_settles_onward_not_back() {
        // From just past the last section on to the first: keep going right.
        assert_eq!(settle_target(2.3, 0, 3, true), 3.);
        // From just before the first back to the last: keep going left.
        assert_eq!(settle_target(-0.3, 2, 3, true), -1.);
        assert_eq!(settle_target(0.3, 1, 3, true), 1.);
        assert_eq!(settle_target(2.3, 0, 3, false), 0.);
    }
}
