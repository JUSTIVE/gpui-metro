//! Selection controls: toggle switch, check box, radio button. All
//! controlled: the caller owns the value and receives the new one.

use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::{
    Icon, Theme,
    motion::{ease_out_expo, generation_id, lerp, track},
    theme::size,
};

pub type SelectRadio = Rc<dyn Fn(&mut Window, &mut App)>;
pub type ToggleHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// The WP8 toggle switch: a header, the On/Off word, and the rectangular track.
#[derive(IntoElement)]
pub struct ToggleSwitch {
    id: ElementId,
    on: bool,
    header: Option<SharedString>,
    labels: (SharedString, SharedString),
    disabled: bool,
    on_change: Option<ToggleHandler>,
}

pub fn toggle_switch(id: impl Into<ElementId>, on: bool) -> ToggleSwitch {
    ToggleSwitch {
        id: id.into(),
        on,
        header: None,
        labels: ("On".into(), "Off".into()),
        disabled: false,
        on_change: None,
    }
}

impl ToggleSwitch {
    pub fn header(mut self, header: impl Into<SharedString>) -> Self {
        self.header = Some(header.into());
        self
    }
    pub fn labels(mut self, on: impl Into<SharedString>, off: impl Into<SharedString>) -> Self {
        self.labels = (on.into(), off.into());
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn on_change(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

const TRACK_W: f32 = 76.;
const TRACK_H: f32 = 32.;
const THUMB_W: f32 = 20.;

impl RenderOnce for ToggleSwitch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let font = Theme::font(cx);
        let reduced = Theme::get(cx).reduced_motion;
        let on = self.on;
        let fg = if self.disabled { p.disabled } else { p.foreground };
        let fill = if self.disabled { p.disabled } else { p.accent };

        let key = SharedString::from(format!("{}-track", self.id));
        let (prev, generation) = track(ElementId::from(key.clone()), on, window, cx);
        let travel = TRACK_W - THUMB_W;
        let end = if on { travel } else { 0. };
        let start = if prev { travel } else { 0. };

        // Thumb and the accent fill that trails it, positioned by `x`.
        let layer = move |x: f32| {
            div()
                .absolute()
                .inset_0()
                .child(
                    div()
                        .absolute()
                        .top(px(4.))
                        .bottom(px(4.))
                        .left(px(4.))
                        .w(px((x - 2.).max(0.)))
                        .bg(fill),
                )
                .child(div().absolute().top_0().bottom_0().left(px(x)).w(px(THUMB_W)).bg(fg))
        };
        let thumb = if generation == 0 || reduced {
            layer(end).into_any_element()
        } else {
            div()
                .absolute()
                .inset_0()
                .with_animation(
                    generation_id(&key, generation),
                    Animation::new(Duration::from_millis(220)).with_easing(ease_out_expo),
                    move |el, t| el.child(layer(lerp(start, end, t))),
                )
                .into_any_element()
        };

        let on_change = self.on_change.clone();
        let disabled = self.disabled;
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(4.))
            .font_family(font)
            .when_some(self.header, |el, h| {
                el.child(div().text_size(px(size::NORMAL)).text_color(p.subtle).child(h))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(size::LARGE))
                            .text_color(fg)
                            .child(if on { self.labels.0 } else { self.labels.1 }),
                    )
                    .child(
                        div()
                            .relative()
                            .w(px(TRACK_W))
                            .h(px(TRACK_H))
                            .border_2()
                            .border_color(fg)
                            .child(thumb),
                    ),
            )
            .when(!disabled, |el| el.cursor_pointer())
            .when_some(on_change.filter(|_| !disabled), |el, f| el.on_click(move |_, w, cx| f(!on, w, cx)))
    }
}

/// Square check box with a label.
#[derive(IntoElement)]
pub struct CheckBox {
    id: ElementId,
    checked: bool,
    label: Option<SharedString>,
    disabled: bool,
    on_change: Option<ToggleHandler>,
}

pub fn check_box(id: impl Into<ElementId>, checked: bool) -> CheckBox {
    CheckBox { id: id.into(), checked, label: None, disabled: false, on_change: None }
}

impl CheckBox {
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn on_change(mut self, f: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for CheckBox {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let fg = if self.disabled { p.disabled } else { p.foreground };
        let checked = self.checked;
        let disabled = self.disabled;
        let accent = p.accent;
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(14.))
            .py(px(6.))
            .font_family(Theme::font(cx))
            .text_size(px(size::MEDIUM))
            .text_color(fg)
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(32.))
                    .border_2()
                    .border_color(fg)
                    .when(checked, |el| el.child(Icon::Check.render(px(22.), fg))),
            )
            .when_some(self.label, |el, l| el.child(l))
            .when(!disabled, |el| el.cursor_pointer().active(move |s| s.text_color(accent)))
            .when_some(self.on_change.filter(|_| !disabled), |el, f| el.on_click(move |_, w, cx| f(!checked, w, cx)))
    }
}

/// Round radio button with a label. Group them yourself: the caller decides
/// which one is selected.
#[derive(IntoElement)]
pub struct RadioButton {
    id: ElementId,
    selected: bool,
    label: Option<SharedString>,
    disabled: bool,
    on_select: Option<SelectRadio>,
}

pub fn radio_button(id: impl Into<ElementId>, selected: bool) -> RadioButton {
    RadioButton { id: id.into(), selected, label: None, disabled: false, on_select: None }
}

impl RadioButton {
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn on_select(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for RadioButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let fg = if self.disabled { p.disabled } else { p.foreground };
        let disabled = self.disabled;
        let accent = p.accent;
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(14.))
            .py(px(6.))
            .font_family(Theme::font(cx))
            .text_size(px(size::MEDIUM))
            .text_color(fg)
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size(px(32.))
                    .rounded_full()
                    .border_2()
                    .border_color(fg)
                    .when(self.selected, |el| el.child(div().size(px(14.)).rounded_full().bg(fg))),
            )
            .when_some(self.label, |el, l| el.child(l))
            .when(!disabled, |el| el.cursor_pointer().active(move |s| s.text_color(accent)))
            .when_some(self.on_select.filter(|_| !disabled), |el, f| el.on_click(move |_, w, cx| f(w, cx)))
    }
}
