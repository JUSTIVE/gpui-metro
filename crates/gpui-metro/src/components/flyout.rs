//! The settings flyout (Windows 8's `SettingsFlyout`): a full-height panel
//! that slides in from the right edge over the page, under an accent header
//! with a back button and a title. The page isn't dimmed; tapping anywhere
//! outside the panel dismisses it (light dismiss).
//!
//! Render it every frame with whether it should be open: it slides in when
//! `open` turns true and slides back out by itself when it turns false, so
//! the caller never has to keep a closing panel on screen.

use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder,
};

use crate::theme::u;

use crate::{
    Icon, Theme,
    button::ClickHandler,
    motion::{ease_in_exp6, ease_out_exp6},
    theme::{SEMILIGHT, size},
};

/// The two widths Windows 8 allowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlyoutWidth {
    /// 346 px: the usual settings pane.
    Narrow,
    /// 646 px: room for richer content.
    Wide,
}

impl FlyoutWidth {
    pub fn px(self) -> f32 {
        match self {
            FlyoutWidth::Narrow => 346.,
            FlyoutWidth::Wide => 646.,
        }
    }
}

/// Height of the accent header.
pub const FLYOUT_HEADER: f32 = 80.;
/// Sliding in: WinJS `showPanel`, a long hard deceleration.
pub const FLYOUT_IN: Duration = Duration::from_millis(550);
/// Sliding out: quicker, accelerating away.
pub const FLYOUT_OUT: Duration = Duration::from_millis(250);

/// A settings flyout. Place it as the last child of a `relative()` root.
#[derive(IntoElement)]
pub struct SettingsFlyout {
    id: ElementId,
    open: bool,
    title: SharedString,
    width: FlyoutWidth,
    content: Vec<AnyElement>,
    on_dismiss: Option<ClickHandler>,
    on_back: Option<ClickHandler>,
}

/// `settings_flyout("options", self.options_open).title("options").child(…).on_dismiss(…)`
pub fn settings_flyout(id: impl Into<ElementId>, open: bool) -> SettingsFlyout {
    SettingsFlyout {
        id: id.into(),
        open,
        title: SharedString::default(),
        width: FlyoutWidth::Narrow,
        content: Vec::new(),
        on_dismiss: None,
        on_back: None,
    }
}

impl SettingsFlyout {
    /// The header title (rendered lower-case, as Metro set it).
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }
    pub fn width(mut self, width: FlyoutWidth) -> Self {
        self.width = width;
        self
    }
    /// Content under the header; it scrolls if it runs long.
    pub fn child(mut self, e: impl IntoElement) -> Self {
        self.content.push(e.into_any_element());
        self
    }
    pub fn children(mut self, es: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.content.extend(es.into_iter().map(IntoElement::into_any_element));
        self
    }
    /// Light dismiss: a tap outside the panel. Set `open` false in response.
    pub fn on_dismiss(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(f));
        self
    }
    /// The header's back button (on Windows 8, back to the settings pane).
    /// Without it, back dismisses.
    pub fn on_back(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_back = Some(Rc::new(f));
        self
    }
}

/// Whether the panel was last asked to be open, and since when.
struct Shown {
    open: bool,
    since: Instant,
}

/// How far the panel sits off its resting place, as a fraction of its
/// width (0 = in place, 1 = just off the right edge), `elapsed` after it
/// was asked to open or close; `None` once it has closed.
pub fn flyout_offset(open: bool, elapsed: Duration) -> Option<f32> {
    if open {
        let t = elapsed.as_secs_f32() / FLYOUT_IN.as_secs_f32();
        Some(1. - ease_out_exp6(t.min(1.)))
    } else {
        let t = elapsed.as_secs_f32() / FLYOUT_OUT.as_secs_f32();
        (t < 1.).then(|| ease_in_exp6(t))
    }
}

impl RenderOnce for SettingsFlyout {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let reduced = Theme::get(cx).reduced_motion;
        let open = self.open;
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Shown {
            open,
            // Mounted closed: already long gone, so nothing slides out.
            since: if open { Instant::now() } else { Instant::now() - FLYOUT_OUT * 2 },
        });
        let since = state.update(cx, |s, _| {
            if s.open != open {
                *s = Shown { open, since: Instant::now() };
            }
            s.since
        });
        let offset = if reduced { open.then_some(0.) } else { flyout_offset(open, since.elapsed()) };
        let Some(offset) = offset else {
            return div().into_any_element();
        };
        let settled = (open && offset == 0.) || reduced;
        if !settled {
            window.request_animation_frame();
        }

        let p = Theme::palette(cx);
        let ink = p.accent_ink;
        let w = self.width.px();
        let back = self.on_back.or_else(|| self.on_dismiss.clone());
        let header = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(u(14.))
            .h(u(FLYOUT_HEADER))
            .px(u(24.))
            .bg(p.accent)
            .text_color(ink)
            .child(
                div()
                    .id("flyout-back")
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(u(32.))
                    .rounded_full()
                    .border_2()
                    .border_color(ink)
                    .cursor_pointer()
                    .hover(|s| s.opacity(0.8))
                    .active(|s| s.bg(ink.opacity(0.25)))
                    .child(Icon::Back.render(u(16.), ink))
                    .when_some(back, |el, f| el.on_click(move |e, w, cx| f(e, w, cx))),
            )
            .child(
                div()
                    .text_size(u(size::LARGE))
                    .font_weight(SEMILIGHT)
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(SharedString::from(self.title.to_lowercase())),
            );
        let panel = div()
            .id("flyout-panel")
            .absolute()
            .top_0()
            .bottom_0()
            .right(u(-w * offset))
            .w(u(w))
            .flex()
            .flex_col()
            .bg(p.background)
            .border_l_1()
            .border_color(p.accent)
            .font_family(Theme::font(cx))
            .text_color(p.foreground)
            .occlude()
            .child(header)
            .child(
                div()
                    .id("flyout-content")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px(u(24.))
                    .py(u(20.))
                    .gap(u(18.))
                    .children(self.content),
            );

        // Light dismiss: while open, a press anywhere outside the panel
        // closes it (and doesn't reach the page). Closing, the page is live.
        let catcher = open.then(|| {
            let dismiss = self.on_dismiss.clone();
            div()
                .id("flyout-dismiss")
                .absolute()
                .inset_0()
                .occlude()
                .when_some(dismiss, |el, f| el.on_click(move |e, w, cx| f(e, w, cx)))
        });
        div().id(self.id).absolute().inset_0().children(catcher).child(panel).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slides_in_from_off_the_edge_and_settles() {
        assert_eq!(flyout_offset(true, Duration::ZERO), Some(1.));
        let mid = flyout_offset(true, FLYOUT_IN / 4).unwrap();
        assert!(mid > 0. && mid < 0.5, "decelerates: most of the way in early ({mid})");
        assert_eq!(flyout_offset(true, FLYOUT_IN), Some(0.));
        assert_eq!(flyout_offset(true, FLYOUT_IN * 3), Some(0.));
    }

    #[test]
    fn slides_out_then_is_gone() {
        assert_eq!(flyout_offset(false, Duration::ZERO), Some(0.));
        assert!(flyout_offset(false, FLYOUT_OUT / 2).unwrap() < 0.5, "accelerates away");
        assert_eq!(flyout_offset(false, FLYOUT_OUT), None);
    }

    #[test]
    fn widths_are_the_windows_8_ones() {
        assert_eq!(FlyoutWidth::Narrow.px(), 346.);
        assert_eq!(FlyoutWidth::Wide.px(), 646.);
    }
}
