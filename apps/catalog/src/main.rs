//! The gpui-metro component catalog: browse every component of the library
//! in a live phone frame, under any ground and accent.
//!
//! Environment (handy for screenshots):
//! * `CATALOG_PAGE=pivot` opens straight onto a page (any page name; case,
//!   spaces and dashes are ignored).
//! * `CATALOG_MODE=light` starts on the light ground.

mod app;
mod demos;
mod pages;
mod phone;
mod sidebar;
mod topbar;

use gpui::{App, AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
use gpui_metro::{Mode, Theme};

use crate::{app::Catalog, pages::Page};

fn main() {
    gpui_platform::application().with_assets(gpui_metro::Assets::new()).run(|cx: &mut App| {
        gpui_metro::init(cx);
        // Live tiles slide together every 3 s; between waves nothing is drawn.
        gpui_metro::motion::set_live_tile_beat(Some(3.), cx);
        if std::env::var("CATALOG_MODE").is_ok_and(|m| m.eq_ignore_ascii_case("light")) {
            Theme::set_mode(Mode::Light, cx);
        }
        let page = std::env::var("CATALOG_PAGE").ok().and_then(|p| Page::find(&p)).unwrap_or(Page::Palette);

        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1280.), px(880.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions { title: Some("gpui-metro catalog".into()), ..Default::default() }),
                window_min_size: Some(size(px(1200.), px(640.))),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Catalog::new(page, window, cx)),
        )
        .expect("failed to open the catalog window");
        cx.activate(true);
    });
}
