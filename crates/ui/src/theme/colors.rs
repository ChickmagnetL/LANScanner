use iced::{Color, Theme};

#[derive(Debug, Clone, Copy)]
pub struct SurfacePalette {
    pub canvas: Color,
    pub shell: Color,
    pub titlebar: Color,
    pub card: Color,
    pub input: Color,
    pub primary: Color,
    pub text: Color,
    pub muted_text: Color,
    pub border: Color,
    pub success: Color,
    pub danger: Color,
    pub validation_button: Color,
    pub overlay: Color,
}

pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

pub const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    }
}

pub const BRAND_BLUE: Color = rgb(0x3B, 0x82, 0xF6);
pub const SUCCESS_GREEN: Color = rgb(0x22, 0xC5, 0x5E);
pub const DANGER_RED: Color = rgb(0xEF, 0x44, 0x44);
pub const MODAL_BACKDROP: Color = rgba(0, 0, 0, 0.6);

pub const LIGHT: SurfacePalette = SurfacePalette {
    canvas: rgb(0xFA, 0xFA, 0xFA),
    shell: rgb(0xFF, 0xFF, 0xFF),
    titlebar: rgb(0xFF, 0xFF, 0xFF),
    card: rgb(0xFF, 0xFF, 0xFF),
    input: rgb(0xF4, 0xF5, 0xF7),
    primary: BRAND_BLUE,
    text: rgb(0x1E, 0x29, 0x37),
    muted_text: rgb(0x6B, 0x72, 0x80),
    border: rgba(0x00, 0x00, 0x00, 0.06),
    success: SUCCESS_GREEN,
    danger: DANGER_RED,
    validation_button: rgb(0x1A, 0x1A, 0x1A),
    overlay: rgba(0xFF, 0xFF, 0xFF, 0.985),
};

pub const DARK: SurfacePalette = SurfacePalette {
    canvas: rgb(0x12, 0x12, 0x12),
    shell: rgb(0x11, 0x14, 0x1A),
    titlebar: rgb(0x11, 0x14, 0x1A),
    card: rgb(0x17, 0x1A, 0x21),
    input: rgb(0x27, 0x27, 0x2A),
    primary: BRAND_BLUE,
    text: rgb(0xE8, 0xEB, 0xF1),
    muted_text: rgb(0x98, 0xA1, 0xAE),
    border: rgba(0x6C, 0x74, 0x82, 0.54),
    success: SUCCESS_GREEN,
    danger: DANGER_RED,
    validation_button: rgb(0xF3, 0xF4, 0xF6),
    overlay: rgba(0x16, 0x19, 0x20, 0.975),
};

// LanScan Specific UI Colors
pub const LIGHT_ROW_HOVER: Color = rgb(0xF8, 0xF9, 0xFA);
pub const DARK_ROW_HOVER: Color = rgb(0x25, 0x25, 0x25);

pub const LIGHT_SELECTION: Color = rgb(0xEF, 0xF6, 0xFF);
pub const DARK_SELECTION: Color = rgb(0x1E, 0x3A, 0x8A);

pub const LIGHT_ACCENT_SOFT: Color = rgb(0xF0, 0xF5, 0xFF);
pub const DARK_ACCENT_SOFT: Color = rgba(0x3B, 0x82, 0xF6, 0.1);

pub fn palette(theme: &Theme) -> SurfacePalette {
    if theme.palette().background == DARK.canvas {
        DARK
    } else {
        LIGHT
    }
}
