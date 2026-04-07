use iced::widget::{Space, button, container, row};
use iced::{Alignment, Element, Length, Theme, border};

use crate::theme::colors;

const TRACK_WIDTH: f32 = 36.0;
const TRACK_HEIGHT: f32 = 20.0;
const KNOB_SIZE: f32 = 14.0;
const TRACK_PADDING: f32 = 2.0;

pub fn view<'a, Message>(is_on: bool, on_toggle: Option<Message>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let knob = container(Space::new().width(Length::Fixed(KNOB_SIZE)))
        .width(KNOB_SIZE)
        .height(KNOB_SIZE)
        .style(|theme: &Theme| {
            let is_dark = colors::palette(theme).card == colors::DARK.card;
            container::Style::default()
                .background(if is_dark {
                    colors::rgb(0xF8, 0xFA, 0xFC)
                } else {
                    colors::LIGHT.card
                })
                .border(iced::Border {
                    color: colors::rgba(0x00, 0x00, 0x00, 0.04),
                    width: 1.0,
                    radius: border::radius(12),
                })
                .shadow(iced::Shadow {
                    color: colors::rgba(0x00, 0x00, 0x00, if is_dark { 0.18 } else { 0.10 }),
                    offset: iced::Vector::new(0.0, 1.0),
                    blur_radius: 2.0,
                })
        });

    let track_content: Element<'a, Message> = if is_on {
        row![Space::new().width(Length::Fill), knob]
            .align_y(Alignment::Center)
            .into()
    } else {
        row![knob, Space::new().width(Length::Fill)]
            .align_y(Alignment::Center)
            .into()
    };

    button(track_content)
        .width(Length::Fixed(TRACK_WIDTH))
        .height(Length::Fixed(TRACK_HEIGHT))
        .padding(TRACK_PADDING)
        .style(move |theme: &Theme, status| {
            let palette = colors::palette(theme);
            let is_dark = palette.card == colors::DARK.card;
            let background = if is_on {
                match status {
                    button::Status::Pressed => colors::rgb(0x2D, 0x74, 0xF0),
                    button::Status::Hovered => colors::rgb(0x5B, 0x93, 0xF8),
                    button::Status::Disabled => colors::rgba(0x5A, 0x8F, 0xF5, 0.40),
                    _ => colors::rgb(0x4A, 0x85, 0xF6),
                }
            } else {
                let base = if is_dark {
                    colors::rgb(0x4E, 0x56, 0x60)
                } else {
                    colors::rgb(0xD8, 0xDE, 0xE8)
                };
                let hovered = if is_dark {
                    colors::rgb(0x5A, 0x61, 0x6D)
                } else {
                    colors::rgb(0xC9, 0xD2, 0xDE)
                };

                match status {
                    button::Status::Disabled => {
                        if is_dark {
                            colors::rgba(0x52, 0x58, 0x64, 0.72)
                        } else {
                            colors::rgba(0xD1, 0xD8, 0xE2, 0.72)
                        }
                    }
                    button::Status::Hovered | button::Status::Pressed => hovered,
                    _ => base,
                }
            };
            let border_color = if is_on {
                match status {
                    button::Status::Pressed => colors::rgb(0x2B, 0x6C, 0xF0),
                    button::Status::Hovered => colors::rgb(0x4A, 0x85, 0xF6),
                    button::Status::Disabled => colors::rgba(0x5A, 0x8F, 0xF5, 0.22),
                    _ => colors::rgb(0x4A, 0x85, 0xF6),
                }
            } else {
                match status {
                    button::Status::Hovered | button::Status::Pressed => {
                        colors::rgba(0x86, 0x95, 0xA8, if is_dark { 0.36 } else { 0.30 })
                    }
                    _ => colors::rgba(0x9C, 0xA7, 0xB7, if is_dark { 0.42 } else { 0.56 }),
                }
            };
            button::Style {
                snap: false,
                background: Some(iced::Background::Color(background)),
                text_color: colors::LIGHT.card,
                border: iced::Border {
                    color: border_color,
                    width: 1.0,
                    radius: border::radius(12),
                },
                shadow: iced::Shadow::default(),
            }
        })
        .on_press_maybe(on_toggle)
        .into()
}
