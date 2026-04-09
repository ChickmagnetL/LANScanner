use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Fill, Length, Theme, border};
use ssh_core::network::NetworkInterface;

use crate::theme::{
    self, AppLanguage, colors, fonts,
    icons::{self, AssetGlyph, Glyph},
};

pub const CARD_PADDING: u16 = 20;
pub const CONTROL_SPACING: u16 = 12;
pub const TITLE_ROW_HEIGHT: f32 = 40.0;
pub const ACTION_BUTTON_HEIGHT: f32 = 44.0;
const TITLE_TO_DROPDOWN_SPACING: f32 = 10.0;
pub const CARD_HEIGHT: f32 = (CARD_PADDING as f32 * 2.0)
    + TITLE_ROW_HEIGHT
    + TITLE_TO_DROPDOWN_SPACING
    + crate::widgets::dropdown::TRIGGER_HEIGHT
    + CONTROL_SPACING as f32
    + ACTION_BUTTON_HEIGHT;

const HEADER_ICON_SLOT: f32 = 20.0;
const HEADER_ICON_GLYPH: f32 = 16.0;
const HEADER_GAP: f32 = 12.0;
const REFRESH_BUTTON_EDGE: f32 = 28.0;
const REFRESH_ICON_SLOT: f32 = 18.0;
const REFRESH_ICON_GLYPH: f32 = 14.0;
const ACTION_BUTTON_RADIUS: f32 = 12.0;
const ACTION_BUTTON_LABEL_SIZE: f32 = 14.0;
const TITLE_LABEL_SIZE: f32 = 15.0;
const LIGHT_TITLE_TONE: iced::Color = colors::rgb(0x1F, 0x29, 0x37);
const LIGHT_ICON_TONE: iced::Color = colors::rgb(0x4B, 0x55, 0x63);
const REFRESH_ACTIVE_TONE: iced::Color = colors::rgb(0x3B, 0x82, 0xF6);
const DISABLED_ICON_TONE: iced::Color = colors::rgb(0x9C, 0xA3, 0xAF);

#[derive(Debug, Clone, Copy)]
enum IconToneRole {
    RefreshIdle,
    RefreshActive,
    Disabled,
}

pub struct ScanCardProps<'a, Message> {
    pub app_language: AppLanguage,
    pub dropdown: Element<'a, Message>,
    pub selected_network: Option<&'a NetworkInterface>,
    pub is_refreshing: bool,
    pub is_scanning: bool,
    pub is_blocked: bool,
    pub spinner_frame: &'static str,
    pub on_refresh: Message,
    pub on_start_scan: Message,
}

pub fn view<'a, Message>(props: ScanCardProps<'a, Message>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let ScanCardProps {
        app_language,
        dropdown,
        selected_network,
        is_refreshing,
        is_scanning,
        is_blocked,
        spinner_frame,
        on_refresh,
        on_start_scan,
    } = props;

    let title_row = container(
        row![
            row![
                network_icon(),
                text(scan_card_title(app_language))
                    .font(fonts::semibold())
                    .size(TITLE_LABEL_SIZE)
                    .style(|theme: &Theme| theme::solid_text(title_tone(theme))),
            ]
            .spacing(HEADER_GAP)
            .align_y(Alignment::Center),
            Space::new().width(Length::Fill),
            refresh_button(
                is_refreshing,
                is_scanning,
                is_blocked,
                spinner_frame,
                on_refresh
            ),
        ]
        .align_y(Alignment::Center),
    )
    .height(Length::Fixed(TITLE_ROW_HEIGHT));

    let scan_enabled = !is_blocked && !is_scanning && selected_network.is_some() && !is_refreshing;
    let scan_button = button(scan_button_content(
        app_language,
        is_scanning,
        selected_network.is_some(),
        spinner_frame,
    ))
    .width(Fill)
    .height(Length::Fixed(ACTION_BUTTON_HEIGHT))
    .padding([0, 16])
    .style(move |theme: &Theme, status| scan_button_style(theme, status, is_scanning))
    .on_press_maybe(scan_enabled.then_some(on_start_scan));

    let controls = column![
        title_row,
        column![dropdown, scan_button].spacing(CONTROL_SPACING as f32)
    ]
    .spacing(TITLE_TO_DROPDOWN_SPACING);

    container(controls)
        .width(Fill)
        .padding(CARD_PADDING)
        .style(theme::styles::card)
        .into()
}

fn refresh_button<'a, Message>(
    is_refreshing: bool,
    is_scanning: bool,
    is_blocked: bool,
    spinner_frame: &'static str,
    on_refresh: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let disabled = is_scanning || is_blocked;
    let tone_role = if is_refreshing {
        IconToneRole::RefreshActive
    } else if disabled {
        IconToneRole::Disabled
    } else {
        IconToneRole::RefreshIdle
    };
    button(refresh_badge(is_refreshing, spinner_frame, tone_role))
        .width(Length::Fixed(REFRESH_BUTTON_EDGE))
        .height(Length::Fixed(REFRESH_BUTTON_EDGE))
        .padding(0)
        .style(move |theme: &Theme, status| {
            let (background, border_color) = match status {
                button::Status::Pressed => (
                    colors::rgba(0x3B, 0x82, 0xF6, if disabled { 0.08 } else { 0.12 }),
                    colors::rgba(0x3B, 0x82, 0xF6, if disabled { 0.14 } else { 0.24 }),
                ),
                button::Status::Hovered => (
                    colors::rgba(0x3B, 0x82, 0xF6, if disabled { 0.06 } else { 0.08 }),
                    colors::rgba(0x3B, 0x82, 0xF6, if disabled { 0.10 } else { 0.18 }),
                ),
                _ => (iced::Color::TRANSPARENT, iced::Color::TRANSPARENT),
            };

            button::Style {
                snap: false,
                background: Some(iced::Background::Color(background)),
                text_color: icon_tone(theme, tone_role),
                border: iced::Border {
                    color: border_color,
                    width: if border_color.a > 0.0 { 1.0 } else { 0.0 },
                    radius: border::radius(REFRESH_BUTTON_EDGE / 2.0),
                },
                shadow: iced::Shadow::default(),
            }
        })
        .on_press_maybe((!is_refreshing && !is_scanning && !is_blocked).then_some(on_refresh))
        .into()
}

fn scan_button_content<'a, Message: 'a>(
    app_language: AppLanguage,
    is_scanning: bool,
    has_selected_network: bool,
    spinner_frame: &'static str,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = if is_scanning {
        row![
            icons::themed_rotating_refresh_centered(
                spinner_frame,
                16.0,
                14.0,
                scan_button_loading_tone,
            ),
            text(scan_button_loading_label(app_language))
                .font(fonts::semibold())
                .size(ACTION_BUTTON_LABEL_SIZE)
                .style(|theme: &Theme| theme::solid_text(scan_button_loading_tone(theme))),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    } else if has_selected_network {
        text(scan_button_ready_label(app_language))
            .font(fonts::semibold())
            .size(ACTION_BUTTON_LABEL_SIZE)
            .style(|_| theme::solid_text(iced::Color::WHITE))
            .into()
    } else {
        text(scan_button_placeholder_label(app_language))
            .font(fonts::semibold())
            .size(ACTION_BUTTON_LABEL_SIZE)
            .style(|_| theme::solid_text(iced::Color::WHITE))
            .into()
    };

    container(content).center_x(Fill).center_y(Fill).into()
}

fn scan_button_style(theme: &Theme, status: button::Status, is_scanning: bool) -> button::Style {
    if is_scanning {
        let is_dark = colors::palette(theme).card == colors::DARK.card;
        let (background, border_color) = match status {
            button::Status::Hovered | button::Status::Pressed => (
                colors::rgba(0x3B, 0x82, 0xF6, if is_dark { 0.38 } else { 0.26 }),
                colors::rgba(0x3B, 0x82, 0xF6, if is_dark { 0.50 } else { 0.38 }),
            ),
            _ => (
                colors::rgba(0x3B, 0x82, 0xF6, if is_dark { 0.28 } else { 0.18 }),
                colors::rgba(0x3B, 0x82, 0xF6, if is_dark { 0.40 } else { 0.28 }),
            ),
        };

        return button::Style {
            snap: false,
            background: Some(iced::Background::Color(background)),
            text_color: scan_button_loading_tone(theme),
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: border::radius(ACTION_BUTTON_RADIUS),
            },
            shadow: iced::Shadow::default(),
        };
    }

    let mut style = crate::theme::styles::primary_button(theme, status);
    style.border.radius = border::radius(ACTION_BUTTON_RADIUS);
    style
}

fn scan_card_title(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "扫描网络",
        AppLanguage::English => "Scan Network",
    }
}

fn scan_button_loading_label(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "扫描中...",
        AppLanguage::English => "Scanning...",
    }
}

fn scan_button_ready_label(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "开始扫描",
        AppLanguage::English => "Start Scan",
    }
}

fn scan_button_placeholder_label(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "请选择网卡",
        AppLanguage::English => "Select Network",
    }
}

fn title_tone(theme: &Theme) -> iced::Color {
    let palette = colors::palette(theme);
    if palette.card == colors::DARK.card {
        palette.text
    } else {
        LIGHT_TITLE_TONE
    }
}

fn network_icon<'a, Message: 'a>() -> Element<'a, Message> {
    icons::themed_asset_centered(
        AssetGlyph::Network,
        HEADER_ICON_SLOT,
        HEADER_ICON_GLYPH,
        header_icon_tone,
    )
}

fn refresh_badge<'a, Message: 'a>(
    is_refreshing: bool,
    spinner_frame: &'static str,
    tone_role: IconToneRole,
) -> Element<'a, Message> {
    let icon: Element<'a, Message> = match tone_role {
        IconToneRole::RefreshIdle => icons::themed_asset_centered(
            AssetGlyph::RefreshCw,
            REFRESH_ICON_SLOT,
            REFRESH_ICON_GLYPH,
            refresh_idle_tone,
        ),
        IconToneRole::RefreshActive if is_refreshing => icons::rotating_refresh_centered(
            spinner_frame,
            REFRESH_ICON_SLOT,
            REFRESH_ICON_GLYPH,
            REFRESH_ACTIVE_TONE,
        ),
        _ => icons::centered(
            Glyph::Refresh,
            REFRESH_ICON_SLOT,
            REFRESH_ICON_GLYPH,
            match tone_role {
                IconToneRole::RefreshActive => REFRESH_ACTIVE_TONE,
                IconToneRole::Disabled => DISABLED_ICON_TONE,
                _ => LIGHT_ICON_TONE,
            },
        ),
    };

    container(icon)
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
}

fn scan_button_loading_tone(theme: &Theme) -> iced::Color {
    if is_dark_card(theme) {
        colors::rgb(0xBF, 0xDB, 0xFE)
    } else {
        colors::rgb(0x1D, 0x4E, 0x89)
    }
}

fn is_dark_card(theme: &Theme) -> bool {
    colors::palette(theme).card == colors::DARK.card
}

fn header_icon_tone(theme: &Theme) -> iced::Color {
    if is_dark_card(theme) {
        colors::LIGHT.text
    } else {
        LIGHT_ICON_TONE
    }
}

fn refresh_idle_tone(theme: &Theme) -> iced::Color {
    if is_dark_card(theme) {
        colors::palette(theme).text
    } else {
        LIGHT_ICON_TONE
    }
}

fn icon_tone(theme: &Theme, tone_role: IconToneRole) -> iced::Color {
    match tone_role {
        IconToneRole::RefreshIdle => refresh_idle_tone(theme),
        IconToneRole::RefreshActive => REFRESH_ACTIVE_TONE,
        IconToneRole::Disabled => DISABLED_ICON_TONE,
    }
}
