use iced::widget::container;
use iced::{Color, Element, Length, Theme, border, widget::svg};

use super::colors;

pub const TITLEBAR_BUTTON_EDGE: f32 = 36.0;
pub const TITLEBAR_BUTTON_GLYPH: f32 = 16.0;
pub const TITLEBAR_BUTTON_SPACING: f32 = 4.0;
pub const TITLEBAR_SIDE_SAFE_INSET: f32 = 10.0;
pub const TITLEBAR_HORIZONTAL_PADDING: f32 = 16.0;
pub const TITLEBAR_BUTTON_GROUP_WIDTH: f32 = 132.0;
pub const TITLEBAR_LOGO_SLOT: f32 = 24.0;
pub const TITLEBAR_LOGO_GLYPH: f32 = 15.0;
pub const TITLEBAR_TOOL_BUTTON_EDGE: f32 = TITLEBAR_BUTTON_EDGE;
pub const TITLEBAR_TOOL_GLYPH: f32 = TITLEBAR_BUTTON_GLYPH;
pub const TITLEBAR_CONTROL_BUTTON_EDGE: f32 = TITLEBAR_BUTTON_EDGE;
pub const TITLEBAR_CONTROL_GLYPH: f32 = TITLEBAR_BUTTON_GLYPH;
pub const DROPDOWN_CHIP_SLOT: (f32, f32) = (40.0, 32.0);
pub const DROPDOWN_CHEVRON_SLOT: f32 = 14.0;
pub const DROPDOWN_CHEVRON_GLYPH: f32 = 14.0;

const SUN_ASSET: &[u8] = include_bytes!("../../../../icon/sun.svg");
const MOON_ASSET: &[u8] = include_bytes!("../../../../icon/moon.svg");
const HELP_ASSET: &[u8] = include_bytes!("../../../../icon/circle-question-mark.svg");
const LOCK_ASSET: &[u8] = include_bytes!("../../../../icon/lock.svg");
const SETTINGS_2_ASSET: &[u8] = include_bytes!("../../../../icon/settings-2.svg");
const KEY_ROUND_ASSET: &[u8] = include_bytes!("../../../../icon/key-round.svg");
const PENCIL_ASSET: &[u8] = include_bytes!("../../../../icon/pencil.svg");
const PLUS_ASSET: &[u8] = include_bytes!("../../../../icon/plus.svg");
const TRASH_2_ASSET: &[u8] = include_bytes!("../../../../icon/trash-2.svg");
const RADAR_ASSET: &[u8] = include_bytes!("../../../../icon/radar.svg");
const REFRESH_CW_ASSET: &[u8] = include_bytes!("../../../../icon/refresh-cw.svg");
const SEARCH_ASSET: &[u8] = include_bytes!("../../../../icon/search.svg");
const MINUS_ASSET: &[u8] = include_bytes!("../../../../icon/minus.svg");
const SQUARE_ASSET: &[u8] = include_bytes!("../../../../icon/square.svg");
const X_ASSET: &[u8] = include_bytes!("../../../../icon/x.svg");
const CHECK_ASSET: &[u8] = include_bytes!("../../../../icon/check.svg");
const CIRCLE_CHECK_ASSET: &[u8] = include_bytes!("../../../../icon/circle-check.svg");
const CIRCLE_DASHED_ASSET: &[u8] = include_bytes!("../../../../icon/circle-dashed.svg");
const CIRCLE_X_ASSET: &[u8] = include_bytes!("../../../../icon/circle-x.svg");
const CHEVRON_DOWN_ASSET: &[u8] = include_bytes!("../../../../icon/chevron-down.svg");
const CHEVRON_UP_ASSET: &[u8] = include_bytes!("../../../../icon/chevron-up.svg");
const NETWORK_ASSET: &[u8] = include_bytes!("../../../../icon/network.svg");
const WIFI_ASSET: &[u8] = include_bytes!("../../../../icon/wifi.svg");
const ETHERNET_PORT_ASSET: &[u8] = include_bytes!("../../../../icon/ethernet-port.svg");
const CONTAINER_ASSET: &[u8] = include_bytes!("../../../../icon/container.svg");
const LAPTOP_ASSET: &[u8] = include_bytes!("../../../../icon/laptop.svg");
const SERVER_ASSET: &[u8] = include_bytes!("../../../../icon/server.svg");
const MONITOR_ASSET: &[u8] = include_bytes!("../../../../icon/monitor.svg");
const CODE_ASSET: &[u8] = include_bytes!("../../../../icon/code.svg");
const TERMINAL_ASSET: &[u8] = include_bytes!("../../../../icon/terminal.svg");
const LOADER_ASSET: &[u8] = include_bytes!("../../../../icon/loader.svg");
const LOADER_CIRCLE_ASSET: &[u8] = include_bytes!("../../../../icon/loader-circle.svg");
const LOADER_PINWHEEL_ASSET: &[u8] = include_bytes!("../../../../icon/loader-pinwheel.svg");
const GITHUB_ASSET: &[u8] = include_bytes!("../../../../icon/github.svg");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Sun,
    Moon,
    Help,
    Lock,
    SlidersHorizontal,
    Pencil,
    KeyRound,
    Plus,
    Trash,
    Radar,
    Refresh,
    Search,
    Minimize,
    Maximize,
    Restore,
    Close,
    Check,
    CircleCheck,
    Pending,
    CircleX,
    ChevronDown,
    ChevronUp,
    Network,
    Wifi,
    Ethernet,
    Docker,
    Laptop,
    Server,
    Desktop,
    Code,
    Display,
    Terminal,
    Spinner1,
    Spinner2,
    Spinner3,
    Spinner4,
    GitHub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameToken {
    BrandMark,
    DropdownChip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetGlyph {
    Network,
    RefreshCw,
}

#[derive(Debug, Clone, Copy)]
pub struct FrameSpec {
    pub width: f32,
    pub height: f32,
    pub icon_size: f32,
    pub tone: Color,
    pub background: Color,
    pub border_color: Color,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ThemedAssetFrameSpec {
    pub width: f32,
    pub height: f32,
    pub icon_size: f32,
    pub tone: fn(&Theme) -> Color,
    pub background: fn(&Theme) -> Color,
    pub border_color: fn(&Theme) -> Color,
    pub radius: f32,
}

pub fn glyph<'a, Message>(kind: Glyph, size: f32, tone: Color) -> Element<'a, Message>
where
    Message: 'a,
{
    container(
        svg(svg::Handle::from_memory(glyph_bytes(kind)))
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .style(move |_theme, _status| svg::Style { color: Some(tone) }),
    )
    .width(size)
    .height(size)
    .center_x(Length::Fixed(size))
    .center_y(Length::Fixed(size))
    .into()
}

pub fn centered<'a, Message>(kind: Glyph, slot: f32, size: f32, tone: Color) -> Element<'a, Message>
where
    Message: 'a,
{
    container(glyph(kind, size, tone))
        .width(slot)
        .height(slot)
        .center_x(Length::Fixed(slot))
        .center_y(Length::Fixed(slot))
        .into()
}

pub fn themed_centered<'a, Message>(
    kind: Glyph,
    slot: f32,
    size: f32,
    tone: fn(&Theme) -> Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(themed_glyph(kind, size, tone))
        .width(slot)
        .height(slot)
        .center_x(Length::Fixed(slot))
        .center_y(Length::Fixed(slot))
        .into()
}

pub fn rotating_refresh_centered<'a, Message>(
    frame: &str,
    slot: f32,
    size: f32,
    tone: Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(rotating_refresh(frame, size, tone))
        .width(slot)
        .height(slot)
        .center_x(Length::Fixed(slot))
        .center_y(Length::Fixed(slot))
        .into()
}

pub fn themed_rotating_refresh_centered<'a, Message>(
    frame: &str,
    slot: f32,
    size: f32,
    tone: fn(&Theme) -> Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(themed_rotating_refresh(frame, size, tone))
        .width(slot)
        .height(slot)
        .center_x(Length::Fixed(slot))
        .center_y(Length::Fixed(slot))
        .into()
}

pub fn centered_compact<'a, Message>(
    kind: Glyph,
    slot: f32,
    size: f32,
    tone: Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(
        container(
            svg(svg::Handle::from_memory(glyph_bytes(kind)))
                .width(Length::Fixed(size))
                .height(Length::Fixed(size))
                .style(move |_theme, _status| svg::Style { color: Some(tone) }),
        )
        .width(size)
        .height(size)
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size)),
    )
    .width(slot)
    .height(slot)
    .center_x(Length::Fixed(slot))
    .center_y(Length::Fixed(slot))
    .into()
}

pub fn framed<'a, Message>(kind: Glyph, spec: FrameSpec) -> Element<'a, Message>
where
    Message: 'a,
{
    container(glyph(kind, spec.icon_size, spec.tone))
        .width(spec.width)
        .height(spec.height)
        .center_x(Length::Fixed(spec.width))
        .center_y(Length::Fixed(spec.height))
        .style(move |_| {
            container::Style::default()
                .background(spec.background)
                .border(iced::Border {
                    color: spec.border_color,
                    width: 1.0,
                    radius: border::radius(spec.radius),
                })
        })
        .into()
}

pub fn titlebar_centered<'a, Message>(
    kind: Glyph,
    slot: f32,
    size: f32,
    tone: Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    centered(kind, slot, size, tone)
}

pub fn titlebar_centered_compact<'a, Message>(
    kind: Glyph,
    slot: f32,
    size: f32,
    tone: Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    centered_compact(kind, slot, size, tone)
}

pub fn titlebar_framed<'a, Message>(kind: Glyph, spec: FrameSpec) -> Element<'a, Message>
where
    Message: 'a,
{
    framed(kind, spec)
}

pub fn themed_asset_centered<'a, Message>(
    kind: AssetGlyph,
    slot: f32,
    size: f32,
    tone: fn(&Theme) -> Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(themed_asset(kind, size, tone))
        .width(slot)
        .height(slot)
        .center_x(Length::Fixed(slot))
        .center_y(Length::Fixed(slot))
        .into()
}

pub fn themed_asset_framed<'a, Message>(
    kind: AssetGlyph,
    spec: ThemedAssetFrameSpec,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(themed_asset(kind, spec.icon_size, spec.tone))
        .width(spec.width)
        .height(spec.height)
        .center_x(Length::Fixed(spec.width))
        .center_y(Length::Fixed(spec.height))
        .style(move |theme: &Theme| {
            container::Style::default()
                .background((spec.background)(theme))
                .border(iced::Border {
                    color: (spec.border_color)(theme),
                    width: 1.0,
                    radius: border::radius(spec.radius),
                })
        })
        .into()
}

pub fn themed_framed<'a, Message>(kind: Glyph, spec: ThemedAssetFrameSpec) -> Element<'a, Message>
where
    Message: 'a,
{
    container(themed_glyph(kind, spec.icon_size, spec.tone))
        .width(spec.width)
        .height(spec.height)
        .center_x(Length::Fixed(spec.width))
        .center_y(Length::Fixed(spec.height))
        .style(move |theme: &Theme| {
            container::Style::default()
                .background((spec.background)(theme))
                .border(iced::Border {
                    color: (spec.border_color)(theme),
                    width: 1.0,
                    radius: border::radius(spec.radius),
                })
        })
        .into()
}

pub fn brand_frame(token: FrameToken) -> FrameSpec {
    let (width, height, icon_size, radius) = frame_metrics(token);

    FrameSpec {
        width,
        height,
        icon_size,
        tone: Color::WHITE,
        background: colors::BRAND_BLUE,
        border_color: colors::BRAND_BLUE,
        radius,
    }
}

pub fn spinner(frame: &str) -> Glyph {
    match frame {
        "-" => Glyph::Spinner1,
        "\\" => Glyph::Spinner2,
        "|" => Glyph::Spinner3,
        "/" => Glyph::Spinner4,
        _ => Glyph::Pending,
    }
}

fn frame_metrics(token: FrameToken) -> (f32, f32, f32, f32) {
    match token {
        FrameToken::BrandMark => (
            TITLEBAR_LOGO_SLOT,
            TITLEBAR_LOGO_SLOT,
            TITLEBAR_LOGO_GLYPH,
            8.0,
        ),
        FrameToken::DropdownChip => (DROPDOWN_CHIP_SLOT.0, DROPDOWN_CHIP_SLOT.1, 16.0, 10.0),
    }
}

fn themed_asset<'a, Message>(
    kind: AssetGlyph,
    size: f32,
    tone: fn(&Theme) -> Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(
        svg(svg::Handle::from_memory(asset_bytes(kind)))
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .style(move |theme: &Theme, _status| svg::Style {
                color: Some(tone(theme)),
            }),
    )
    .width(size)
    .height(size)
    .center_x(Length::Fixed(size))
    .center_y(Length::Fixed(size))
    .into()
}

fn rotating_refresh<'a, Message>(frame: &str, size: f32, tone: Color) -> Element<'a, Message>
where
    Message: 'a,
{
    let svg_bytes = rotated_refresh_svg(refresh_rotation_degrees(frame));
    container(
        svg(svg::Handle::from_memory(svg_bytes))
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .style(move |_theme, _status| svg::Style { color: Some(tone) }),
    )
    .width(size)
    .height(size)
    .center_x(Length::Fixed(size))
    .center_y(Length::Fixed(size))
    .into()
}

pub fn themed_glyph<'a, Message>(
    kind: Glyph,
    size: f32,
    tone: fn(&Theme) -> Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(
        svg(svg::Handle::from_memory(glyph_bytes(kind)))
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .style(move |theme: &Theme, _status| svg::Style {
                color: Some(tone(theme)),
            }),
    )
    .width(size)
    .height(size)
    .center_x(Length::Fixed(size))
    .center_y(Length::Fixed(size))
    .into()
}

fn themed_rotating_refresh<'a, Message>(
    frame: &str,
    size: f32,
    tone: fn(&Theme) -> Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let svg_bytes = rotated_refresh_svg(refresh_rotation_degrees(frame));
    container(
        svg(svg::Handle::from_memory(svg_bytes))
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .style(move |theme: &Theme, _status| svg::Style {
                color: Some(tone(theme)),
            }),
    )
    .width(size)
    .height(size)
    .center_x(Length::Fixed(size))
    .center_y(Length::Fixed(size))
    .into()
}

fn refresh_rotation_degrees(frame: &str) -> f32 {
    let index: f32 = frame.parse().unwrap_or(0.0);
    index * (360.0 / 12.0)
}

fn rotated_refresh_svg(angle: f32) -> Vec<u8> {
    format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" "#,
            r#"viewBox="0 0 24 24" fill="none" stroke="currentColor" "#,
            r#"stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
            r#"<g transform="rotate({a}, 12, 12)">"#,
            r#"<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/>"#,
            r#"<path d="M21 3v5h-5"/>"#,
            r#"<path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/>"#,
            r#"<path d="M8 16H3v5"/>"#,
            r#"</g></svg>"#,
        ),
        a = angle,
    )
    .into_bytes()
}

fn asset_bytes(kind: AssetGlyph) -> &'static [u8] {
    match kind {
        AssetGlyph::Network => NETWORK_ASSET,
        AssetGlyph::RefreshCw => REFRESH_CW_ASSET,
    }
}

fn glyph_bytes(kind: Glyph) -> &'static [u8] {
    match kind {
        Glyph::Sun => SUN_ASSET,
        Glyph::Moon => MOON_ASSET,
        Glyph::Help => HELP_ASSET,
        Glyph::Lock => LOCK_ASSET,
        Glyph::SlidersHorizontal => SETTINGS_2_ASSET,
        Glyph::Pencil => PENCIL_ASSET,
        Glyph::KeyRound => KEY_ROUND_ASSET,
        Glyph::Plus => PLUS_ASSET,
        Glyph::Trash => TRASH_2_ASSET,
        Glyph::Radar => RADAR_ASSET,
        Glyph::Refresh => REFRESH_CW_ASSET,
        Glyph::Search => SEARCH_ASSET,
        Glyph::Minimize => MINUS_ASSET,
        Glyph::Maximize | Glyph::Restore => SQUARE_ASSET,
        Glyph::Close => X_ASSET,
        Glyph::Check => CHECK_ASSET,
        Glyph::CircleCheck => CIRCLE_CHECK_ASSET,
        Glyph::Pending => CIRCLE_DASHED_ASSET,
        Glyph::CircleX => CIRCLE_X_ASSET,
        Glyph::ChevronDown => CHEVRON_DOWN_ASSET,
        Glyph::ChevronUp => CHEVRON_UP_ASSET,
        Glyph::Network => NETWORK_ASSET,
        Glyph::Wifi => WIFI_ASSET,
        Glyph::Ethernet => ETHERNET_PORT_ASSET,
        Glyph::Docker => CONTAINER_ASSET,
        Glyph::Laptop => LAPTOP_ASSET,
        Glyph::Server => SERVER_ASSET,
        Glyph::Desktop | Glyph::Display => MONITOR_ASSET,
        Glyph::Code => CODE_ASSET,
        Glyph::Terminal => TERMINAL_ASSET,
        Glyph::Spinner1 => LOADER_ASSET,
        Glyph::Spinner2 => LOADER_CIRCLE_ASSET,
        Glyph::Spinner3 => LOADER_PINWHEEL_ASSET,
        Glyph::Spinner4 => CIRCLE_DASHED_ASSET,
        Glyph::GitHub => GITHUB_ASSET,
    }
}
