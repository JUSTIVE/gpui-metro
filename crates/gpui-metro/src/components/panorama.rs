//! Panorama: one long horizontal canvas. A title far wider than the screen
//! drifts behind sections that snap one at a time, with the next section
//! peeking in from the right edge. The title travels slower than the
//! sections — the parallax that made the hubs feel deep.

use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::{
    Theme,
    chrome::PAGE_MARGIN,
    motion::{ease_out_expo, generation_id, lerp, track},
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

impl RenderOnce for Panorama {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let reduced = Theme::get(cx).reduced_motion;
        let len = self.sections.len();
        let selected = self.selected.min(len.saturating_sub(1));
        let key = format!("{}-pano", self.id);
        let (prev, generation) = track(ElementId::from(SharedString::from(key.clone())), selected, window, cx);
        let sw = self.section_width;
        let animate = generation > 0 && !reduced;
        let dur = Duration::from_millis(480);

        let strip_left = move |pos: f32| px(PAGE_MARGIN - pos * sw);
        let title_left = move |pos: f32| px(PAGE_MARGIN - 6. - pos * sw * PARALLAX);
        let (from, to) = (prev as f32, selected as f32);

        let title = div()
            .absolute()
            .top(px(-18.))
            .whitespace_nowrap()
            .text_size(px(size::PANORAMA))
            .font_weight(LIGHT)
            .text_color(p.foreground)
            .child(SharedString::from(self.title.to_lowercase()));
        let title: AnyElement = if animate {
            title
                .with_animation(
                    generation_id(&format!("{key}-t"), generation),
                    Animation::new(dur).with_easing(ease_out_expo),
                    move |el, t| el.left(title_left(lerp(from, to, t))),
                )
                .into_any_element()
        } else {
            title.left(title_left(to)).into_any_element()
        };

        let on_select = self.on_select.clone();
        let sections = self.sections.into_iter().enumerate().map(|(ix, s)| {
            let on_select = on_select.clone();
            div()
                .id(("pano-section", ix))
                .flex()
                .flex_col()
                .flex_none()
                .w(px(sw))
                .pr(px(PAGE_MARGIN))
                .h_full()
                .child(
                    div()
                        .text_size(px(size::EXTRA_LARGE + 4.))
                        .font_weight(LIGHT)
                        .pb(px(10.))
                        .child(SharedString::from(s.header.to_lowercase())),
                )
                .child(div().flex_1().min_h_0().child(s.content))
                // Tapping the peeking section brings it in.
                .when_some(on_select.filter(|_| ix != selected), |el, f| {
                    el.cursor_pointer().on_click(move |_, w, cx| f(ix, w, cx))
                })
        });
        let strip = div().absolute().top(px(132.)).bottom_0().flex().children(sections);
        let strip: AnyElement = if animate {
            strip
                .with_animation(
                    generation_id(&format!("{key}-s"), generation),
                    Animation::new(dur).with_easing(ease_out_expo),
                    move |el, t| el.left(strip_left(lerp(from, to, t))),
                )
                .into_any_element()
        } else {
            strip.left(strip_left(to)).into_any_element()
        };

        let pager_state = window.use_keyed_state(
            ElementId::from(SharedString::from(format!("{key}-pager"))),
            cx,
            |_, cx| Pager::new(cx),
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
            .when_some(on_select, |el, f| {
                el.child(pager::surface(
                    pager_state,
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
