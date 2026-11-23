//! # gpui-metro
//!
//! The Windows Phone "Metro" design language for [GPUI]: type-first layouts,
//! flat accent colour, live tiles, pivots and panoramas.
//!
//! ```ignore
//! use gpui::*;
//! use gpui_metro::prelude::*;
//!
//! gpui_platform::application().with_assets(gpui_metro::Assets::new()).run(|cx| {
//!     gpui_metro::init(cx);
//!     Theme::set_accent(Accent::Cobalt, cx);
//!     // open a window and render tiles, pivots, …
//! });
//! ```
//!
//! Every component reads the global [`Theme`] — dark or light ground plus one
//! of twenty accents — so switching either re-skins the whole app at once.
//!
//! [GPUI]: https://www.gpui.rs

mod components;
pub mod icons;
pub mod motion;
pub mod scaled;
pub mod text_input;
pub mod theme;

pub use components::*;
pub use icons::{Assets, Icon, icon};
pub use scaled::{Scaled, scaled};
pub use theme::{Accent, Mode, Palette, Theme, u};

pub use app_bar::{AppBar, AppBarMode, app_bar};
pub use button::{Button, HyperlinkButton, RoundButton, button, hyperlink, round_button};
pub use chrome::{PAGE_MARGIN, PageHeader, SystemTray, page_header, system_tray};
pub use dialog::{MessageBox, Toast, message_box, toast};
pub use list::{ALPHABET, JumpList, Leading, ListGroup, ListItem, ListPicker, jump_list, list_item, list_picker};
pub use panorama::{Panorama, panorama};
pub use pivot::{Pivot, pivot};
pub use progress::{ProgressBar, ProgressRing, progress_bar, progress_dots, progress_ring};
pub use slider::{Slider, slider};
pub use text::{Text, TextStyle, text};
pub use text_box::{TextBox, text_box};
pub use text_input::{InputEvent, TextInput};
pub use tile::{TILE_GAP, TILE_UNIT, Tile, TileGrid, TileSize, tile, tile_grid};
pub use toggle::{CheckBox, RadioButton, ToggleSwitch, check_box, radio_button, toggle_switch};

/// Installs the default theme (dark, cobalt) and the text-input key bindings.
/// Call once at startup, before opening a window.
pub fn init(cx: &mut gpui::App) {
    if !cx.has_global::<Theme>() {
        cx.set_global(Theme::default());
    }
    text_input::bind_keys(cx);
}

/// Everything an app usually needs in scope.
pub mod prelude {
    pub use crate::{
        Accent, Icon, Mode, Palette, Theme, TextStyle, TileSize, app_bar, button, check_box, hyperlink, jump_list,
        list_item, list_picker, message_box, page_header, panorama, pivot, progress_bar, progress_dots,
        progress_ring, radio_button, round_button, scaled, slider, system_tray, text, text_box, tile, tile_grid,
        toast, toggle_switch, u,
    };
}
