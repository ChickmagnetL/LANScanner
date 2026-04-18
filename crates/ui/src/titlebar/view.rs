use iced::widget::{Space, button, column, container, mouse_area, row, text};
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
#[cfg(target_os = "linux")]
const TITLEBAR_LOGO_CUSTOM_GLYPH_OFFSET_X: f32 = TITLEBAR_LOGO_GLYPH * (1.5 / 24.0);
#[cfg(not(target_os = "linux"))]
const TITLEBAR_LOGO_CUSTOM_GLYPH_OFFSET_X: f32 = 0.0;
const TITLEBAR_DIVIDER_HEIGHT: f32 = 1.0;
const MACOS_TITLEBAR_HEIGHT: u32 = TITLEBAR_HEIGHT;
const MACOS_TRAFFIC_LIGHTS_RESERVED_WIDTH: f32 = 88.0;
const MACOS_SIDE_WIDTH: f32 = TITLEBAR_SIDE_WIDTH;
const MACOS_TITLEBAR_TOP_PADDING: f32 = 0.0;
const MACOS_TITLEBAR_BOTTOM_PADDING: f32 = 4.0;

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
    radius: f32,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let left_tools = side_slot(
        tool_buttons(
            theme_mode,
            app_language,
            on_toggle_theme,
            on_help,
            on_toggle_language,
        ),
        false,
    );

    let center_drag_zone = brand_drag_zone(on_window_action, TITLEBAR_LOGO_CUSTOM_GLYPH_OFFSET_X);

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

    titlebar_shell(
        left_tools,
        center_drag_zone,
        right_controls,
        radius,
        TITLEBAR_HEIGHT,
        Padding {
            top: 0.0,
            right: TITLEBAR_HORIZONTAL_PADDING,
            bottom: 0.0,
            left: TITLEBAR_HORIZONTAL_PADDING,
        },
    )
}

pub fn macos_overlay_view<'a, Message>(
    theme_mode: ThemeMode,
    app_language: AppLanguage,
    on_toggle_theme: Message,
    on_help: Message,
    on_toggle_language: Message,
    on_window_action: impl Fn(WindowAction) -> Message + Copy + 'a,
    radius: f32,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let leading_traffic_lights_reserve = side_slot_width(
        Space::new()
            .width(Length::Fixed(MACOS_TRAFFIC_LIGHTS_RESERVED_WIDTH))
            .height(Length::Shrink)
            .into(),
        false,
        MACOS_SIDE_WIDTH,
    );

    let center_drag_zone = brand_drag_zone(on_window_action, 0.0);

    let trailing_tools = side_slot_width(
        tool_buttons(
            theme_mode,
            app_language,
            on_toggle_theme,
            on_help,
            on_toggle_language,
        ),
        true,
        MACOS_SIDE_WIDTH,
    );

    titlebar_shell(
        leading_traffic_lights_reserve,
        center_drag_zone,
        trailing_tools,
        radius,
        MACOS_TITLEBAR_HEIGHT,
        Padding {
            top: MACOS_TITLEBAR_TOP_PADDING,
            right: TITLEBAR_HORIZONTAL_PADDING,
            bottom: MACOS_TITLEBAR_BOTTOM_PADDING,
            left: TITLEBAR_HORIZONTAL_PADDING,
        },
    )
}

fn logo<'a, Message: 'a>(icon_offset_x: f32) -> Element<'a, Message> {
    let logo_spec = FrameSpec {
        width: TITLEBAR_LOGO_EDGE,
        height: TITLEBAR_LOGO_EDGE,
        icon_size: TITLEBAR_LOGO_GLYPH,
        tone: iced::Color::WHITE,
        background: colors::BRAND_BLUE,
        border_color: colors::BRAND_BLUE,
        radius: TITLEBAR_LOGO_RADIUS,
    };

    container(icons::titlebar_framed_with_icon_offset(
        Glyph::Radar,
        logo_spec,
        icon_offset_x,
    ))
    .width(TITLEBAR_LOGO_EDGE)
    .height(TITLEBAR_LOGO_EDGE)
    .center_x(Length::Fixed(TITLEBAR_LOGO_EDGE))
    .center_y(Length::Fixed(TITLEBAR_LOGO_EDGE))
    .into()
}

fn title_font() -> Font {
    fonts::semibold()
}

fn tool_buttons<'a, Message>(
    theme_mode: ThemeMode,
    app_language: AppLanguage,
    on_toggle_theme: Message,
    on_help: Message,
    on_toggle_language: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let theme_icon = theme_toggle_glyph(theme_mode);
    let language_icon = language_toggle_glyph(app_language);

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
    .into()
}

fn brand_drag_zone<'a, Message>(
    on_window_action: impl Fn(WindowAction) -> Message + Copy + 'a,
    logo_icon_offset_x: f32,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    mouse_area(
        container(
            row![
                logo(logo_icon_offset_x),
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
    .on_press(on_window_action(WindowAction::Drag))
    .into()
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

fn titlebar_shell<'a, Message>(
    left: Element<'a, Message>,
    center: Element<'a, Message>,
    right: Element<'a, Message>,
    radius: f32,
    height: u32,
    padding: Padding,
) -> Element<'a, Message>
where
    Message: 'a,
{
    container(
        column![
            container(
                row![left, center, right]
                    .align_y(Alignment::Center)
                    .height(Fill),
            )
            .height(Fill)
            .padding(padding),
            container("")
                .width(Fill)
                .height(TITLEBAR_DIVIDER_HEIGHT)
                .style(crate::theme::styles::titlebar_divider),
        ]
        .height(height),
    )
    .style(move |theme| crate::theme::styles::titlebar_with_radius(theme, radius))
    .into()
}

fn side_slot<'a, Message>(content: Element<'a, Message>, trailing: bool) -> Element<'a, Message>
where
    Message: 'a,
{
    side_slot_width(content, trailing, TITLEBAR_SIDE_WIDTH)
}

fn side_slot_width<'a, Message>(
    content: Element<'a, Message>,
    trailing: bool,
    width: f32,
) -> Element<'a, Message>
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
        .width(Length::Fixed(width))
        .padding(padding)
        .align_x(alignment)
        .center_y(Fill)
        .into()
}

fn theme_toggle_glyph(theme_mode: ThemeMode) -> Glyph {
    if theme_mode == ThemeMode::Dark {
        Glyph::Sun
    } else {
        Glyph::Moon
    }
}

fn language_toggle_glyph(app_language: AppLanguage) -> Glyph {
    match app_language {
        AppLanguage::Chinese | AppLanguage::English => Glyph::Languages,
    }
}
