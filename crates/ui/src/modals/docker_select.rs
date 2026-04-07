use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Fill, Length, Theme, border};
use ssh_core::docker::Container;

use crate::theme::{
    self, colors, fonts,
    icons::{self, Glyph},
};

const SECTION_SPACING: u16 = 16;
const FOOTER_HEIGHT: f32 = 68.0;

pub struct DockerSelectProps<'a, Message>
where
    Message: Clone + 'a,
{
    pub containers: &'a [Container],
    pub selected_container_id: Option<&'a str>,
    pub on_select: fn(String) -> Message,
    pub on_close: Message,
    pub on_connect: Option<Message>,
}

pub fn view<'a, Message>(props: DockerSelectProps<'a, Message>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let DockerSelectProps {
        containers,
        selected_container_id,
        on_select,
        on_close,
        on_connect,
    } = props;

    let items = containers
        .iter()
        .fold(column!().spacing(10), |column, container_item| {
            let is_selected = selected_container_id == Some(container_item.id.as_str());
            column.push(container_button(container_item, is_selected, on_select))
        });

    let header = container(
        row![
            text("选择 Docker 容器")
                .font(fonts::semibold())
                .size(16)
                .style(|theme: &Theme| theme::text_primary(theme)),
            Space::new().width(Length::Fill),
            close_button(on_close.clone()),
        ]
        .align_y(Alignment::Center),
    )
    .padding([20, 24]);

    let divider = container(Space::new().height(1.0))
        .width(Fill)
        .style(theme::styles::titlebar_divider);

    let body = container(
        column![
            text("将通过 VS Code Dev Containers 连接到目标容器。")
                .size(13)
                .style(|theme: &Theme| theme::text_muted(theme)),
            scrollable(items)
                .height(Length::Fixed(260.0))
                .style(theme::styles::custom_scrollbar),
            row![
                button(
                    text("取消")
                        .font(fonts::semibold())
                        .size(13)
                        .style(|theme: &Theme| theme::text_primary(theme)),
                )
                .padding([12, 20])
                .style(|theme: &Theme, status| cancel_button_style(theme, status))
                .on_press(on_close),
                Space::new().width(Length::Fill),
                button(
                    row![
                        icons::centered(Glyph::Code, 14.0, 12.0, colors::LIGHT.card),
                        text("连接")
                            .font(fonts::semibold())
                            .size(13)
                            .style(|_| theme::solid_text(colors::LIGHT.card)),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding([12, 24])
                .style(crate::theme::styles::primary_button)
                .on_press_maybe(on_connect),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ]
        .spacing(20),
    )
    .padding(24);

    column![header, divider, body].width(Fill).into()
}

fn container_button<'a, Message>(
    container_item: &'a Container,
    is_selected: bool,
    on_select: fn(String) -> Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    button(
        container(
            row![
                row![
                    icons::framed(
                        Glyph::Docker,
                        crate::theme::icons::FrameSpec {
                            width: 40.0,
                            height: 40.0,
                            icon_size: 13.0,
                            tone: colors::rgb(0x3B, 0x82, 0xF6),
                            background: colors::rgba(0x3B, 0x82, 0xF6, 0.10),
                            border_color: colors::rgba(0x3B, 0x82, 0xF6, 0.24),
                            radius: 12.0,
                        },
                    ),
                    column![
                        text(&container_item.name)
                            .font(fonts::semibold())
                            .size(14)
                            .style(|theme: &Theme| theme::text_primary(theme)),
                        text(&container_item.image)
                            .size(12)
                            .style(|theme: &Theme| theme::text_muted(theme)),
                    ]
                    .spacing(4)
                    .width(Fill),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .width(Fill),
                status_badge(container_item),
            ]
            .align_y(Alignment::Center),
        )
        .padding([12, 14]),
    )
    .width(Fill)
    .style(move |theme: &Theme, status| list_button_style(theme, status, is_selected))
    .on_press(on_select(container_item.id.clone()))
    .into()
}

fn status_badge<'a, Message>(container_item: &Container) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let (label, tone, background) = if container_item.is_running {
        (
            "running",
            colors::rgb(0x22, 0xC5, 0x5E),
            colors::rgba(0x22, 0xC5, 0x5E, 0.12),
        )
    } else {
        (
            "stopped",
            colors::rgb(0x6B, 0x72, 0x80),
            colors::rgba(0x9C, 0xA3, 0xAF, 0.14),
        )
    };

    container(
        text(label)
            .font(fonts::body())
            .size(11)
            .style(move |_| theme::solid_text(tone)),
    )
    .padding([5, 10])
    .style(move |_| {
        container::Style::default()
            .background(background)
            .border(iced::Border {
                color: colors::rgba(
                    (tone.r * 255.0).round() as u8,
                    (tone.g * 255.0).round() as u8,
                    (tone.b * 255.0).round() as u8,
                    0.24,
                ),
                width: 1.0,
                radius: border::radius(999),
            })
    })
    .into()
}

fn list_button_style(theme: &Theme, status: button::Status, is_selected: bool) -> button::Style {
    let palette = colors::palette(theme);
    let background = match (is_selected, status) {
        (true, _) => colors::rgba(0x3B, 0x82, 0xF6, 0.10),
        (_, button::Status::Hovered | button::Status::Pressed) => {
            colors::rgba(0x3B, 0x82, 0xF6, 0.06)
        }
        _ => palette.input,
    };
    let border_color = if is_selected {
        colors::rgba(0x3B, 0x82, 0xF6, 0.24)
    } else {
        palette.border
    };

    button::Style {
        snap: false,
        background: Some(iced::Background::Color(background)),
        text_color: palette.text,
        border: iced::Border {
            color: border_color,
            width: 1.0,
            radius: border::radius(14),
        },
        shadow: iced::Shadow::default(),
    }
}

fn close_button<'a, Message>(on_press: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    button(
        container(icons::centered(
            Glyph::Close,
            14.0,
            12.0,
            colors::rgb(0x6B, 0x72, 0x80),
        ))
        .width(30)
        .height(30)
        .center_x(Length::Fixed(30.0))
        .center_y(Length::Fixed(30.0)),
    )
    .style(crate::theme::styles::titlebar_button)
    .on_press(on_press)
    .into()
}

fn cancel_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = colors::palette(theme);
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => colors::rgba(0x9C, 0xA3, 0xAF, 0.12),
        _ => palette.card,
    };

    button::Style {
        snap: false,
        background: Some(iced::Background::Color(background)),
        text_color: palette.text,
        border: iced::Border {
            color: palette.border,
            width: 1.0,
            radius: border::radius(12),
        },
        shadow: iced::Shadow::default(),
    }
}
