//! Pivot: a row of huge lower-case headers over one pane of content. The
//! selected header is always leftmost; the others follow it in order and
//! wrap around, dimmed. Tap a header, Shift + wheel, or swipe sideways to move.

use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, TextRun, Window, div, prelude::FluentBuilder, px,
};

use crate::{
    Theme, TextStyle,
    chrome::PAGE_MARGIN,
    motion::{Direction, ease_out_exp6, generation_id, slide_in, track},
    pager::{self, Pager},
    text,
    theme::{LIGHT, size},
};

pub type SelectHandler = Rc<dyn Fn(usize, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Pivot {
    id: ElementId,
    title: Option<SharedString>,
    headers: Vec<SharedString>,
    selected: usize,
    content: Option<AnyElement>,
    on_select: Option<SelectHandler>,
}

/// `pivot(id, ["all", "live", "harvested"], selected).child(pane_for(selected))`
pub fn pivot<S: Into<SharedString>>(
    id: impl Into<ElementId>,
    headers: impl IntoIterator<Item = S>,
    selected: usize,
) -> Pivot {
    let headers: Vec<SharedString> = headers.into_iter().map(Into::into).collect();
    let selected = selected.min(headers.len().saturating_sub(1));
    Pivot { id: id.into(), title: None, headers, selected, content: None, on_select: None }
}

impl Pivot {
    /// The application title above the headers (rendered upper-case).
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }
    /// The pane for the selected header.
    pub fn child(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }
    pub fn on_select(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

const HEADER_SIZE: f32 = size::HUGE * 0.86;
const HEADER_GAP: f32 = 22.;
/// Header glide and pane slide (WP pivot).
const GLIDE: std::time::Duration = std::time::Duration::from_millis(500);

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// How the header row moves from `prev` to `selected`: which headers it
/// lays out, in order, and the row offset at the start and end.
#[derive(Clone, Debug, PartialEq)]
pub struct HeaderGlide {
    pub order: Vec<usize>,
    pub from: f32,
    pub to: f32,
}

/// Forward: the row starts at `prev` and travels left until `selected`
/// leads. Backward: it starts already scrolled so `prev` leads, with the
/// new header just off the left edge, and travels right.
pub fn header_glide(widths: &[f32], prev: usize, selected: usize, direction: Direction) -> HeaderGlide {
    let len = widths.len();
    let span = |start: usize, count: usize| -> f32 {
        (0..count).map(|i| widths[(start + i) % len] + HEADER_GAP).sum()
    };
    if direction == Direction::Backward {
        let back = (prev + len - selected) % len;
        HeaderGlide { order: (0..len + back).map(|i| (selected + i) % len).collect(), from: -span(selected, back), to: 0. }
    } else {
        let ahead = (selected + len - prev) % len;
        HeaderGlide { order: (0..len + ahead).map(|i| (prev + i) % len).collect(), from: 0., to: -span(prev, ahead) }
    }
}

/// Header order with `selected` first, wrapping.
pub fn rotated(len: usize, selected: usize) -> Vec<usize> {
    (0..len).map(|i| (selected + i) % len.max(1)).collect()
}

/// Is moving from `from` to `to` a forward step (wrapping the short way)?
pub fn is_forward(from: usize, to: usize, len: usize) -> bool {
    if len == 0 {
        return true;
    }
    let ahead = (to + len - from) % len;
    ahead != 0 && ahead <= len / 2
}

impl RenderOnce for Pivot {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let len = self.headers.len();
        let selected = self.selected;
        let key = SharedString::from(format!("{}-pivot", self.id));
        let (prev, generation) = track(ElementId::from(key.clone()), selected, window, cx);
        let direction =
            if generation == 0 || is_forward(prev, selected, len) { Direction::Forward } else { Direction::Backward };

        let pager_state =
            window.use_keyed_state(ElementId::from(SharedString::from(format!("{key}-pager"))), cx, |_, cx| Pager::new(cx));

        // Headers glide: the row is laid out from where the old selection sat
        // and travels until the new one leads (Metro's pivot header motion).
        let font_size = px(HEADER_SIZE);
        let labels: Vec<SharedString> = self.headers.iter().map(|h| SharedString::from(h.to_lowercase())).collect();
        let font = gpui::Font { weight: LIGHT, ..gpui::font(Theme::font(cx)) };
        let widths: Vec<f32> = labels
            .iter()
            .map(|l| {
                let run = TextRun {
                    len: l.len(),
                    font: font.clone(),
                    color: Hsla::default(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                f32::from(window.text_system().shape_line(l.clone(), font_size, &[run], None).width)
            })
            .collect();
        let reduced = Theme::get(cx).reduced_motion;
        let glide = (generation > 0 && !reduced && len > 1).then(|| header_glide(&widths, prev, selected, direction));
        let order = match &glide {
            Some(g) => g.order.clone(),
            None => rotated(len, selected),
        };
        // Keyed by position: a header can appear twice in a gliding row.
        let header = |(pos, ix): (usize, usize)| {
            let is_sel = ix == selected;
            let on_select = self.on_select.clone();
            div()
                .id(("pivot-header", pos))
                .flex_none()
                .text_size(font_size)
                .font_weight(LIGHT)
                .text_color(if is_sel { p.foreground } else { p.subtle })
                .cursor_pointer()
                .child(labels[ix].clone())
                .when_some(on_select.filter(|_| !is_sel), |el, f| {
                    el.on_click(move |_, w, cx| f(ix, w, cx))
                })
        };
        let row = div().relative().flex().gap(px(HEADER_GAP)).children(order.into_iter().enumerate().map(header));
        let row: AnyElement = match glide {
            Some(g) => row
                .with_animation(
                    generation_id(&format!("{key}-h"), generation),
                    Animation::new(GLIDE).with_easing(ease_out_exp6),
                    move |el, t| el.left(px(lerp(g.from, g.to, t))),
                )
                .into_any_element(),
            None => row.into_any_element(),
        };

        let on_select = self.on_select.clone();
        let content = self.content.unwrap_or_else(|| div().into_any_element());
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .relative()
            .size_full()
            .overflow_hidden()
            .font_family(Theme::font(cx))
            .text_color(p.foreground)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_none()
                    .pt(px(12.))
                    .px(px(PAGE_MARGIN))
                    .when_some(self.title, |el, t| el.child(text(TextStyle::AppTitle, t)))
                    .child(div().ml(px(-2.)).overflow_hidden().child(row)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .pt(px(8.))
                    .child(if generation == 0 {
                        div().size_full().child(content).into_any_element()
                    } else {
                        slide_in(generation_id(&format!("{key}-c"), generation), direction, div().size_full().child(content), cx)
                    }),
            )
            .when_some(on_select.filter(|_| len > 1), |el, f| {
                el.child(pager::surface(
                    pager_state,
                    Rc::new(move |step, w, cx| {
                        let next = if step > 0 { (selected + 1) % len } else { (selected + len - 1) % len };
                        f(next, w, cx);
                    }),
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_header_leads() {
        assert_eq!(rotated(4, 0), vec![0, 1, 2, 3]);
        assert_eq!(rotated(4, 2), vec![2, 3, 0, 1]);
    }

    #[test]
    fn headers_glide_so_the_new_one_ends_up_leading() {
        let w = [100., 80., 120., 60.];
        let g = header_glide(&w, 0, 2, Direction::Forward);
        assert_eq!(g.order[..2], [0, 1]);
        assert_eq!(g.order[2], 2);
        assert_eq!((g.from, g.to), (0., -(100. + 22. + 80. + 22.)));
        let b = header_glide(&w, 0, 3, Direction::Backward);
        assert_eq!(b.order[0], 3, "the new header waits off the left edge");
        assert_eq!(b.order[1], 0);
        assert_eq!((b.from, b.to), (-(60. + 22.), 0.));
    }

    #[test]
    fn direction_wraps_the_short_way() {
        assert!(is_forward(0, 1, 4));
        assert!(!is_forward(1, 0, 4));
        assert!(is_forward(3, 0, 4), "last → first is a forward swipe");
        assert!(!is_forward(0, 3, 4));
    }
}
