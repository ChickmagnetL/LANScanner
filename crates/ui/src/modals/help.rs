use std::sync::LazyLock;

use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Fill, Length, Theme, border};
use ssh_core::scanner::{
    Device as ScannerDevice, DeviceIdentityKind, DeviceStatus as ScannerDeviceStatus, DeviceType,
};

use crate::theme::{
    self, colors, fonts,
    icons::{self, FrameSpec, Glyph},
};

const FOOTER_HEIGHT: f32 = 56.0;

static RUSTDESK_HELP_PREVIEW_DEVICES: LazyLock<Vec<ScannerDevice>> = LazyLock::new(|| {
    vec![
        ScannerDevice {
            id: String::from("192.168.31.12"),
            name: String::from("Desktop-12"),
            ip: String::from("192.168.31.12"),
            identity_kind: DeviceIdentityKind::Computer,
            device_type: DeviceType::Desktop,
            status: ScannerDeviceStatus::Ready,
        },
        ScannerDevice {
            id: String::from("192.168.31.28"),
            name: String::from("Server-28"),
            ip: String::from("192.168.31.28"),
            identity_kind: DeviceIdentityKind::Jetson,
            device_type: DeviceType::Server,
            status: ScannerDeviceStatus::Untested,
        },
        ScannerDevice {
            id: String::from("192.168.31.44"),
            name: String::from("Laptop-44"),
            ip: String::from("192.168.31.44"),
            identity_kind: DeviceIdentityKind::Computer,
            device_type: DeviceType::Laptop,
            status: ScannerDeviceStatus::Denied,
        },
    ]
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpGuideSection {
    Basic,
    RustDesk,
}

pub struct HelpGuideProps<Message>
where
    Message: Clone,
{
    pub on_close: Message,
    pub on_open_github: Message,
    pub show_rustdesk_section: bool,
    pub on_show_basic: Message,
    pub on_show_rustdesk: Message,
}

pub fn view<'a, Message>(props: HelpGuideProps<Message>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let HelpGuideProps {
        on_close,
        on_open_github,
        show_rustdesk_section,
        on_show_basic,
        on_show_rustdesk,
    } = props;
    let active_section = if show_rustdesk_section {
        HelpGuideSection::RustDesk
    } else {
        HelpGuideSection::Basic
    };

    let steps: Element<'a, Message> = match active_section {
        HelpGuideSection::Basic => basic_steps(),
        HelpGuideSection::RustDesk => rustdesk_steps(on_show_rustdesk.clone()),
    };

    let header = container(
        row![
            Space::new().width(Length::Fill),
            section_switcher(
                active_section,
                on_show_basic.clone(),
                on_show_rustdesk.clone(),
            ),
            Space::new().width(Length::Fill),
        ]
        .align_y(Alignment::Center),
    )
    .padding([11, 14]);

    let divider = container(Space::new().height(1.0))
        .width(Fill)
        .style(theme::styles::titlebar_divider);

    let body = container(
        scrollable(steps)
            .height(Fill)
            .style(theme::styles::custom_scrollbar),
    )
    .height(Fill)
    .padding([16, 28]);

    let footer_divider = container(Space::new().height(1.0))
        .width(Fill)
        .style(theme::styles::titlebar_divider);

    column![header, divider, body, footer_divider, footer(on_close, on_open_github)]
        .width(Fill)
        .height(Fill)
        .into()
}

fn basic_steps<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    column![
        step_card(1, step_one_description(), scan_preview()),
        step_card(2, step_two_description(), credential_preview()),
        step_card(3, step_three_description(), result_list_preview()),
        step_card(4, step_four_description(), connect_preview()),
    ]
    .spacing(28)
    .padding(iced::Padding {
        top: 0.0,
        right: 12.0,
        bottom: 0.0,
        left: 0.0,
    })
    .width(Fill)
    .into()
}

fn rustdesk_steps<'a, Message>(preview_action: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    column![
        rustdesk_intro(),
        step_card(
            1,
            rustdesk_step_one_description(),
            rustdesk_credential_preview(preview_action.clone()),
        ),
        step_card(
            2,
            rustdesk_step_two_description(),
            rustdesk_quick_connect_preview(preview_action.clone()),
        ),
        step_card_text_only(3, rustdesk_step_three_description(),),
        rustdesk_troubleshooting_block(),
    ]
    .spacing(24)
    .padding(iced::Padding {
        top: 0.0,
        right: 12.0,
        bottom: 0.0,
        left: 0.0,
    })
    .width(Fill)
    .into()
}

fn section_switcher<'a, Message>(
    active: HelpGuideSection,
    on_show_basic: Message,
    on_show_rustdesk: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        row![
            section_button("基础使用", active == HelpGuideSection::Basic, on_show_basic,),
            section_button(
                "RustDesk 使用",
                active == HelpGuideSection::RustDesk,
                on_show_rustdesk,
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

fn section_button<'a, Message>(
    label: &'static str,
    is_active: bool,
    on_press: Message,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    button(
        text(label)
            .size(12)
            .font(fonts::semibold())
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
    .on_press_maybe((!is_active).then_some(on_press))
    .into()
}

fn step_card<'a, Message>(
    index: u8,
    description: Element<'a, Message>,
    preview: Element<'a, Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    row![
        step_badge(index),
        column![
            container(description).width(Fill),
            container(container(preview).width(Fill).max_width(338.0),)
                .width(Fill)
                .center_x(Fill),
        ]
        .spacing(12)
        .width(Fill),
    ]
    .spacing(16)
    .align_y(Alignment::Start)
    .width(Fill)
    .into()
}

fn step_card_text_only<'a, Message>(
    index: u8,
    description: Element<'a, Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    row![step_badge(index), container(description).width(Fill),]
        .spacing(16)
        .align_y(Alignment::Start)
        .width(Fill)
        .into()
}

fn step_badge<'a, Message>(index: u8) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        text(format!("{index}"))
            .font(fonts::semibold())
            .size(12)
            .style(|_| theme::solid_text(colors::BRAND_BLUE)),
    )
    .width(24)
    .height(24)
    .center_x(Length::Fixed(24.0))
    .center_y(Length::Fixed(24.0))
    .style(|theme: &Theme| {
        let is_dark = colors::palette(theme).card == colors::DARK.card;
        container::Style::default()
            .background(if is_dark {
                colors::rgba(0x3B, 0x82, 0xF6, 0.25)
            } else {
                colors::rgba(0x3B, 0x82, 0xF6, 0.12)
            })
            .border(iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: border::radius(999),
            })
    })
    .into()
}

// ── Step descriptions ────────────────────────────────────────────────────────

fn step_one_description<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    description_text(
        "在左侧选择要扫描的网卡（如 Wi-Fi 或以太网），点击\u{201c}开始扫描\u{201d}发现局域网内的设备。",
    )
}

fn step_two_description<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    column![
        description_text("在\u{201c}SSH 登录凭证\u{201d}区域，可手动填写用户名和密码，或在列表中选择已保存的凭证。"),
        description_text("如果没有填写凭证，则默认不验证；填写后扫描时自动检测。"),
    ]
    .spacing(2)
    .into()
}

fn step_three_description<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    description_text(
        "扫描完成后，右侧\u{201c}扫描结果\u{201d}会显示全部在线设备，这时状态全为\u{201c}未检测\u{201d}。",
    )
}

fn step_four_description<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    description_text(
        "检测完成后，设备显示\u{201c}就绪\u{201d}或\u{201c}拒绝\u{201d}。点击\u{201c}就绪\u{201d}设备，即可在右侧面板一键连接。",
    )
}

fn description_text<'a, Message>(value: &'static str) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        text(value)
            .width(Fill)
            .size(13)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .style(|theme: &Theme| theme::text_muted(theme)),
    )
    .width(Fill)
    .into()
}

fn rustdesk_intro<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        column![
            text("RustDesk 特性")
                .size(13)
                .font(fonts::semibold())
                .style(|theme: &Theme| theme::text_primary(theme)),
            rustdesk_supporting_text("RustDesk 是开源远程桌面工具，通常比传统 VNC 更快，并支持文件传输。"),
            rustdesk_supporting_text(
                "要通过本应用做 IP 直连，目标设备需先自行部署并运行 RustDesk，并在设置中启用 Direct IP。"
            ),
        ]
        .spacing(6),
    )
    .width(Fill)
    .padding([12, 14])
    .style(|theme: &Theme| {
        let palette = colors::palette(theme);
        container::Style::default()
            .background(palette.input)
            .border(iced::Border {
                color: palette.border,
                width: 1.0,
                radius: border::radius(12),
            })
    })
    .into()
}

fn rustdesk_step_one_description<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    column![
        rustdesk_body_text(
            "先在“SSH 登录凭证”卡片里打开“RustDesk 凭证（可选）”，会出现“RustDesk 密码（可选）”输入框。"
        ),
        rustdesk_body_text(
            "这是通过 IP 直连时使用的可选项：填了会在启动 RustDesk 时自动带上密码；不填也能继续连接，后续在客户端里手动输入即可。"
        ),
        rustdesk_body_text(
            "RustDesk 连接不需要用户名。"
        ),
    ]
    .spacing(4)
    .width(Fill)
    .into()
}

fn rustdesk_step_two_description<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    column![
        rustdesk_body_text("在扫描结果里点击目标 IP，然后点击 RustDesk 连接。"),
        rustdesk_body_text(
            "预览使用的是右侧真实快速连接区，只显示当前选中设备信息与 RustDesk 按钮。"
        ),
    ]
    .spacing(4)
    .width(Fill)
    .into()
}

fn rustdesk_step_three_description<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    column![
        rustdesk_body_text("点击后应用会自动启动 RustDesk 客户端，并尝试连接到该目标 IP。"),
        rustdesk_body_text(
            "若已填写 RustDesk 密码，会自动带入；若未填写，则在客户端内手动输入密码后进入桌面。"
        ),
    ]
    .spacing(4)
    .width(Fill)
    .into()
}

fn rustdesk_body_text<'a, Message>(value: &'static str) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        text(value)
            .width(Fill)
            .size(13)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .style(|theme: &Theme| theme::text_muted(theme)),
    )
    .width(Fill)
    .into()
}

fn rustdesk_supporting_text<'a, Message>(value: &'static str) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    rustdesk_body_text(value)
}

// ── Step 1: Scan preview ─────────────────────────────────────────────────────

fn scan_preview<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(mini_card(
        column![
            // Title row
            row![
                icons::centered(Glyph::Network, 18.0, 14.0, colors::rgb(0x1F, 0x29, 0x37)),
                text("扫描网络")
                    .size(13)
                    .font(fonts::semibold())
                    .style(|theme: &Theme| theme::text_primary(theme)),
                Space::new().width(Fill),
                icons::centered(Glyph::Refresh, 16.0, 14.0, colors::rgb(0x9C, 0xA3, 0xAF)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            // Network dropdown
            input_row_with_icon(Glyph::Wifi, "Wi-Fi (Home) (192.168.1.0/24)"),
            // Scan button
            mini_blue_button("开始扫描"),
        ]
        .spacing(10),
    ))
    .width(Fill)
    .into()
}

// ── Step 2: Credential preview ───────────────────────────────────────────────

fn credential_preview<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(mini_card(
        column![
            // Title row
            row![
                icons::centered(Glyph::Lock, 18.0, 16.0, colors::rgb(0x1F, 0x29, 0x37)),
                text("SSH 登录凭证")
                    .size(13)
                    .font(fonts::semibold())
                    .style(|theme: &Theme| theme::text_primary(theme)),
                Space::new().width(Fill),
                text("管理")
                    .size(12)
                    .font(fonts::semibold())
                    .style(|_| theme::solid_text(colors::BRAND_BLUE)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            // Username selector
            input_row_selector("root"),
            // Password field
            input_row_text("\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"),
        ]
        .spacing(8),
    ))
    .width(Fill)
    .into()
}

fn rustdesk_credential_preview<'a, Message>(preview_action: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(mini_card(
        column![
            row![
                text("RustDesk凭证（可选）")
                    .size(13)
                    .font(fonts::semibold())
                    .style(|theme: &Theme| theme::text_primary(theme)),
                Space::new().width(Fill),
                crate::widgets::toggle::view(true, Some(preview_action)),
            ]
            .align_y(Alignment::Center),
            input_row_text("RustDesk 密码（可选）"),
        ]
        .spacing(10),
    ))
    .width(Fill)
    .into()
}

fn device_type_icon<'a, Message>(device_type: DeviceType) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let glyph = match device_type {
        DeviceType::Laptop => Glyph::Laptop,
        DeviceType::Server => Glyph::Server,
        DeviceType::Desktop => Glyph::Desktop,
    };

    icons::centered(glyph, 20.0, 14.0, colors::rgb(0x6B, 0x72, 0x80))
}

fn rustdesk_device_identity_icon<'a, Message>(device_type: DeviceType) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(device_type_icon(device_type))
        .width(40)
        .height(40)
        .center_x(Length::Fixed(40.0))
        .center_y(Length::Fixed(40.0))
        .style(|theme: &Theme| {
            let palette = colors::palette(theme);
            container::Style::default()
                .background(palette.input)
                .border(iced::Border {
                    color: palette.border,
                    width: 1.0,
                    radius: border::radius(12),
                })
        })
        .into()
}

fn rustdesk_launcher_preview<'a, Message>(on_press: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let accent = colors::rgb(0x4F, 0x46, 0xE5);
    let accent_soft = colors::rgba(0x4F, 0x46, 0xE5, 0.10);
    let accent_border = colors::rgba(0x4F, 0x46, 0xE5, 0.28);

    button(
        row![
            icons::framed(
                Glyph::Desktop,
                FrameSpec {
                    width: 36.0,
                    height: 36.0,
                    icon_size: 18.0,
                    tone: accent,
                    background: accent_soft,
                    border_color: accent_border,
                    radius: 10.0,
                },
            ),
            text("RustDesk")
                .size(14)
                .font(fonts::semibold())
                .style(|theme: &Theme| theme::text_primary(theme))
                .width(Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Fill),
    )
    .width(Fill)
    .padding([13.0, 14.0])
    .style(move |theme: &Theme, status| {
        theme::styles::connect_button(
            theme,
            status,
            accent,
            theme::styles::ConnectButtonState::Available,
        )
    })
    .on_press(on_press)
    .into()
}

fn rustdesk_quick_connect_preview<'a, Message>(preview_action: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let selected_device = RUSTDESK_HELP_PREVIEW_DEVICES
        .iter()
        .find(|device| device.status == ScannerDeviceStatus::Ready)
        .or_else(|| RUSTDESK_HELP_PREVIEW_DEVICES.first())
        .expect("RustDesk help preview requires at least one sample device");

    container(mini_card(
        column![
            row![
                rustdesk_device_identity_icon(selected_device.device_type),
                column![
                    text(selected_device.name.as_str())
                        .size(13)
                        .font(fonts::semibold())
                        .style(|theme: &Theme| theme::text_primary(theme)),
                    text(selected_device.ip.as_str())
                        .size(12)
                        .style(|theme: &Theme| theme::text_muted(theme)),
                ]
                .spacing(2)
                .width(Fill),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .width(Fill),
            container(Space::new().width(Fill).height(1.0)).style(|theme: &Theme| {
                let palette = colors::palette(theme);
                container::Style::default().background(palette.border)
            }),
            rustdesk_launcher_preview(preview_action),
        ]
        .spacing(14),
    ))
    .width(Fill)
    .max_width(338.0)
    .into()
}

fn rustdesk_troubleshooting_block<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        column![
            container(Space::new().height(1.0))
                .width(Fill)
                .style(theme::styles::titlebar_divider),
            column![
                text("可能遇到的问题")
                    .size(14)
                    .font(fonts::semibold())
                    .style(|theme: &Theme| theme::text_primary(theme)),
                rustdesk_supporting_text("若连接失败，可按以下方向依次排查。"),
            ]
            .spacing(4),
            container(
                column![
                    rustdesk_failure_hint("目标设备未部署 RustDesk，或服务未运行。"),
                    rustdesk_failure_hint("目标设备默认端口 21118 不可达。"),
                    rustdesk_failure_hint("目标设备未在 RustDesk 设置中启用 Direct IP。"),
                    rustdesk_failure_hint(
                        "部分无显示输出设备需配置虚拟显示器（dummy display）后再连接。"
                    ),
                ]
                .spacing(10),
            )
            .width(Fill)
            .padding([12, 14])
            .style(|theme: &Theme| {
                let palette = colors::palette(theme);
                container::Style::default()
                    .background(palette.input)
                    .border(iced::Border {
                        color: palette.border,
                        width: 1.0,
                        radius: border::radius(12),
                    })
            }),
        ]
        .spacing(12),
    )
    .width(Fill)
    .padding(iced::Padding {
        top: 8.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    })
    .into()
}

fn rustdesk_failure_hint<'a, Message>(value: &'static str) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    row![
        text("•")
            .size(13)
            .font(fonts::semibold())
            .style(|theme: &Theme| theme::text_muted(theme))
            .width(Length::Fixed(10.0)),
        container(
            text(value)
                .size(13)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                .style(|theme: &Theme| theme::text_muted(theme)),
        )
        .width(Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Start)
    .into()
}

// ── Step 3: Result list preview ───────────────────────────────────────────────

fn result_list_preview<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(mini_card_with_header(
        "扫描结果",
        column![
            result_device_row(
                Glyph::Desktop,
                "Windows-Desktop",
                "192.168.1.105",
                DeviceStatus::Untested,
                false
            ),
            result_device_row(
                Glyph::Server,
                "Ubuntu-Server",
                "192.168.1.42",
                DeviceStatus::Untested,
                false
            ),
            result_device_row(
                Glyph::Desktop,
                "MacBook-Pro-M2",
                "192.168.1.10",
                DeviceStatus::Untested,
                false
            ),
        ]
        .spacing(2)
        .into(),
    ))
    .width(Fill)
    .into()
}

// ── Step 4: Connect preview ───────────────────────────────────────────────────

fn connect_preview<'a, Message>() -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(mini_card_with_header(
        "扫描结果",
        row![
            // Left: simplified device list (icon + name + status dot, no IP)
            column![
                connect_device_row(Glyph::Desktop, "Win-PC", DeviceStatus::Ready, true),
                connect_device_row(Glyph::Server, "Ubuntu", DeviceStatus::Denied, false),
                connect_device_row(Glyph::Desktop, "MacBook", DeviceStatus::Ready, false),
            ]
            .spacing(3)
            .width(Length::FillPortion(1)),
            // Vertical divider
            container(Space::new().width(1.0))
                .height(Length::Fixed(108.0))
                .style(theme::styles::titlebar_divider),
            // Right: quick connect panel
            container(
                column![
                    container(icons::centered(
                        Glyph::Desktop,
                        30.0,
                        14.0,
                        colors::BRAND_BLUE
                    ),)
                    .width(30)
                    .height(30)
                    .center_x(Length::Fixed(30.0))
                    .center_y(Length::Fixed(30.0))
                    .style(|theme: &Theme| {
                        let palette = colors::palette(theme);
                        container::Style::default()
                            .background(palette.card)
                            .border(iced::Border {
                                color: palette.border,
                                width: 1.0,
                                radius: border::radius(8),
                            })
                    }),
                    text("Win-PC")
                        .size(10)
                        .font(fonts::semibold())
                        .style(|theme: &Theme| theme::text_primary(theme)),
                    column![
                        tool_chip(Glyph::Code, "VS Code", colors::rgb(0x3B, 0x82, 0xF6)),
                        tool_chip(Glyph::Docker, "Docker", colors::rgb(0x06, 0xB6, 0xD4)),
                        tool_chip(Glyph::Terminal, "Shell", colors::rgb(0xF9, 0x73, 0x16)),
                    ]
                    .spacing(3)
                    .width(Fill),
                ]
                .spacing(4)
                .align_x(Alignment::Center),
            )
            .width(Length::FillPortion(1))
            .padding([4, 8]),
        ]
        .spacing(0)
        .into(),
    ))
    .width(Fill)
    .into()
}

// ── Shared mini-UI primitives ─────────────────────────────────────────────────

fn mini_card<'a, Message>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(content.into())
        .width(Fill)
        .padding([14, 16])
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

fn mini_card_with_header<'a, Message>(
    title: &'static str,
    body: Element<'a, Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        column![
            // Header bar
            container(
                text(title)
                    .size(12)
                    .font(fonts::semibold())
                    .style(|theme: &Theme| theme::text_primary(theme)),
            )
            .width(Fill)
            .padding([10, 14]),
            // Bottom separator
            container(Space::new().height(1.0))
                .width(Fill)
                .style(theme::styles::titlebar_divider),
            // Body
            container(body).width(Fill).padding([6, 8]),
        ]
        .spacing(0),
    )
    .width(Fill)
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

fn input_row_with_icon<'a, Message>(icon: Glyph, label: &'static str) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        row![
            icons::centered(icon, 16.0, 14.0, colors::rgb(0x6B, 0x72, 0x80)),
            text(label)
                .size(12)
                .style(|theme: &Theme| theme::text_primary(theme))
                .width(Fill),
            icons::centered(
                Glyph::ChevronDown,
                16.0,
                13.0,
                colors::rgb(0x9C, 0xA3, 0xAF)
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([9, 12])
    .style(|theme: &Theme| {
        let palette = colors::palette(theme);
        container::Style::default()
            .background(palette.input)
            .border(iced::Border {
                color: palette.border,
                width: 1.0,
                radius: border::radius(10),
            })
    })
    .into()
}

fn input_row_selector<'a, Message>(label: &'static str) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        row![
            text(label)
                .size(12)
                .style(|theme: &Theme| theme::text_primary(theme))
                .width(Fill),
            icons::centered(
                Glyph::ChevronDown,
                16.0,
                13.0,
                colors::rgb(0x9C, 0xA3, 0xAF)
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([9, 12])
    .style(|theme: &Theme| {
        let palette = colors::palette(theme);
        container::Style::default()
            .background(palette.input)
            .border(iced::Border {
                color: palette.border,
                width: 1.0,
                radius: border::radius(10),
            })
    })
    .into()
}

fn input_row_text<'a, Message>(label: &'static str) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        text(label)
            .size(12)
            .style(|theme: &Theme| theme::text_muted(theme))
            .width(Fill),
    )
    .width(Fill)
    .padding([9, 12])
    .style(|theme: &Theme| {
        let palette = colors::palette(theme);
        container::Style::default()
            .background(palette.input)
            .border(iced::Border {
                color: palette.border,
                width: 1.0,
                radius: border::radius(10),
            })
    })
    .into()
}

fn mini_blue_button<'a, Message>(label: &'static str) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        text(label)
            .size(13)
            .font(fonts::semibold())
            .style(|_| theme::solid_text(colors::LIGHT.card)),
    )
    .width(Fill)
    .padding([9, 0])
    .center_x(Fill)
    .style(|_| {
        container::Style::default()
            .background(colors::BRAND_BLUE)
            .border(iced::Border {
                color: colors::BRAND_BLUE,
                width: 1.0,
                radius: border::radius(10),
            })
    })
    .into()
}

#[derive(Clone, Copy)]
enum DeviceStatus {
    Untested,
    Ready,
    Denied,
}

fn result_device_row<'a, Message>(
    icon: Glyph,
    name: &'static str,
    ip: &'static str,
    status: DeviceStatus,
    selected: bool,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let icon_color = if selected {
        colors::BRAND_BLUE
    } else {
        colors::rgb(0x6B, 0x72, 0x80)
    };

    let row_content = row![
        // Icon box
        container(icons::centered(icon, 28.0, 14.0, icon_color))
            .width(28)
            .height(28)
            .center_x(Length::Fixed(28.0))
            .center_y(Length::Fixed(28.0))
            .style(move |theme: &Theme| {
                let palette = colors::palette(theme);
                if selected {
                    container::Style::default()
                        .background(palette.card)
                        .border(iced::Border {
                            color: palette.border,
                            width: 1.0,
                            radius: border::radius(8),
                        })
                } else {
                    container::Style::default()
                        .background(palette.input)
                        .border(iced::Border {
                            color: iced::Color::TRANSPARENT,
                            width: 0.0,
                            radius: border::radius(8),
                        })
                }
            }),
        // Name + IP
        column![
            text(name)
                .size(11)
                .font(fonts::semibold())
                .style(move |theme: &Theme| if selected {
                    theme::solid_text(colors::BRAND_BLUE)
                } else {
                    theme::text_primary(theme)
                }),
            text(ip).size(10).style(move |theme: &Theme| if selected {
                theme::solid_text(colors::rgba(0x3B, 0x82, 0xF6, 0.70))
            } else {
                theme::text_muted(theme)
            }),
        ]
        .spacing(1)
        .width(Fill),
        // Status badge
        status_badge(status),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if selected {
        container(row_content)
            .width(Fill)
            .padding([4, 6])
            .style(|_| {
                container::Style::default()
                    .background(colors::rgba(0x3B, 0x82, 0xF6, 0.08))
                    .border(iced::Border {
                        color: colors::rgba(0x3B, 0x82, 0xF6, 0.20),
                        width: 1.0,
                        radius: border::radius(10),
                    })
            })
            .into()
    } else {
        container(row_content).width(Fill).padding([4, 6]).into()
    }
}

fn status_badge<'a, Message>(status: DeviceStatus) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let (label, fg, bg, border_color) = match status {
        DeviceStatus::Untested => (
            "未检测",
            colors::rgb(0x9C, 0xA3, 0xAF),
            colors::rgba(0x9C, 0xA3, 0xAF, 0.08),
            colors::rgba(0x9C, 0xA3, 0xAF, 0.24),
        ),
        DeviceStatus::Ready => (
            "\u{2713} 就绪",
            colors::rgb(0x10, 0xB9, 0x81),
            colors::rgba(0x10, 0xB9, 0x81, 0.08),
            colors::rgba(0x10, 0xB9, 0x81, 0.30),
        ),
        DeviceStatus::Denied => (
            "\u{2717} 拒绝",
            colors::rgb(0xEF, 0x44, 0x44),
            colors::rgba(0xEF, 0x44, 0x44, 0.08),
            colors::rgba(0xEF, 0x44, 0x44, 0.30),
        ),
    };

    container(text(label).size(10).style(move |_| theme::solid_text(fg)))
        .padding([3, 6])
        .style(move |_| {
            container::Style::default()
                .background(bg)
                .border(iced::Border {
                    color: border_color,
                    width: 1.0,
                    radius: border::radius(999),
                })
        })
        .into()
}

fn tool_chip<'a, Message>(
    icon: Glyph,
    name: &'static str,
    icon_color: Color,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        row![
            container(icons::centered(icon, 22.0, 12.0, icon_color))
                .width(22)
                .height(22)
                .center_x(Length::Fixed(22.0))
                .center_y(Length::Fixed(22.0))
                .style(move |_| container::Style::default()
                    .background(Color {
                        a: 0.12,
                        ..icon_color
                    })
                    .border(iced::Border {
                        color: iced::Color::TRANSPARENT,
                        width: 0.0,
                        radius: border::radius(6),
                    })),
            text(name)
                .size(10)
                .font(fonts::semibold())
                .style(|theme: &Theme| theme::text_primary(theme)),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([4, 6])
    .style(|theme: &Theme| {
        let palette = colors::palette(theme);
        container::Style::default()
            .background(palette.card)
            .border(iced::Border {
                color: palette.border,
                width: 1.0,
                radius: border::radius(7),
            })
    })
    .into()
}

// ── Step 4 simplified device row (no IP, compact) ────────────────────────────

fn connect_device_row<'a, Message>(
    icon: Glyph,
    name: &'static str,
    status: DeviceStatus,
    selected: bool,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let icon_color = if selected {
        colors::BRAND_BLUE
    } else {
        colors::rgb(0x6B, 0x72, 0x80)
    };

    let (status_color, status_label) = match status {
        DeviceStatus::Untested => (colors::rgb(0x9C, 0xA3, 0xAF), ""),
        DeviceStatus::Ready => (colors::rgb(0x10, 0xB9, 0x81), "\u{2713}"),
        DeviceStatus::Denied => (colors::rgb(0xEF, 0x44, 0x44), "\u{2717}"),
    };

    let row_content = row![
        container(icons::centered(icon, 24.0, 12.0, icon_color))
            .width(24)
            .height(24)
            .center_x(Length::Fixed(24.0))
            .center_y(Length::Fixed(24.0))
            .style(move |theme: &Theme| {
                let palette = colors::palette(theme);
                if selected {
                    container::Style::default()
                        .background(palette.card)
                        .border(iced::Border {
                            color: palette.border,
                            width: 1.0,
                            radius: border::radius(6),
                        })
                } else {
                    container::Style::default()
                        .background(palette.input)
                        .border(iced::Border {
                            color: iced::Color::TRANSPARENT,
                            width: 0.0,
                            radius: border::radius(6),
                        })
                }
            }),
        text(name)
            .size(10)
            .font(fonts::semibold())
            .width(Fill)
            .style(move |theme: &Theme| if selected {
                theme::solid_text(colors::BRAND_BLUE)
            } else {
                theme::text_primary(theme)
            }),
        text(status_label)
            .size(10)
            .style(move |_| theme::solid_text(status_color)),
    ]
    .spacing(5)
    .align_y(Alignment::Center);

    if selected {
        container(row_content)
            .width(Fill)
            .padding([3, 4])
            .style(|_| {
                container::Style::default()
                    .background(colors::rgba(0x3B, 0x82, 0xF6, 0.08))
                    .border(iced::Border {
                        color: colors::rgba(0x3B, 0x82, 0xF6, 0.20),
                        width: 1.0,
                        radius: border::radius(7),
                    })
            })
            .into()
    } else {
        container(row_content).width(Fill).padding([3, 4]).into()
    }
}

// ── Footer ───────────────────────────────────────────────────────────────────

fn footer<'a, Message>(on_close: Message, on_open_github: Message) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    container(
        row![
            button(icons::centered(
                Glyph::GitHub,
                20.0,
                18.0,
                colors::rgb(0x6B, 0x72, 0x80),
            ))
            .padding([4, 4])
            .style(|_, status| {
                let icon_color = match status {
                    button::Status::Hovered | button::Status::Pressed => {
                        colors::rgb(0x37, 0x41, 0x51)
                    }
                    _ => colors::rgb(0x6B, 0x72, 0x80),
                };
                button::Style {
                    snap: false,
                    background: Some(iced::Background::Color(iced::Color::TRANSPARENT)),
                    text_color: icon_color,
                    border: iced::Border::default(),
                    shadow: iced::Shadow::default(),
                }
            })
            .on_press(on_open_github),
            Space::new().width(Fill),
            button(
                text("开始使用")
                    .font(fonts::semibold())
                    .size(13)
                    .style(|_| theme::solid_text(colors::LIGHT.card)),
            )
            .padding([8, 20])
            .style(crate::theme::styles::primary_button)
            .on_press(on_close),
        ]
        .align_y(Alignment::Center),
    )
    .height(Length::Fixed(FOOTER_HEIGHT))
    .width(Fill)
    .padding([10, 20])
    .style(|theme: &Theme| {
        let palette = colors::palette(theme);
        container::Style::default()
            .background(palette.input)
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
    })
    .into()
}
