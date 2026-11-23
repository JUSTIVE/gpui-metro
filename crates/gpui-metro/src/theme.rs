//! The Metro theme: one ground (dark or light), one accent, one typeface.
//!
//! Windows Phone let the user pick exactly two things — background and accent —
//! and every app was expected to follow. This module keeps that contract: an
//! app reads [`Theme::get`] and never hard-codes a colour.

use gpui::{App, FontWeight, Global, Hsla, Rgba, SharedString, rgb};

/// Dark or light background, the first of the phone's two theme choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Dark,
    Light,
}

impl Mode {
    pub fn toggled(self) -> Self {
        match self {
            Mode::Dark => Mode::Light,
            Mode::Light => Mode::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Dark => "dark",
            Mode::Light => "light",
        }
    }
}

/// The twenty accent colours of Windows Phone 8.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Accent {
    Lime,
    Green,
    Emerald,
    Teal,
    Cyan,
    Cobalt,
    Indigo,
    Violet,
    Pink,
    Magenta,
    Crimson,
    Red,
    Orange,
    Amber,
    Yellow,
    Brown,
    Olive,
    Steel,
    Mauve,
    Taupe,
}

impl Accent {
    pub const ALL: [Accent; 20] = [
        Accent::Lime,
        Accent::Green,
        Accent::Emerald,
        Accent::Teal,
        Accent::Cyan,
        Accent::Cobalt,
        Accent::Indigo,
        Accent::Violet,
        Accent::Pink,
        Accent::Magenta,
        Accent::Crimson,
        Accent::Red,
        Accent::Orange,
        Accent::Amber,
        Accent::Yellow,
        Accent::Brown,
        Accent::Olive,
        Accent::Steel,
        Accent::Mauve,
        Accent::Taupe,
    ];

    pub fn hex(self) -> u32 {
        match self {
            Accent::Lime => 0xA4C400,
            Accent::Green => 0x60A917,
            Accent::Emerald => 0x008A00,
            Accent::Teal => 0x00ABA9,
            Accent::Cyan => 0x1BA1E2,
            Accent::Cobalt => 0x0050EF,
            Accent::Indigo => 0x6A00FF,
            Accent::Violet => 0xAA00FF,
            Accent::Pink => 0xF472D0,
            Accent::Magenta => 0xD80073,
            Accent::Crimson => 0xA20025,
            Accent::Red => 0xE51400,
            Accent::Orange => 0xFA6800,
            Accent::Amber => 0xF0A30A,
            Accent::Yellow => 0xE3C800,
            Accent::Brown => 0x825A2C,
            Accent::Olive => 0x6D8764,
            Accent::Steel => 0x647687,
            Accent::Mauve => 0x76608A,
            Accent::Taupe => 0x87794E,
        }
    }

    pub fn color(self) -> Hsla {
        rgb(self.hex()).into()
    }

    pub fn name(self) -> &'static str {
        match self {
            Accent::Lime => "lime",
            Accent::Green => "green",
            Accent::Emerald => "emerald",
            Accent::Teal => "teal",
            Accent::Cyan => "cyan",
            Accent::Cobalt => "cobalt",
            Accent::Indigo => "indigo",
            Accent::Violet => "violet",
            Accent::Pink => "pink",
            Accent::Magenta => "magenta",
            Accent::Crimson => "crimson",
            Accent::Red => "red",
            Accent::Orange => "orange",
            Accent::Amber => "amber",
            Accent::Yellow => "yellow",
            Accent::Brown => "brown",
            Accent::Olive => "olive",
            Accent::Steel => "steel",
            Accent::Mauve => "mauve",
            Accent::Taupe => "taupe",
        }
    }

    /// Text that sits on a filled accent. Metro always used white, even on
    /// yellow and lime; we keep that for fidelity.
    pub fn ink(self) -> Hsla {
        gpui::white()
    }
}

/// The resolved colour tokens for a mode + accent pair. Names follow the
/// Windows Phone resource keys (`PhoneForegroundBrush`, `PhoneSubtleBrush`, …).
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub background: Hsla,
    pub foreground: Hsla,
    /// Secondary text: dates, captions, inactive pivot headers.
    pub subtle: Hsla,
    /// Disabled text and borders.
    pub disabled: Hsla,
    /// App bar, message box, context menu: the "chrome" layer.
    pub chrome: Hsla,
    /// Control borders (buttons, text boxes, check boxes).
    pub border: Hsla,
    /// Text box / list picker fill when idle.
    pub input: Hsla,
    /// Text box fill when focused (always the opposite of the ground).
    pub input_focused: Hsla,
    /// Ink inside a focused text box.
    pub input_ink: Hsla,
    /// Dim layer behind a message box.
    pub scrim: Hsla,
    pub accent: Hsla,
    pub accent_ink: Hsla,
}

fn rgba(hex: u32, alpha: f32) -> Hsla {
    let c: Rgba = rgb(hex);
    Hsla::from(Rgba { a: alpha, ..c })
}

impl Palette {
    pub fn resolve(mode: Mode, accent: Accent) -> Self {
        let (accent, accent_ink) = (accent.color(), accent.ink());
        match mode {
            Mode::Dark => Palette {
                background: rgb(0x000000).into(),
                foreground: rgb(0xFFFFFF).into(),
                subtle: rgba(0xFFFFFF, 0.6),
                disabled: rgba(0xFFFFFF, 0.4),
                chrome: rgb(0x1F1F1F).into(),
                border: rgba(0xFFFFFF, 0.75),
                input: rgba(0xFFFFFF, 0.75),
                input_focused: rgb(0xFFFFFF).into(),
                input_ink: rgb(0x000000).into(),
                scrim: rgba(0x000000, 0.6),
                accent,
                accent_ink,
            },
            Mode::Light => Palette {
                background: rgb(0xFFFFFF).into(),
                foreground: rgb(0x000000).into(),
                subtle: rgba(0x000000, 0.4),
                disabled: rgba(0x000000, 0.26),
                chrome: rgb(0xDDDDDD).into(),
                border: rgba(0x000000, 0.6),
                input: rgba(0x000000, 0.15),
                input_focused: rgb(0xFFFFFF).into(),
                input_ink: rgb(0x000000).into(),
                scrim: rgba(0xFFFFFF, 0.6),
                accent,
                accent_ink,
            },
        }
    }
}

/// The Metro type ramp (WP8 `PhoneFontSize*` resources), in px.
pub mod size {
    /// `PhoneFontSizeSmall`
    pub const SMALL: f32 = 14.;
    /// `PhoneFontSizeNormal`
    pub const NORMAL: f32 = 17.;
    /// `PhoneFontSizeMedium`
    pub const MEDIUM: f32 = 20.;
    /// `PhoneFontSizeMediumLarge`
    pub const MEDIUM_LARGE: f32 = 22.;
    /// `PhoneFontSizeLarge`
    pub const LARGE: f32 = 27.;
    /// `PhoneFontSizeExtraLarge` — pivot headers.
    pub const EXTRA_LARGE: f32 = 36.;
    /// `PhoneFontSizeExtraExtraLarge` — page titles.
    pub const HUGE: f32 = 56.;
    /// Panorama title.
    pub const PANORAMA: f32 = 112.;
}

/// Metro is set in Segoe WP; we fall back to the closest face per platform
/// that still carries light weights.
pub fn default_font_family() -> SharedString {
    if cfg!(target_os = "windows") {
        "Segoe UI".into()
    } else if cfg!(target_os = "macos") {
        "Helvetica Neue".into()
    } else {
        "Noto Sans".into()
    }
}

/// Weight of the big Metro type (page titles, pivot headers).
pub const LIGHT: FontWeight = FontWeight::LIGHT;
/// `PhoneFontFamilySemiLight`.
pub const SEMILIGHT: FontWeight = FontWeight::NORMAL;
/// `PhoneFontFamilySemiBold`.
pub const SEMIBOLD: FontWeight = FontWeight::SEMIBOLD;

/// The app-wide theme. Install once with [`crate::init`]; change with
/// [`Theme::set_mode`] / [`Theme::set_accent`] and every window re-renders.
#[derive(Clone, Debug)]
pub struct Theme {
    pub mode: Mode,
    pub accent: Accent,
    pub font_family: SharedString,
    pub palette: Palette,
    /// When true, components skip their entrance/transition motion.
    pub reduced_motion: bool,
}

impl Global for Theme {}

impl Default for Theme {
    fn default() -> Self {
        Self::new(Mode::Dark, Accent::Cobalt)
    }
}

impl Theme {
    pub fn new(mode: Mode, accent: Accent) -> Self {
        Self {
            mode,
            accent,
            font_family: default_font_family(),
            palette: Palette::resolve(mode, accent),
            reduced_motion: false,
        }
    }

    pub fn get(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// The palette, copied out so render code can hold it across `cx` borrows.
    pub fn palette(cx: &App) -> Palette {
        cx.global::<Theme>().palette
    }

    pub fn font(cx: &App) -> SharedString {
        cx.global::<Theme>().font_family.clone()
    }

    pub fn set_mode(mode: Mode, cx: &mut App) {
        let t = cx.global_mut::<Theme>();
        t.mode = mode;
        t.palette = Palette::resolve(mode, t.accent);
        cx.refresh_windows();
    }

    pub fn set_accent(accent: Accent, cx: &mut App) {
        let t = cx.global_mut::<Theme>();
        t.accent = accent;
        t.palette = Palette::resolve(t.mode, accent);
        cx.refresh_windows();
    }

    pub fn set_reduced_motion(reduced: bool, cx: &mut App) {
        cx.global_mut::<Theme>().reduced_motion = reduced;
        cx.refresh_windows();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_accent_has_a_unique_colour_and_name() {
        let mut hexes: Vec<u32> = Accent::ALL.iter().map(|a| a.hex()).collect();
        hexes.sort();
        hexes.dedup();
        assert_eq!(hexes.len(), 20);
        let mut names: Vec<&str> = Accent::ALL.iter().map(|a| a.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 20);
    }

    #[test]
    fn grounds_are_inverted() {
        let d = Palette::resolve(Mode::Dark, Accent::Red);
        let l = Palette::resolve(Mode::Light, Accent::Red);
        assert_eq!(d.background, l.foreground);
        assert_eq!(d.foreground, l.background);
        assert_eq!(d.accent, l.accent);
    }
}
