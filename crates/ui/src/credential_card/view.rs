use iced::widget::{Space, button, column, container, row, stack, svg, text, text_input};
use iced::{Alignment, Element, Fill, Length, Theme, border};

use crate::theme::{
    self, AppLanguage, colors, fonts,
    icons::{self, Glyph},
};

pub const CARD_PADDING: u16 = 20;
pub const SECTION_SPACING: u16 = 10;
pub const FIELD_SPACING: u16 = 9;
pub const HEADER_HEIGHT: f32 = 28.0;
pub const LABEL_ROW_HEIGHT: f32 = 0.0;
pub const ACTION_BUTTON_HEIGHT: f32 = 44.0;
pub const USER_DROPDOWN_TOP_OFFSET: f32 =
    CARD_PADDING as f32 + HEADER_HEIGHT + SECTION_SPACING as f32;

const CONTROL_RADIUS: f32 = 12.0;
const INPUT_VERTICAL_PADDING: u16 = 10;
const INPUT_HORIZONTAL_PADDING: u16 = 14;
const SSH_INPUT_TEXT_SIZE: f32 = 14.0;
const SSH_INPUT_VERTICAL_PADDING: u16 = INPUT_VERTICAL_PADDING;
const TITLE_SPACING: f32 = 8.0;
const MANAGE_BUTTON_HEIGHT: f32 = 24.0;
const MANAGE_BUTTON_WIDTH: f32 = 78.0;
const MANAGE_BUTTON_TEXT_OFFSET: f32 = 28.0;
const SECTION_TITLE_HEIGHT: f32 = 24.0;
const RUSTDESK_SECTION_SPACING: f32 = 9.0;
const SECTION_DIVIDER_PADDING: u16 = 8;
const INPUT_HEIGHT: f32 = crate::widgets::dropdown::TRIGGER_HEIGHT;
const SSH_DROPDOWN_TRIGGER_WIDTH: f32 = 38.0;
const SSH_DROPDOWN_VERTICAL_INSET: u16 = 1;
const MANAGE_BUTTON_ICON_ASSET: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="78" height="24" viewBox="0 0 78 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><g transform="translate(4.8 4.2) scale(0.61)"><path d="M10 5H3" /><path d="M12 19H3" /><path d="M14 3v4" /><path d="M16 17v4" /><path d="M21 12h-9" /><path d="M21 19h-5" /><path d="M21 5h-7" /><path d="M8 10v4" /><path d="M8 12H3" /></g></svg>"#;
const LIGHT_INPUT_BACKGROUND: iced::Color = colors::rgb(0xF4, 0xF5, 0xF7);
const LIGHT_INACTIVE_ICON: iced::Color = colors::rgb(0x4B, 0x55, 0x63);
const LIGHT_TITLE_TEXT: iced::Color = colors::rgb(0x1F, 0x29, 0x37);
const LIGHT_BODY_TEXT: iced::Color = colors::rgb(0x37, 0x41, 0x51);
const LIGHT_MUTED_TEXT: iced::Color = colors::rgb(0x9C, 0xA3, 0xAF);

fn title_text_style(theme: &Theme) -> iced::widget::text::Style {
    if colors::palette(theme).card == colors::DARK.card {
        theme::text_primary(theme)
    } else {
        theme::solid_text(LIGHT_TITLE_TEXT)
    }
}

fn body_text_color(theme: &Theme) -> iced::Color {
    if colors::palette(theme).card == colors::DARK.card {
        colors::palette(theme).text
    } else {
        LIGHT_BODY_TEXT
    }
}

fn muted_text_color(theme: &Theme) -> iced::Color {
    if colors::palette(theme).card == colors::DARK.card {
        colors::palette(theme).muted_text
    } else {
        LIGHT_MUTED_TEXT
    }
}

pub struct CredentialCardProps<
    'a,
    Message,
    UsernameInput,
    PasswordInput,
    VncUserInput,
    VncPasswordInput,
> where
    Message: Clone + 'a,
    UsernameInput: Fn(String) -> Message + 'a,
    PasswordInput: Fn(String) -> Message + 'a,
    VncUserInput: Fn(String) -> Message + 'a,
    VncPasswordInput: Fn(String) -> Message + 'a,
{
    pub app_language: AppLanguage,
    pub dropdown: Element<'a, Message>,
    pub is_dark_theme: bool,
    pub username: &'a str,
    pub password: &'a str,
    pub vnc_enabled: bool,
    pub vnc_user: &'a str,
    pub vnc_password: &'a str,
    pub is_verifying: bool,
    pub has_scanned: bool,
    pub has_devices: bool,
    pub spinner_frame: &'static str,
    pub on_manage: Option<Message>,
    pub on_toggle_vnc: Option<Message>,
    pub on_verify: Option<Message>,
    pub on_username_input: UsernameInput,
    pub on_password_input: PasswordInput,
    pub on_vnc_user_input: VncUserInput,
    pub on_vnc_password_input: VncPasswordInput,
}

pub fn view<'a, Message>(
    props: CredentialCardProps<
        'a,
        Message,
        impl Fn(String) -> Message + 'a,
        impl Fn(String) -> Message + 'a,
        impl Fn(String) -> Message + 'a,
        impl Fn(String) -> Message + 'a,
    >,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let CredentialCardProps {
        app_language,
        dropdown,
        is_dark_theme,
        username,
        password,
        vnc_enabled,
        vnc_user: _,
        vnc_password,
        is_verifying,
        has_scanned,
        has_devices,
        spinner_frame,
        on_manage,
        on_toggle_vnc,
        on_verify,
        on_username_input,
        on_password_input,
        on_vnc_user_input: _,
        on_vnc_password_input,
    } = props;

    let verify_action = if is_verifying { None } else { on_verify };

    let header_icon_tone = if is_dark_theme {
        colors::LIGHT.text
    } else {
        LIGHT_INACTIVE_ICON
    };
    let header = container(
        row![
            row![
                header_icon(header_icon_tone),
                text(credential_card_title(app_language))
                    .font(fonts::semibold())
                    .size(14)
                    .style(title_text_style),
            ]
            .spacing(TITLE_SPACING)
            .align_y(Alignment::Center),
            Space::new().width(Length::Fill),
            manage_button(app_language, on_manage),
        ]
        .align_y(Alignment::Center),
    )
    .height(Length::Fixed(HEADER_HEIGHT));

    let vnc_fields: Element<'a, Message> = if vnc_enabled {
        column![credential_input(
            rustdesk_password_placeholder(app_language),
            vnc_password,
            true,
            (!is_verifying).then_some(on_vnc_password_input),
        ),]
        .spacing(FIELD_SPACING as f32)
        .into()
    } else {
        Space::new().height(Length::Shrink).into()
    };

    let verify_disabled = is_verifying || !has_scanned || !has_devices || verify_action.is_none();
    let verify_button = button(verify_button_content(
        app_language,
        is_verifying,
        spinner_frame,
        is_dark_theme,
    ))
    .width(Fill)
    .height(Length::Fixed(ACTION_BUTTON_HEIGHT))
    .padding([0, 14])
    .style(move |theme: &Theme, status| {
        verify_button_style(theme, status, verify_disabled, is_verifying)
    })
    .on_press_maybe(verify_action);

    let ssh_username_row = merged_username_row(
        app_language,
        username,
        dropdown,
        (!is_verifying).then_some(on_username_input),
    );

    let ssh_fields = column![
        ssh_username_row,
        readonly_secret_input(
            password_placeholder(app_language),
            password,
            on_password_input,
            is_verifying,
        ),
    ]
    .spacing(FIELD_SPACING as f32);

    let rustdesk_title_row = container(
        row![
            text(rustdesk_section_title(app_language))
                .font(fonts::semibold())
                .size(13)
                .style(title_text_style),
            Space::new().width(Length::Fill),
            crate::widgets::toggle::view(vnc_enabled, on_toggle_vnc),
        ]
        .align_y(Alignment::Center),
    )
    .height(Length::Fixed(SECTION_TITLE_HEIGHT));

    let content = column![
        column![
            header,
            ssh_fields,
            container(divider()).padding([SECTION_DIVIDER_PADDING, 0]),
            column![rustdesk_title_row, vnc_fields].spacing(if vnc_enabled {
                RUSTDESK_SECTION_SPACING
            } else {
                0.0
            }),
        ]
        .spacing(SECTION_SPACING as f32),
        Space::new().height(Length::Fill),
        verify_button,
    ]
    .spacing(SECTION_SPACING as f32)
    .height(Fill);

    container(content)
        .width(Fill)
        .height(Fill)
        .padding(CARD_PADDING)
        .style(theme::styles::card)
        .into()
}

fn header_icon<'a, Message: 'a>(tone: iced::Color) -> Element<'a, Message> {
    container(icons::centered(Glyph::Lock, 22.0, 18.0, tone))
        .width(22)
        .height(22)
        .center_x(Length::Fixed(22.0))
        .center_y(Length::Fixed(22.0))
        .into()
}

fn manage_button<'a, Message>(
    app_language: AppLanguage,
    on_press: Option<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    button(
        stack([
            manage_button_icon_layer(),
            row![
                Space::new().width(Length::Fixed(MANAGE_BUTTON_TEXT_OFFSET)),
                text(manage_button_label(app_language))
                    .font(fonts::semibold())
                    .size(11),
            ]
            .width(Length::Fixed(MANAGE_BUTTON_WIDTH))
            .height(Length::Fixed(MANAGE_BUTTON_HEIGHT))
            .align_y(Alignment::Center)
            .into(),
        ])
        .width(Length::Fixed(MANAGE_BUTTON_WIDTH))
        .height(Length::Fixed(MANAGE_BUTTON_HEIGHT)),
    )
    .width(Length::Fixed(MANAGE_BUTTON_WIDTH))
    .height(Length::Fixed(MANAGE_BUTTON_HEIGHT))
    .padding(0)
    .style(manage_button_style)
    .on_press_maybe(on_press)
    .into()
}

fn manage_button_idle_tone(theme: &Theme) -> iced::Color {
    if colors::palette(theme).card == colors::DARK.card {
        colors::rgb(0x93, 0xC5, 0xFD)
    } else {
        colors::BRAND_BLUE
    }
}

fn manage_button_active_tone(theme: &Theme) -> iced::Color {
    if colors::palette(theme).card == colors::DARK.card {
        colors::rgb(0xDB, 0xEA, 0xFE)
    } else {
        colors::rgb(0x1D, 0x4E, 0xD8)
    }
}

fn manage_button_icon_layer<'a, Message: 'a>() -> Element<'a, Message> {
    svg(svg::Handle::from_memory(MANAGE_BUTTON_ICON_ASSET))
        .width(Length::Fixed(MANAGE_BUTTON_WIDTH))
        .height(Length::Fixed(MANAGE_BUTTON_HEIGHT))
        .style(|theme: &Theme, status| svg::Style {
            color: Some(match status {
                svg::Status::Hovered => manage_button_active_tone(theme),
                svg::Status::Idle => manage_button_idle_tone(theme),
            }),
        })
        .into()
}

fn manage_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let text_color = match status {
        button::Status::Hovered | button::Status::Pressed => manage_button_active_tone(theme),
        _ => manage_button_idle_tone(theme),
    };

    button::Style {
        snap: false,
        background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
        text_color,
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(CONTROL_RADIUS),
        },
        shadow: iced::Shadow::default(),
    }
}

fn divider<'a, Message: 'a>() -> Element<'a, Message> {
    container(text(""))
        .width(Fill)
        .height(1)
        .style(|theme: &Theme| {
            let palette = colors::palette(theme);

            container::Style::default().background(palette.border)
        })
        .into()
}

fn verify_button_content<'a, Message: 'a>(
    app_language: AppLanguage,
    is_verifying: bool,
    spinner_frame: &'static str,
    is_dark_theme: bool,
) -> Element<'a, Message> {
    let foreground = if is_dark_theme {
        colors::LIGHT.text
    } else {
        colors::LIGHT.card
    };

    let content: Element<'a, Message> = if is_verifying {
        row![
            icons::rotating_refresh_centered(spinner_frame, 16.0, 13.0, foreground),
            text(verify_button_loading_label(app_language))
                .font(fonts::semibold())
                .size(13)
                .style(move |_| theme::solid_text(foreground)),
        ]
        .spacing(7)
        .align_y(Alignment::Center)
        .into()
    } else {
        text(verify_button_label(app_language))
            .font(fonts::semibold())
            .size(13)
            .style(move |_| theme::solid_text(foreground))
            .into()
    };

    container(content)
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
}

fn verify_button_style(
    theme: &Theme,
    status: button::Status,
    disabled: bool,
    is_loading: bool,
) -> button::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;
    let base = palette.validation_button;
    let background = if is_loading {
        base
    } else if disabled {
        if is_dark {
            colors::rgb(0x61, 0x6B, 0x78)
        } else {
            colors::rgb(0xB8, 0xC1, 0xCD)
        }
    } else {
        match status {
            button::Status::Pressed => {
                if is_dark {
                    colors::rgb(0x3B, 0x82, 0xF6)
                } else {
                    colors::rgb(0x1D, 0x4E, 0xD8)
                }
            }
            button::Status::Hovered => {
                if is_dark {
                    colors::rgb(0x60, 0xA5, 0xFA)
                } else {
                    colors::rgb(0x25, 0x63, 0xEB)
                }
            }
            _ => base,
        }
    };

    button::Style {
        snap: false,
        background: Some(iced::Background::Color(background)),
        text_color: if is_dark {
            colors::LIGHT.text
        } else {
            colors::LIGHT.card
        },
        border: iced::Border {
            color: background,
            width: 1.0,
            radius: border::radius(CONTROL_RADIUS),
        },
        shadow: iced::Shadow::default(),
    }
}

fn credential_input<'a, Message>(
    placeholder: &'static str,
    value: &'a str,
    secure: bool,
    on_input: Option<impl Fn(String) -> Message + 'a>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        text_input(placeholder, value)
            .on_input_maybe(on_input)
            .secure(secure)
            .width(Fill)
            .size(12)
            .font(fonts::body())
            .padding([INPUT_VERTICAL_PADDING, INPUT_HORIZONTAL_PADDING])
            .style(input_style),
    )
    .width(Fill)
    .height(Length::Fixed(INPUT_HEIGHT))
    .center_y(Length::Fixed(INPUT_HEIGHT))
    .into()
}

fn merged_username_row<'a, Message>(
    app_language: AppLanguage,
    username: &'a str,
    dropdown: Element<'a, Message>,
    on_username_input: Option<impl Fn(String) -> Message + 'a>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let username_input = text_input(username_placeholder(app_language), username)
        .on_input_maybe(on_username_input)
        .width(Fill)
        .size(SSH_INPUT_TEXT_SIZE)
        .font(fonts::body())
        .padding([SSH_INPUT_VERTICAL_PADDING, INPUT_HORIZONTAL_PADDING])
        .style(merged_username_input_style);

    container(
        row![
            container(username_input)
                .width(Fill)
                .height(Fill)
                .center_y(Fill),
            container(dropdown)
                .width(Length::Fixed(SSH_DROPDOWN_TRIGGER_WIDTH))
                .height(Fill)
                .padding([SSH_DROPDOWN_VERTICAL_INSET, 0])
                .center_y(Fill),
        ]
        .width(Fill)
        .height(Length::Fixed(INPUT_HEIGHT))
        .spacing(0)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(INPUT_HEIGHT))
    .center_y(Length::Fixed(INPUT_HEIGHT))
    .style(merged_username_row_style)
    .clip(true)
    .into()
}

fn credential_card_title(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "SSH 登录凭证",
        AppLanguage::English => "SSH Credentials",
    }
}

fn manage_button_label(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "管理",
        AppLanguage::English => "Manage",
    }
}

fn username_placeholder(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "用户名",
        AppLanguage::English => "Username",
    }
}

fn password_placeholder(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "密码",
        AppLanguage::English => "Password",
    }
}

fn rustdesk_password_placeholder(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "RustDesk 密码（可选）",
        AppLanguage::English => "RustDesk Password (Optional)",
    }
}

fn rustdesk_section_title(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "RustDesk 凭证（可选）",
        AppLanguage::English => "RustDesk Credentials",
    }
}

fn verify_button_label(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "检测 SSH 凭证",
        AppLanguage::English => "Verify SSH",
    }
}

fn verify_button_loading_label(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "检测中...",
        AppLanguage::English => "Verifying...",
    }
}

fn merged_username_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = input_style(theme, status);
    style.background = iced::Background::Color(iced::Color::TRANSPARENT);
    style.border.color = iced::Color::TRANSPARENT;
    style.border.width = 0.0;
    style
}

fn merged_username_row_style(theme: &Theme) -> container::Style {
    let palette = colors::palette(theme);
    let background = if palette.card == colors::DARK.card {
        palette.input
    } else {
        LIGHT_INPUT_BACKGROUND
    };

    container::Style::default()
        .background(background)
        .border(iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(CONTROL_RADIUS),
        })
}

fn input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;
    let is_focused = matches!(&status, text_input::Status::Focused { .. });

    let (background, border_color) = match status {
        text_input::Status::Focused { .. } => (
            if is_dark {
                palette.input
            } else {
                LIGHT_INPUT_BACKGROUND
            },
            palette.primary,
        ),
        text_input::Status::Hovered => (
            if is_dark {
                palette.input
            } else {
                LIGHT_INPUT_BACKGROUND
            },
            iced::Color::TRANSPARENT,
        ),
        text_input::Status::Disabled => (
            if is_dark {
                colors::rgba(
                    (palette.input.r * 255.0).round() as u8,
                    (palette.input.g * 255.0).round() as u8,
                    (palette.input.b * 255.0).round() as u8,
                    0.66,
                )
            } else {
                colors::rgba(0xF4, 0xF5, 0xF7, 0.82)
            },
            iced::Color::TRANSPARENT,
        ),
        text_input::Status::Active => (
            if is_dark {
                palette.input
            } else {
                LIGHT_INPUT_BACKGROUND
            },
            iced::Color::TRANSPARENT,
        ),
    };

    text_input::Style {
        background: iced::Background::Color(background),
        border: iced::Border {
            color: border_color,
            width: if is_focused { 1.0 } else { 0.0 },
            radius: border::radius(CONTROL_RADIUS),
        },
        icon: muted_text_color(theme),
        placeholder: muted_text_color(theme),
        value: body_text_color(theme),
        selection: colors::rgba(0x3B, 0x82, 0xF6, 0.16),
    }
}

fn readonly_secret_input<'a, Message>(
    placeholder: &'static str,
    value: &'a str,
    on_password_input: impl Fn(String) -> Message + 'a,
    disabled: bool,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        text_input(placeholder, value)
            .secure(true)
            .size(SSH_INPUT_TEXT_SIZE)
            .font(fonts::body())
            .padding([SSH_INPUT_VERTICAL_PADDING, INPUT_HORIZONTAL_PADDING])
            .style(input_style)
            .on_input_maybe((!disabled).then_some(on_password_input)),
    )
    .width(Fill)
    .height(Length::Fixed(INPUT_HEIGHT))
    .center_y(Length::Fixed(INPUT_HEIGHT))
    .into()
}
