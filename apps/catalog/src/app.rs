//! The catalog view: top bar, sidebar, and the selected page with its phone
//! frame and usage snippet. All demo state lives here so it survives browsing.

use std::time::Duration;

use gpui::{
    AppContext, Context, Entity, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Subscription,
    Task, Window, div, prelude::*,
};
use gpui_metro::u;
use gpui_metro::{
    InputEvent, TextInput, TextStyle, Theme,
    motion::{self, Direction},
    text,
    theme::{LIGHT, size},
};

use crate::{demos, demos::DemoState, pages::Page, phone, sidebar, topbar};

/// Pages that are a start screen: their tiles turnstile themselves.
fn is_start_screen(page: Page) -> bool {
    matches!(page, Page::TileGrid | Page::Turnstile)
}

/// How often the motion page replays its specimens.
const MOTION_LOOP: Duration = Duration::from_millis(2600);

/// A start screen's exit: the longest tile wave plus one tile's swing.
const GRID_EXIT: Duration = Duration::from_millis(620);

const MONO: &str = if cfg!(target_os = "macos") { "Menlo" } else { "monospace" };

pub struct Catalog {
    pub page: Page,
    /// The page swinging out while `page` waits to come in.
    leaving: Option<Page>,
    /// Bumps on every page change, so each entrance and exit plays afresh.
    visits: usize,
    _leave: Option<Task<()>>,
    _looper: Task<()>,
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
        // The motion page's specimens replay on their own, on a loop.
        let looper = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(MOTION_LOOP).await;
                let alive = this.update(cx, |this, cx| {
                    this.demo.motion_loop += 1;
                    if this.page == Page::Motion {
                        cx.notify();
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        });
        Self {
            page,
            leaving: None,
            _looper: looper,
            visits: 0,
            _leave: None,
            search,
            query: SharedString::default(),
            demo,
            _subscriptions: vec![filter, submit, echo],
        }
    }

    /// Navigates the phone to `page` the Metro way: the current page
    /// turnstiles out (a start screen's tiles swing away instead), then the
    /// new one turnstiles in.
    pub fn go(&mut self, page: Page, cx: &mut Context<Self>) {
        if page == self.page {
            return;
        }
        self.visits += 1;
        if Theme::get(cx).reduced_motion {
            self.page = page;
            cx.notify();
            return;
        }
        let old = self.leaving.unwrap_or(self.page);
        self.leaving = Some(old);
        self.page = page;
        let exit = if is_start_screen(old) { GRID_EXIT } else { motion::EXIT };
        self._leave = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(exit).await;
            this.update(cx, |this, cx| {
                this.leaving = None;
                this.visits += 1;
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
    }

    fn main_area(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Theme::palette(cx);
        let page = self.page;
        // While the old page leaves, the phone still shows it.
        let shown = self.leaving.unwrap_or(page);
        self.demo.leaving = self.leaving.is_some();
        let mut demo = demos::render(shown, &self.demo, window, cx);
        if !is_start_screen(shown) {
            let key = ("phone-page", self.visits);
            let body = div().size_full().child(demo.body);
            demo.body = if self.leaving.is_some() {
                motion::exit_div(key, Direction::Forward, body, cx)
            } else {
                motion::enter_div(key, Direction::Forward, 0, body, cx)
            };
        }
        let screen = phone::frame(&self.demo, demo, cx);

        let heading = div()
            .flex()
            .flex_col()
            .gap(u(6.))
            .child(text(TextStyle::AppTitle, page.group().name()).color(p.accent))
            .child(div().text_size(u(size::HUGE)).font_weight(LIGHT).line_height(u(64.)).child(page.name()))
            .child(div().max_w(u(860.)).text_size(u(size::NORMAL)).text_color(p.subtle).child(page.description()));

        let snippet = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(u(260.))
            .max_w(u(520.))
            .gap(u(10.))
            .child(text(TextStyle::GroupHeader, "usage"))
            .child(
                div()
                    .p(u(18.))
                    .bg(p.chrome)
                    .font_family(MONO)
                    .text_size(u(13.))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(p.foreground)
                    .children(
                        page.snippet().lines().map(|line| div().h(u(20.)).whitespace_nowrap().child(line.to_string())),
                    ),
            );

        div().id("main").flex_1().min_w_0().h_full().overflow_y_scroll().child(
            div()
                .flex()
                .flex_col()
                .gap(u(28.))
                .px(u(40.))
                .pt(u(28.))
                .pb(u(48.))
                .child(motion::enter(
                    SharedString::from(format!("heading-{}", page.name())),
                    Direction::Forward,
                    0,
                    heading,
                    cx,
                ))
                .child(div().flex().flex_wrap().items_start().gap(u(36.)).child(screen).child(snippet)),
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
