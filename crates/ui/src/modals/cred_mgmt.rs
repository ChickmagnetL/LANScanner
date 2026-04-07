use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Fill, Length, Theme, border};
use ssh_core::credential::Credential;
use std::rc::Rc;

use crate::theme::{
    self, colors, fonts,
    icons::{self, Glyph},
};

const SECTION_SPACING: f32 = 10.0;
const LIST_HEIGHT: f32 = 200.0;
const EDITING_LIST_HEIGHT: f32 = 200.0;

pub struct CredentialManagementProps<'a, Message, UsernameInput, PasswordInput>
where
    Message: Clone + 'a,
    UsernameInput: Fn(String) -> Message + 'a,
    PasswordInput: Fn(String) -> Message + 'a,
{
    pub credentials: &'a [Credential],
    pub editing_username: Option<&'a str>,
    pub username: &'a str,
    pub password: &'a str,
    pub on_username_input: UsernameInput,
    pub on_password_input: PasswordInput,
    pub on_edit: fn(String) -> Message,
    pub on_cancel_edit: Option<Message>,
    pub on_save: Option<Message>,
    pub on_remove: fn(String) -> Message,
    pub on_close: Message,
}

pub fn view<'a, Message>(
    props: CredentialManagementProps<
        'a,
        Message,
        impl Fn(String) -> Message + 'a,
        impl Fn(String) -> Message + 'a,
    >,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let CredentialManagementProps {
        credentials,
        editing_username,
        username,
        password,
        on_username_input,
        on_password_input,
        on_edit,
        on_cancel_edit,
        on_save,
        on_remove,
        on_close,
    } = props;

    let on_username_input = Rc::new(on_username_input);
    let on_password_input = Rc::new(on_password_input);
    let is_editing = editing_username.is_some();
    let loop_on_save = on_save.clone();
    let loop_on_cancel_edit = on_cancel_edit.clone();
    let list_password_input = Rc::clone(&on_password_input);
    let saved_credentials =
        credentials
            .iter()
            .fold(column!().spacing(8), move |column, credential| {
                let is_item_editing = editing_username == Some(credential.username.as_str());

                let edit_message = if is_item_editing {
                    loop_on_cancel_edit
                        .clone()
                        .unwrap_or_else(|| on_edit(credential.username.clone()))
                } else {
                    on_edit(credential.username.clone())
                };
                let mut actions = row![edit_button(edit_message, is_item_editing)]
                    .spacing(4)
                    .align_y(Alignment::Center);
                if credential.can_delete {
                    actions = actions.push(delete_button(on_remove(credential.id.clone())));
                }

                let mut item_body = column![
                    row![
                        text(&credential.username)
                            .font(fonts::semibold())
                            .size(14)
                            .style(|theme: &Theme| theme::text_primary(theme))
                            .width(Fill),
                        actions,
                    ]
                    .align_y(Alignment::Center)
                    .width(Fill),
                ]
                .spacing(8);

                if is_item_editing {
                    let password_input = Rc::clone(&list_password_input);
                    item_body = item_body.push(divider()).push(
                        row![
                            text_input("新密码", password)
                                .on_input(move |value| (*password_input)(value))
                                .secure(true)
                                .padding([8, 10])
                                .size(12)
                                .font(fonts::body())
                                .style(input_style)
                                .width(Fill),
                            inline_confirm_button(loop_on_save.clone()),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    );
                }

                column.push(
                    container(item_body)
                        .padding([12, 14])
                        .style(list_item_style),
                )
            });

    let header = container(
        row![
            text("管理 SSH 凭证")
                .font(fonts::semibold())
                .size(15)
                .style(|theme: &Theme| theme::text_primary(theme)),
            Space::new().width(Length::Fill),
            close_button(on_close.clone()),
        ]
        .align_y(Alignment::Center),
    )
    .padding([16, 20]);

    let header_divider = container(Space::new().height(1.0))
        .width(Fill)
        .style(theme::styles::titlebar_divider);

    let username_field = if is_editing {
        text_input("用户名", "")
            .padding([9, 12])
            .size(13)
            .font(fonts::body())
            .style(input_style)
            .width(Fill)
    } else {
        let username_input_clone = Rc::clone(&on_username_input);
        text_input("用户名", username)
            .on_input(move |value| (*username_input_clone)(value))
            .padding([9, 12])
            .size(13)
            .font(fonts::body())
            .style(input_style)
            .width(Fill)
    };

    let password_field = if is_editing {
        text_input("密码", "")
            .secure(true)
            .padding([9, 12])
            .size(13)
            .font(fonts::body())
            .style(input_style)
            .width(Fill)
    } else {
        let password_input_clone = Rc::clone(&on_password_input);
        text_input("密码", password)
            .on_input(move |value| (*password_input_clone)(value))
            .secure(true)
            .padding([9, 12])
            .size(13)
            .font(fonts::body())
            .style(input_style)
            .width(Fill)
    };

    let list_section = column![
        section_title("已保存的凭证"),
        scrollable(saved_credentials)
            .height(Length::Fixed(if is_editing {
                EDITING_LIST_HEIGHT
            } else {
                LIST_HEIGHT
            }))
            .style(theme::styles::custom_scrollbar),
    ]
    .spacing(SECTION_SPACING);

    let add_section = column![
        section_title("添加新凭证"),
        username_field,
        password_field,
        add_save_button(if is_editing { None } else { on_save.clone() }),
    ]
    .spacing(SECTION_SPACING);

    let footer = column![
        container(Space::new().height(1.0))
            .width(Fill)
            .style(footer_divider_style),
        container(
            row![Space::new().width(Length::Fill), done_button(on_close),]
                .align_y(Alignment::Center)
        )
        .padding([12, 20])
        .style(footer_style),
    ]
    .spacing(0);

    column![
        header,
        header_divider,
        // Scrollable list only
        container(list_section)
            .padding(iced::Padding {
                top: 20.0,
                right: 20.0,
                bottom: 12.0,
                left: 20.0
            })
            .width(Fill),
        // Add section always visible
        container(add_section)
            .padding(iced::Padding {
                top: 0.0,
                right: 20.0,
                bottom: 16.0,
                left: 20.0
            })
            .width(Fill),
        footer,
    ]
    .width(Fill)
    .into()
}

fn divider<'a, Message: 'a>() -> Element<'a, Message> {
    container(Space::new().height(1.0))
        .style(|theme: &Theme| {
            let palette = colors::palette(theme);
            container::Style::default().background(if palette.card == colors::DARK.card {
                colors::rgba(0x6C, 0x74, 0x82, 0.2)
            } else {
                colors::rgb(0xF3, 0xF4, 0xF6)
            })
        })
        .into()
}

fn section_title<'a, Message>(label: &'static str) -> Element<'a, Message>
where
    Message: 'a,
{
    text(label)
        .font(fonts::semibold())
        .size(12)
        .style(|theme: &Theme| theme::text_muted(theme))
        .into()
}

fn list_item_style(theme: &Theme) -> container::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;

    container::Style::default()
        .background(if is_dark {
            colors::rgb(0x1E, 0x1E, 0x1E)
        } else {
            colors::rgb(0xF8, 0xF9, 0xFA)
        })
        .border(iced::Border {
            color: if is_dark {
                colors::rgba(0x3F, 0x3F, 0x46, 1.0)
            } else {
                colors::rgba(0xE5, 0xE7, 0xEB, 0.75)
            },
            width: 1.0,
            radius: border::radius(12),
        })
}

fn footer_divider_style(theme: &Theme) -> container::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;

    container::Style::default().background(if is_dark {
        colors::rgba(0x3F, 0x3F, 0x46, 1.0)
    } else {
        colors::rgb(0xF3, 0xF4, 0xF6)
    })
}

fn footer_style(theme: &Theme) -> container::Style {
    let palette = colors::palette(theme);

    container::Style::default()
        .background(if palette.card == colors::DARK.card {
            colors::rgb(0x1E, 0x1E, 0x1E)
        } else {
            colors::rgb(0xF8, 0xF9, 0xFA)
        })
        .border(iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: iced::border::Radius {
                top_left: 0.0,
                top_right: 0.0,
                bottom_right: 16.0,
                bottom_left: 16.0,
            },
        })
}

fn edit_button<'a, Message>(on_press: Message, is_active: bool) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let icon_tone = if is_active {
        colors::BRAND_BLUE
    } else {
        colors::rgb(0x9C, 0xA3, 0xAF)
    };

    button(icons::centered(Glyph::Pencil, 22.0, 16.0, icon_tone))
        .padding([4, 6])
        .style(move |_theme: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let (background, border_color) = if is_active {
                (
                    colors::rgba(0x3B, 0x82, 0xF6, if hovered { 0.18 } else { 0.12 }),
                    colors::rgba(0x3B, 0x82, 0xF6, 0.28),
                )
            } else if hovered {
                (
                    colors::rgba(0x3B, 0x82, 0xF6, 0.10),
                    colors::rgba(0x3B, 0x82, 0xF6, 0.24),
                )
            } else {
                (iced::Color::TRANSPARENT, iced::Color::TRANSPARENT)
            };

            button::Style {
                snap: false,
                background: Some(iced::Background::Color(background)),
                text_color: icon_tone,
                border: iced::Border {
                    color: border_color,
                    width: 1.0,
                    radius: border::radius(8),
                },
                shadow: iced::Shadow::default(),
            }
        })
        .on_press(on_press)
        .into()
}

fn delete_button<'a, Message>(on_press: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    button(icons::centered(
        Glyph::Trash,
        22.0,
        16.0,
        colors::rgb(0x9C, 0xA3, 0xAF),
    ))
    .padding([4, 6])
    .style(move |_theme: &Theme, status| {
        let (tone, background, border_color) = match status {
            button::Status::Hovered | button::Status::Pressed => (
                colors::DANGER_RED,
                colors::rgba(0xEF, 0x44, 0x44, 0.10),
                colors::rgba(0xEF, 0x44, 0x44, 0.24),
            ),
            _ => (
                colors::rgb(0x9C, 0xA3, 0xAF),
                iced::Color::TRANSPARENT,
                iced::Color::TRANSPARENT,
            ),
        };

        button::Style {
            snap: false,
            background: Some(iced::Background::Color(background)),
            text_color: tone,
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: border::radius(10),
            },
            shadow: iced::Shadow::default(),
        }
    })
    .on_press(on_press)
    .into()
}

fn close_button<'a, Message>(on_press: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    button(icons::centered(
        Glyph::Close,
        16.0,
        11.0,
        colors::rgb(0x9C, 0xA3, 0xAF),
    ))
    .padding(4)
    .style(crate::theme::styles::close_button)
    .on_press(on_press)
    .into()
}

fn add_save_button<'a, Message>(on_press: Option<Message>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let is_disabled = on_press.is_none();
    // Disabled: light blue tint; enabled: BRAND_BLUE (like done button)
    let icon_tone = if is_disabled {
        colors::rgba(0x3B, 0x82, 0xF6, 0.55)
    } else {
        colors::LIGHT.card
    };
    let label_tone = if is_disabled {
        colors::rgba(0x3B, 0x82, 0xF6, 0.55)
    } else {
        colors::LIGHT.card
    };

    button(
        container(
            row![
                icons::centered(Glyph::Plus, 16.0, 12.0, icon_tone),
                text("保存凭证")
                    .font(fonts::semibold())
                    .size(13)
                    .style(move |_| theme::solid_text(label_tone)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .center_x(Fill),
    )
    .width(Fill)
    .padding([10, 12])
    .style(move |_theme: &Theme, status| {
        let (background, border_color) = if is_disabled {
            (
                colors::rgba(0x3B, 0x82, 0xF6, 0.10),
                colors::rgba(0x3B, 0x82, 0xF6, 0.22),
            )
        } else {
            match status {
                button::Status::Hovered => {
                    (colors::rgb(0x25, 0x63, 0xEB), colors::rgb(0x25, 0x63, 0xEB))
                }
                button::Status::Pressed => {
                    (colors::rgb(0x1D, 0x4E, 0xD8), colors::rgb(0x1D, 0x4E, 0xD8))
                }
                _ => (colors::BRAND_BLUE, colors::BRAND_BLUE),
            }
        };

        button::Style {
            snap: false,
            background: Some(iced::Background::Color(background)),
            text_color: label_tone,
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: border::radius(12),
            },
            shadow: iced::Shadow::default(),
        }
    })
    .on_press_maybe(on_press)
    .into()
}

fn done_button<'a, Message>(on_press: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    button(
        text("完成")
            .font(fonts::semibold())
            .size(13)
            .style(|_| theme::solid_text(colors::LIGHT.card)),
    )
    .padding([10, 18])
    .style(crate::theme::styles::primary_button)
    .on_press(on_press)
    .into()
}

fn inline_confirm_button<'a, Message>(on_press: Option<Message>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let is_disabled = on_press.is_none();

    button(icons::centered(
        Glyph::Check,
        14.0,
        10.0,
        colors::LIGHT.card,
    ))
    .width(Length::Fixed(30.0))
    .height(Length::Fixed(30.0))
    .padding(0)
    .style(move |_theme: &Theme, status| {
        let background = if is_disabled {
            colors::rgba(0x3B, 0x82, 0xF6, 0.42)
        } else {
            match status {
                button::Status::Hovered | button::Status::Pressed => colors::rgb(0x25, 0x63, 0xEB),
                _ => colors::BRAND_BLUE,
            }
        };

        button::Style {
            snap: false,
            background: Some(iced::Background::Color(background)),
            text_color: colors::LIGHT.card,
            border: iced::Border {
                color: background,
                width: 1.0,
                radius: border::radius(10),
            },
            shadow: iced::Shadow::default(),
        }
    })
    .on_press_maybe(on_press)
    .into()
}

fn input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;

    let (background, border_color) = match status {
        text_input::Status::Focused { .. } => (palette.card, palette.primary),
        text_input::Status::Hovered => (
            palette.input,
            colors::rgba(0x3B, 0x82, 0xF6, if is_dark { 0.46 } else { 0.30 }),
        ),
        text_input::Status::Disabled => (
            colors::rgba(
                (palette.input.r * 255.0).round() as u8,
                (palette.input.g * 255.0).round() as u8,
                (palette.input.b * 255.0).round() as u8,
                0.66,
            ),
            palette.border,
        ),
        text_input::Status::Active => (palette.input, palette.border),
    };

    text_input::Style {
        background: iced::Background::Color(background),
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: border::radius(12),
        },
        icon: palette.muted_text,
        placeholder: colors::rgb(0x9C, 0xA3, 0xAF),
        value: palette.text,
        selection: colors::rgba(0x3B, 0x82, 0xF6, 0.16),
    }
}
