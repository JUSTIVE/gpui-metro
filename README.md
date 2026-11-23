# gpui-metro

The Windows Phone **Metro** design language for [GPUI](https://www.gpui.rs):
type-first layouts, flat accent colour, live tiles, pivots and panoramas.

```
crates/gpui-metro   the library
apps/catalog        every component, live, in a phone frame (browse + theme switcher)
apps/no-ativ        incubator: a bio-foundry console as a WP8 app suite
```

## Run

```sh
cargo run -p catalog
cargo run -p no-ativ
cargo test -p gpui-metro
```

GPUI is pinned to `0.2.2` with `runtime_shaders`, so the build does not need
Xcode's Metal Toolchain. Shaders compile when the app starts.

## Using the library

```rust
use gpui::*;
use gpui_metro::prelude::*;

fn main() {
    Application::new().with_assets(gpui_metro::Assets::new()).run(|cx| {
        gpui_metro::init(cx);                 // dark ground, cobalt accent, text-input keys
        Theme::set_accent(Accent::Lime, cx);  // any of the 20 WP8 accents
        // cx.open_window(...) and render components from a view
    });
}
```

If your app ships its own assets, chain them: `gpui_metro::Assets::with_fallback(MyAssets)`.

### Components

| area | components |
| --- | --- |
| theme | `Theme` (`Mode::Dark`/`Light` × 20 `Accent`s), `Palette`, type ramp, `TextStyle` |
| tiles | `tile` (small · medium · wide · large, count, icon, live back face), `tile_grid` (start-screen packing) |
| navigation | `pivot`, `panorama`, `app_bar` (glyph buttons, `···`, menu items) |
| controls | `button`, `round_button`, `hyperlink`, `toggle_switch`, `check_box`, `radio_button`, `slider`, `list_picker`, `text_box` |
| progress | `progress_bar`, `progress_dots` (indeterminate), `progress_ring` |
| lists | `list_item`, `jump_list` (grouped long list + jump grid) |
| overlays | `message_box`, `toast` |
| chrome | `system_tray`, `page_header`, `motion::enter` (turnstile/feather entrance) |
| glyphs | `Icon` (47 Metro-style glyphs, embedded) |

Components are controlled: the caller owns values such as the selected
pivot item, the toggle state, or the slider value, and receives changes through
`on_*` callbacks. Purely visual state, such as whether the app bar is expanded
or a transition is in progress, lives inside the component.

### Fidelity notes

GPUI has no 3D transforms, so Metro's turnstile and swivel transitions are
rendered as their 2D silhouettes: a short horizontal travel with a hard
exponential ease-out, staggered per item. Live tiles use the WP7 hub-tile
"peek" (the face slides up to reveal the back) instead of a 3D flip. Segoe WP
is substituted by Helvetica Neue on macOS and Segoe UI on Windows.
