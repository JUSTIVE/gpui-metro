//! Live tiles and the start-screen grid that packs them.
//!
//! A tile is a flat accent square with its name in the bottom-left corner
//! and, optionally, a count. A *live* tile has a back face; it periodically
//! slides up to show it (the WP7 hub-tile peek), then slides back.

use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ClickEvent, ElementId, Hsla, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};

use crate::{Icon, Theme, motion::ease_in_out_cubic, theme::size};

/// Tile footprints in grid units (one unit = one small tile).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileSize {
    /// 1×1
    Small,
    /// 2×2
    Medium,
    /// 4×2
    Wide,
    /// 4×4
    Large,
}

impl TileSize {
    pub fn span(self) -> (usize, usize) {
        match self {
            TileSize::Small => (1, 1),
            TileSize::Medium => (2, 2),
            TileSize::Wide => (4, 2),
            TileSize::Large => (4, 4),
        }
    }

    /// Size in px for a given unit and gutter.
    pub fn pixels(self, unit: f32, gap: f32) -> (f32, f32) {
        let (c, r) = self.span();
        (c as f32 * unit + (c - 1) as f32 * gap, r as f32 * unit + (r - 1) as f32 * gap)
    }

    pub fn name(self) -> &'static str {
        match self {
            TileSize::Small => "small",
            TileSize::Medium => "medium",
            TileSize::Wide => "wide",
            TileSize::Large => "large",
        }
    }
}

/// Default WP8 metrics (scaled for a 480-wide page).
pub const TILE_UNIT: f32 = 99.;
pub const TILE_GAP: f32 = 12.;

/// Packs tiles the way the Windows Phone start screen does: the grid is made
/// of 2×2 blocks; medium and larger tiles align to block corners, and small
/// tiles fill a block's quadrants (top-left, top-right, bottom-left,
/// bottom-right) before moving to the next block. First fit, in order.
///
/// Returns `(column, row)` per tile, in grid units. `columns` is clamped to
/// at least the widest tile.
pub fn pack(sizes: &[TileSize], columns: usize) -> Vec<(usize, usize)> {
    let widest = sizes.iter().map(|s| s.span().0).max().unwrap_or(1);
    let columns = columns.max(widest).max(2);
    let mut occupied: Vec<Vec<bool>> = Vec::new();
    let free = |occ: &Vec<Vec<bool>>, c: usize, r: usize, w: usize, h: usize| {
        if c + w > columns {
            return false;
        }
        (r..r + h).all(|y| (c..c + w).all(|x| occ.get(y).is_none_or(|row| !row[x])))
    };
    let mut out = Vec::with_capacity(sizes.len());
    for &s in sizes {
        let (w, h) = s.span();
        let mut placed = None;
        'search: for by in 0.. {
            for bx in 0..columns.div_ceil(2) {
                let (c0, r0) = (bx * 2, by * 2);
                let candidates: &[(usize, usize)] =
                    if s == TileSize::Small { &[(0, 0), (1, 0), (0, 1), (1, 1)] } else { &[(0, 0)] };
                for &(dx, dy) in candidates {
                    let (c, r) = (c0 + dx, r0 + dy);
                    if free(&occupied, c, r, w, h) {
                        placed = Some((c, r));
                        break 'search;
                    }
                }
            }
        }
        let (c, r) = placed.expect("an empty row always fits");
        while occupied.len() < r + h {
            occupied.push(vec![false; columns]);
        }
        for row in occupied.iter_mut().skip(r).take(h) {
            for cell in row.iter_mut().skip(c).take(w) {
                *cell = true;
            }
        }
        out.push((c, r));
    }
    out
}

/// Total grid height in units for a packing.
pub fn packed_rows(sizes: &[TileSize], positions: &[(usize, usize)]) -> usize {
    sizes.iter().zip(positions).map(|(s, (_, r))| r + s.span().1).max().unwrap_or(0)
}

type Face = Box<dyn FnOnce(TileSize, Hsla) -> AnyElement>;

/// A start-screen tile.
#[derive(IntoElement)]
pub struct Tile {
    id: ElementId,
    size: TileSize,
    title: Option<SharedString>,
    icon: Option<Icon>,
    count: Option<u32>,
    color: Option<Hsla>,
    front: Option<Face>,
    back: Option<Face>,
    /// Seconds for a full front→back→front cycle.
    period: f32,
    /// 0–1 offset into the cycle, so a screen of tiles doesn't flip in unison.
    phase: f32,
    unit: f32,
    gap: f32,
    on_click: Option<crate::button::ClickHandler>,
}

pub fn tile(id: impl Into<ElementId>, size: TileSize) -> Tile {
    Tile {
        id: id.into(),
        size,
        title: None,
        icon: None,
        count: None,
        color: None,
        front: None,
        back: None,
        period: 12.,
        phase: 0.,
        unit: TILE_UNIT,
        gap: TILE_GAP,
        on_click: None,
    }
}

impl Tile {
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// The notification count (hidden at 0, as on the phone).
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }
    /// Override the accent background.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
    /// Replace the default front face (icon + count). The closure gets the
    /// tile size and the background colour.
    pub fn front<E: IntoElement>(mut self, f: impl FnOnce(TileSize, Hsla) -> E + 'static) -> Self {
        self.front = Some(Box::new(move |s, c| f(s, c).into_any_element()));
        self
    }
    /// Make the tile live with a back face.
    pub fn back<E: IntoElement>(mut self, f: impl FnOnce(TileSize, Hsla) -> E + 'static) -> Self {
        self.back = Some(Box::new(move |s, c| f(s, c).into_any_element()));
        self
    }
    pub fn period(mut self, seconds: f32) -> Self {
        self.period = seconds.max(6.);
        self
    }
    pub fn phase(mut self, phase: f32) -> Self {
        self.phase = phase.rem_euclid(1.);
        self
    }
    pub fn metrics(mut self, unit: f32, gap: f32) -> Self {
        self.unit = unit;
        self.gap = gap;
        self
    }
    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(f));
        self
    }
}

/// Fraction of the face scrolled away (0 = front showing, 1 = back showing)
/// at cycle time `t`. `slide` is the fraction of the cycle each slide takes.
pub fn peek_offset(t: f32, slide: f32) -> f32 {
    let slide = slide.clamp(0.001, 0.5);
    match t {
        t if t < 0.5 - slide => 0.,
        t if t < 0.5 => ease_in_out_cubic((t - (0.5 - slide)) / slide),
        t if t < 1. - slide => 1.,
        t => 1. - ease_in_out_cubic((t - (1. - slide)) / slide),
    }
}

/// How long one slide between faces takes, regardless of the cycle length.
pub const PEEK_SLIDE_SECS: f32 = 0.9;

impl RenderOnce for Tile {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = Theme::palette(cx);
        let font = Theme::font(cx);
        let reduced = Theme::get(cx).reduced_motion;
        let bg = self.color.unwrap_or(p.accent);
        let ink = p.accent_ink;
        let (w, h) = self.size.pixels(self.unit, self.gap);
        let size = self.size;

        let title = self.title.clone();
        let label = move || {
            title.clone().map(|t| {
                div()
                    .absolute()
                    .left(px(8.))
                    .bottom(px(6.))
                    .text_size(px(size::SMALL))
                    .text_color(ink)
                    .child(t)
            })
        };

        let front: AnyElement = match self.front {
            Some(f) => f(size, bg),
            None => default_front(size, self.icon, self.count, ink),
        };
        let face = |content: AnyElement, with_label: bool| {
            div().relative().w(px(w)).h(px(h)).flex_none().overflow_hidden().child(content).when(with_label, |el| {
                el.children(label())
            })
        };

        let body: AnyElement = match self.back {
            Some(back) if !reduced => {
                let back = back(size, bg);
                let period = Duration::from_secs_f32(self.period);
                let phase = self.phase;
                let slide = PEEK_SLIDE_SECS / self.period;
                // The strip holds both faces stacked; it rides up by one face.
                div()
                    .absolute()
                    .left_0()
                    .w(px(w))
                    .h(px(h * 2.))
                    .flex()
                    .flex_col()
                    .child(face(front, true))
                    .child(face(back, true))
                    .with_animation(
                        ElementId::from((SharedString::from(format!("{}-peek", self.id)), 0usize)),
                        Animation::new(period).repeat(),
                        move |el, t| el.top(px(-h * peek_offset((t + phase).fract(), slide))),
                    )
                    .into_any_element()
            }
            _ => face(front, true).into_any_element(),
        };

        div()
            .id(self.id)
            .relative()
            .flex_none()
            .w(px(w))
            .h(px(h))
            .overflow_hidden()
            .bg(bg)
            .font_family(font)
            .child(body)
            .when_some(self.on_click, |el, f| {
                el.cursor_pointer().active(|s| s.opacity(0.8)).on_click(move |e, w, cx| f(e, w, cx))
            })
    }
}

fn default_front(size: TileSize, icon: Option<Icon>, count: Option<u32>, ink: Hsla) -> AnyElement {
    let count = count.filter(|&c| c > 0);
    let icon_px = match size {
        TileSize::Small => 40.,
        TileSize::Medium | TileSize::Wide => 64.,
        TileSize::Large => 96.,
    };
    match size {
        TileSize::Small => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.))
            .children(icon.map(|i| i.render(px(icon_px), ink)))
            .children(count.map(|c| {
                div().absolute().top(px(4.)).right(px(8.)).text_size(px(size::NORMAL)).text_color(ink).child(c.to_string())
            }))
            .into_any_element(),
        _ => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(8.))
            .children(icon.map(|i| i.render(px(icon_px), ink)))
            .children(count.map(|c| {
                div()
                    .text_size(px(size::HUGE * 0.8))
                    .font_weight(crate::theme::LIGHT)
                    .text_color(ink)
                    .child(c.to_string())
            }))
            .into_any_element(),
    }
}

/// The start screen: tiles packed by [`pack`] and positioned absolutely.
#[derive(IntoElement)]
pub struct TileGrid {
    tiles: Vec<Tile>,
    columns: usize,
    unit: f32,
    gap: f32,
}

pub fn tile_grid() -> TileGrid {
    TileGrid { tiles: Vec::new(), columns: 4, unit: TILE_UNIT, gap: TILE_GAP }
}

impl TileGrid {
    pub fn tile(mut self, tile: Tile) -> Self {
        self.tiles.push(tile);
        self
    }
    pub fn tiles(mut self, tiles: impl IntoIterator<Item = Tile>) -> Self {
        self.tiles.extend(tiles);
        self
    }
    /// Grid width in small-tile units (4 on WP8 by default, 6 with the "show more tiles" setting).
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns;
        self
    }
    pub fn metrics(mut self, unit: f32, gap: f32) -> Self {
        self.unit = unit;
        self.gap = gap;
        self
    }

    /// The width this grid occupies, for centring.
    pub fn width(&self) -> f32 {
        self.columns as f32 * self.unit + (self.columns.saturating_sub(1)) as f32 * self.gap
    }
}

impl RenderOnce for TileGrid {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let sizes: Vec<TileSize> = self.tiles.iter().map(|t| t.size).collect();
        let positions = pack(&sizes, self.columns);
        let rows = packed_rows(&sizes, &positions);
        let step = self.unit + self.gap;
        let width = self.width();
        let height = rows as f32 * step - if rows > 0 { self.gap } else { 0. };
        let (unit, gap) = (self.unit, self.gap);
        div().relative().flex_none().w(px(width)).h(px(height)).children(
            self.tiles.into_iter().zip(positions).map(move |(t, (c, r))| {
                div()
                    .absolute()
                    .left(px(c as f32 * step))
                    .top(px(r as f32 * step))
                    .child(t.metrics(unit, gap))
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::TileSize::*;
    use super::*;

    #[test]
    fn smalls_fill_a_block_before_moving_on() {
        let p = pack(&[Small, Small, Small, Small, Small], 4);
        assert_eq!(p, vec![(0, 0), (1, 0), (0, 1), (1, 1), (2, 0)]);
    }

    #[test]
    fn medium_and_wide_align_to_blocks() {
        let p = pack(&[Medium, Small, Wide, Medium, Medium], 4);
        assert_eq!(p, vec![(0, 0), (2, 0), (0, 2), (0, 4), (2, 4)]);
    }

    #[test]
    fn smalls_backfill_holes_left_by_wide_tiles() {
        // Medium + 2 smalls; wide forces a new band; then a small goes back up.
        let p = pack(&[Medium, Small, Small, Wide, Small], 4);
        assert_eq!(p, vec![(0, 0), (2, 0), (3, 0), (0, 2), (2, 1)]);
        assert_eq!(packed_rows(&[Medium, Small, Small, Wide, Small], &p), 4);
    }

    #[test]
    fn tiles_never_overlap() {
        let sizes = [Wide, Small, Medium, Small, Small, Large, Small, Medium, Wide, Small, Small, Small];
        for cols in [4, 6, 8] {
            let p = pack(&sizes, cols);
            let mut cells = std::collections::HashSet::new();
            for (s, (c, r)) in sizes.iter().zip(&p) {
                let (w, h) = s.span();
                assert!(c + w <= cols);
                for y in *r..r + h {
                    for x in *c..c + w {
                        assert!(cells.insert((x, y)), "overlap at {x},{y} with {cols} columns");
                    }
                }
            }
        }
    }

    #[test]
    fn pixel_sizes_include_gutters() {
        assert_eq!(Small.pixels(99., 12.), (99., 99.));
        assert_eq!(Medium.pixels(99., 12.), (210., 210.));
        assert_eq!(Wide.pixels(99., 12.), (432., 210.));
    }

    #[test]
    fn peek_rests_on_each_face() {
        let slide = PEEK_SLIDE_SECS / 12.;
        assert_eq!(peek_offset(0.1, slide), 0.);
        assert_eq!(peek_offset(0.7, slide), 1.);
        assert!(peek_offset(0.47, slide) > 0. && peek_offset(0.47, slide) < 1.);
        assert!(peek_offset(0.999, slide) < 0.1);
        // The slide lasts the same wall-clock time whatever the period.
        for period in [6f32, 12., 20.] {
            let slide = PEEK_SLIDE_SECS / period;
            assert!((slide * period - PEEK_SLIDE_SECS).abs() < 1e-5);
        }
    }
}
