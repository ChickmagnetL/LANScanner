use iced::widget::{Space, button, container, row, text};
use iced::{Alignment, Element, Length, Theme, border};
use ui::theme::{self, colors};

use crate::message::Message;

use super::super::{ScanResultFilter, ShellApp};

pub(super) fn scan_result_filter_controls(app: &ShellApp) -> Element<'_, Message> {
    if !app.has_scanned {
        return Space::new()
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into();
    }

    container(
        row![
            scan_result_filter_button(
                "全部在线",
                app.scan_result_filter == ScanResultFilter::AllOnline,
                Message::ShowAllOnlineResults
            ),
            scan_result_filter_button(
                "SSH",
                app.scan_result_filter == ScanResultFilter::SshReady,
                Message::ShowSshReadyResults
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([3, 4])
    .style(|theme: &Theme| {
        let palette = colors::palette(theme);
        container::Style::default()
            .background(palette.input)
            .border(iced::Border {
                color: palette.border,
                width: 1.0,
                radius: border::radius(999),
            })
    })
    .into()
}

fn scan_result_filter_button<'a>(
    label: &'static str,
    is_active: bool,
    message: Message,
) -> Element<'a, Message> {
    button(
        text(label)
            .size(12)
            .font(ui::theme::fonts::semibold())
            .style(move |theme: &Theme| {
                if is_active {
                    theme::solid_text(colors::LIGHT.card)
                } else {
                    theme::text_muted(theme)
                }
            }),
    )
    .padding([5, 14])
    .style(move |theme: &Theme, status| {
        let palette = colors::palette(theme);
        let is_dark = palette.card == colors::DARK.card;
        let background = if is_active {
            match status {
                button::Status::Pressed => colors::rgb(0x1D, 0x4E, 0xD8),
                button::Status::Hovered => colors::rgb(0x25, 0x63, 0xEB),
                _ => colors::BRAND_BLUE,
            }
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            if is_dark {
                colors::rgba(0xFF, 0xFF, 0xFF, 0.08)
            } else {
                colors::rgba(0xE5, 0xE7, 0xEB, 0.90)
            }
        } else {
            iced::Color::TRANSPARENT
        };
        button::Style {
            snap: false,
            background: Some(iced::Background::Color(background)),
            text_color: if is_active {
                colors::LIGHT.card
            } else {
                palette.muted_text
            },
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: border::radius(999),
            },
            shadow: iced::Shadow::default(),
        }
    })
    .on_press_maybe((!is_active).then_some(message))
    .into()
}
