pub mod colors;
pub mod fonts;
pub mod icons;
pub mod styles;

use iced::widget::text;
use iced::{Color, Theme, theme};

pub use colors::SurfacePalette;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AppLanguage {
    #[default]
    Chinese,
    English,
}

impl AppLanguage {
    pub fn toggle(self) -> Self {
        match self {
            Self::Chinese => Self::English,
            Self::English => Self::Chinese,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    pub fn theme(self) -> Theme {
        Theme::custom(self.name().to_string(), self.palette())
    }

    pub fn palette(self) -> theme::Palette {
        let palette = match self {
            Self::Light => colors::LIGHT,
            Self::Dark => colors::DARK,
        };

        theme::Palette {
            background: palette.canvas,
            text: palette.text,
            primary: palette.primary,
            success: palette.success,
            warning: colors::rgb(0xF5, 0x9E, 0x0B),
            danger: palette.danger,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Light => "LanScan Light",
            Self::Dark => "LanScan Dark",
        }
    }
}

pub fn text_primary(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(colors::palette(theme).text),
    }
}

pub fn text_muted(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(colors::palette(theme).muted_text),
    }
}

pub fn solid_text(color: Color) -> text::Style {
    text::Style { color: Some(color) }
}
