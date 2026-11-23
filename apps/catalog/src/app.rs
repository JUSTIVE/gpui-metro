//! The catalog view: top bar, sidebar, and the selected page with its phone
//! frame and usage snippet. All demo state lives here so it survives browsing.

use gpui::{
    AppContext, Context, Entity, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Subscription,
    Window, div, prelude::*, px,
};
use gpui_metro::{
    InputEvent, TextInput, TextStyle, Theme,
    motion::{self, Direction},
    text,
    theme::{LIGHT, size},
};

use crate::{demos, demos::DemoState, pages::Page, phone, sidebar, topbar};

const MONO: &str = if cfg!(target_os = "macos") { "Menlo" } else { "monospace" };

pub struct Catalog {
    pub page: Page,
    pub search: Entity<TextInput>,
    pub query: SharedString,
    pub demo: DemoState,
    _subscriptions: Vec<Subscription>,
}

impl Catalog {
    pub fn new(page: Page, _: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| TextInput::new("search components", cx));
        let filter = cx.subscribe(&search, |this, input, e: &InputEvent, cx| {
            if let InputEvent::Changed = e {
                this.query = input.read(cx).text().to_string().into();
                cx.notify();
            }
        });
        let demo = DemoState::new(cx);
        let submit = cx.subscribe(&demo.message_input, |this, input, e: &InputEvent, cx| {
            if let InputEvent::Submit = e {
                let sent = input.read(cx).text().trim().to_string();
                if !sent.is_empty() {
                    this.demo.submitted = Some(sent.into());
                    input.update(cx, |i, cx| i.set_text("", cx));
                    cx.notify();
                }
            }
        });
        // The text-box demo echoes what is typed into the name field.
        let echo = cx.subscribe(&demo.name_input, |_, _, e: &InputEvent, cx| {
            if let InputEvent::Changed = e {
                cx.notify();
            }
        });
        Self { page, search, query: SharedString::default(), demo, _subscriptions: vec![filter, submit, echo] }
    }

    fn main_area(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Theme::palette(cx);
        let page = self.page;
        let demo = demos::render(page, &self.demo, window, cx);
        let screen = phone::frame(&self.demo, demo, cx);

        let heading = div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .child(text(TextStyle::AppTitle, page.group().name()).color(p.accent))
            .child(div().text_size(px(size::HUGE)).font_weight(LIGHT).line_height(px(64.)).child(page.name()))
            .child(div().max_w(px(860.)).text_size(px(size::NORMAL)).text_color(p.subtle).child(page.description()));

        let snippet = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(260.))
            .max_w(px(520.))
            .gap(px(10.))
            .child(text(TextStyle::GroupHeader, "usage"))
            .child(
                div()
                    .p(px(18.))
                    .bg(p.chrome)
                    .font_family(MONO)
                    .text_size(px(13.))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(p.foreground)
                    .children(
                        page.snippet().lines().map(|line| div().h(px(20.)).whitespace_nowrap().child(line.to_string())),
                    ),
            );

        div().id("main").flex_1().min_w_0().h_full().overflow_y_scroll().child(
            div()
                .flex()
                .flex_col()
                .gap(px(28.))
                .px(px(40.))
                .pt(px(28.))
                .pb(px(48.))
                .child(motion::enter(
                    SharedString::from(format!("heading-{}", page.name())),
                    Direction::Forward,
                    0,
                    heading,
                    cx,
                ))
                .child(div().flex().flex_wrap().items_start().gap(px(36.)).child(screen).child(snippet)),
        )
    }
}

impl Render for Catalog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Theme::palette(cx);
        let sidebar = sidebar::render(self.page, &self.query, &self.search, cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(p.background)
            .text_color(p.foreground)
            .font_family(Theme::font(cx))
            .child(topbar::render(cx))
            .child(div().flex().flex_1().min_h_0().child(sidebar).child(self.main_area(window, cx)))
    }
}
