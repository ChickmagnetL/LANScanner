use iced::widget::{Space, button, column, container, opaque, row, stack, text};
use iced::{Alignment, Element, Fill, Length, Theme, border};
use ssh_core::scanner::{Device, DeviceIdentityKind};

use crate::theme::{
    self, AppLanguage, colors, fonts,
    icons::{self, FrameSpec, Glyph},
};

pub struct SelectedDetailState<'a, Message>
where
    Message: Clone + 'a,
{
    pub device: &'a Device,
    pub status_text: String,
    pub active_launcher_key: Option<&'static str>,
    pub on_shell: Option<Message>,
    pub on_vscode: Option<Message>,
    pub on_vnc: Option<Message>,
    pub on_mobaxterm: Option<Message>,
    pub on_docker: Option<Message>,
    pub on_rustdesk: Option<Message>,
    pub on_close: Option<Message>,
}

pub enum DetailState<'a, Message>
where
    Message: Clone + 'a,
{
    Idle,
    RefreshingNetworks {
        spinner_frame: &'static str,
    },
    Scanning {
        spinner_frame: &'static str,
        progress: Option<(usize, usize)>,
    },
    EmptyResults,
    NoSelection,
    Selected(SelectedDetailState<'a, Message>),
}

const EMPTY_STATE_PANEL_WIDTH: f32 = 284.0;
const EMPTY_STATE_DESCRIPTION_WIDTH: f32 = 240.0;
const EMPTY_STATE_ICON_EDGE: f32 = 68.0;
const QUICK_CONNECT_PANEL_WIDTH: f32 = 320.0;
const QUICK_CONNECT_ICON_EDGE: f32 = 36.0;

pub fn view<'a, Message>(
    state: DetailState<'a, Message>,
    app_language: AppLanguage,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    match state {
        DetailState::Idle => centered_state(
            DetailVisual::Glyph(Glyph::Search),
            DetailVisual::Glyph(Glyph::Pending),
            detail_idle_eyebrow(app_language),
            detail_idle_title(app_language),
            detail_idle_description(app_language),
            colors::rgb(0x3B, 0x82, 0xF6),
            colors::rgba(0x3B, 0x82, 0xF6, 0.10),
        ),
        DetailState::RefreshingNetworks { spinner_frame } => centered_state(
            DetailVisual::RefreshCw {
                frame: spinner_frame,
            },
            DetailVisual::RefreshCw {
                frame: spinner_frame,
            },
            detail_refreshing_eyebrow(app_language),
            detail_refreshing_title(app_language),
            detail_refreshing_description(app_language),
            colors::rgb(0x3B, 0x82, 0xF6),
            colors::rgba(0x3B, 0x82, 0xF6, 0.10),
        ),
        DetailState::Scanning {
            spinner_frame,
            progress,
        } => {
            let description = match progress {
                Some((scanned, total)) if total > 0 => {
                    detail_scanning_progress_description(app_language, scanned, total)
                }
                _ => detail_scanning_loading_description(app_language),
            };

            stack([
                centered_state(
                    DetailVisual::RefreshCw {
                        frame: spinner_frame,
                    },
                    DetailVisual::RefreshCw {
                        frame: spinner_frame,
                    },
                    detail_scanning_eyebrow(app_language),
                    detail_scanning_title(app_language),
                    detail_scanning_description(app_language),
                    colors::rgb(0x3B, 0x82, 0xF6),
                    colors::rgba(0x3B, 0x82, 0xF6, 0.10),
                ),
                opaque(scanning_overlay(spinner_frame, description, app_language)),
            ])
            .width(Fill)
            .height(Fill)
            .into()
        }
        DetailState::EmptyResults => centered_state(
            DetailVisual::Glyph(Glyph::Search),
            DetailVisual::Glyph(Glyph::Search),
            detail_empty_results_eyebrow(app_language),
            detail_empty_results_title(app_language),
            detail_empty_results_description(app_language),
            colors::rgb(0xEA, 0x58, 0x0C),
            colors::rgba(0xEA, 0x58, 0x0C, 0.10),
        ),
        DetailState::NoSelection => centered_state(
            DetailVisual::Glyph(Glyph::Server),
            DetailVisual::Glyph(Glyph::Search),
            detail_no_selection_eyebrow(app_language),
            detail_no_selection_title(app_language),
            detail_no_selection_description(app_language),
            colors::rgb(0x6B, 0x72, 0x80),
            colors::rgba(0x9C, 0xA3, 0xAF, 0.10),
        ),
        DetailState::Selected(state) => selected_device_state(state, app_language),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetailVisual {
    Glyph(Glyph),
    RefreshCw { frame: &'static str },
}

impl DetailVisual {
    fn centered<'a, Message: 'a>(
        self,
        slot: f32,
        size: f32,
        tone: iced::Color,
    ) -> Element<'a, Message> {
        match self {
            Self::Glyph(glyph) => icons::centered(glyph, slot, size, tone),
            Self::RefreshCw { frame } => icons::rotating_refresh_centered(frame, slot, size, tone),
        }
    }
}

fn detail_idle_eyebrow(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "等待开始",
        AppLanguage::English => "Ready To Start",
    }
}

fn detail_idle_title(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "尚未进行扫描",
        AppLanguage::English => "No Scan Yet",
    }
}

fn detail_idle_description(app_language: AppLanguage) -> String {
    match app_language {
        AppLanguage::Chinese => {
            String::from("选择左侧网卡后开始扫描，结果面板会自动联动展示设备详情。")
        }
        AppLanguage::English => String::from(
            "Select a network on the left to begin scanning. Device details will update here automatically.",
        ),
    }
}

fn detail_refreshing_eyebrow(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "同步网卡",
        AppLanguage::English => "Refreshing Networks",
    }
}

fn detail_refreshing_title(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "正在读取网络接口",
        AppLanguage::English => "Reading Network Interfaces",
    }
}

fn detail_refreshing_description(app_language: AppLanguage) -> String {
    match app_language {
        AppLanguage::Chinese => {
            String::from("系统网卡、IP 范围和网络名称正在同步，完成后即可选择网卡开始扫描。")
        }
        AppLanguage::English => String::from(
            "Syncing system interfaces, IP ranges, and network names. You can start scanning once the list is ready.",
        ),
    }
}

fn detail_scanning_eyebrow(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "结果面板已激活",
        AppLanguage::English => "Detail Panel Active",
    }
}

fn detail_scanning_title(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "扫描进行中",
        AppLanguage::English => "Scan In Progress",
    }
}

fn detail_scanning_description(app_language: AppLanguage) -> String {
    match app_language {
        AppLanguage::Chinese => String::from("设备列表与详情区正在等待新的扫描结果。"),
        AppLanguage::English => {
            String::from("The device list and detail panel are waiting for new scan results.")
        }
    }
}

fn detail_scanning_loading_description(app_language: AppLanguage) -> String {
    match app_language {
        AppLanguage::Chinese => String::from("正在初始化扫描任务，请稍候。"),
        AppLanguage::English => String::from("Initializing the scan task. Please wait a moment."),
    }
}

fn detail_scanning_progress_description(
    app_language: AppLanguage,
    scanned: usize,
    total: usize,
) -> String {
    match app_language {
        AppLanguage::Chinese => format!("当前扫描进度 {scanned}/{total}，完成后结果会自动填充。"),
        AppLanguage::English => {
            format!("Progress {scanned}/{total}. Results will populate here automatically.")
        }
    }
}

fn scanning_overlay_eyebrow(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "正在扫描局域网",
        AppLanguage::English => "Scanning LAN",
    }
}

fn scanning_overlay_title(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "扫描任务进行中",
        AppLanguage::English => "Scan Running",
    }
}

fn detail_empty_results_eyebrow(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "扫描完成",
        AppLanguage::English => "Scan Complete",
    }
}

fn detail_empty_results_title(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "当前网段未发现设备",
        AppLanguage::English => "No Devices Found",
    }
}

fn detail_empty_results_description(app_language: AppLanguage) -> String {
    match app_language {
        AppLanguage::Chinese => {
            String::from("这个网段里暂时没有开放 SSH 端口的主机，可以更换网卡后重新扫描。")
        }
        AppLanguage::English => String::from(
            "No hosts with an open SSH port were found on this subnet. Switch networks and try again.",
        ),
    }
}

fn detail_no_selection_eyebrow(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "等待选择",
        AppLanguage::English => "Waiting For Selection",
    }
}

fn detail_no_selection_title(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "请选择左侧设备",
        AppLanguage::English => "Select A Device",
    }
}

fn detail_no_selection_description(app_language: AppLanguage) -> String {
    match app_language {
        AppLanguage::Chinese => {
            String::from("点击设备列表中的任意一项，这里会联动显示名称、IP 和快速连接入口。")
        }
        AppLanguage::English => String::from(
            "Click any device in the list to show its name, IP address, and quick-connect actions here.",
        ),
    }
}

fn launcher_temporarily_locked_label(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "稍后可用",
        AppLanguage::English => "Locked",
    }
}

fn launcher_unavailable_label(app_language: AppLanguage) -> &'static str {
    match app_language {
        AppLanguage::Chinese => "不可用",
        AppLanguage::English => "Unavailable",
    }
}

fn centered_state<'a, Message>(
    icon_visual: DetailVisual,
    tag_visual: DetailVisual,
    eyebrow: &'static str,
    title: &'static str,
    description: String,
    accent: iced::Color,
    background: iced::Color,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let panel = container(
        column![
            state_tag(eyebrow, tag_visual, accent, background),
            detail_icon(icon_visual, accent, background, EMPTY_STATE_ICON_EDGE),
            text(title)
                .font(fonts::semibold())
                .size(16)
                .style(|theme: &Theme| theme::text_primary(theme)),
            container(
                text(description)
                    .size(12)
                    .style(|theme: &Theme| theme::text_muted(theme)),
            )
            .width(Length::Fixed(EMPTY_STATE_DESCRIPTION_WIDTH))
            .center_x(Length::Fixed(EMPTY_STATE_DESCRIPTION_WIDTH)),
        ]
        .spacing(16)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(EMPTY_STATE_PANEL_WIDTH))
    .padding([24.0, 22.0])
    .style(|theme: &Theme| {
        let palette = colors::palette(theme);

        container::Style::default()
            .background(palette.input)
            .border(iced::Border {
                color: palette.border,
                width: 1.0,
                radius: border::radius(16),
            })
    });

    container(panel)
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .padding(20.0)
        .into()
}

fn scanning_overlay<'a, Message>(
    spinner_frame: &'static str,
    description: String,
    app_language: AppLanguage,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let accent = colors::rgb(0x3B, 0x82, 0xF6);
    let accent_soft = colors::rgba(0x3B, 0x82, 0xF6, 0.12);
    let panel = container(
        column![
            state_tag(
                scanning_overlay_eyebrow(app_language),
                DetailVisual::RefreshCw {
                    frame: spinner_frame,
                },
                accent,
                accent_soft,
            ),
            detail_icon(
                DetailVisual::RefreshCw {
                    frame: spinner_frame,
                },
                accent,
                accent_soft,
                EMPTY_STATE_ICON_EDGE,
            ),
            text(scanning_overlay_title(app_language))
                .font(fonts::semibold())
                .size(16)
                .style(|theme: &Theme| theme::text_primary(theme)),
            container(
                text(description)
                    .size(12)
                    .style(|theme: &Theme| theme::text_muted(theme)),
            )
            .width(Length::Fixed(EMPTY_STATE_DESCRIPTION_WIDTH))
            .center_x(Length::Fixed(EMPTY_STATE_DESCRIPTION_WIDTH)),
        ]
        .spacing(16)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(EMPTY_STATE_PANEL_WIDTH))
    .padding([24.0, 22.0])
    .style(|theme: &Theme| {
        let palette = colors::palette(theme);

        container::Style::default()
            .background(palette.card)
            .border(iced::Border {
                color: colors::rgba(0x3B, 0x82, 0xF6, 0.20),
                width: 1.0,
                radius: border::radius(18),
            })
    });

    container(panel)
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .padding(20.0)
        .style(|theme: &Theme| {
            let palette = colors::palette(theme);
            let backdrop = if palette.card == colors::DARK.card {
                colors::rgba(0x11, 0x11, 0x11, 0.62)
            } else {
                colors::rgba(0xFF, 0xFF, 0xFF, 0.70)
            };

            container::Style::default().background(backdrop)
        })
        .into()
}

fn selected_device_state<'a, Message>(
    state: SelectedDetailState<'a, Message>,
    app_language: AppLanguage,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let SelectedDetailState {
        device,
        status_text: _status_text,
        active_launcher_key,
        on_shell,
        on_vscode,
        on_vnc,
        on_mobaxterm,
        on_docker,
        on_rustdesk,
        on_close: _on_close,
    } = state;

    let launchers = column![
        launcher_card(
            app_language,
            "Shell",
            Glyph::Terminal,
            colors::rgb(0x7C, 0x3A, 0xED),
            colors::rgba(0x7C, 0x3A, 0xED, 0.10),
            LauncherCardState::new(
                LauncherCapability::Supported,
                active_launcher_key == Some("shell"),
                on_shell,
            ),
        ),
        launcher_card(
            app_language,
            "VS Code",
            Glyph::Code,
            colors::rgb(0x3B, 0x82, 0xF6),
            colors::rgba(0x3B, 0x82, 0xF6, 0.10),
            LauncherCardState::new(
                LauncherCapability::Supported,
                active_launcher_key == Some("vscode"),
                on_vscode,
            ),
        ),
        launcher_card(
            app_language,
            "VNC Viewer",
            Glyph::Display,
            colors::rgb(0x22, 0xC5, 0x5E),
            colors::rgba(0x22, 0xC5, 0x5E, 0.10),
            LauncherCardState::new(
                LauncherCapability::Supported,
                active_launcher_key == Some("vnc"),
                on_vnc,
            ),
        ),
        launcher_card(
            app_language,
            "MobaXterm",
            Glyph::Laptop,
            colors::rgb(0xF9, 0x73, 0x16),
            colors::rgba(0xF9, 0x73, 0x16, 0.12),
            LauncherCardState::new(
                LauncherCapability::Supported,
                active_launcher_key == Some("mobaxterm"),
                on_mobaxterm,
            ),
        ),
        launcher_card(
            app_language,
            "Docker",
            Glyph::Docker,
            colors::rgb(0x08, 0x91, 0xB2),
            colors::rgba(0x08, 0x91, 0xB2, 0.10),
            LauncherCardState::new(
                LauncherCapability::Supported,
                active_launcher_key == Some("docker"),
                on_docker,
            ),
        ),
        launcher_card(
            app_language,
            "RustDesk",
            Glyph::Desktop,
            colors::rgb(0x4F, 0x46, 0xE5),
            colors::rgba(0x4F, 0x46, 0xE5, 0.10),
            LauncherCardState::new(
                LauncherCapability::Supported,
                active_launcher_key == Some("rustdesk"),
                on_rustdesk,
            ),
        ),
    ]
    .spacing(10)
    .width(Fill);

    let quick_connect_section = column![
        row![
            device_icon(device.identity_kind, false),
            column![
                text(&device.name)
                    .font(fonts::semibold())
                    .size(16)
                    .style(|theme: &Theme| theme::text_primary(theme)),
                text(format!(
                    "{} · {}",
                    identity_kind_label(device.identity_kind, app_language),
                    device.ip
                ))
                .size(13)
                .style(|theme: &Theme| theme::text_muted(theme)),
            ]
            .spacing(4)
            .width(Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Fill),
        container(Space::new().width(Fill).height(1.0)).style(|theme: &Theme| {
            let palette = colors::palette(theme);
            container::Style::default().background(palette.border)
        }),
        launchers,
    ]
    .spacing(14)
    .width(Fill);

    container(quick_connect_section)
        .width(Length::Fixed(QUICK_CONNECT_PANEL_WIDTH))
        .padding(iced::Padding {
            top: 20.0,
            right: 20.0,
            bottom: 16.0,
            left: 20.0,
        })
        .into()
}

fn device_icon<'a, Message: 'a>(
    identity_kind: DeviceIdentityKind,
    selected: bool,
) -> Element<'a, Message> {
    let accent = identity_accent(identity_kind);
    let tone = if selected { iced::Color::WHITE } else { accent };

    container(icons::centered(
        device_identity_glyph(identity_kind),
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
            (palette.input, with_alpha(accent, 0.24))
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

fn device_identity_glyph(identity_kind: DeviceIdentityKind) -> Glyph {
    match identity_kind {
        DeviceIdentityKind::RaspberryPi | DeviceIdentityKind::Jetson => Glyph::Server,
        DeviceIdentityKind::Computer => Glyph::Desktop,
        DeviceIdentityKind::Unknown => Glyph::Search,
    }
}

fn identity_accent(identity_kind: DeviceIdentityKind) -> iced::Color {
    match identity_kind {
        DeviceIdentityKind::RaspberryPi => colors::rgb(0x16, 0xA3, 0x4A),
        DeviceIdentityKind::Jetson => colors::rgb(0x0E, 0x9F, 0x6E),
        DeviceIdentityKind::Computer => colors::rgb(0x25, 0x63, 0xEB),
        DeviceIdentityKind::Unknown => colors::rgb(0x6B, 0x72, 0x80),
    }
}

fn identity_kind_label(
    identity_kind: DeviceIdentityKind,
    app_language: AppLanguage,
) -> &'static str {
    match identity_kind {
        DeviceIdentityKind::RaspberryPi => "Raspberry Pi",
        DeviceIdentityKind::Jetson => "NVIDIA Jetson",
        DeviceIdentityKind::Computer => match app_language {
            AppLanguage::Chinese => "计算机",
            AppLanguage::English => "Computer",
        },
        DeviceIdentityKind::Unknown => match app_language {
            AppLanguage::Chinese => "未知设备",
            AppLanguage::English => "Unknown Device",
        },
    }
}

fn with_alpha(color: iced::Color, alpha: f32) -> iced::Color {
    colors::rgba(
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
        alpha,
    )
}

fn detail_icon<'a, Message>(
    visual: DetailVisual,
    accent: iced::Color,
    background: iced::Color,
    size: f32,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let icon_size = (size * 0.36).clamp(16.0, 28.0);
    let radius = (size * 0.32).round();

    container(visual.centered(size, icon_size, accent))
        .width(size)
        .height(size)
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(move |_| {
            container::Style::default()
                .background(background)
                .border(iced::Border {
                    color: colors::rgba(
                        (accent.r * 255.0).round() as u8,
                        (accent.g * 255.0).round() as u8,
                        (accent.b * 255.0).round() as u8,
                        0.28,
                    ),
                    width: 1.0,
                    radius: border::radius(radius),
                })
        })
        .into()
}

fn launcher_card<'a, Message>(
    app_language: AppLanguage,
    label: &'static str,
    glyph: Glyph,
    accent: iced::Color,
    accent_soft: iced::Color,
    card_state: LauncherCardState<Message>,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let state = card_state.visual_state();
    let visually_disabled = matches!(state, LauncherVisualState::Unavailable);
    let on_press = card_state.into_action();
    let muted_tone = colors::rgb(0x9C, 0xA3, 0xAF);
    let locked_tone = colors::rgb(0x64, 0x74, 0x8B);
    let badge = match state {
        LauncherVisualState::Available => None,
        LauncherVisualState::Processing => None,
        LauncherVisualState::TemporarilyLocked => Some((
            launcher_temporarily_locked_label(app_language),
            locked_tone,
            colors::rgba(0x64, 0x74, 0x8B, 0.12),
        )),
        LauncherVisualState::Unavailable => Some((
            launcher_unavailable_label(app_language),
            muted_tone,
            colors::rgba(0x9C, 0xA3, 0xAF, 0.14),
        )),
    };
    let icon_tone = if visually_disabled {
        muted_tone
    } else {
        accent
    };
    let icon_background = if visually_disabled {
        colors::rgba(0x9C, 0xA3, 0xAF, 0.10)
    } else {
        accent_soft
    };
    let icon_border = if visually_disabled {
        colors::rgba(0x9C, 0xA3, 0xAF, 0.28)
    } else {
        colors::rgba(
            (accent.r * 255.0).round() as u8,
            (accent.g * 255.0).round() as u8,
            (accent.b * 255.0).round() as u8,
            0.28,
        )
    };

    let connect_state = match state {
        LauncherVisualState::Available => theme::styles::ConnectButtonState::Available,
        LauncherVisualState::Processing => theme::styles::ConnectButtonState::Processing,
        LauncherVisualState::TemporarilyLocked => {
            theme::styles::ConnectButtonState::TemporarilyLocked
        }
        LauncherVisualState::Unavailable => theme::styles::ConnectButtonState::Unavailable,
    };

    let mut content = row![
        container(icons::framed(
            glyph,
            FrameSpec {
                width: QUICK_CONNECT_ICON_EDGE,
                height: QUICK_CONNECT_ICON_EDGE,
                icon_size: 18.0,
                tone: icon_tone,
                background: icon_background,
                border_color: icon_border,
                radius: 10.0,
            },
        ))
        .padding(0),
        text(label)
            .size(14)
            .font(fonts::semibold())
            .style(|theme: &Theme| theme::text_primary(theme))
            .width(Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .width(Fill);

    if let Some((badge_label, badge_tone, badge_background)) = badge {
        content = content.push(
            container(
                text(badge_label)
                    .size(11)
                    .style(move |_| theme::solid_text(badge_tone)),
            )
            .padding([4.0, 9.0])
            .style(move |_| {
                container::Style::default()
                    .background(badge_background)
                    .border(iced::Border {
                        color: colors::rgba(
                            (badge_tone.r * 255.0).round() as u8,
                            (badge_tone.g * 255.0).round() as u8,
                            (badge_tone.b * 255.0).round() as u8,
                            0.20,
                        ),
                        width: 1.0,
                        radius: border::radius(999),
                    })
            }),
        );
    }

    button(content)
        .width(Fill)
        .padding([13.0, 14.0])
        .style(move |theme: &Theme, status| {
            theme::styles::connect_button(theme, status, accent, connect_state)
        })
        .on_press_maybe(on_press)
        .into()
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LauncherCapability {
    Supported,
    Unsupported,
}

struct LauncherCardState<Message> {
    capability: LauncherCapability,
    on_press: Option<Message>,
    is_actionable: bool,
    is_processing: bool,
}

impl<Message> LauncherCardState<Message> {
    fn new(capability: LauncherCapability, is_processing: bool, on_press: Option<Message>) -> Self {
        let is_actionable = on_press.is_some();

        Self {
            capability,
            on_press,
            is_actionable,
            is_processing,
        }
    }

    fn visual_state(&self) -> LauncherVisualState {
        match self.capability {
            LauncherCapability::Unsupported => LauncherVisualState::Unavailable,
            LauncherCapability::Supported if self.is_processing => LauncherVisualState::Processing,
            LauncherCapability::Supported if self.is_actionable => LauncherVisualState::Available,
            LauncherCapability::Supported => LauncherVisualState::TemporarilyLocked,
        }
    }

    fn is_actionable(&self) -> bool {
        matches!(self.visual_state(), LauncherVisualState::Available)
    }

    fn into_action(self) -> Option<Message> {
        if self.is_actionable() {
            self.on_press
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LauncherVisualState {
    Available,
    Processing,
    TemporarilyLocked,
    Unavailable,
}

fn state_tag<'a, Message: 'a>(
    label: &'static str,
    visual: DetailVisual,
    tone: iced::Color,
    background: iced::Color,
) -> Element<'a, Message> {
    container(
        row![
            container(visual.centered(10.0, 6.5, tone))
                .width(12)
                .height(12)
                .center_x(Length::Fixed(12.0))
                .center_y(Length::Fixed(12.0))
                .style(move |_| {
                    container::Style::default()
                        .background(background)
                        .border(iced::Border {
                            color: colors::rgba(
                                (tone.r * 255.0).round() as u8,
                                (tone.g * 255.0).round() as u8,
                                (tone.b * 255.0).round() as u8,
                                0.28,
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
    .padding([5.0, 9.0])
    .style(move |_| {
        container::Style::default()
            .background(background)
            .border(iced::Border {
                color: colors::rgba(
                    (tone.r * 255.0).round() as u8,
                    (tone.g * 255.0).round() as u8,
                    (tone.b * 255.0).round() as u8,
                    0.20,
                ),
                width: 1.0,
                radius: border::radius(999),
            })
    })
    .into()
}

fn launcher_state_badge<'a, Message: 'a>(
    label: &'static str,
    tone: iced::Color,
    background: iced::Color,
) -> Element<'a, Message> {
    let glyph = match label {
        "处理中" | "Processing" => Glyph::Pending,
        "稍后可用" | "Locked" => Glyph::Lock,
        "不可用" | "Unavailable" => Glyph::Close,
        _ => Glyph::Check,
    };

    container(icons::centered_compact(glyph, 16.0, 8.0, tone))
        .width(22)
        .height(22)
        .center_x(Length::Fixed(22.0))
        .center_y(Length::Fixed(22.0))
        .style(move |_| {
            container::Style::default()
                .background(background)
                .border(iced::Border {
                    color: colors::rgba(
                        (tone.r * 255.0).round() as u8,
                        (tone.g * 255.0).round() as u8,
                        (tone.b * 255.0).round() as u8,
                        0.26,
                    ),
                    width: 1.0,
                    radius: border::radius(999),
                })
        })
        .into()
}
