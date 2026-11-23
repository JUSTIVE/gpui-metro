//! The catalog view: top bar, sidebar, and the selected page with its phone
//! frame and usage snippet. All demo state lives here so it survives browsing.

use std::time::Duration;

use gpui::{
    AppContext, Context, Entity, StyleRefinement, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Subscription,
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
    /// The page turn in progress, driven by its own clock so the page keeps
    /// one element tree (scroll, component state) from entrance to exit.
    turn: Option<motion::Turnstile>,
    /// A start screen's exit (its tiles' wave) ends on this timer.
    _leave: Option<Task<()>>,
    _looper: Task<()>,
    pub query: SharedString,
    pub demo: DemoState,
    /// The index and the top bar are views of their own, embedded cached:
    /// while the phone animates, they're reused instead of laid out again.
    sidebar: Entity<sidebar::Sidebar>,
    topbar: Entity<topbar::TopBar>,
    _subscriptions: Vec<Subscription>,
}

impl Catalog {
    pub fn new(page: Page, _: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| TextInput::new("search components", cx));
        let filter = cx.subscribe(&search, |this, input, e: &InputEvent, cx| {
            if let InputEvent::Changed = e {
                this.query = input.read(cx).text().to_string().into();
                this.sync_sidebar(cx);
                cx.notify();
            }
        });
        let demo = DemoState::new(cx);
        let catalog = cx.entity().downgrade();
        let sidebar = cx.new(|cx| sidebar::Sidebar::new(page, search, catalog, cx));
        let topbar = cx.new(topbar::TopBar::new);
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
            turn: Some(motion::Turnstile::enter(Direction::Forward)),
            _leave: None,
            query: SharedString::default(),
            demo,
            sidebar,
            topbar,
            _subscriptions: vec![filter, submit, echo],
        }
    }

    /// Navigates the phone to `page` the Metro way: the current page
    /// turnstiles out (a start screen's tiles swing away instead), then the
    /// new one turnstiles in.
    /// Tells the cached sidebar what to show (it redraws only on a change).
    fn sync_sidebar(&self, cx: &mut Context<Self>) {
        let (page, query) = (self.page, self.query.clone());
        self.sidebar.update(cx, |s, cx| s.show(page, &query, cx));
    }

    pub fn go(&mut self, page: Page, cx: &mut Context<Self>) {
        if page == self.page {
            return;
        }
        // Mark the new page in the index at once, even while the old one leaves.
        let query = self.query.clone();
        self.sidebar.update(cx, |s, cx| s.show(page, &query, cx));
        if Theme::get(cx).reduced_motion {
            self.page = page;
            cx.notify();
            return;
        }
        // Mid-transition, the page on screen is whichever is leaving.
        let old = self.leaving.unwrap_or(self.page);
        self.page = page;
        if is_start_screen(old) {
            // The tiles swing away by themselves; wait for the wave.
            self.leaving = Some(old);
            self.turn = None;
            self._leave = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(GRID_EXIT).await;
                this.update(cx, |this, cx| {
                    this.leaving = None;
                    this.turn = (!is_start_screen(this.page)).then(|| motion::Turnstile::enter(Direction::Forward));
                    cx.notify();
                })
                .ok();
            }));
        } else if self.leaving.is_none() {
            self.leaving = Some(old);
            self.turn = Some(motion::Turnstile::exit(Direction::Forward));
        }
        cx.notify();
    }

    /// Advances the page turn: an exit hands over to the new page's
    /// entrance, an entrance settles. Keeps frames coming while it runs.
    fn step_turn(&mut self, window: &mut Window) {
        let Some(turn) = self.turn else { return };
        if !turn.done() {
            window.request_animation_frame();
            return;
        }
        if turn.is_exit() {
            self.leaving = None;
            self.turn = (!is_start_screen(self.page)).then(|| motion::Turnstile::enter(Direction::Forward));
            window.request_animation_frame();
        } else {
            self.turn = None;
        }
    }

    fn main_area(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Theme::palette(cx);
        self.step_turn(window);
        let page = self.page;
        // While the old page leaves, the phone still shows it.
        let shown = self.leaving.unwrap_or(page);
        self.demo.leaving = self.leaving.is_some();
        let mut demo = demos::render(shown, &self.demo, window, cx);
        if !is_start_screen(shown) {
            // Always the same structure, turned or at rest, so the page's
            // element tree survives its own entrance and exit.
            let body = div().size_full().child(demo.body);
            demo.body = match &self.turn {
                Some(turn) => turn.frame(body, cx),
                None => motion::turned(0., body, cx),
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
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(p.background)
            .text_color(p.foreground)
            .font_family(Theme::font(cx))
            .child(self.topbar.clone().cached(StyleRefinement::default().w_full().h(u(topbar::HEIGHT)).flex_none()))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        self.sidebar
                            .clone()
                            .cached(StyleRefinement::default().w(u(sidebar::WIDTH)).h_full().flex_none()),
                    )
                    .child(self.main_area(window, cx)),
            )
    }
}
