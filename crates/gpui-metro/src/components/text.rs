//! Type is the interface. These are the Metro text styles as `Div`s so they
//! compose with the rest of the fluent API.

use gpui::{App, Div, FontWeight, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px, relative};

use crate::theme::{LIGHT, SEMIBOLD, SEMILIGHT, Theme, size};

/// A Metro text style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextStyle {
    /// `PhoneTextTitle1Style`: page titles, 56 light.
    Title1,
    /// `PhoneTextTitle2Style`: 36 light.
    Title2,
    /// `PhoneTextTitle3Style`: 27 semilight.
    Title3,
    /// `PhoneTextExtraLargeStyle`: 36 semilight.
    ExtraLarge,
    /// `PhoneTextLargeStyle`: 27 semilight.
    Large,
    /// `PhoneTextNormalStyle`: 17 normal.
    Normal,
    /// `PhoneTextSubtleStyle`: 17, subtle colour.
    Subtle,
    /// `PhoneTextAccentStyle`: 17 semibold, accent colour.
    Accent,
    /// `PhoneTextSmallStyle`: 14, subtle colour.
    Small,
    /// Application title above a page title: 14 semibold, upper-case.
    AppTitle,
    /// Group header in lists: 22 semibold accent.
    GroupHeader,
}

impl TextStyle {
    pub fn size(self) -> f32 {
        match self {
            TextStyle::Title1 => size::HUGE,
            TextStyle::Title2 | TextStyle::ExtraLarge => size::EXTRA_LARGE,
            TextStyle::Title3 | TextStyle::Large => size::LARGE,
            TextStyle::Normal | TextStyle::Subtle | TextStyle::Accent => size::NORMAL,
            TextStyle::Small | TextStyle::AppTitle => size::SMALL,
            TextStyle::GroupHeader => size::MEDIUM_LARGE,
        }
    }

    pub fn weight(self) -> FontWeight {
        match self {
            TextStyle::Title1 | TextStyle::Title2 => LIGHT,
            TextStyle::Title3 | TextStyle::ExtraLarge | TextStyle::Large => SEMILIGHT,
            TextStyle::Normal | TextStyle::Subtle | TextStyle::Small => FontWeight::NORMAL,
            TextStyle::Accent | TextStyle::AppTitle | TextStyle::GroupHeader => SEMIBOLD,
        }
    }

    pub fn color(self, cx: &App) -> Hsla {
        let p = Theme::palette(cx);
        match self {
            TextStyle::Subtle | TextStyle::Small => p.subtle,
            TextStyle::Accent | TextStyle::GroupHeader => p.accent,
            _ => p.foreground,
        }
    }

    /// An empty `Div` carrying this style; add children or more styling.
    pub fn div(self, cx: &App) -> Div {
        div()
            .font_family(Theme::font(cx))
            .text_size(px(self.size()))
            .font_weight(self.weight())
            .line_height(relative(if self.size() > 30. { 1.1 } else { 1.3 }))
            .text_color(self.color(cx))
    }
}

/// `text(TextStyle::Large, "strains")` — a styled run of text.
#[derive(IntoElement)]
pub struct Text {
    style: TextStyle,
    content: SharedString,
    color: Option<Hsla>,
    lowercase: bool,
}

pub fn text(style: TextStyle, content: impl Into<SharedString>) -> Text {
    Text { style, content: content.into(), color: None, lowercase: false }
}

impl Text {
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Metro set headings in lower case. Opt in per call.
    pub fn lowercase(mut self) -> Self {
        self.lowercase = true;
        self
    }
}

impl RenderOnce for Text {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let content = match (self.style, self.lowercase) {
            (TextStyle::AppTitle, _) => SharedString::from(self.content.to_uppercase()),
            (_, true) => SharedString::from(self.content.to_lowercase()),
            _ => self.content,
        };
        let mut el = self.style.div(cx);
        if let Some(c) = self.color {
            el = el.text_color(c);
        }
        el.child(content)
    }
}
