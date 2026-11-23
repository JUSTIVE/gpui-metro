//! Lists: the two-line Metro list item, the grouped long list with its
//! jump grid, and the inline list picker.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::{
    Icon, Theme,
    button::ClickHandler,
    chrome::PAGE_MARGIN,
    motion::{Direction, enter},
    theme::{LIGHT, SEMILIGHT, size},
};

/// Leading square on a list item.
pub enum Leading {
    Icon(Icon),
    Glyph(SharedString),
    Element(AnyElement),
}

/// Title (large, light) over a subtitle (accent or subtle).
#[derive(IntoElement)]
pub struct ListItem {
    id: ElementId,
    title: SharedString,
    subtitle: Option<SharedString>,
    detail: Option<SharedString>,
    subtitle_accent: bool,
    leading: Option<Leading>,
    leading_color: Option<Hsla>,
    trailing: Option<AnyElement>,
    on_click: Option<ClickHandler>,
}

pub fn list_item(id: impl Into<ElementId>, title: impl Into<SharedString>) -> ListItem {
    ListItem {
        id: id.into(),
        title: title.into(),
        subtitle: None,
        detail: None,
        subtitle_accent: true,
        leading: None,
        leading_color: None,
        trailing: None,
        on_click: None,
    }
}

impl ListItem {
    pub fn subtitle(mut self, s: impl Into<SharedString>) -> Self {
        self.subtitle = Some(s.into());
        self
    }
    /// A third, subtle line.
    pub fn detail(mut self, s: impl Into<SharedString>) -> Self {
        self.detail = Some(s.into());
        self
    }
    /// Render the subtitle subtle instead of accent.
    pub fn subtle_subtitle(mut self) -> Self {
        self.subtitle_accent = false;
        self
    }
    pub fn leading(mut self, leading: Leading) -> Self {
        self.leading = Some(leading);
        self
    }
    pub fn leading_icon(self, icon: Icon) -> Self {
        self.leading(Leading::Icon(icon))
    }
    pub fn leading_color(mut self, c: Hsla) -> Self {
        self.leading_color = Some(c);
        self
    }
    pub fn trailing(mut self, e: impl IntoElement) -> Self {
        self.trailing = Some(e.into_any_element());
        self
    }
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let sq = p.accent_ink;
        let leading = self.leading.map(|l| {
            let bg = self.leading_color.unwrap_or(p.accent);
            div().flex().flex_none().items_center().justify_center().size(px(62.)).bg(bg).child(match l {
                Leading::Icon(i) => i.render(px(32.), sq).into_any_element(),
                Leading::Glyph(g) => div().text_size(px(size::LARGE)).text_color(sq).child(g).into_any_element(),
                Leading::Element(e) => e,
            })
        });
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(14.))
            .py(px(8.))
            .font_family(Theme::font(cx))
            .children(leading)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(size::LARGE + 3.))
                            .font_weight(SEMILIGHT)
                            .line_height(px(34.))
                            .text_color(p.foreground)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(self.title),
                    )
                    .when_some(self.subtitle, |el, s| {
                        el.child(
                            div()
                                .text_size(px(size::NORMAL))
                                .text_color(if self.subtitle_accent { p.accent } else { p.subtle })
                                .child(s),
                        )
                    })
                    .when_some(self.detail, |el, s| {
                        el.child(div().text_size(px(size::NORMAL)).text_color(p.subtle).child(s))
                    }),
            )
            .children(self.trailing)
            .when_some(self.on_click, |el, f| {
                el.cursor_pointer().active(|s| s.opacity(0.6)).on_click(move |e, w, cx| f(e, w, cx))
            })
    }
}

/// A group of a [`JumpList`].
pub struct ListGroup {
    pub key: SharedString,
    pub items: Vec<AnyElement>,
}

impl ListGroup {
    pub fn new(key: impl Into<SharedString>, items: impl IntoIterator<Item = AnyElement>) -> Self {
        Self { key: key.into(), items: items.into_iter().collect() }
    }
}

/// The alphabet the jump grid shows by default.
pub const ALPHABET: [&str; 27] = [
    "#", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u",
    "v", "w", "x", "y", "z",
];

/// The `LongListSelector`: groups under accent header squares; tapping a
/// header opens the jump grid, tapping a key scrolls to its group.
#[derive(IntoElement)]
pub struct JumpList {
    id: ElementId,
    groups: Vec<ListGroup>,
    keys: Option<Vec<SharedString>>,
}

pub fn jump_list(id: impl Into<ElementId>) -> JumpList {
    JumpList { id: id.into(), groups: Vec::new(), keys: None }
}

impl JumpList {
    pub fn group(mut self, group: ListGroup) -> Self {
        self.groups.push(group);
        self
    }
    pub fn groups(mut self, groups: impl IntoIterator<Item = ListGroup>) -> Self {
        self.groups.extend(groups);
        self
    }
    /// Keys shown in the jump grid. Defaults to the group keys; pass
    /// [`ALPHABET`] for the classic a–z grid with empty letters greyed.
    pub fn keys<S: Into<SharedString>>(mut self, keys: impl IntoIterator<Item = S>) -> Self {
        self.keys = Some(keys.into_iter().map(Into::into).collect());
        self
    }
}

struct JumpState {
    scroll: ScrollHandle,
    grid_open: bool,
}

impl RenderOnce for JumpList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let state =
            window.use_keyed_state(self.id.clone(), cx, |_, _| JumpState { scroll: ScrollHandle::new(), grid_open: false });
        let (scroll, grid_open) = {
            let s = state.read(cx);
            (s.scroll.clone(), s.grid_open)
        };
        let group_keys: Vec<SharedString> = self.groups.iter().map(|g| g.key.clone()).collect();
        let keys = self.keys.unwrap_or_else(|| group_keys.clone());

        let header = |ix: usize, key: SharedString| {
            let state = state.clone();
            div()
                .id(("jump-header", ix))
                .flex()
                .items_end()
                .size(px(52.))
                .p(px(6.))
                .mt(px(14.))
                .mb(px(6.))
                .bg(p.accent)
                .text_size(px(size::LARGE + 3.))
                .text_color(p.accent_ink)
                .cursor_pointer()
                .child(key)
                .on_click(move |_, _, cx| state.update(cx, |s, cx| {
                    s.grid_open = true;
                    cx.notify();
                }))
        };

        let list = div()
            .id("jump-scroll")
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&scroll)
            .px(px(PAGE_MARGIN))
            .children(self.groups.into_iter().enumerate().map(|(ix, g)| {
                div().flex().flex_col().child(header(ix, g.key)).children(g.items)
            }))
            .child(div().h(px(80.)));

        let grid = grid_open.then(|| {
            let cells = keys.iter().cloned().enumerate().map(|(ix, key)| {
                let target = group_keys.iter().position(|k| k == &key);
                let state = state.clone();
                let scroll = scroll.clone();
                let enabled = target.is_some();
                div()
                    .id(("jump-key", ix))
                    .flex()
                    .items_end()
                    .size(px(96.))
                    .p(px(8.))
                    .bg(if enabled { p.accent } else { p.chrome })
                    .text_size(px(size::EXTRA_LARGE + 6.))
                    .font_weight(LIGHT)
                    .text_color(if enabled { p.accent_ink } else { p.disabled })
                    .child(key)
                    .when_some(target, |el, t| {
                        el.cursor_pointer().on_click(move |_, _, cx| {
                            scroll.scroll_to_top_of_item(t);
                            state.update(cx, |s, cx| {
                                s.grid_open = false;
                                cx.notify();
                            });
                        })
                    })
            });
            let close = state.clone();
            div()
                .id("jump-grid")
                .absolute()
                .inset_0()
                .bg(p.background.opacity(0.92))
                .occlude()
                .on_click(move |_, _, cx| close.update(cx, |s, cx| {
                    s.grid_open = false;
                    cx.notify();
                }))
                .child(enter(
                    "jump-grid-in",
                    Direction::Down,
                    0,
                    div().flex().flex_wrap().gap(px(10.)).p(px(PAGE_MARGIN)).pt(px(40.)).children(cells),
                    cx,
                ))
        });

        div().relative().size_full().font_family(Theme::font(cx)).child(list).children(grid)
    }
}

/// The WP toolkit `ListPicker`: shows the current choice in a field; tap
/// to expand the choices in place, tap one to pick it.
#[derive(IntoElement)]
pub struct ListPicker {
    id: ElementId,
    header: Option<SharedString>,
    items: Vec<SharedString>,
    selected: usize,
    on_select: Option<Rc<dyn Fn(usize, &mut Window, &mut App)>>,
}

pub fn list_picker<S: Into<SharedString>>(
    id: impl Into<ElementId>,
    items: impl IntoIterator<Item = S>,
    selected: usize,
) -> ListPicker {
    ListPicker {
        id: id.into(),
        header: None,
        items: items.into_iter().map(Into::into).collect(),
        selected,
        on_select: None,
    }
}

impl ListPicker {
    pub fn header(mut self, h: impl Into<SharedString>) -> Self {
        self.header = Some(h.into());
        self
    }
    pub fn on_select(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(f));
        self
    }
}

impl RenderOnce for ListPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let open_state = window.use_keyed_state(self.id.clone(), cx, |_, _| false);
        let open = *open_state.read(cx);
        let selected = self.selected.min(self.items.len().saturating_sub(1));

        let field = if open {
            div().flex().flex_col().children(self.items.iter().cloned().enumerate().map(|(ix, item)| {
                let f = self.on_select.clone();
                let open_state = open_state.clone();
                div()
                    .id(("picker-item", ix))
                    .px(px(10.))
                    .py(px(6.))
                    .text_color(if ix == selected { p.accent } else { p.foreground })
                    .cursor_pointer()
                    .child(item)
                    .on_click(move |_, w, cx| {
                        open_state.update(cx, |o, cx| {
                            *o = false;
                            cx.notify();
                        });
                        if let Some(f) = &f {
                            f(ix, w, cx);
                        }
                    })
            }))
        } else {
            let open_state = open_state.clone();
            div().child(
                div()
                    .id("picker-current")
                    .px(px(10.))
                    .py(px(6.))
                    .cursor_pointer()
                    .child(self.items.get(selected).cloned().unwrap_or_default())
                    .on_click(move |_, _, cx| open_state.update(cx, |o, cx| {
                        *o = true;
                        cx.notify();
                    })),
            )
        };

        let close = open_state.clone();
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(4.))
            .font_family(Theme::font(cx))
            .when_some(self.header, |el, h| el.child(div().text_size(px(size::NORMAL)).text_color(p.subtle).child(h)))
            .child(
                div()
                    .border_2()
                    .border_color(if open { p.accent } else { p.border })
                    .bg(if open { p.background } else { p.input.opacity(0.2) })
                    .text_size(px(size::MEDIUM))
                    .text_color(p.foreground)
                    .py(px(2.))
                    .child(field),
            )
            .when(open, |el| el.on_mouse_down_out(move |_, _, cx| close.update(cx, |o, cx| {
                *o = false;
                cx.notify();
            })))
    }
}
