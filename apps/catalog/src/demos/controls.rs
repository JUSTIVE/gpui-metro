//! Buttons, selection controls, slider, list picker and text box.

use gpui::{AnyElement, Context, ParentElement, Styled, Window, div, px};
use gpui_metro::{
    Accent, Icon, TextStyle, Theme, button, check_box, hyperlink, list_picker, radio_button, round_button, slider,
    text, text_box, toggle_switch,
};

use super::{DemoState, caption, on, page, stack, tap};
use crate::app::Catalog;

/// A horizontal row of controls at their natural width.
fn row() -> gpui::Div {
    div().flex().flex_wrap().items_center().gap(px(12.))
}

pub fn buttons(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let press = |label: &'static str| {
        tap(cx, move |s| {
            s.button_taps += 1;
            s.last_button = label.into();
        })
    };
    page(
        "button",
        stack()
            .child(caption("normal · accent"))
            .child(
                row()
                    .child(button("btn-send", "send").on_click(press("send")))
                    .child(button("btn-save", "save").accent().on_click(press("save"))),
            )
            .child(caption("disabled"))
            .child(row().child(button("btn-delete", "delete").disabled(true).on_click(press("delete"))))
            .child(caption("with a glyph"))
            .child(
                row()
                    .child(button("btn-share", "share").icon(Icon::Share).on_click(press("share")))
                    .child(button("btn-upload", "upload").icon(Icon::Upload).accent().on_click(press("upload"))),
            )
            .child(caption("full width"))
            .child(button("btn-sign-in", "sign in").full_width().on_click(press("sign in")))
            .child(text(TextStyle::Large, format!("{} taps · last: {}", s.button_taps, s.last_button))),
    )
}

pub fn round_buttons(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let transport = row()
        .gap(px(20.))
        .child(round_button("rb-previous", Icon::Previous))
        .child(
            round_button("rb-play", if s.playing { Icon::Pause } else { Icon::Play })
                .diameter(72.)
                .on_click(tap(cx, |s| s.playing = !s.playing)),
        )
        .child(round_button("rb-next", Icon::Next));
    page(
        "round button",
        stack()
            .child(caption("transport — the big one toggles play / pause"))
            .child(transport)
            .child(text(TextStyle::Subtle, if s.playing { "playing" } else { "paused" }))
            .child(caption("filled = toggled on"))
            .child(
                row()
                    .gap(px(20.))
                    .child(
                        round_button("rb-favorite", Icon::Favorite)
                            .filled(s.favorite)
                            .on_click(tap(cx, |s| s.favorite = !s.favorite)),
                    )
                    .child(text(TextStyle::Normal, if s.favorite { "favourite" } else { "not a favourite" })),
            )
            .child(caption("sizes · disabled"))
            .child(
                row()
                    .gap(px(20.))
                    .child(round_button("rb-36", Icon::Add).diameter(36.))
                    .child(round_button("rb-48", Icon::Add))
                    .child(round_button("rb-64", Icon::Add).diameter(64.))
                    .child(round_button("rb-disabled", Icon::Delete).disabled(true)),
            ),
    )
}

pub fn hyperlinks(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let links = ["privacy statement", "learn more about accents", "forgot your password?"];
    page(
        "hyperlink",
        stack()
            .child(text(TextStyle::Normal, "Find My Phone can ring, lock or erase this phone from the web."))
            .children(links.into_iter().enumerate().map(|(ix, label)| {
                hyperlink(("link", ix), label).on_click(tap(cx, move |s| s.last_link = Some(label.into())))
            }))
            .child(text(
                TextStyle::Subtle,
                match &s.last_link {
                    Some(l) => format!("opened: {l}"),
                    None => "tap a link".into(),
                },
            )),
    )
}

pub fn toggle_switches(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    page(
        "toggle switch",
        stack()
            .child(
                toggle_switch("ts-wifi", s.wifi && !s.airplane)
                    .header("wi-fi networking")
                    .disabled(s.airplane)
                    .on_change(on(cx, |s, v| s.wifi = v)),
            )
            .child(
                toggle_switch("ts-bluetooth", s.bluetooth)
                    .header("bluetooth")
                    .labels("Visible", "Hidden")
                    .on_change(on(cx, |s, v| s.bluetooth = v)),
            )
            .child(
                toggle_switch("ts-airplane", s.airplane)
                    .header("airplane mode")
                    .on_change(on(cx, |s, v| s.airplane = v)),
            )
            .child(caption("airplane mode disables the wi-fi switch"))
            .child(toggle_switch("ts-location", true).header("location (managed by policy)").disabled(true)),
    )
}

pub fn check_boxes(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let labels = ["sync contacts", "sync calendar", "sync photos"];
    let boxes = labels.into_iter().enumerate().map(|(ix, label)| {
        check_box(("cb", ix), s.checks[ix]).label(label).on_change(on(cx, move |s, v| s.checks[ix] = v))
    });
    let count = s.checks.iter().filter(|&&c| c).count();
    page(
        "check box",
        stack()
            .child(div().flex().flex_col().children(boxes))
            .child(check_box("cb-disabled", true).label("sync mail (required)").disabled(true))
            .child(text(TextStyle::Subtle, format!("{count} of {} selected", labels.len()))),
    )
}

const ROUTES: [&str; 3] = ["fastest", "shortest", "avoid tolls"];

pub fn radio_buttons(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let radios = ROUTES.into_iter().enumerate().map(|(ix, label)| {
        let select = on(cx, move |s, ()| s.route = ix);
        radio_button(("route", ix), s.route == ix).label(label).on_select(move |w, cx| select((), w, cx))
    });
    page(
        "radio button",
        stack()
            .child(caption("route preference"))
            .child(
                div().flex().flex_col().children(radios).child(
                    radio_button("route-scenic", false).label("scenic (needs a data connection)").disabled(true),
                ),
            )
            .child(text(TextStyle::Accent, format!("routing: {}", ROUTES[s.route]))),
    )
}

pub fn sliders(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    page(
        "slider",
        stack()
            .child(
                slider("sl-volume", s.volume)
                    .header(format!("ringer volume · {:.0}", s.volume * 30.))
                    .on_change(on(cx, |s, v| s.volume = v)),
            )
            .child(
                slider("sl-brightness", s.brightness)
                    .header(format!("brightness · step 0.1 · {:.0}%", s.brightness * 100.))
                    .step(0.1)
                    .on_change(on(cx, |s, v| s.brightness = v)),
            )
            .child(slider("sl-disabled", 0.3).header("locked by policy").disabled(true))
            .child(caption("drag past the ends: the thumb keeps tracking")),
    )
}

const RINGTONES: [&str; 6] = ["chimes", "cloud", "glimmer", "minimal", "pulse", "xylophone"];

pub fn list_pickers(s: &DemoState, cx: &mut Context<Catalog>) -> AnyElement {
    let accent = Theme::get(cx).accent;
    let current = Accent::ALL.iter().position(|&a| a == accent).unwrap_or(0);
    page(
        "list picker",
        stack()
            .child(
                list_picker("lp-ringtone", RINGTONES, s.ringtone)
                    .header("ringtone")
                    .on_select(on(cx, |s, ix| s.ringtone = ix)),
            )
            .child(text(TextStyle::Subtle, format!("plays “{}” for calls", RINGTONES[s.ringtone])))
            .child(
                list_picker("lp-accent", Accent::ALL.map(Accent::name), current)
                    .header("accent colour (changes the theme)")
                    .on_select(|ix, _, cx| Theme::set_accent(Accent::ALL[ix], cx)),
            ),
    )
}

pub fn text_boxes(s: &DemoState, window: &Window, cx: &mut Context<Catalog>) -> AnyElement {
    let name = s.name_input.read(cx).text().trim().to_string();
    let focused = s.name_input.read(cx).is_focused(window);
    let greeting = if name.is_empty() { "hello, stranger".to_string() } else { format!("hello, {name}") };
    page(
        "text box",
        stack()
            .child(text_box(&s.name_input).header("name"))
            .child(text(TextStyle::Large, greeting))
            .child(caption(if focused { "focused: white field, accent border" } else { "click the field to focus" }))
            .child(text_box(&s.message_input).header("message"))
            .child(text(
                TextStyle::Accent,
                match &s.submitted {
                    Some(m) => format!("sent: {m}"),
                    None => "nothing sent yet".into(),
                },
            ))
            .child(caption("⌥← ⌥→ by word · ⌘A · ⌘Z · double-click selects a word")),
    )
}
