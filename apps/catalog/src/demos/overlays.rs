//! Message box and toast, both drawn over the phone screen.

use std::time::Duration;

use gpui::{Context, IntoElement, ParentElement, Styled, Task, div};
use gpui_metro::{Icon, TextStyle, button, check_box, message_box, text, toast};

use super::{Demo, DemoState, caption, on, page, stack, tap};
use crate::app::Catalog;

/// How long a toast stays up.
const TOAST_FOR: Duration = Duration::from_secs(3);

/// The toast on screen and the timer that will take it down. Replacing it
/// drops the task, which cancels the old timer.
pub struct ToastState {
    generation: usize,
    _dismiss: Task<()>,
}

pub fn message_box_page(s: &DemoState, cx: &mut Context<Catalog>) -> Demo {
    let body = page(
        "message box",
        stack()
            .child(text(TextStyle::Normal, "Remove a saved network from this phone."))
            .child(div().flex().child(button("mb-open", "forget network").on_click(tap(cx, |s| s.message_open = true))))
            .child(text(TextStyle::Accent, s.message_result.clone()))
            .child(caption(if s.dont_ask { "\"don't ask again\" is ticked" } else { "will ask every time" })),
    );
    let overlay = s.message_open.then(|| {
        let answer = |result: &'static str| {
            tap(cx, move |s| {
                s.message_result = result.into();
                s.message_open = false;
            })
        };
        message_box("mb", "forget home-5g?")
            .message("You'll need the password to join this network again.")
            .child(
                check_box("mb-dont-ask", s.dont_ask).label("don't ask again").on_change(on(cx, |s, v| s.dont_ask = v)),
            )
            .button("forget", answer("forgot home-5g"))
            .button("cancel", answer("kept home-5g"))
            .on_dismiss(answer("dismissed by tapping outside"))
            .into_any_element()
    });
    Demo { body, overlay }
}

fn show_toast(c: &mut Catalog, cx: &mut Context<Catalog>) {
    c.demo.toasts_sent += 1;
    let dismiss = cx.spawn(async move |this, cx| {
        cx.background_executor().timer(TOAST_FOR).await;
        this.update(cx, |c, cx| {
            c.demo.toast = None;
            cx.notify();
        })
        .ok();
    });
    c.demo.toast = Some(ToastState { generation: c.demo.toasts_sent, _dismiss: dismiss });
    cx.notify();
}

pub fn toast_page(s: &DemoState, cx: &mut Context<Catalog>) -> Demo {
    let body = page(
        "toast",
        stack()
            .child(text(TextStyle::Normal, "Toasts interrupt from the top edge and leave on their own."))
            .child(
                div()
                    .flex()
                    .child(button("toast-send", "send a toast").on_click(cx.listener(|c, _, _, cx| show_toast(c, cx)))),
            )
            .child(text(TextStyle::Accent, s.toast_log.clone()))
            .child(caption(format!(
                "{} sent · auto-dismiss after {} s · tap to open",
                s.toasts_sent,
                TOAST_FOR.as_secs()
            ))),
    );
    let overlay = s.toast.as_ref().map(|t| {
        let n = t.generation;
        // One kind of toast: a newer one updates it in place.
        toast("toast-kristin", "Kristin")
            .icon(Icon::Mail)
            .message(format!("are we still on for tonight? ({n})"))
            .on_click(tap(cx, move |s| {
                s.toast = None;
                s.toast_log = format!("opened message {n}").into();
            }))
            .into_any_element()
    });
    Demo { body, overlay }
}
