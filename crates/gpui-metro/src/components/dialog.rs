//! Interruptions. The message box is a full-width chrome band pinned to the
//! top of the screen over a dimmed page; the toast is an accent strip that
//! drops in from the top edge. Both are rendered by the caller while shown.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder,
};

use crate::theme::u;

use crate::{
    Icon, Theme,
    button::{ClickHandler, button},
    chrome::PAGE_MARGIN,
    motion::swivel_in,
    theme::{SEMIBOLD, size},
};

/// A modal message box. Place it as the last child of a `relative()` root;
/// it covers the root and blocks the page beneath.
#[derive(IntoElement)]
pub struct MessageBox {
    id: ElementId,
    title: SharedString,
    message: Option<SharedString>,
    content: Option<AnyElement>,
    buttons: Vec<(SharedString, ClickHandler)>,
    on_dismiss: Option<ClickHandler>,
}

pub fn message_box(id: impl Into<ElementId>, title: impl Into<SharedString>) -> MessageBox {
    MessageBox { id: id.into(), title: title.into(), message: None, content: None, buttons: Vec::new(), on_dismiss: None }
}

impl MessageBox {
    pub fn message(mut self, m: impl Into<SharedString>) -> Self {
        self.message = Some(m.into());
        self
    }
    /// Arbitrary body below the message (a check box, a list picker…).
    pub fn child(mut self, e: impl IntoElement) -> Self {
        self.content = Some(e.into_any_element());
        self
    }
    /// Up to two buttons, laid out side by side, first on the left.
    pub fn button(
        mut self,
        label: impl Into<SharedString>,
        f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.buttons.push((label.into(), Rc::new(f)));
        self
    }
    /// Tapping the dimmed page outside the box.
    pub fn on_dismiss(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for MessageBox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let buttons = self.buttons.into_iter().enumerate().map(|(ix, (label, f))| {
            div().flex_1().child(button(("msgbox-btn", ix), label).full_width().on_click(move |e, w, cx| f(e, w, cx)))
        });
        let band = div()
            .id("msgbox-band")
            .w_full()
            .bg(p.chrome)
            .px(u(PAGE_MARGIN))
            .pt(u(40.))
            .pb(u(20.))
            .flex()
            .flex_col()
            .gap(u(14.))
            .font_family(Theme::font(cx))
            .text_color(p.foreground)
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(div().text_size(u(size::LARGE)).font_weight(SEMIBOLD).child(self.title))
            .when_some(self.message, |el, m| el.child(div().text_size(u(size::MEDIUM)).child(m)))
            .children(self.content)
            .child(div().flex().gap(u(12.)).pt(u(8.)).children(buttons));

        div()
            .id(self.id)
            .absolute()
            .inset_0()
            .bg(p.scrim)
            .occlude()
            .when_some(self.on_dismiss, |el, f| el.on_click(move |e, w, cx| f(e, w, cx)))
            .child(swivel_in("msgbox-in", div().w_full().child(band), window, cx))
    }
}

/// A toast notification. It swivels in once per `id`: render a newer toast
/// of the same kind under the same id and it updates in place instead of
/// interrupting again.
#[derive(IntoElement)]
pub struct Toast {
    id: ElementId,
    title: SharedString,
    message: Option<SharedString>,
    icon: Option<Icon>,
    on_click: Option<ClickHandler>,
}

pub fn toast(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Toast {
    Toast { id: id.into(), title: title.into(), message: None, icon: None, on_click: None }
}

impl Toast {
    pub fn message(mut self, m: impl Into<SharedString>) -> Self {
        self.message = Some(m.into());
        self
    }
    pub fn icon(mut self, i: Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for Toast {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let ink = p.accent_ink;
        let band = div()
            .id(self.id.clone())
            .w_full()
            .flex()
            .items_center()
            .gap(u(10.))
            .px(u(16.))
            .h(u(60.))
            .bg(p.accent)
            .font_family(Theme::font(cx))
            .text_size(u(size::NORMAL))
            .text_color(ink)
            .children(self.icon.map(|i| i.render(u(22.), ink)))
            .child(div().font_weight(SEMIBOLD).flex_none().child(self.title))
            .when_some(self.message, |el, m| {
                el.child(div().flex_1().min_w_0().overflow_hidden().whitespace_nowrap().text_ellipsis().child(m))
            })
            .when_some(self.on_click, |el, f| el.cursor_pointer().on_click(move |e, w, cx| f(e, w, cx)));
        div().absolute().top_0().left_0().right_0().child(swivel_in(
            ElementId::from((SharedString::from(format!("{}-in", self.id)), 0usize)),
            div().w_full().child(band),
            window,
            cx,
        ))
    }
}
