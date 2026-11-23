//! The editing engine behind [`crate::TextBox`]: a single-line field that
//! behaves like `<input type="text">` on macOS — GPUI ships none.
//!
//! What a person's fingers expect from a web field:
//!
//! * **Caret moves**: ←/→ by character (collapsing a selection to its edge
//!   first), ⌥←/⌥→ by word, ⌘←/⌘→ · Home/End · ↑/↓ · ⌃A/⌃E to the ends,
//!   ⌃B/⌃F by character.
//! * **Selection**: ⇧ with any of the above extends from a fixed anchor,
//!   ⌘A selects all; double-click selects a word (dragging extends by words),
//!   triple-click selects everything, ⇧-click extends.
//! * **Deletion**: ⌫/⌦ by character (⌃H/⌃D too), ⌥⌫/⌥⌦ by word, ⌘⌫ to the
//!   start, ⌘⌦ and ⌃K to the end; any key with a selection deletes it.
//! * **Undo**: ⌘Z / ⇧⌘Z, with a run of typing undone as one step.
//! * **Display**: a blinking caret in the text colour (solid while you
//!   type), the selection only while focused, long text scrolls to keep the
//!   caret in view, Tab-ing in selects the whole value, pasted line breaks
//!   are stripped.
//! * IME composition, Enter (submit) and the character palette.

use std::ops::Range;
use std::time::Duration;

use gpui::{
    App, Bounds, ClipboardItem, ContentMask, Context, CursorStyle, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, Hsla, KeyBinding, LayoutId,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, ShapedLine,
    SharedString, Style, Task, TextRun, UTF16Selection, UnderlineStyle, Window, actions, div, fill, point,
    prelude::*, px, relative, size,
};
use unicode_segmentation::*;

actions!(
    text_input,
    [
        Backspace,
        Delete,
        DeleteWordLeft,
        DeleteWordRight,
        DeleteToStart,
        DeleteToEnd,
        Left,
        Right,
        WordLeft,
        WordRight,
        Home,
        End,
        SelectLeft,
        SelectRight,
        SelectWordLeft,
        SelectWordRight,
        SelectToStart,
        SelectToEnd,
        SelectAll,
        Undo,
        Redo,
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
        Submit,
    ]
);

pub enum InputEvent {
    Changed,
    Submit,
}

const CONTEXT: &str = "TextInput";
/// Chrome's caret blink interval on macOS.
const BLINK: Duration = Duration::from_millis(530);

pub fn bind_keys(cx: &mut App) {
    let c = Some(CONTEXT);
    cx.bind_keys([
        // deletion
        KeyBinding::new("backspace", Backspace, c),
        KeyBinding::new("shift-backspace", Backspace, c),
        KeyBinding::new("ctrl-h", Backspace, c),
        KeyBinding::new("delete", Delete, c),
        KeyBinding::new("ctrl-d", Delete, c),
        KeyBinding::new("alt-backspace", DeleteWordLeft, c),
        KeyBinding::new("ctrl-w", DeleteWordLeft, c),
        KeyBinding::new("alt-delete", DeleteWordRight, c),
        KeyBinding::new("cmd-backspace", DeleteToStart, c),
        KeyBinding::new("cmd-delete", DeleteToEnd, c),
        KeyBinding::new("ctrl-k", DeleteToEnd, c),
        // caret
        KeyBinding::new("left", Left, c),
        KeyBinding::new("ctrl-b", Left, c),
        KeyBinding::new("right", Right, c),
        KeyBinding::new("ctrl-f", Right, c),
        KeyBinding::new("alt-left", WordLeft, c),
        KeyBinding::new("alt-right", WordRight, c),
        KeyBinding::new("cmd-left", Home, c),
        KeyBinding::new("home", Home, c),
        KeyBinding::new("up", Home, c),
        KeyBinding::new("ctrl-a", Home, c),
        KeyBinding::new("ctrl-p", Home, c),
        KeyBinding::new("cmd-up", Home, c),
        KeyBinding::new("cmd-right", End, c),
        KeyBinding::new("end", End, c),
        KeyBinding::new("down", End, c),
        KeyBinding::new("ctrl-e", End, c),
        KeyBinding::new("ctrl-n", End, c),
        KeyBinding::new("cmd-down", End, c),
        // selection
        KeyBinding::new("shift-left", SelectLeft, c),
        KeyBinding::new("shift-right", SelectRight, c),
        KeyBinding::new("alt-shift-left", SelectWordLeft, c),
        KeyBinding::new("alt-shift-right", SelectWordRight, c),
        KeyBinding::new("cmd-shift-left", SelectToStart, c),
        KeyBinding::new("shift-home", SelectToStart, c),
        KeyBinding::new("shift-up", SelectToStart, c),
        KeyBinding::new("cmd-shift-up", SelectToStart, c),
        KeyBinding::new("cmd-shift-right", SelectToEnd, c),
        KeyBinding::new("shift-end", SelectToEnd, c),
        KeyBinding::new("shift-down", SelectToEnd, c),
        KeyBinding::new("cmd-shift-down", SelectToEnd, c),
        KeyBinding::new("cmd-a", SelectAll, c),
        // history & clipboard
        KeyBinding::new("cmd-z", Undo, c),
        KeyBinding::new("cmd-shift-z", Redo, c),
        KeyBinding::new("cmd-v", Paste, c),
        KeyBinding::new("cmd-c", Copy, c),
        KeyBinding::new("cmd-x", Cut, c),
        KeyBinding::new("enter", Submit, c),
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, c),
    ]);
}

/// What the last edit was, so a run of typing undoes as one step.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EditKind {
    Insert,
    Delete,
    Other,
}

#[derive(Clone)]
struct Snapshot {
    content: SharedString,
    anchor: usize,
    head: usize,
}

/// How a mouse drag extends: by character, by word (after a double-click),
/// or not at all (after a triple-click selected everything).
#[derive(Clone, Copy, PartialEq, Eq)]
enum DragUnit {
    Char,
    Word,
    All,
}

pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    /// The selection is anchor..head in either order; the caret is the head.
    anchor: usize,
    head: usize,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    /// Horizontal scroll of an overflowing value, in px.
    scroll_x: Pixels,
    drag: Option<(DragUnit, Range<usize>)>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_edit: EditKind,
    caret_on: bool,
    blink: Option<Task<()>>,
    was_focused: bool,
    pub placeholder_color: Hsla,
    pub caret_color: Option<Hsla>,
    pub selection_color: Hsla,
    /// Select the whole value when the field gains focus.
    pub select_on_focus: bool,
}

impl EventEmitter<InputEvent> for TextInput {}

impl TextInput {
    pub fn new(placeholder: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle().tab_index(0).tab_stop(true),
            content: SharedString::default(),
            placeholder: placeholder.into(),
            anchor: 0,
            head: 0,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            scroll_x: px(0.),
            drag: None,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: EditKind::Other,
            caret_on: true,
            blink: None,
            was_focused: false,
            placeholder_color: gpui::hsla(0., 0., 0.5, 0.6),
            caret_color: None,
            selection_color: gpui::hsla(0.6, 0.8, 0.6, 0.3),
            select_on_focus: false,
        }
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = text.into();
        let end = self.content.len();
        self.anchor = end;
        self.head = end;
        self.marked_range = None;
        self.undo.clear();
        self.redo.clear();
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus_handle.is_focused(window)
    }

    #[allow(dead_code)]
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.set_text("", cx);
    }

    fn selected_range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    fn submit(&mut self, _: &Submit, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(InputEvent::Submit);
    }

    // ---- caret & selection -------------------------------------------------

    /// Collapses the selection to `offset`.
    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.anchor = offset;
        self.head = offset;
        self.after_caret_change(cx);
    }

    /// Moves the head, keeping the anchor where the selection started.
    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.head = offset;
        self.after_caret_change(cx);
    }

    fn after_caret_change(&mut self, cx: &mut Context<Self>) {
        // A caret move ends a typing run: the next keystroke is a new undo step.
        self.last_edit = EditKind::Other;
        self.wake_caret(cx);
        cx.notify();
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let r = self.selected_range();
        if r.is_empty() { self.move_to(self.previous_boundary(self.head), cx) } else { self.move_to(r.start, cx) }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let r = self.selected_range();
        if r.is_empty() { self.move_to(self.next_boundary(self.head), cx) } else { self.move_to(r.end, cx) }
    }

    fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        let from = self.selected_range().start;
        self.move_to(self.word_start_before(from), cx);
    }

    fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        let from = self.selected_range().end;
        self.move_to(self.word_end_after(from), cx);
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.head), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.head), cx);
    }

    fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.word_start_before(self.head), cx);
    }

    fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.word_end_after(self.head), cx);
    }

    fn select_to_start(&mut self, _: &SelectToStart, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(0, cx);
    }

    fn select_to_end(&mut self, _: &SelectToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.content.len(), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.select_everything(cx);
    }

    fn select_everything(&mut self, cx: &mut Context<Self>) {
        self.anchor = 0;
        self.head = self.content.len();
        self.after_caret_change(cx);
    }

    // ---- deletion ------------------------------------------------------------

    /// Deletes the selection if there is one, otherwise `head..to`.
    fn delete_toward(&mut self, to: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range().is_empty() {
            self.anchor = to;
        }
        self.edit(EditKind::Delete, "", window, cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        let to = self.previous_boundary(self.head);
        self.delete_toward(to, window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        let to = self.next_boundary(self.head);
        self.delete_toward(to, window, cx);
    }

    fn delete_word_left(&mut self, _: &DeleteWordLeft, window: &mut Window, cx: &mut Context<Self>) {
        let to = self.word_start_before(self.head);
        self.delete_toward(to, window, cx);
    }

    fn delete_word_right(&mut self, _: &DeleteWordRight, window: &mut Window, cx: &mut Context<Self>) {
        let to = self.word_end_after(self.head);
        self.delete_toward(to, window, cx);
    }

    fn delete_to_start(&mut self, _: &DeleteToStart, window: &mut Window, cx: &mut Context<Self>) {
        self.delete_toward(0, window, cx);
    }

    fn delete_to_end(&mut self, _: &DeleteToEnd, window: &mut Window, cx: &mut Context<Self>) {
        let end = self.content.len();
        self.delete_toward(end, window, cx);
    }

    // ---- editing & history ---------------------------------------------------

    fn snapshot(&self) -> Snapshot {
        Snapshot { content: self.content.clone(), anchor: self.anchor, head: self.head }
    }

    fn restore(&mut self, s: Snapshot, cx: &mut Context<Self>) {
        self.content = s.content;
        self.anchor = s.anchor;
        self.head = s.head;
        self.marked_range = None;
        self.last_edit = EditKind::Other;
        self.wake_caret(cx);
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    /// Replaces the selection with `text`, recording history the way a
    /// browser does: consecutive inserts (or deletes) coalesce.
    fn edit(&mut self, kind: EditKind, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let range = self.selected_range();
        self.replace(range, text, kind, window, cx);
    }

    fn replace(&mut self, range: Range<usize>, text: &str, kind: EditKind, _: &mut Window, cx: &mut Context<Self>) {
        if range.is_empty() && text.is_empty() {
            return;
        }
        let coalesce = kind != EditKind::Other && kind == self.last_edit && !text.contains(' ');
        if !coalesce {
            self.undo.push(self.snapshot());
            if self.undo.len() > 200 {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.last_edit = kind;
        self.content = (self.content[..range.start].to_owned() + text + &self.content[range.end..]).into();
        let caret = range.start + text.len();
        self.anchor = caret;
        self.head = caret;
        self.marked_range = None;
        self.wake_caret(cx);
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(previous, cx);
        }
    }

    fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(next, cx);
        }
    }

    // ---- clipboard -------------------------------------------------------------

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            // A single-line field drops line breaks, like the value sanitizer.
            let text: String = text.chars().filter(|c| *c != '\n' && *c != '\r').collect();
            self.edit(EditKind::Other, &text, window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let r = self.selected_range();
        if !r.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.content[r].to_string()));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        let r = self.selected_range();
        if !r.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.content[r].to_string()));
            self.edit(EditKind::Other, "", window, cx);
        }
    }

    fn show_character_palette(&mut self, _: &ShowCharacterPalette, window: &mut Window, _: &mut Context<Self>) {
        window.show_character_palette();
    }

    // ---- mouse ------------------------------------------------------------------

    fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
        let at = self.index_for_mouse_position(event.position);
        match event.click_count {
            2 => {
                let word = self.word_range_at(at);
                self.anchor = word.start;
                self.head = word.end;
                self.drag = Some((DragUnit::Word, word));
            }
            n if n >= 3 => {
                self.select_everything(cx);
                self.drag = Some((DragUnit::All, 0..self.content.len()));
                return;
            }
            _ if event.modifiers.shift => {
                self.head = at;
                self.drag = Some((DragUnit::Char, self.anchor..self.anchor));
            }
            _ => {
                self.anchor = at;
                self.head = at;
                self.drag = Some((DragUnit::Char, at..at));
            }
        }
        self.after_caret_change(cx);
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.drag = None;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some((unit, origin)) = self.drag.clone() else { return };
        if !event.dragging() {
            self.drag = None;
            return;
        }
        let at = self.index_for_mouse_position(event.position);
        match unit {
            DragUnit::All => return,
            DragUnit::Char => {
                self.anchor = origin.start;
                self.head = at;
            }
            // Extending a double-click grows by whole words from the first one.
            DragUnit::Word => {
                let word = self.word_range_at(at);
                if at < origin.start {
                    self.anchor = origin.end;
                    self.head = word.start;
                } else {
                    self.anchor = origin.start;
                    self.head = word.end.max(origin.end);
                }
            }
        }
        self.wake_caret(cx);
        cx.notify();
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref()) else {
            return 0;
        };
        let x = position.x - bounds.left() + self.scroll_x;
        if x <= px(0.) {
            return 0;
        }
        line.closest_index_for_x(x)
    }

    // ---- caret blink ------------------------------------------------------------

    /// Solid now, blinking again after an interval — any edit or caret move.
    fn wake_caret(&mut self, cx: &mut Context<Self>) {
        self.caret_on = true;
        self.blink = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(BLINK).await;
                let alive = this
                    .update(cx, |input, cx| {
                        input.caret_on = !input.caret_on;
                        cx.notify();
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            }
        }));
    }

    // ---- text boundaries ---------------------------------------------------------

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    /// Unicode word segments that are words (letters/digits), as ranges.
    fn words(&self) -> Vec<Range<usize>> {
        self.content
            .split_word_bound_indices()
            .filter(|(_, w)| w.chars().any(|c| c.is_alphanumeric()))
            .map(|(i, w)| i..i + w.len())
            .collect()
    }

    /// ⌥←: skip separators, then the word — the start of the previous word.
    fn word_start_before(&self, offset: usize) -> usize {
        self.words().into_iter().rev().find(|w| w.start < offset).map(|w| w.start).unwrap_or(0)
    }

    /// ⌥→: skip separators, then the word — the end of the next word.
    fn word_end_after(&self, offset: usize) -> usize {
        self.words().into_iter().find(|w| w.end > offset).map(|w| w.end).unwrap_or(self.content.len())
    }

    /// What a double-click at `offset` selects: the word under it, or the run
    /// of separators between words.
    fn word_range_at(&self, offset: usize) -> Range<usize> {
        for (i, segment) in self.content.split_word_bound_indices() {
            let range = i..i + segment.len();
            if range.contains(&offset) || (offset == self.content.len() && range.end == offset) {
                return range;
            }
        }
        offset..offset
    }

    // ---- UTF-16 bridging (IME) ------------------------------------------------------

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range()),
            reversed: self.head < self.anchor,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_range.as_ref().map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range());
        self.replace(range, new_text, EditKind::Insert, window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range());
        self.content = (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..]).into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        let selected = new_selected_range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .map(|r| r.start + range.start..r.end + range.start)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        self.anchor = selected.start;
        self.head = selected.end;
        self.wake_caret(cx);
        cx.emit(InputEvent::Changed);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        let left = bounds.left() - self.scroll_x;
        Some(Bounds::from_corners(
            point(left + last_layout.x_for_index(range.start), bounds.top()),
            point(left + last_layout.x_for_index(range.end), bounds.bottom()),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;
        let utf8_index = last_layout.index_for_x(point.x - line_point.x + self.scroll_x)?;
        Some(self.offset_to_utf16(utf8_index))
    }
}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
    scroll_x: Pixels,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let focused = self.input.read(cx).focus_handle.is_focused(window);
        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected_range = input.selected_range();
        let cursor = input.head;
        let style = window.text_style();

        let (display_text, text_color) = if content.is_empty() {
            (input.placeholder.clone(), input.placeholder_color)
        } else {
            (content, style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun { len: marked_range.start, ..run.clone() },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle { color: Some(run.color), thickness: px(1.0), wavy: false }),
                    ..run.clone()
                },
                TextRun { len: display_text.len() - marked_range.end, ..run },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window.text_system().shape_line(display_text, font_size, &runs, None);

        // Keep the caret in view: scroll just far enough, like a browser.
        let caret_x = if input.content.is_empty() { px(0.) } else { line.x_for_index(cursor) };
        let width = bounds.size.width;
        let mut scroll_x = input.scroll_x;
        if caret_x - scroll_x > width - px(2.) {
            scroll_x = caret_x - width + px(2.);
        }
        if caret_x < scroll_x {
            scroll_x = caret_x;
        }
        let max_scroll = (line.width - width + px(2.)).max(px(0.));
        scroll_x = scroll_x.clamp(px(0.), max_scroll);

        let left = bounds.left() - scroll_x;
        let caret_color = input.caret_color.unwrap_or(style.color);
        let (selection, cursor) = if !focused {
            (None, None)
        } else if selected_range.is_empty() {
            let cursor = input.caret_on.then(|| {
                fill(Bounds::new(point(left + caret_x, bounds.top()), size(px(1.5), bounds.size.height)), caret_color)
            });
            (None, cursor)
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(left + line.x_for_index(selected_range.start), bounds.top()),
                        point(left + line.x_for_index(selected_range.end), bounds.bottom()),
                    ),
                    input.selection_color,
                )),
                None,
            )
        };
        PrepaintState { line: Some(line), cursor, selection, scroll_x }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(&focus_handle, ElementInputHandler::new(bounds, self.input.clone()), cx);
        let scroll_x = prepaint.scroll_x;
        let line = prepaint.line.take().expect("prepaint shaped the line");
        // An overflowing value is clipped to the field, never drawn past it.
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            if let Some(selection) = prepaint.selection.take() {
                window.paint_quad(selection)
            }
            let origin = point(bounds.origin.x - scroll_x, bounds.origin.y);
            line.paint(origin, window.line_height(), gpui::TextAlign::Left, None, window, cx).ok();
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        });

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
            input.scroll_x = scroll_x;
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus_handle.is_focused(window);
        if focused && !self.was_focused {
            // Tab-ing into a field selects its value; a click places the caret.
            if self.select_on_focus {
                self.anchor = 0;
                self.head = self.content.len();
            }
            self.wake_caret(cx);
        }
        if !focused && self.was_focused {
            // A blurred field stops blinking and forgets its drag.
            self.blink = None;
            self.drag = None;
        }
        self.was_focused = focused;

        div()
            .flex()
            .w_full()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_word_left))
            .on_action(cx.listener(Self::delete_word_right))
            .on_action(cx.listener(Self::delete_to_start))
            .on_action(cx.listener(Self::delete_to_end))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::word_left))
            .on_action(cx.listener(Self::word_right))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::select_to_start))
            .on_action(cx.listener(Self::select_to_end))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::submit))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(TextElement { input: cx.entity() })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    fn input(text: &str, cx: &mut TestAppContext) -> Entity<TextInput> {
        let text = text.to_string();
        cx.new(|cx| {
            let mut i = TextInput::new("", cx);
            i.content = text.into();
            i
        })
    }

    struct Host(Entity<TextInput>);
    impl Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(400.)).child(self.0.clone())
        }
    }

    /// Types into a focused field through the real key bindings.
    fn field(cx: &mut TestAppContext) -> (Entity<TextInput>, &mut gpui::VisualTestContext) {
        cx.update(bind_keys);
        let input = cx.new(|cx| TextInput::new("", cx));
        let host_input = input.clone();
        let (_, vcx) = cx.add_window_view(|_, _| Host(host_input));
        let focus = input.read_with(vcx, |i, _| i.focus_handle.clone());
        vcx.update(|window, cx| window.focus(&focus, cx));
        vcx.run_until_parked();
        (input, vcx)
    }

    fn state(input: &Entity<TextInput>, cx: &mut gpui::VisualTestContext) -> (String, usize, usize) {
        input.read_with(cx, |i, _| (i.content.to_string(), i.anchor, i.head))
    }

    #[gpui::test]
    fn keys_edit_like_a_web_field(cx: &mut TestAppContext) {
        let (input, cx) = field(cx);
        cx.simulate_input("hello brave world");
        assert_eq!(state(&input, cx), ("hello brave world".into(), 17, 17));

        cx.simulate_keystrokes("alt-backspace");
        assert_eq!(state(&input, cx).0, "hello brave ");
        cx.simulate_keystrokes("alt-left");
        assert_eq!(state(&input, cx).2, 6, "⌥← lands on the start of the previous word");
        cx.simulate_keystrokes("cmd-left");
        assert_eq!(state(&input, cx).2, 0);
        cx.simulate_keystrokes("alt-shift-right");
        assert_eq!(state(&input, cx), ("hello brave ".into(), 0, 5), "⇧⌥→ selects the word");
        cx.simulate_keystrokes("right");
        assert_eq!(state(&input, cx), ("hello brave ".into(), 5, 5), "→ collapses to the selection's end");
        cx.simulate_keystrokes("cmd-right");
        assert_eq!(state(&input, cx).2, 12);
        cx.simulate_keystrokes("cmd-shift-left");
        assert_eq!(state(&input, cx), ("hello brave ".into(), 12, 0), "anchor stays put, head moves");
        cx.simulate_keystrokes("backspace");
        assert_eq!(state(&input, cx), ("".into(), 0, 0), "a selection deletes as a whole");

        cx.simulate_keystrokes("cmd-z");
        assert_eq!(state(&input, cx).0, "hello brave ");
        cx.simulate_keystrokes("cmd-z");
        assert_eq!(state(&input, cx).0, "hello brave world");
        cx.simulate_keystrokes("cmd-shift-z");
        assert_eq!(state(&input, cx).0, "hello brave ");

        cx.simulate_keystrokes("cmd-a");
        cx.simulate_input("abc def");
        cx.simulate_keystrokes("left left left cmd-backspace");
        assert_eq!(state(&input, cx), ("def".into(), 0, 0), "⌘⌫ deletes to the start");
        cx.simulate_keystrokes("ctrl-k");
        assert_eq!(state(&input, cx).0, "", "⌃K kills to the end");
        cx.simulate_input("one two");
        cx.simulate_keystrokes("ctrl-a alt-delete");
        assert_eq!(state(&input, cx).0, " two", "⌥⌦ deletes the next word");
        cx.simulate_keystrokes("down");
        assert_eq!(state(&input, cx).2, 4, "↓ goes to the end of a single-line field");
    }

    #[gpui::test]
    fn word_motion_matches_macos(cx: &mut TestAppContext) {
        let i = input("osaka-wetlab  bay 42", cx);
        i.read_with(cx, |i, _| {
            // ⌥→ from 0 lands at the end of "osaka", then "wetlab", "bay", "42".
            assert_eq!(i.word_end_after(0), 5);
            assert_eq!(i.word_end_after(5), 12);
            assert_eq!(i.word_end_after(12), 17);
            assert_eq!(i.word_end_after(17), 20);
            assert_eq!(i.word_end_after(20), 20);
            // ⌥← walks back by word starts, skipping the separators.
            assert_eq!(i.word_start_before(20), 18);
            assert_eq!(i.word_start_before(18), 14);
            assert_eq!(i.word_start_before(14), 6);
            assert_eq!(i.word_start_before(6), 0);
            // Double-click on a word, and on the separator run.
            assert_eq!(i.word_range_at(8), 6..12);
            assert_eq!(i.word_range_at(12), 12..14);
        });
    }
}
