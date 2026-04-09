use iced::widget::{button, column, container, mouse_area, row, text};
use iced::{Alignment, Element, Fill, Font, Length, Padding, Theme, alignment::Horizontal};
use platform::window::WindowAction;

use crate::theme::{
    self, AppLanguage, ThemeMode, colors, fonts,
    icons::{self, FrameSpec, Glyph},
};

const TITLEBAR_HEIGHT: u32 = 54;
const TITLE_TEXT_SIZE: u32 = 16;
const TITLE_ROW_SPACING: f32 = 8.0;
const TITLEBAR_TOOL_SPACING: f32 = icons::TITLEBAR_BUTTON_SPACING + 1.0;
const TITLEBAR_CONTROL_SPACING: f32 = icons::TITLEBAR_BUTTON_SPACING - 1.0;
const TITLEBAR_HORIZONTAL_PADDING: f32 = 20.0;
const TITLEBAR_SIDE_SAFE_INSET: f32 = 0.0;
const TITLEBAR_SIDE_WIDTH: f32 = icons::TITLEBAR_BUTTON_GROUP_WIDTH - 6.0;
const TITLEBAR_TOOL_BUTTON_EDGE: f32 = icons::TITLEBAR_TOOL_BUTTON_EDGE;
const TITLEBAR_TOOL_GLYPH: f32 = 18.0;
const TITLEBAR_CONTROL_BUTTON_EDGE: f32 = icons::TITLEBAR_CONTROL_BUTTON_EDGE;
const TITLEBAR_CONTROL_GLYPH: f32 = 16.0;
const TITLEBAR_MAXIMIZE_GLYPH: f32 = 14.0;
const TITLEBAR_LOGO_EDGE: f32 = icons::TITLEBAR_LOGO_SLOT - 1.0;
const TITLEBAR_LOGO_GLYPH: f32 = icons::TITLEBAR_LOGO_GLYPH;
const TITLEBAR_LOGO_RADIUS: f32 = 9.0;
const TITLEBAR_DIVIDER_HEIGHT: f32 = 1.0;

#[derive(Clone, Copy)]
enum ButtonRole {
    Tool,
    Control,
    Close,
}

pub fn view<'a, Message>(
    theme_mode: ThemeMode,
    app_language: AppLanguage,
    is_maximized: bool,
    on_toggle_theme: Message,
    on_help: Message,
    on_toggle_language: Message,
    on_window_action: impl Fn(WindowAction) -> Message + Copy + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let theme_icon = if theme_mode == ThemeMode::Dark {
        Glyph::Sun
    } else {
        Glyph::Moon
    };

    let language_icon = match app_language {
        AppLanguage::Chinese | AppLanguage::English => Glyph::Languages,
    };

    let left_tools = side_slot(
        row![
            icon_button(theme_mode, theme_icon, on_toggle_theme, ButtonRole::Tool),
            icon_button(theme_mode, Glyph::Help, on_help, ButtonRole::Tool),
            icon_button(
                theme_mode,
                language_icon,
                on_toggle_language,
                ButtonRole::Tool,
            ),
        ]
        .spacing(TITLEBAR_TOOL_SPACING)
        .align_y(Alignment::Center)
        .into(),
        false,
    );

    let center_drag_zone = mouse_area(
        container(
            row![
                logo(),
                text("LANScanner")
                    .font(title_font())
                    .size(TITLE_TEXT_SIZE)
                    .style(|theme: &Theme| theme::text_primary(theme)),
            ]
            .spacing(TITLE_ROW_SPACING)
            .align_y(Alignment::Center),
        )
        .center_x(Fill)
        .center_y(Fill)
        .width(Fill),
    )
    .on_press(on_window_action(WindowAction::Drag));

    let right_controls = side_slot(
        row![
            icon_button(
                theme_mode,
                Glyph::Minimize,
                on_window_action(WindowAction::Minimize),
                ButtonRole::Control,
            ),
            icon_button(
                theme_mode,
                if is_maximized {
                    Glyph::Restore
                } else {
                    Glyph::Maximize
                },
                on_window_action(WindowAction::ToggleMaximize),
                ButtonRole::Control,
            ),
            icon_button(
                theme_mode,
                Glyph::Close,
                on_window_action(WindowAction::Close),
                ButtonRole::Close,
            ),
        ]
        .spacing(TITLEBAR_CONTROL_SPACING)
        .align_y(Alignment::Center)
        .into(),
        true,
    );

    container(
        column![
            container(
                row![left_tools, center_drag_zone, right_controls]
                    .align_y(Alignment::Center)
                    .height(Fill),
            )
            .height(Fill)
            .padding(Padding {
                top: 0.0,
                right: TITLEBAR_HORIZONTAL_PADDING,
                bottom: 0.0,
                left: TITLEBAR_HORIZONTAL_PADDING,
            }),
            container("")
                .width(Fill)
                .height(TITLEBAR_DIVIDER_HEIGHT)
                .style(crate::theme::styles::titlebar_divider),
        ]
        .height(TITLEBAR_HEIGHT),
    )
    .style(crate::theme::styles::titlebar)
    .into()
}

fn logo<'a, Message: 'a>() -> Element<'a, Message> {
    let logo_spec = FrameSpec {
        width: TITLEBAR_LOGO_EDGE,
        height: TITLEBAR_LOGO_EDGE,
        icon_size: TITLEBAR_LOGO_GLYPH,
        tone: iced::Color::WHITE,
        background: colors::BRAND_BLUE,
        border_color: colors::BRAND_BLUE,
        radius: TITLEBAR_LOGO_RADIUS,
    };

    container(icons::titlebar_framed(Glyph::Radar, logo_spec))
        .width(TITLEBAR_LOGO_EDGE)
        .height(TITLEBAR_LOGO_EDGE)
        .center_x(Length::Fixed(TITLEBAR_LOGO_EDGE))
        .center_y(Length::Fixed(TITLEBAR_LOGO_EDGE))
        .into()
}

fn title_font() -> Font {
    fonts::semibold()
}

fn icon_button<'a, Message>(
    theme_mode: ThemeMode,
    glyph: Glyph,
    message: Message,
    role: ButtonRole,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let is_dark = theme_mode == ThemeMode::Dark;
    let (style, edge, base_icon_size, tone): (
        fn(&Theme, button::Status) -> button::Style,
        f32,
        f32,
        _,
    ) = match role {
        ButtonRole::Tool => (
            crate::theme::styles::titlebar_tool_button,
            TITLEBAR_TOOL_BUTTON_EDGE,
            TITLEBAR_TOOL_GLYPH,
            if is_dark {
                colors::rgb(0xD2, 0xD8, 0xE2)
            } else {
                colors::rgb(0x4A, 0x55, 0x64)
            },
        ),
        ButtonRole::Control => (
            crate::theme::styles::titlebar_button,
            TITLEBAR_CONTROL_BUTTON_EDGE,
            TITLEBAR_CONTROL_GLYPH,
            if is_dark {
                colors::rgb(0xC3, 0xC9, 0xD4)
            } else {
                colors::rgb(0x5E, 0x66, 0x74)
            },
        ),
        ButtonRole::Close => (
            crate::theme::styles::close_button,
            TITLEBAR_CONTROL_BUTTON_EDGE,
            TITLEBAR_CONTROL_GLYPH,
            if is_dark {
                colors::rgb(0xC3, 0xC9, 0xD4)
            } else {
                colors::rgb(0x5E, 0x66, 0x74)
            },
        ),
    };
    let icon_size = if matches!(glyph, Glyph::Maximize | Glyph::Restore) {
        TITLEBAR_MAXIMIZE_GLYPH
    } else {
        base_icon_size
    };

    let content = match role {
        ButtonRole::Tool => icons::titlebar_centered(glyph, edge, icon_size, tone),
        ButtonRole::Control | ButtonRole::Close => {
            icons::titlebar_centered_compact(glyph, edge, icon_size, tone)
        }
    };

    button(content)
        .width(Length::Fixed(edge))
        .height(Length::Fixed(edge))
        .padding(0)
        .style(style)
        .on_press(message)
        .into()
}

fn side_slot<'a, Message>(content: Element<'a, Message>, trailing: bool) -> Element<'a, Message>
where
    Message: 'a,
{
    let alignment = if trailing {
        Horizontal::Right
    } else {
        Horizontal::Left
    };

    let padding = if trailing {
        Padding {
            top: 0.0,
            right: TITLEBAR_SIDE_SAFE_INSET,
            bottom: 0.0,
            left: 0.0,
        }
    } else {
        Padding {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: TITLEBAR_SIDE_SAFE_INSET,
        }
    };

    container(content)
        .width(Length::Fixed(TITLEBAR_SIDE_WIDTH))
        .padding(padding)
        .align_x(alignment)
        .center_y(Fill)
        .into()
}
