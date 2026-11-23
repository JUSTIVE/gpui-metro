//! no ativ — a cyberpunk bio-foundry, as a Windows Phone app. The incubator
//! for gpui-metro: a real-feeling domain that leans on every component.
//!
//! `NO_ATIV_PAGE=<page>` boots straight into a page over the start screen
//! (runs, run, telemetry, strains, strain, incidents, incident, new-incident,
//! reagents, shipments, freezer, deck, people, settings).

mod app;
mod data;
mod nav;
mod pages;
mod store;

use gpui::{
    Bounds, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size,
};

fn main() {
    gpui_platform::application()
        .with_assets(gpui_metro::Assets::new())
        .run(|cx| {
            gpui_metro::init(cx);
            app::bind_keys(cx);
            let boot = std::env::var("NO_ATIV_PAGE").ok();
            let screen = size(px(480.), px(800.));
            let window = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, screen, cx))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("no ativ".into()),
                        ..Default::default()
                    }),
                    is_resizable: false,
                    window_min_size: Some(screen),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| app::NoAtiv::new(boot.as_deref(), window, cx)),
            );
            if let Err(e) = window {
                eprintln!("no ativ: could not open a window: {e:#}");
                cx.quit();
            }
            cx.activate(true);
        });
}
