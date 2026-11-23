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

GPUI comes from [Zed's repository](https://github.com/zed-industries/zed),
pinned to a rev in the workspace `Cargo.toml`. Apps start through
`gpui_platform::application()`, which is built with `runtime_shaders`, so
Xcode's Metal Toolchain is not needed; shaders compile when the app starts.
To move to a newer Zed, bump `rev` on both `gpui` and `gpui_platform` together.

## Using the library

```rust
use gpui_metro::prelude::*;

fn main() {
    gpui_platform::application().with_assets(gpui_metro::Assets::new()).run(|cx| {
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

GPUI can't transform boxes or text, so Metro's 3D turnstile is *projected*:
an element turned about the screen's left edge is drawn at the span a
perspective camera would see. It narrows toward the hinge, and tiles also
shrink at the far side. The content is clipped rather than reflowed
(`motion::turn`). Timings follow the WP Toolkit: forward-in from −80° and
back-in from 50°, both 350 ms on an exponential ease-out (exponent 6), with
items feathered 40 ms apart. Tiles turnstile out to 90° on an ease-in, in
reading order, with the tapped tile last.

Pivot headers glide from where the old selection sat, and the pane slides in
from the side you moved to. Panorama content follows the input (Shift +
wheel, or a trackpad swipe), pages once you pass a third of a section, and
otherwise springs back. The title moves at 0.4× for parallax.

Live tiles use the WP7 hub-tile "peek" (the face slides up) rather than a
3D flip. Segoe WP is substituted by Helvetica Neue on macOS and Segoe UI on
Windows.
