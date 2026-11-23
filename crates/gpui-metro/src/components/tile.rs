//! Live tiles and the start-screen grid that packs them.
//!
//! A tile is a flat accent square with its name in the bottom-left corner
//! and, optionally, a count. A *live* tile has a back face; it periodically
//! slides up to show it (the WP7 hub-tile peek), then slides back.

use std::rc::Rc;
use std::time::{Duration, Instant};

use std::hash::{Hash, Hasher};

use gpui::{
    AnyElement, App, ClickEvent, Context, ElementId, Hsla, InteractiveElement, IntoElement,
    ParentElement, Render, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, div,
};

use crate::theme::u;

use crate::{
    Icon, Theme,
    chrome::PAGE_MARGIN,
    motion::{self, ENTER, Turn, ease_in_exp6, ease_in_out_cubic, ease_out_exp6, turn},
    theme::size,
};

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
    /// What the faces show, as a hash; see [`Tile::content_key`].
    content_key: Option<u64>,
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
        content_key: None,
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
    /// Everything the faces show, hashed. A live tile given one draws its
    /// faces in a view of its own and keeps the last drawing until the key
    /// changes, so a slide redraws this tile and not the page around it —
    /// with many live tiles one is nearly always sliding, and without it
    /// every frame of every slide redraws them all. Without a key the tile
    /// cannot tell when its faces changed, and redraws with its page.
    ///
    /// The page itself is still put together again every frame of a slide:
    /// a view that asks for a frame dirties every view above it. Keep the
    /// grid out of a `cached` view of its own, too — gpui redraws everything
    /// under a cached view it redraws, tiles included.
    pub fn content_key(mut self, key: impl Hash) -> Self {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut h);
        self.content_key = Some(h.finish());
        self
    }
}

/// A live tile's sliding body, as a view of its own: the peek's frames
/// notify this view, which redraws one tile (see [`Tile::content_key`]).
/// The tile hands it its faces every time the page draws it.
#[derive(Default)]
struct LiveBody {
    id: Option<ElementId>,
    front: Option<Face>,
    back: Option<Face>,
    size: Option<TileSize>,
    bg: Option<Hsla>,
    ink: Option<Hsla>,
    title: Option<SharedString>,
    w: f32,
    h: f32,
    period: f32,
    phase: f32,
    key: Option<u64>,
}

impl Render for LiveBody {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (Some(id), Some(front), Some(back), Some(size), Some(bg), Some(ink)) =
            (self.id.clone(), self.front.take(), self.back.take(), self.size, self.bg, self.ink)
        else {
            return div().into_any_element();
        };
        let (w, h) = (self.w, self.h);
        let slide = PEEK_SLIDE_SECS / self.period;
        let t = peek_clock(&id, self.period, self.phase, slide, window, cx);
        let title = self.title.clone();
        let face = |content: AnyElement| {
            div().relative().w(u(w)).h(u(h)).flex_none().overflow_hidden().child(content).children(
                title.clone().map(|t| {
                    div().absolute().left(u(8.)).bottom(u(6.)).text_size(u(size::SMALL)).text_color(ink).child(t)
                }),
            )
        };
        // The strip holds both faces stacked; it rides up by one face.
        div()
            .relative()
            .size_full()
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top(u(-h * peek_offset(t, slide)))
                    .w(u(w))
                    .h(u(h * 2.))
                    .flex()
                    .flex_col()
                    .child(face(front(size, bg)))
                    .child(face(back(size, bg))),
            )
            .into_any_element()
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

/// Cycle-fraction until the next slide starts, or `None` mid-slide.
pub fn peek_wait(t: f32, slide: f32) -> Option<f32> {
    let slide = slide.clamp(0.001, 0.5);
    match t {
        t if t < 0.5 - slide => Some(0.5 - slide - t),
        t if t < 0.5 => None,
        t if t < 1. - slide => Some(1. - slide - t),
        _ => None,
    }
}

/// A live tile's clock: when it started, and the wake-up booked for its
/// next slide.
struct PeekClock {
    start: Instant,
    wake: Option<(Instant, gpui::Task<()>)>,
}

/// Where in its cycle (0–1) a live tile is now. Frames are requested only
/// while it slides; resting on a face, it books one wake-up for the next
/// slide instead. (A repeating animation redrew the whole window every
/// frame, forever, for a tile that sits still 85% of the time.)
fn peek_clock(id: &ElementId, period: f32, phase: f32, slide: f32, window: &mut Window, cx: &mut App) -> f32 {
    let key = ElementId::from((SharedString::from(format!("{id}-peek")), 0usize));
    let clock = window.use_keyed_state(key, cx, |_, _| PeekClock { start: Instant::now(), wake: None });
    let now = Instant::now();
    let start = clock.read(cx).start;
    let t = (now.saturating_duration_since(start).as_secs_f32() / period + phase).fract();
    match peek_wait(t, slide) {
        None => window.request_animation_frame(),
        Some(wait) => {
            let view = window.current_view();
            let weak = clock.downgrade();
            let booked = clock.read(cx).wake.as_ref().is_some_and(|(at, _)| *at > now);
            if !booked {
                let dt = Duration::from_secs_f32(wait * period);
                let task = window.spawn(cx, async move |cx| {
                    cx.background_executor().timer(dt).await;
                    cx.update(|_, cx| {
                        // Cleared first: a timer that fires a hair early
                        // must not leave a stale booking that blocks the next.
                        weak.update(cx, |c, _| c.wake = None).ok();
                        cx.notify(view);
                    })
                    .ok();
                });
                clock.update(cx, |c, _| c.wake = Some((now + dt, task)));
            }
        }
    }
    t
}

impl RenderOnce for Tile {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
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
                    .left(u(8.))
                    .bottom(u(6.))
                    .text_size(u(size::SMALL))
                    .text_color(ink)
                    .child(t)
            })
        };

        let body: AnyElement = match self.back {
            Some(back) if !reduced => {
                let (icon, count) = (self.icon, self.count);
                let front: Face = match self.front {
                    Some(f) => f,
                    None => Box::new(move |s, _| default_front(s, icon, count, ink)),
                };
                let key = self.content_key;
                let view = window.use_keyed_state(
                    ElementId::from((SharedString::from(format!("{}-live", self.id)), 0usize)),
                    cx,
                    |_, _| LiveBody::default(),
                );
                // Drawn afresh when what it shows changed (or nothing says),
                // and from its last drawing otherwise.
                let fresh = key.is_none() || view.read(cx).key != key;
                let (id, title, period, phase) = (self.id.clone(), self.title.clone(), self.period, self.phase);
                view.update(cx, |v, _| {
                    *v = LiveBody {
                        id: Some(id),
                        front: Some(front),
                        back: Some(back),
                        size: Some(size),
                        bg: Some(bg),
                        ink: Some(ink),
                        title,
                        w,
                        h,
                        period,
                        phase,
                        key,
                    };
                });
                if fresh {
                    view.into_any_element()
                } else {
                    view.cached(StyleRefinement::default().w(u(w)).h(u(h))).into_any_element()
                }
            }
            _ => {
                let front: AnyElement = match self.front {
                    Some(f) => f(size, bg),
                    None => default_front(size, self.icon, self.count, ink),
                };
                div()
                    .relative()
                    .w(u(w))
                    .h(u(h))
                    .flex_none()
                    .overflow_hidden()
                    .child(front)
                    .children(label())
                    .into_any_element()
            }
        };

        let face = div().relative().w(u(w)).h(u(h)).overflow_hidden().bg(bg).font_family(font).child(body);
        match self.on_click {
            // A tappable tile tilts under the pointer.
            Some(f) => motion::tilt(self.id, face, window, cx)
                .flex_none()
                .w(u(w))
                .h(u(h))
                .cursor_pointer()
                .on_click(move |e, w, cx| f(e, w, cx))
                .into_any_element(),
            None => div().id(self.id).flex_none().child(face).into_any_element(),
        }
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
            .gap(u(4.))
            .children(icon.map(|i| i.render(u(icon_px), ink)))
            .children(count.map(|c| {
                div().absolute().top(u(4.)).right(u(8.)).text_size(u(size::NORMAL)).text_color(ink).child(c.to_string())
            }))
            .into_any_element(),
        _ => div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .gap(u(8.))
            .children(icon.map(|i| i.render(u(icon_px), ink)))
            .children(count.map(|c| {
                div()
                    .text_size(u(size::HUGE * 0.8))
                    .font_weight(crate::theme::LIGHT)
                    .text_color(ink)
                    .child(c.to_string())
            }))
            .into_any_element(),
    }
}

/// The start screen: tiles packed by [`pack`] and positioned absolutely.
/// On a phone that is one column of blocks; on the desktop (Windows 8) it
/// is several named groups side by side, each packed on its own.
#[derive(IntoElement)]
pub struct TileGrid {
    id: ElementId,
    tiles: Vec<Tile>,
    /// Group index per tile.
    group_of: Vec<usize>,
    /// Group titles; the first group is the implicit one `tile()` fills.
    groups: Vec<Option<SharedString>>,
    columns: usize,
    unit: f32,
    gap: f32,
    turnstile: bool,
    replay: usize,
    leave: bool,
    hinge: f32,
}

pub fn tile_grid() -> TileGrid {
    TileGrid {
        id: "tile-grid".into(),
        tiles: Vec::new(),
        group_of: Vec::new(),
        groups: vec![None],
        columns: 4,
        unit: TILE_UNIT,
        gap: TILE_GAP,
        turnstile: false,
        replay: 0,
        leave: false,
        hinge: PAGE_MARGIN,
    }
}

/// Space between groups, and the height of the group title row.
pub const GROUP_GAP: f32 = 56.;
pub const GROUP_HEADER: f32 = 44.;

impl TileGrid {
    /// Needed only to tell two grids in the same view apart.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }
    /// Adds a tile to the current (last) group.
    pub fn tile(mut self, tile: Tile) -> Self {
        self.group_of.push(self.groups.len() - 1);
        self.tiles.push(tile);
        self
    }
    pub fn tiles(self, tiles: impl IntoIterator<Item = Tile>) -> Self {
        tiles.into_iter().fold(self, |g, t| g.tile(t))
    }
    /// Starts a new named group to the right of the previous one (the
    /// Windows 8 start screen). Later `tile()` calls fill it.
    pub fn group(mut self, title: impl Into<SharedString>, tiles: impl IntoIterator<Item = Tile>) -> Self {
        let title = Some(title.into());
        if self.tiles.is_empty() && self.groups.len() == 1 {
            self.groups[0] = title;
        } else {
            self.groups.push(title);
        }
        self.tiles(tiles)
    }
    /// Distance from the turnstile's hinge (the screen's left edge) to the
    /// grid's left edge, in design px. Defaults to the page margin; set it
    /// when the grid sits further in or is scrolled.
    pub fn hinge(mut self, distance: f32) -> Self {
        self.hinge = distance;
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
    /// The start screen's turnstile. When the grid appears, its tiles swing
    /// in one after another; launching from a tile swings them away in a
    /// wave from the top-left, the tapped tile last, and only then does the
    /// tile's `on_click` run. Taps during the exit are ignored. With reduced
    /// motion nothing moves and clicks run at once.
    pub fn turnstile(mut self, on: bool) -> Self {
        self.turnstile = on;
        self
    }
    /// Swing every tile away now (in reading order), as when the start
    /// screen is left without a tile being tapped. Keep it set while the
    /// grid is still on screen; remove the grid after [`EXIT_TILE`] plus
    /// the wave (at most ~0.5 s).
    pub fn leave(mut self, leave: bool) -> Self {
        self.leave = leave;
        self
    }
    /// Change this to replay the entrance without remounting the grid.
    pub fn replay(mut self, key: usize) -> Self {
        self.replay = key;
        self
    }

    /// The width of one group, in design px.
    pub fn group_width(&self) -> f32 {
        self.columns as f32 * self.unit + (self.columns.saturating_sub(1)) as f32 * self.gap
    }

    /// The width this grid occupies, for centring or scrolling.
    pub fn width(&self) -> f32 {
        let n = self.group_count() as f32;
        n * self.group_width() + (n - 1.).max(0.) * GROUP_GAP
    }

    fn group_count(&self) -> usize {
        self.group_of.iter().max().map_or(1, |g| g + 1)
    }
}

/// How long each tile takes to swing out.
pub const EXIT_TILE: Duration = Duration::from_millis(250);
/// Gap between rows in the wave (compressed on tall grids).
const EXIT_STEP: Duration = Duration::from_millis(45);
/// The whole wave (before the tapped tile) never takes longer than this.
const EXIT_WAVE_MAX: Duration = Duration::from_millis(260);
/// The tapped tile lingers this long after the last of the others starts.
const EXIT_HOLD: Duration = Duration::from_millis(90);

/// Rank of each tile's row among the rows in use (0 = top), so a feather
/// moves whole rows: tiles side by side turn together and never cross.
fn row_ranks(positions: &[(usize, usize)], skip: usize) -> (Vec<usize>, usize) {
    let mut rows: Vec<usize> = positions.iter().enumerate().filter(|&(i, _)| i != skip).map(|(_, p)| p.1).collect();
    rows.sort_unstable();
    rows.dedup();
    let ranks = positions.iter().map(|p| rows.binary_search(&p.1).unwrap_or(0)).collect();
    (ranks, rows.len())
}

/// Start delays for the turnstile-out, per tile, plus the total time until
/// the last tile has gone. Rows leave top to bottom, a whole row at once
/// (the Toolkit's feather goes by vertical position); the tapped tile goes
/// last, on its own.
pub fn exit_schedule(positions: &[(usize, usize)], tapped: usize) -> (Vec<Duration>, Duration) {
    let (ranks, rows) = row_ranks(positions, tapped);
    let step = match rows {
        0 | 1 => EXIT_STEP,
        n => EXIT_STEP.min(EXIT_WAVE_MAX / (n as u32 - 1)),
    };
    let mut delays: Vec<Duration> = ranks.iter().map(|&r| step * r as u32).collect();
    let last_row = rows.checked_sub(1).map(|r| step * r as u32);
    let tapped_delay = last_row.map_or(Duration::ZERO, |d| d + EXIT_HOLD);
    if tapped < delays.len() {
        delays[tapped] = tapped_delay;
    }
    (delays, tapped_delay + EXIT_TILE)
}

/// Where a tile turns in from when the start screen appears: edge-on, so it
/// opens from nothing rather than popping in half-turned.
const ENTER_FROM_DEG: f32 = 90.;
/// Gap between rows as they turn in.
const ENTER_STEP: Duration = Duration::from_millis(40);
const ENTER_WAVE_MAX: Duration = Duration::from_millis(240);

/// Start delays for the turnstile-in: rows top to bottom, a row at a time.
pub fn enter_schedule(positions: &[(usize, usize)]) -> Vec<Duration> {
    let (ranks, rows) = row_ranks(positions, usize::MAX);
    let step = match rows {
        0 | 1 => ENTER_STEP,
        n => ENTER_STEP.min(ENTER_WAVE_MAX / (n as u32 - 1)),
    };
    ranks.iter().map(|&r| step * r as u32).collect()
}

/// A tile's place in the grid, for projecting it as it turns.
#[derive(Clone, Copy)]
struct Cell {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    /// Distance from the hinge to the grid's left edge.
    hinge: f32,
}

impl Cell {
    /// The tile turned by `degrees` about the hinge, as seen in perspective:
    /// its span narrows toward the hinge and its height follows the
    /// perspective scale. Returns (span, scale, height).
    fn project(self, degrees: f32) -> (Turn, f32, f32) {
        let r = turn(self.x + self.hinge, self.w, degrees);
        let k = (r.scale_left + r.scale_right) / 2.;
        (r, k, self.h * k)
    }

    /// How far the tile's middle has receded from the screen at `degrees`.
    fn depth(self, degrees: f32) -> f32 {
        (self.x + self.hinge + self.w / 2.) * degrees.to_radians().sin()
    }

    /// `tile` turned by `degrees`, drawn in the same structure whether it is
    /// turning or at rest, so the tile (and its tilt) keeps its state. The
    /// tile is clipped to its projected span (narrower toward the hinge,
    /// shorter by the perspective) but never rescaled: re-rasterising its
    /// text and glyph at a new size every frame made it shimmer.
    fn render(self, tile: Tile, degrees: f32, visible: bool) -> AnyElement {
        let (r, _, h) = self.project(degrees);
        let crop = (self.h - h) / 2.;
        div()
            .absolute()
            .overflow_hidden()
            .left(u(r.left - self.hinge))
            .top(u(self.y + crop))
            .w(u(r.width))
            .h(u(h))
            .opacity(if visible && r.width >= 1. { 1. } else { 0. })
            .child(div().absolute().left_0().top(u(-crop)).child(tile))
            .into_any_element()
    }
}

/// The grid's turnstile clock. Tiles are drawn from it every frame (no
/// animation elements), so a pressed tile stays the same element from its
/// entrance through its exit.
struct GridMotion {
    /// When the entrance started (on appearing, or on a new `replay`).
    entered_at: Instant,
    replay: usize,
    /// The exit in progress: the tapped tile (`usize::MAX` for none) and when.
    exit: Option<(usize, Instant)>,
}

/// Seconds since `t`, minus `delay` (negative while still waiting).
fn since(t: Instant, delay: Duration, now: Instant) -> f32 {
    now.saturating_duration_since(t).as_secs_f32() - delay.as_secs_f32()
}

impl RenderOnce for TileGrid {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let step = self.unit + self.gap;
        let width = self.width();
        let (unit, gap, hinge) = (self.unit, self.gap, self.hinge);
        let group_count = self.group_count();
        let titled = self.groups.iter().any(Option::is_some);
        let top = if titled { GROUP_HEADER } else { 0. };
        let group_x = |g: usize| g as f32 * (self.group_width() + GROUP_GAP);

        // Pack each group on its own, then place it in the row of groups.
        // `positions` are in grid units across the whole row (for the exit
        // wave's reading order); `origins` are design px.
        let mut positions = vec![(0, 0); self.tiles.len()];
        let mut origins = vec![(0., 0.); self.tiles.len()];
        let mut rows = 0;
        for g in 0..group_count {
            let members: Vec<usize> = (0..self.tiles.len()).filter(|&i| self.group_of[i] == g).collect();
            let sizes: Vec<TileSize> = members.iter().map(|&i| self.tiles[i].size).collect();
            let packed = pack(&sizes, self.columns);
            rows = rows.max(packed_rows(&sizes, &packed));
            for (&i, &(c, r)) in members.iter().zip(&packed) {
                positions[i] = (g * (self.columns + 1) + c, r);
                origins[i] = (group_x(g) + c as f32 * step, top + r as f32 * step);
            }
        }
        let height = top + rows as f32 * step - if rows > 0 { self.gap } else { 0. };
        let headers: Vec<AnyElement> = self
            .groups
            .iter()
            .enumerate()
            .filter_map(|(g, title)| {
                let title = title.clone()?;
                Some(
                    div()
                        .absolute()
                        .left(u(group_x(g)))
                        .top_0()
                        .text_size(u(size::MEDIUM_LARGE))
                        .font_weight(crate::theme::SEMILIGHT)
                        .text_color(Theme::palette(cx).foreground)
                        .child(SharedString::from(title.to_lowercase()))
                        .into_any_element(),
                )
            })
            .collect();

        let reduced = Theme::get(cx).reduced_motion;
        let now = Instant::now();
        let motion = (self.turnstile && !reduced).then(|| {
            window.use_keyed_state(self.id.clone(), cx, |_, _| GridMotion {
                entered_at: Instant::now(),
                replay: self.replay,
                exit: None,
            })
        });
        // Sync the clock with the props: a new replay restarts the entrance;
        // `leave` starts an exit with no tapped tile.
        let state = motion.as_ref().map(|m| {
            m.update(cx, |m, _| {
                if m.replay != self.replay {
                    m.replay = self.replay;
                    m.entered_at = now;
                    m.exit = None;
                }
                match (self.leave, m.exit) {
                    (true, None) => m.exit = Some((usize::MAX, now)),
                    (false, Some((usize::MAX, _))) => {
                        m.exit = None;
                        m.entered_at = now;
                    }
                    _ => {}
                }
                (m.entered_at, m.exit)
            })
        });
        let enter_delays = enter_schedule(&positions);
        let exit_delays = state.and_then(|(_, exit)| exit).map(|(t, _)| exit_schedule(&positions, t).0);
        // Exit length per tapped tile, so each click knows when to launch.
        // Each click works out its own exit length when it happens; doing it
        // for every tile here cost O(n²) per frame for a value used once.
        let positions = Rc::new(positions);

        // Each tile's angle now, and whether it is still moving.
        let pose = |ix: usize| -> (f32, bool, bool) {
            match state {
                None => (0., true, false),
                Some((_, Some((_, started)))) => {
                    let delay = exit_delays.as_ref().map_or(Duration::ZERO, |d| d[ix]);
                    let p = (since(started, delay, now) / EXIT_TILE.as_secs_f32()).clamp(0., 1.);
                    (90. * ease_in_exp6(p), p < 1., p < 1.)
                }
                Some((entered, None)) => {
                    let e = since(entered, enter_delays[ix], now);
                    if e < 0. {
                        // Waiting for its turn: not shown yet.
                        return (ENTER_FROM_DEG, false, true);
                    }
                    let p = (e / ENTER.as_secs_f32()).clamp(0., 1.);
                    (ENTER_FROM_DEG * (1. - ease_out_exp6(p)), true, p < 1.)
                }
            }
        };
        let mut moving = false;
        // Paint order: a tile turned further away sits deeper, so it goes
        // behind its neighbours. Without this a row-mate swinging toward the
        // hinge sweeps across the tapped tile (still flat, in front) instead
        // of passing behind it. Element state is keyed by id, so reordering
        // children keeps every tile's state.
        let mut layers: Vec<(f32, AnyElement)> = Vec::with_capacity(self.tiles.len());

        self
            .tiles
            .into_iter()
            .zip(origins)
            .enumerate()
            .for_each(|(ix, (mut t, (x, y)))| {
                // Route the click through the exit: animate first, launch after.
                // Only a turnstile grid reroutes the click. Taking `on_click`
                // inside a tuple pattern would drop it on every other grid.
                if let Some(motion) = motion.clone()
                    && let Some(launch) = t.on_click.take()
                {
                    let positions = positions.clone();
                    t.on_click = Some(Rc::new(move |e: &ClickEvent, window: &mut Window, cx: &mut App| {
                        let started = motion.update(cx, |m, cx| {
                            if m.exit.is_some() {
                                return false;
                            }
                            m.exit = Some((ix, Instant::now()));
                            cx.notify();
                            true
                        });
                        if !started {
                            return;
                        }
                        let total = exit_schedule(&positions, ix).1;
                        let (e, launch, motion) = (e.clone(), launch.clone(), motion.clone());
                        window
                            .spawn(cx, async move |cx| {
                                cx.background_executor().timer(total).await;
                                cx.update(|window, cx| {
                                    // Re-arm first, so a grid that stays on screen comes back.
                                    motion.update(cx, |m, cx| {
                                        m.exit = None;
                                        cx.notify();
                                    });
                                    launch(&e, window, cx);
                                })
                                .ok();
                            })
                            .detach();
                    }));
                }
                let (tw, th) = t.size.pixels(unit, gap);
                let (degrees, visible, running) = pose(ix);
                moving |= running;
                let cell = Cell { x, y, w: tw, h: th, hinge };
                layers.push((cell.depth(degrees), cell.render(t.metrics(unit, gap), degrees, visible)));
            });
        // Stable: tiles at the same depth keep their order.
        layers.sort_by(|a, b| b.0.total_cmp(&a.0));
        let tiles = layers.into_iter().map(|(_, el)| el);
        if moving {
            window.request_animation_frame();
        }

        div().relative().flex_none().w(u(width)).h(u(height)).children(headers).children(tiles)
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
    fn exit_waves_in_reading_order_and_the_tapped_tile_goes_last() {
        let sizes = [Wide, Medium, Small, Small, Small, Small];
        let positions = pack(&sizes, 4);
        let tapped = 1;
        let (delays, total) = exit_schedule(&positions, tapped);
        let others: Vec<usize> = (0..sizes.len()).filter(|&i| i != tapped).collect();
        assert!(others.iter().all(|&i| delays[i] < delays[tapped]), "{delays:?}");
        // Reading order: a tile higher up (or further left on the same row) never leaves later.
        for &a in &others {
            for &b in &others {
                let (pa, pb) = (positions[a], positions[b]);
                if (pa.1, pa.0) < (pb.1, pb.0) {
                    assert!(delays[a] <= delays[b], "{a} {pa:?} vs {b} {pb:?}");
                }
            }
        }
        assert_eq!(total, delays[tapped] + EXIT_TILE);
    }

    #[test]
    fn groups_sit_side_by_side_and_each_packs_alone() {
        let g = tile_grid()
            .group("floor", [tile("a", Medium), tile("b", Small)])
            .group("biology", [tile("c", Wide)]);
        assert_eq!(g.group_count(), 2);
        assert_eq!(g.group_of, vec![0, 0, 1]);
        assert_eq!(g.groups.len(), 2, "the first group takes the implicit slot");
        assert_eq!(g.width(), 2. * g.group_width() + GROUP_GAP);
        assert_eq!(tile_grid().tile(tile("x", Small)).width(), tile_grid().group_width());
    }

    #[test]
    fn tiles_turn_in_top_left_first() {
        let sizes = [Wide, Medium, Small, Small];
        let positions = pack(&sizes, 4);
        let d = enter_schedule(&positions);
        // wide (row 0) first; the medium and the two smalls beside it share row 2.
        assert_eq!(d[0], Duration::ZERO);
        assert!(d[1] > d[0]);
        assert_eq!((d[1], d[2], d[3]), (d[1], d[1], d[1]), "a row turns together: {d:?}");
        assert!(enter_schedule(&vec![(0, 0); 80]).iter().all(|&x| x <= ENTER_WAVE_MAX));
    }

    #[test]
    fn exit_stays_short_on_big_grids() {
        let sizes = vec![Small; 60];
        let positions = pack(&sizes, 4);
        let (_, total) = exit_schedule(&positions, 30);
        assert!(total <= EXIT_WAVE_MAX + EXIT_HOLD + EXIT_TILE, "{total:?}");
        let (delays, total) = exit_schedule(&[(0, 0)], 0);
        assert_eq!((delays[0], total), (Duration::ZERO, EXIT_TILE));
    }

    #[test]
    fn a_resting_tile_waits_exactly_until_its_next_slide() {
        let slide = 0.1;
        assert!((peek_wait(0.1, slide).unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(peek_wait(0.45, slide), None, "mid-slide: draw every frame");
        assert!((peek_wait(0.6, slide).unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(peek_wait(0.95, slide), None);
        // Wherever it waits, the face doesn't move until the wait is over.
        for t in [0.0, 0.2, 0.39, 0.5, 0.7, 0.89] {
            let w = peek_wait(t, slide).unwrap();
            assert_eq!(peek_offset(t, slide), peek_offset(t + w * 0.999, slide), "t = {t}");
        }
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
