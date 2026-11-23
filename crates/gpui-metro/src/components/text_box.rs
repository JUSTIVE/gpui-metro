//! The Metro text box: a translucent field that turns solid white with black
//! ink when it has focus, framed by a 2 px border that takes the accent.

use gpui::{App, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, prelude::FluentBuilder};

use crate::theme::u;

use crate::{Theme, text_input::TextInput, theme::size};

#[derive(IntoElement)]
pub struct TextBox {
    input: Entity<TextInput>,
    header: Option<SharedString>,
}

/// Wraps a [`TextInput`] entity (create it with `cx.new(|cx| TextInput::new("placeholder", cx))`).
pub fn text_box(input: &Entity<TextInput>) -> TextBox {
    TextBox { input: input.clone(), header: None }
}

impl TextBox {
    pub fn header(mut self, h: impl Into<SharedString>) -> Self {
        self.header = Some(h.into());
        self
    }
}

impl RenderOnce for TextBox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let focused = self.input.read(cx).is_focused(window);
        let (bg, ink, border) = if focused {
            (p.input_focused, p.input_ink, p.accent)
        } else {
            (p.input, p.input_ink, p.input)
        };
        // Colours the engine paints itself; set without notifying (render-time sync).
        let accent = p.accent;
        self.input.update(cx, |i, _| {
            i.selection_color = accent.opacity(0.45);
            i.placeholder_color = gpui::black().opacity(0.45);
            i.caret_color = Some(gpui::black());
        });
        div()
            .flex()
            .flex_col()
            .gap(u(4.))
            .font_family(Theme::font(cx))
            .when_some(self.header, |el, h| el.child(div().text_size(u(size::NORMAL)).text_color(p.subtle).child(h)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(u(48.))
                    .px(u(10.))
                    .bg(bg)
                    .border_2()
                    .border_color(border)
                    .text_size(u(size::MEDIUM))
                    .text_color(ink)
                    .child(self.input),
            )
    }
}
