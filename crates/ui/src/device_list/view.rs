use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Fill, Length, Theme, border};
use ssh_core::scanner::{Device, DeviceStatus, DeviceType};

use crate::theme::{
    self, colors, fonts,
    icons::{self, FrameSpec, Glyph},
};

pub enum PlaceholderState {
    Idle,
    RefreshingNetworks {
        spinner_frame: &'static str,
    },
    Scanning {
        spinner_frame: &'static str,
        progress: Option<(usize, usize)>,
    },
    EmptyResults,
}

#[derive(Debug, Clone, Copy)]
enum PlaceholderVisual {
    Glyph(Glyph),
    RotatingRefresh(&'static str),
}

pub fn view<'a, Message>(
    devices: &'a [Device],
    selected_device_id: Option<&'a str>,
    on_select: impl Fn(String) -> Message + Copy + 'a,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    if devices.is_empty() {
        return placeholder(PlaceholderState::EmptyResults);
    }

    let items = devices.iter().fold(
        column!().spacing(4).padding([12.0, 12.0]),
        |column, device| {
            let is_selected = selected_device_id == Some(device.id.as_str());
            let select_message = on_select(device.id.clone());
            let item = button(
                row![
                    row![
                        device_icon(device.device_type, is_selected),
                        column![
                            text(&device.name).font(fonts::semibold()).size(14).style(
                                move |theme: &Theme| {
                                    let palette = colors::palette(theme);

                                    if is_selected {
                                        theme::solid_text(palette.primary)
                                    } else {
                                        theme::text_primary(theme)
                                    }
                                }
                            ),
                            ip_selection_affordance(&device.ip, is_selected),
                        ]
                        .spacing(5)
                        .width(Fill),
                    ]
                    .spacing(12)
                    .width(Fill)
                    .align_y(Alignment::Center),
                    Space::new().width(Length::Shrink),
                    status_badge(device.status),
                ]
                .height(64.0)
                .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding([0.0, 12.0])
            .style(move |theme: &Theme, status| {
                let palette = colors::palette(theme);
                let is_dark = palette.card == colors::DARK.card;

                let background = if is_selected {
                    if is_dark {
                        colors::DARK_SELECTION
                    } else {
                        colors::LIGHT_SELECTION
                    }
                } else {
                    match status {
                        button::Status::Hovered | button::Status::Pressed => {
                            if is_dark {
                                colors::DARK_ROW_HOVER
                            } else {
                                colors::LIGHT_ROW_HOVER
                            }
                        }
                        _ => iced::Color::TRANSPARENT,
                    }
                };

                button::Style {
                    snap: false,
                    background: Some(iced::Background::Color(background)),
                    text_color: palette.text,
                    border: iced::Border {
                        color: if is_selected {
                            if is_dark {
                                colors::rgba(0x3B, 0x82, 0xF6, 0.4)
                            } else {
                                colors::rgba(0x3B, 0x82, 0xF6, 0.2)
                            }
                        } else {
                            iced::Color::TRANSPARENT
                        },
                        width: if is_selected { 1.0 } else { 0.0 },
                        radius: border::radius(12),
                    },
                    shadow: iced::Shadow::default(),
                }
            })
            .on_press(select_message);

            column.push(item)
        },
    );

    scrollable(items)
        .width(Fill)
        .height(Fill)
        .style(theme::styles::custom_scrollbar)
        .into()
}

fn ip_selection_affordance<'a, Message>(ip: &'a str, selected: bool) -> Element<'a, Message>
where
    Message: 'a,
{
    container(
        text(format!("IP: {ip}"))
            .size(12)
            .style(move |theme: &Theme| {
                if selected {
                    theme::solid_text(colors::rgb(0x1D, 0x4E, 0x89))
                } else {
                    theme::text_muted(theme)
                }
            }),
    )
    .padding([4.0, 10.0])
    .style(move |theme: &Theme| {
        let palette = colors::palette(theme);
        let is_dark = palette.card == colors::DARK.card;
        let (background, border_color) = if selected {
            (
                colors::rgba(0x3B, 0x82, 0xF6, if is_dark { 0.24 } else { 0.16 }),
                colors::rgba(0x3B, 0x82, 0xF6, if is_dark { 0.46 } else { 0.30 }),
            )
        } else {
            (palette.input, palette.border)
        };

        container::Style::default()
            .background(background)
            .border(iced::Border {
                color: border_color,
                width: 1.0,
                radius: border::radius(999),
            })
    })
    .into()
}

pub fn placeholder<'a, Message>(state: PlaceholderState) -> Element<'a, Message>
where
    Message: 'a,
{
    container(empty_state_panel(state))
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .padding(18)
        .into()
}

fn empty_state_panel<'a, Message: 'a>(state: PlaceholderState) -> Element<'a, Message> {
    if matches!(state, PlaceholderState::Idle) {
        return idle_state_panel();
    }

    let (visual, chip_label, accent, title, description) = match state {
        PlaceholderState::Idle => unreachable!(),
        PlaceholderState::RefreshingNetworks { spinner_frame } => (
            PlaceholderVisual::RotatingRefresh(spinner_frame),
            "同步网卡",
            colors::rgb(0x3B, 0x82, 0xF6),
            "正在准备扫描上下文",
            String::from("正在读取本机网络接口与目标网段，完成后即可选择可扫描网卡。"),
        ),
        PlaceholderState::Scanning {
            spinner_frame,
            progress,
        } => (
            PlaceholderVisual::RotatingRefresh(spinner_frame),
            "扫描进行中",
            colors::rgb(0x3B, 0x82, 0xF6),
            "结果列表等待回填",
            match progress {
                Some((scanned, total)) if total > 0 => {
                    format!("当前扫描进度 {scanned}/{total}，发现的设备会在这里逐步回到列表。")
                }
                _ => String::from("扫描任务刚刚启动，正在建立目标列表并等待第一批结果返回。"),
            },
        ),
        PlaceholderState::EmptyResults => (
            PlaceholderVisual::Glyph(Glyph::Search),
            "本轮未发现设备",
            colors::rgb(0xEA, 0x58, 0x0C),
            "列表保持为空",
            String::from("当前网段没有发现开放 SSH 端口的设备，可以切换网卡或稍后重新扫描。"),
        ),
    };

    container(
        column![
            status_chip(visual, chip_label, accent),
            empty_state_icon(visual, accent),
            text(title)
                .font(fonts::semibold())
                .size(14)
                .style(|theme: &Theme| theme::text_primary(theme)),
            text(description)
                .size(12)
                .style(|theme: &Theme| theme::text_muted(theme)),
        ]
        .spacing(12)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(236.0))
    .padding([22, 20])
    .style(|theme: &Theme| {
        let palette = colors::palette(theme);

        container::Style::default()
            .background(palette.card)
            .border(iced::Border {
                color: palette.border,
                width: 1.0,
                radius: border::radius(16),
            })
    })
    .into()
}

fn idle_state_panel<'a, Message: 'a>() -> Element<'a, Message> {
    const PANEL_WIDTH: f32 = 208.0;
    const ICON_SIZE: f32 = 58.0;
    const ICON_GLYPH: f32 = 20.0;
    const TITLE_SIZE: f32 = 12.0;

    let tone = colors::rgb(0x8A, 0x93, 0xA1);
    let background = colors::rgba(0x8A, 0x93, 0xA1, 0.08);

    let icon = icons::framed(
        Glyph::Search,
        FrameSpec {
            width: ICON_SIZE,
            height: ICON_SIZE,
            icon_size: ICON_GLYPH,
            tone,
            background,
            border_color: colors::rgba(0x8A, 0x93, 0xA1, 0.12),
            radius: 8.0,
        },
    );

    let panel = container(
        column![
            container(icon).width(Fill).center_x(Fill),
            container(
                text("尚未进行扫描")
                    .font(fonts::semibold())
                    .size(TITLE_SIZE)
                    .style(move |_| theme::solid_text(tone)),
            )
            .width(Fill)
            .center_x(Fill),
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(PANEL_WIDTH))
    .padding([2, 2]);

    container(panel)
        .width(Fill)
        .padding(iced::Padding {
            top: 0.0,
            right: 0.0,
            bottom: 12.0,
            left: 0.0,
        })
        .center_x(Fill)
        .into()
}

fn empty_state_icon<'a, Message: 'a>(
    visual: PlaceholderVisual,
    tone: iced::Color,
) -> Element<'a, Message> {
    let icon = placeholder_visual_centered(visual, tone, 48.0, 16.0);

    container(icon)
        .width(48)
        .height(48)
        .center_x(Length::Fixed(48.0))
        .center_y(Length::Fixed(48.0))
        .style(move |_| {
            container::Style::default()
                .background(colors::rgba(
                    (tone.r * 255.0).round() as u8,
                    (tone.g * 255.0).round() as u8,
                    (tone.b * 255.0).round() as u8,
                    0.12,
                ))
                .border(iced::Border {
                    color: colors::rgba(
                        (tone.r * 255.0).round() as u8,
                        (tone.g * 255.0).round() as u8,
                        (tone.b * 255.0).round() as u8,
                        0.22,
                    ),
                    width: 1.0,
                    radius: border::radius(16),
                })
        })
        .into()
}

fn device_icon<'a, Message: 'a>(device_type: DeviceType, selected: bool) -> Element<'a, Message> {
    let tone = if selected {
        iced::Color::WHITE
    } else {
        colors::rgb(0x6B, 0x72, 0x80)
    };

    container(icons::centered(
        device_type_glyph(device_type),
        40.0,
        14.0,
        tone,
    ))
    .width(40)
    .height(40)
    .center_x(Length::Fixed(40.0))
    .center_y(Length::Fixed(40.0))
    .style(move |theme: &Theme| {
        let palette = colors::palette(theme);
        let (background, border_color) = if selected {
            (palette.primary, palette.primary)
        } else {
            (palette.input, palette.border)
        };

        container::Style::default()
            .background(background)
            .border(iced::Border {
                color: border_color,
                width: 1.0,
                radius: border::radius(12),
            })
    })
    .into()
}

fn status_badge<'a, Message: 'a>(status: DeviceStatus) -> Element<'a, Message> {
    let (label, glyph, tone) = match status {
        DeviceStatus::Untested => ("未检测", Glyph::Pending, colors::rgb(0x9C, 0xA3, 0xAF)),
        DeviceStatus::Ready => (
            "验证成功",
            Glyph::CircleCheck,
            colors::rgb(0x22, 0xC5, 0x5E),
        ),
        DeviceStatus::Denied | DeviceStatus::Error => {
            ("验证失败", Glyph::CircleX, colors::rgb(0xEF, 0x44, 0x44))
        }
    };

    container(
        row![
            container(icons::centered(glyph, 20.0, 14.0, tone))
                .width(22)
                .height(22)
                .center_x(Length::Fixed(22.0))
                .center_y(Length::Fixed(22.0))
                .style(move |_theme: &Theme| {
                    container::Style::default()
                        .background(colors::rgba(
                            (tone.r * 255.0).round() as u8,
                            (tone.g * 255.0).round() as u8,
                            (tone.b * 255.0).round() as u8,
                            0.1,
                        ))
                        .border(iced::Border {
                            color: colors::rgba(
                                (tone.r * 255.0).round() as u8,
                                (tone.g * 255.0).round() as u8,
                                (tone.b * 255.0).round() as u8,
                                0.3,
                            ),
                            width: 1.0,
                            radius: border::radius(999),
                        })
                }),
            text(label)
                .font(fonts::body())
                .size(11)
                .style(move |_| theme::solid_text(tone)),
        ]
        .spacing(7)
        .align_y(Alignment::Center),
    )
    .padding([5, 9])
    .style(move |theme: &Theme| {
        let is_dark = colors::palette(theme).card == colors::DARK.card;
        container::Style::default()
            .background(colors::rgba(
                (tone.r * 255.0).round() as u8,
                (tone.g * 255.0).round() as u8,
                (tone.b * 255.0).round() as u8,
                if is_dark { 0.1 } else { 0.08 },
            ))
            .border(iced::Border {
                color: colors::rgba(
                    (tone.r * 255.0).round() as u8,
                    (tone.g * 255.0).round() as u8,
                    (tone.b * 255.0).round() as u8,
                    0.2,
                ),
                width: 1.0,
                radius: border::radius(999),
            })
    })
    .into()
}

fn status_chip<'a, Message: 'a>(
    visual: PlaceholderVisual,
    label: &'static str,
    tone: iced::Color,
) -> Element<'a, Message> {
    container(
        row![
            container(placeholder_visual_centered(visual, tone, 10.0, 6.5))
                .width(14)
                .height(14)
                .center_x(Length::Fixed(14.0))
                .center_y(Length::Fixed(14.0))
                .style(move |theme| {
                    let is_dark = colors::palette(theme).card == colors::DARK.card;
                    container::Style::default()
                        .background(if is_dark {
                            colors::DARK_ACCENT_SOFT
                        } else {
                            colors::LIGHT_ACCENT_SOFT
                        })
                        .border(iced::Border {
                            color: colors::rgba(
                                (tone.r * 255.0).round() as u8,
                                (tone.g * 255.0).round() as u8,
                                (tone.b * 255.0).round() as u8,
                                0.32,
                            ),
                            width: 1.0,
                            radius: border::radius(999),
                        })
                }),
            text(label)
                .font(fonts::body())
                .size(11)
                .style(move |_| theme::solid_text(tone)),
        ]
        .spacing(7)
        .align_y(Alignment::Center),
    )
    .padding([5, 9])
    .style(move |theme| {
        let is_dark = colors::palette(theme).card == colors::DARK.card;
        container::Style::default()
            .background(if is_dark {
                colors::DARK_ACCENT_SOFT
            } else {
                colors::LIGHT_ACCENT_SOFT
            })
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

fn placeholder_visual_centered<'a, Message: 'a>(
    visual: PlaceholderVisual,
    tone: iced::Color,
    slot: f32,
    size: f32,
) -> Element<'a, Message> {
    match visual {
        PlaceholderVisual::Glyph(glyph) => icons::centered(glyph, slot, size, tone),
        PlaceholderVisual::RotatingRefresh(frame) => {
            icons::rotating_refresh_centered(frame, slot, size, tone)
        }
    }
}

fn device_type_glyph(device_type: DeviceType) -> Glyph {
    match device_type {
        DeviceType::Laptop => Glyph::Laptop,
        DeviceType::Server => Glyph::Server,
        DeviceType::Desktop => Glyph::Desktop,
    }
}
