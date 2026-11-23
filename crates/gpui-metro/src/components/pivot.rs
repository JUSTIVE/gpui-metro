//! Pivot: a row of huge lower-case headers over one pane of content. The
//! selected header is always leftmost; the others follow it in order and
//! wrap around, dimmed. Tap a header, Shift + wheel, or swipe sideways to move.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::{
    Theme, TextStyle,
    chrome::PAGE_MARGIN,
    motion::{Direction, enter, enter_div, generation_id, track},
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

        let headers = rotated(len, selected).into_iter().map(|ix| {
            let is_sel = ix == selected;
            let on_select = self.on_select.clone();
            div()
                .id(("pivot-header", ix))
                .flex_none()
                .text_size(px(size::HUGE * 0.86))
                .font_weight(LIGHT)
                .text_color(if is_sel { p.foreground } else { p.subtle })
                .cursor_pointer()
                .child(SharedString::from(self.headers[ix].to_lowercase()))
                .when_some(on_select.filter(|_| !is_sel), |el, f| el.on_click(move |_, w, cx| f(ix, w, cx)))
        });

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
                    .child(
                        div()
                            .flex()
                            .gap(px(22.))
                            .ml(px(-2.))
                            .overflow_hidden()
                            .child(enter(
                                generation_id(&format!("{key}-h"), generation),
                                direction,
                                0,
                                div().flex().gap(px(22.)).children(headers),
                                cx,
                            )),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .pt(px(8.))
                    .child(enter_div(
                        generation_id(&format!("{key}-c"), generation),
                        direction,
                        1,
                        div().size_full().child(content),
                        cx,
                    )),
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
    fn direction_wraps_the_short_way() {
        assert!(is_forward(0, 1, 4));
        assert!(!is_forward(1, 0, 4));
        assert!(is_forward(3, 0, 4), "last → first is a forward swipe");
        assert!(!is_forward(0, 3, 4));
    }
}
