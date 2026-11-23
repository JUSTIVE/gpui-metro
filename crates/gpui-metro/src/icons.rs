//! Metro glyphs: geometric, single-weight, square caps. Embedded into the
//! binary and served through [`Assets`], tinted by the element's text colour.

use std::borrow::Cow;

use gpui::{AssetSource, Hsla, Pixels, SharedString, Styled, Svg, svg};

macro_rules! icons {
    ($($variant:ident => $file:literal),+ $(,)?) => {
        /// Every glyph shipped with gpui-metro.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Icon { $($variant),+ }

        impl Icon {
            pub const ALL: &'static [Icon] = &[$(Icon::$variant),+];

            /// Asset path, as served by [`Assets`].
            pub fn path(self) -> &'static str {
                match self { $(Icon::$variant => concat!("metro/icons/", $file, ".svg")),+ }
            }

            pub fn name(self) -> &'static str {
                match self { $(Icon::$variant => $file),+ }
            }

            fn bytes(path: &str) -> Option<&'static [u8]> {
                match path {
                    $(concat!("metro/icons/", $file, ".svg") =>
                        Some(include_bytes!(concat!("../assets/icons/", $file, ".svg"))),)+
                    _ => None,
                }
            }
        }
    };
}

icons! {
    Add => "add",
    Alert => "alert",
    Back => "back",
    Battery => "battery",
    Calendar => "calendar",
    Camera => "camera",
    Check => "check",
    ChevronDown => "chevron_down",
    ChevronRight => "chevron_right",
    Clock => "clock",
    Close => "close",
    Delete => "delete",
    Dna => "dna",
    Download => "download",
    Edit => "edit",
    Favorite => "favorite",
    Filter => "filter",
    Flask => "flask",
    Forward => "forward",
    Gauge => "gauge",
    Grid => "grid",
    Home => "home",
    List => "list",
    Lock => "lock",
    Mail => "mail",
    More => "more",
    Next => "next",
    Pause => "pause",
    People => "people",
    Phone => "phone",
    Pin => "pin",
    Play => "play",
    Previous => "previous",
    Refresh => "refresh",
    Save => "save",
    Search => "search",
    Settings => "settings",
    Share => "share",
    Signal => "signal",
    Snow => "snow",
    Sync => "sync",
    Thermometer => "thermometer",
    Truck => "truck",
    Upload => "upload",
    Wave => "wave",
    Wifi => "wifi",
}

impl Icon {
    /// An `svg()` element for this glyph at `size`, tinted `color`.
    pub fn render(self, size: impl Into<Pixels>, color: Hsla) -> Svg {
        let size = size.into();
        svg().path(self.path()).size(size).flex_none().text_color(color)
    }
}

/// Shorthand for [`Icon::render`].
pub fn icon(icon: Icon, size: impl Into<Pixels>, color: Hsla) -> Svg {
    icon.render(size, color)
}

/// Serves `metro/icons/*.svg`. Pass to `Application::with_assets`; if the app
/// has its own assets, wrap them: `Assets::with_fallback(MyAssets)`.
pub struct Assets {
    fallback: Option<Box<dyn AssetSource>>,
}

impl Assets {
    pub fn new() -> Self {
        Self { fallback: None }
    }

    pub fn with_fallback(fallback: impl AssetSource) -> Self {
        Self { fallback: Some(Box::new(fallback)) }
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = Icon::bytes(path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        match &self.fallback {
            Some(f) => f.load(path),
            None => Ok(None),
        }
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        let mut out: Vec<SharedString> = Icon::ALL
            .iter()
            .map(|i| i.path())
            .filter(|p| p.starts_with(path))
            .map(SharedString::from)
            .collect();
        if let Some(f) = &self.fallback {
            out.extend(f.list(path)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_is_embedded() {
        let a = Assets::new();
        for i in Icon::ALL {
            let bytes = a.load(i.path()).unwrap().expect(i.name());
            assert!(bytes.starts_with(b"<svg"), "{} is not svg", i.name());
        }
        assert_eq!(a.list("metro/icons/").unwrap().len(), Icon::ALL.len());
    }
}
