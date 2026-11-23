//! The phone: system tray, the page on top of the back stack, its app bar,
//! the hardware buttons, and whatever is interrupting (message box, toast).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{
    App, ClipboardItem, FocusHandle, KeyBinding, SharedString, Subscription, Task, Window, actions,
    div, prelude::*, px,
};
use gpui_metro::{Icon, Theme, message_box, motion, motion::Direction, system_tray, toast};

use crate::data::{seed, to_fixed};
use crate::nav::{IncidentForm, Nav, Page, PageView};
use crate::pages;
use crate::store::Store;

actions!(no_ativ, [GoBack]);

const CONTEXT: &str = "NoAtiv";
/// Telemetry interval.
const TICK: Duration = Duration::from_secs(2);
const TOAST_FOR: Duration = Duration::from_secs(3);

pub fn bind_keys(cx: &mut App) {
    // The text box binds backspace itself, deeper in the key context, so
    // these only fire when no text box has focus.
    cx.bind_keys([
        KeyBinding::new("escape", GoBack, Some(CONTEXT)),
        KeyBinding::new("backspace", GoBack, Some(CONTEXT)),
    ]);
}

pub enum Dialog {
    Harvest {
        run_id: String,
    },
    Notice {
        title: SharedString,
        message: SharedString,
    },
}

struct ActiveToast {
    title: SharedString,
    message: SharedString,
    icon: Icon,
    serial: usize,
}

pub struct NoAtiv {
    pub store: gpui::Entity<Store>,
    stack: Vec<Page>,
    /// Bumps on every push and pop so entrances replay.
    generation: usize,
    direction: Direction,
    dialog: Option<Dialog>,
    toast: Option<ActiveToast>,
    toasts: usize,
    focus: FocusHandle,
    _toast_timer: Option<Task<()>>,
    _ticker: Task<()>,
    _store_changed: Subscription,
}

impl NoAtiv {
    /// `boot` is the page to open over the start screen (for `NO_ATIV_PAGE`).
    pub fn new(boot: Option<&str>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = cx.new(|_| Store::new(seed()));
        let ticker = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                let ticked = this.update(cx, |app, cx| {
                    app.store.update(cx, |s, cx| {
                        s.tick();
                        cx.notify();
                    })
                });
                if ticked.is_err() {
                    break;
                }
            }
        });
        let focus = cx.focus_handle();
        window.focus(&focus);
        let mut app = Self {
            _store_changed: cx.observe(&store, |_, _, cx| cx.notify()),
            store,
            stack: vec![Page::Start],
            generation: 0,
            direction: Direction::Forward,
            dialog: None,
            toast: None,
            toasts: 0,
            focus,
            _toast_timer: None,
            _ticker: ticker,
        };
        if let Some(page) = boot.and_then(|name| app.boot_page(name, cx)) {
            app.stack.push(page);
        }
        app
    }

    fn boot_page(&self, name: &str, cx: &mut App) -> Option<Page> {
        let store = self.store.read(cx);
        let first_live = store.live_runs().next().map(|r| r.id.clone());
        let page = match name {
            "runs" => Page::Runs { pivot: 0 },
            "run" => Page::Run { id: first_live? },
            "telemetry" => Page::Telemetry { zone: 0 },
            "strains" => Page::Strains,
            "strain" => Page::Strain {
                id: store.data.strains.first()?.id.clone(),
            },
            "incidents" => Page::Incidents { pivot: 0 },
            "incident" => Page::Incident {
                id: store.open_incidents().next()?.id.clone(),
            },
            "new-incident" => {
                let choices = store.live_runs().map(|r| r.id.clone()).collect();
                Page::NewIncident(Box::new(IncidentForm::new(choices, None, cx)))
            }
            "reagents" => Page::Reagents { pivot: 0 },
            "shipments" => Page::Shipments { pivot: 0 },
            "freezer" => Page::Freezer,
            "deck" => Page::Deck { pivot: 0 },
            "people" => Page::People { pivot: 0 },
            "settings" => Page::Settings,
            _ => return None,
        };
        Some(page)
    }

    /* navigation --------------------------------------------------------- */

    pub fn push(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.stack.push(page);
        self.navigated(Direction::Forward, window, cx);
    }

    /// The hardware back button: dismisses the keyboard, then a message box,
    /// then the page.
    pub fn back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.focused(cx).is_some_and(|f| f != self.focus) {
            window.focus(&self.focus);
        } else if self.dialog.take().is_none() {
            self.pop(window, cx);
        }
        cx.notify();
    }

    pub fn pop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.stack.len() > 1 {
            self.stack.pop();
            self.navigated(Direction::Backward, window, cx);
        }
    }

    /// Opens the new-incident form, optionally pre-linked to a run.
    pub fn new_incident(
        &mut self,
        run_id: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let choices = self
            .store
            .read(cx)
            .live_runs()
            .map(|r| r.id.clone())
            .collect();
        let form = IncidentForm::new(choices, run_id, cx);
        self.push(Page::NewIncident(Box::new(form)), window, cx);
    }

    /// The start button: straight back to the tiles.
    pub fn home(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        if self.stack.len() > 1 {
            self.stack.truncate(1);
            self.navigated(Direction::Backward, window, cx);
        }
        cx.notify();
    }

    fn navigated(&mut self, direction: Direction, window: &mut Window, cx: &mut Context<Self>) {
        self.generation += 1;
        self.direction = direction;
        window.focus(&self.focus);
        cx.notify();
    }

    pub fn top(&self) -> &Page {
        self.stack.last().expect("the start screen is never popped")
    }

    pub fn update_page(&mut self, f: impl FnOnce(&mut Page)) {
        if let Some(page) = self.stack.last_mut() {
            f(page);
        }
    }

    /* interruptions ------------------------------------------------------ */

    pub fn show(&mut self, dialog: Dialog, cx: &mut Context<Self>) {
        self.dialog = Some(dialog);
        cx.notify();
    }

    /// The message box's "harvest": closes it, harvests, and says so.
    fn harvest(&mut self, run_id: &str, cx: &mut Context<Self>) {
        self.dialog = None;
        let result = self.store.update(cx, |s, cx| {
            cx.notify();
            s.harvest(run_id).map(|r| (r.code.clone(), r.titer.clone()))
        });
        match result {
            Ok((code, titer)) => {
                self.toast(Icon::Flask, code, format!("harvested · titer {titer}"), cx)
            }
            Err(e) => {
                let message = e.to_string().into();
                self.show(
                    Dialog::Notice {
                        title: "can't harvest".into(),
                        message,
                    },
                    cx,
                )
            }
        }
    }

    #[cfg(test)]
    pub fn has_dialog(&self) -> bool {
        self.dialog.is_some()
    }

    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.dialog = None;
        cx.notify();
    }

    pub fn toast(
        &mut self,
        icon: Icon,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.toasts += 1;
        let serial = self.toasts;
        self.toast = Some(ActiveToast {
            title: title.into(),
            message: message.into(),
            icon,
            serial,
        });
        self._toast_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TOAST_FOR).await;
            this.update(cx, |app, cx| {
                if app.toast.as_ref().is_some_and(|t| t.serial == serial) {
                    app.toast = None;
                    cx.notify();
                }
            })
            .ok();
        }));
        cx.notify();
    }

    pub fn copy(&mut self, text: String, cx: &mut Context<Self>) {
        self.toast(Icon::Check, "copied", text.clone(), cx);
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    fn render_dialog(&self, nav: &Nav, cx: &App) -> Option<gpui::AnyElement> {
        let dismiss = nav.click(|app, _, cx| app.dismiss(cx));
        let el = match self.dialog.as_ref()? {
            Dialog::Harvest { run_id } => {
                let store = self.store.read(cx);
                let run = store.run(run_id)?;
                let id = run_id.clone();
                message_box("harvest-box", format!("harvest {}?", run.code))
                    .message(format!(
                        "The culture leaves {} at OD600 {} and goes downstream. This can't be undone.",
                        run.vessel,
                        to_fixed(run.od600, 1)
                    ))
                    .button("harvest", nav.click(move |app, _, cx| app.harvest(&id, cx)))
                    .button("cancel", dismiss)
                    .on_dismiss(nav.click(|app, _, cx| app.dismiss(cx)))
            }
            Dialog::Notice { title, message } => message_box("notice-box", title.clone())
                .message(message.clone())
                .button("ok", nav.click(|app, _, cx| app.dismiss(cx)))
                .on_dismiss(nav.click(|app, _, cx| app.dismiss(cx))),
        };
        Some(el.into_any_element())
    }
}

/// `hh:mm` on the foundry floor (Seoul, UTC+9).
fn floor_clock() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        + 9 * 3600;
    format!("{}:{:02}", (secs / 3600) % 24, (secs / 60) % 60)
}

impl Render for NoAtiv {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = Theme::palette(cx);
        let nav = Nav::new(cx.entity().downgrade(), self.generation, self.direction);
        let PageView { body, app_bar } = pages::render(self.top(), self.store.read(cx), &nav, cx);
        let page_key = SharedString::from(format!("page-{}", self.generation));
        let body = motion::enter_div(
            page_key,
            self.direction,
            0,
            div().size_full().child(body),
            cx,
        );

        let hardware =
            |id: &'static str,
             icon: Icon,
             f: fn(&mut NoAtiv, &mut Window, &mut Context<NoAtiv>)| {
                div()
                    .id(id)
                    .flex()
                    .flex_1()
                    .h_full()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .active(|s| s.opacity(0.5))
                    .child(icon.render(px(22.), gpui::white().opacity(0.8)))
                    .on_click(nav.click(f))
            };

        div()
            .id("no-ativ")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|app, _: &GoBack, window, cx| app.back(window, cx)))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(p.background)
            .text_color(p.foreground)
            .font_family(Theme::font(cx))
            .child(system_tray(floor_clock()).battery(0.72))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .child(div().flex_1().min_h_0().overflow_hidden().child(body))
                    .children(app_bar),
            )
            .child(
                // The phone's capacitive keys: back and start.
                div()
                    .flex()
                    .flex_none()
                    .h(px(40.))
                    .bg(gpui::black())
                    .child(hardware("hw-back", Icon::Back, NoAtiv::back))
                    .child(hardware("hw-start", Icon::Home, NoAtiv::home)),
            )
            .children(self.render_dialog(&nav, cx))
            .children(self.toast.as_ref().map(|t| {
                toast(("toast", t.serial), t.title.clone())
                    .message(t.message.clone())
                    .icon(t.icon)
                    .on_click(nav.click(|app, _, cx| {
                        app.toast = None;
                        cx.notify();
                    }))
            }))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use gpui::{Focusable, TestAppContext, VisualTestContext};

    use super::*;
    use crate::data::RunPhase;

    const PAGES: [&str; 14] = [
        "runs",
        "run",
        "telemetry",
        "strains",
        "strain",
        "incidents",
        "incident",
        "new-incident",
        "reagents",
        "shipments",
        "freezer",
        "deck",
        "people",
        "settings",
    ];

    pub(crate) fn boot<'a>(
        page: Option<&'static str>,
        cx: &'a mut TestAppContext,
    ) -> (gpui::Entity<NoAtiv>, &'a mut VisualTestContext) {
        cx.update(|cx| {
            gpui_metro::init(cx);
            bind_keys(cx);
        });
        let (app, vcx) = cx.add_window_view(|window, cx| NoAtiv::new(page, window, cx));
        vcx.run_until_parked();
        (app, vcx)
    }

    pub(crate) fn depth(app: &gpui::Entity<NoAtiv>, cx: &mut VisualTestContext) -> usize {
        app.read_with(cx, |a, _| a.stack.len())
    }

    #[gpui::test]
    fn every_page_renders_and_back_returns_to_start(cx: &mut TestAppContext) {
        for page in PAGES {
            let (app, cx) = boot(Some(page), cx);
            assert_eq!(depth(&app, cx), 2, "{page} boots over the start screen");
            cx.simulate_keystrokes("escape");
            assert_eq!(depth(&app, cx), 1, "escape pops {page}");
            cx.simulate_keystrokes("backspace");
            assert_eq!(depth(&app, cx), 1, "the start screen is never popped");
        }
    }

    #[gpui::test]
    fn typing_in_the_form_does_not_navigate(cx: &mut TestAppContext) {
        let (app, cx) = boot(Some("new-incident"), cx);
        let input = app.read_with(cx, |a, _| match a.top() {
            Page::NewIncident(form) => form.title.clone(),
            _ => unreachable!(),
        });
        let focus = input.read_with(cx, |i, cx| i.focus_handle(cx));
        cx.update(|window, _| window.focus(&focus));
        cx.simulate_input("foam");
        cx.simulate_keystrokes("backspace");
        assert_eq!(input.read_with(cx, |i, _| i.text().to_string()), "foa");
        assert_eq!(depth(&app, cx), 2, "backspace edits the field");
        cx.simulate_keystrokes("escape");
        assert_eq!(depth(&app, cx), 2, "the first back dismisses the keyboard");
        cx.simulate_keystrokes("escape");
        assert_eq!(depth(&app, cx), 1);
    }

    #[gpui::test]
    fn harvest_from_the_message_box(cx: &mut TestAppContext) {
        let (app, cx) = boot(Some("run"), cx);
        let id = app.read_with(cx, |a, _| match a.top() {
            Page::Run { id } => id.clone(),
            _ => unreachable!(),
        });
        app.update(cx, |a, cx| {
            a.show(Dialog::Harvest { run_id: id.clone() }, cx)
        });
        cx.run_until_parked();
        app.update(cx, |a, cx| a.harvest(&id, cx));
        cx.run_until_parked();
        let (dialog, toast, phase) = app.read_with(cx, |a, cx| {
            (
                a.dialog.is_some(),
                a.toast.as_ref().map(|t| t.title.clone()),
                a.store.read(cx).run(&id).unwrap().phase,
            )
        });
        assert!(!dialog);
        assert_eq!(phase, RunPhase::Harvested);
        assert!(toast.is_some(), "a toast confirms it");

        app.update(cx, |a, cx| a.harvest(&id, cx));
        assert!(
            app.read_with(cx, |a, _| matches!(a.dialog, Some(Dialog::Notice { .. }))),
            "twice is refused"
        );
    }

    #[gpui::test]
    fn toasts_dismiss_themselves_and_telemetry_ticks(cx: &mut TestAppContext) {
        let (app, cx) = boot(None, cx);
        let before = app.read_with(cx, |a, cx| a.store.read(cx).run("r1").map(|r| r.od600));
        app.update(cx, |a, cx| a.toast(Icon::Check, "hello", "world", cx));
        assert!(app.read_with(cx, |a, _| a.toast.is_some()));
        cx.executor().advance_clock(TOAST_FOR + TICK);
        cx.run_until_parked();
        assert!(
            app.read_with(cx, |a, _| a.toast.is_none()),
            "gone after three seconds"
        );
        let after = app.read_with(cx, |a, cx| {
            a.store.read(cx).run("r1").map(|r| (r.od600, r.phase))
        });
        let (od, phase) = after.unwrap();
        assert!(
            od > before.unwrap() || phase != RunPhase::Log,
            "the ticker grew the culture"
        );
    }
}
