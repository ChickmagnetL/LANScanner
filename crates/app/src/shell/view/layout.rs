use iced::widget::{Space, button, column, container, row, stack, text};
use iced::{Alignment, Element, Fill, Length, Theme};
use ssh_core::credential::Credential;
use ssh_core::network::{InterfaceType, NetworkInterface};
use ssh_core::scanner::{Device, DeviceStatus};
use ui::device_detail::SelectedDetailState;
use ui::theme::icons::{self, Glyph};
use ui::theme::{self, ThemeMode};
use ui::widgets::dropdown::{self, DropdownEntry};

use crate::message::Message;

use super::super::{ActiveModal, PendingToolAction, ShellApp};
use super::dropdown_metrics::{
    CONTENT_PADDING, CONTENT_SPACING, LEFT_COLUMN_SPACING, LEFT_COLUMN_WIDTH, RIGHT_PANEL_PADDING,
    WINDOW_PADDING, credential_dropdown_left, credential_dropdown_top, credential_dropdown_width,
    scan_dropdown_left, scan_dropdown_top, scan_dropdown_width,
};
use super::filters::scan_result_filter_controls;
use super::header_status::{
    RESULT_HEADER_FILTER_SLOT_HEIGHT, RESULT_HEADER_STATUS_SLOT_HEIGHT, result_header_status,
};
use super::resize_overlay::window_resize_overlay;

pub(super) fn view(app: &ShellApp) -> Element<'_, Message> {
    let header = ui::titlebar::view(
        app.theme_mode,
        app.is_window_maximized,
        Message::ToggleTheme,
        Message::OpenHelpModal,
        Message::WindowAction,
    );

    dropdown::render(
        dropdown::DropdownProps {
            items: &app.credentials,
            selection: app.selected_credential(),
            placeholder: dropdown::DropdownPlaceholder {
                glyph: Glyph::KeyRound,
                title: "用户名",
                subtitle: None,
            },
            show_trigger_icon: false,
            show_option_icon: false,
            footer_action: (!app.is_verifying && !app.is_connecting)
                .then_some(Message::OpenCredModal),
            state: dropdown::DropdownState {
                is_open: app.user_dropdown_open,
                on_toggle: app.user_dropdown_toggle_message(),
                on_dismiss: Message::UserDropdownClosed,
            },
            placement: dropdown::DropdownPlacement {
                left: credential_dropdown_left(),
                top: credential_dropdown_top(),
                width: credential_dropdown_width(),
            },
            describe: describe_credential,
            on_selected: |credential: Credential| Message::SelectUser(credential.username),
        },
        |_credential_dropdown| {
            dropdown::render(
                dropdown::DropdownProps {
                    items: &app.networks,
                    selection: app.selected_network(),
                    placeholder: dropdown::DropdownPlaceholder {
                        glyph: Glyph::Wifi,
                        title: "选择网络接口...",
                        subtitle: None,
                    },
                    show_trigger_icon: true,
                    show_option_icon: false,
                    footer_action: None,
                    state: dropdown::DropdownState {
                        is_open: app.network_dropdown_open,
                        on_toggle: app.network_dropdown_toggle_message(),
                        on_dismiss: Message::NetworkDropdownClosed,
                    },
                    placement: dropdown::DropdownPlacement {
                        left: scan_dropdown_left(),
                        top: scan_dropdown_top(),
                        width: scan_dropdown_width(),
                    },
                    describe: |network| describe_network(network, &app.networks),
                    on_selected: |network: NetworkInterface| Message::SelectNetwork(network.id),
                },
                |network_dropdown| {
                    let left_column = column![
                        ui::scan_card::view(ui::scan_card::ScanCardProps {
                            dropdown: network_dropdown,
                            selected_network: app.selected_network(),
                            is_refreshing: app.is_refreshing_networks,
                            is_scanning: app.is_scanning,
                            is_blocked: app.is_verifying || app.is_connecting,
                            spinner_frame: app.spinner_frame(),
                            on_refresh: Message::RefreshNetworks,
                            on_start_scan: if app.is_scanning {
                                Message::CancelScan
                            } else {
                                Message::StartScan
                            },
                        }),
                        ui::credential_card::view(ui::credential_card::CredentialCardProps {
                            dropdown: credential_dropdown_affordance(app),
                            is_dark_theme: matches!(app.theme_mode, ThemeMode::Dark),
                            username: &app.ssh_username,
                            password: &app.password,
                            vnc_enabled: app.vnc_enabled,
                            vnc_user: &app.vnc_user,
                            vnc_password: &app.vnc_password,
                            is_verifying: app.is_verifying,
                            has_scanned: app.has_scanned,
                            has_devices: !app.devices.is_empty(),
                            spinner_frame: app.spinner_frame(),
                            on_manage: (!app.is_verifying && !app.is_connecting)
                                .then_some(Message::OpenCredModal),
                            on_toggle_vnc: (!app.is_verifying && !app.is_connecting)
                                .then_some(Message::ToggleVnc),
                            on_verify: app.can_start_verify().then_some(Message::StartVerify),
                            on_username_input: Message::SetUsername,
                            on_password_input: Message::SetPassword,
                            on_vnc_user_input: Message::SetVncUser,
                            on_vnc_password_input: Message::SetVncPassword,
                        }),
                    ]
                    .width(LEFT_COLUMN_WIDTH)
                    .spacing(LEFT_COLUMN_SPACING);

                    let result_title_slot: Element<'_, Message> = container(
                        text("扫描结果")
                            .font(ui::theme::fonts::semibold())
                            .size(14)
                            .style(|theme: &Theme| theme::text_primary(theme)),
                    )
                    .center_y(Length::Fixed(24.0))
                    .into();

                    let status_strip_slot = container(
                        match result_header_status(
                            app.notice.as_ref(),
                            app.connection_status.as_deref(),
                        ) {
                            Some(status_strip) => status_strip,
                            None => Space::new()
                                .width(Length::Shrink)
                                .height(Length::Shrink)
                                .into(),
                        },
                    )
                    .height(Length::Fixed(RESULT_HEADER_STATUS_SLOT_HEIGHT))
                    .center_y(Length::Fixed(RESULT_HEADER_STATUS_SLOT_HEIGHT));

                    let result_filter_controls = scan_result_filter_controls(app);
                    let result_filter_slot = container(result_filter_controls)
                        .height(Length::Fixed(RESULT_HEADER_FILTER_SLOT_HEIGHT))
                        .center_y(Length::Fixed(RESULT_HEADER_FILTER_SLOT_HEIGHT));
                    let header_row = row![
                        result_title_slot,
                        Space::new().width(Length::Fixed(14.0)),
                        result_filter_slot,
                        Space::new().width(Fill),
                        status_strip_slot,
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center);

                    let result_header = container(header_row).padding(iced::Padding {
                        top: 18.0,
                        right: 20.0,
                        bottom: 16.0,
                        left: 20.0,
                    });

                    let list_content = if app.is_scanning && app.devices.is_empty() {
                        ui::device_list::placeholder(ui::device_list::PlaceholderState::Scanning {
                            spinner_frame: app.spinner_frame(),
                            progress: app.scan_progress,
                        })
                    } else if app.has_scanned {
                        if app.devices.is_empty() {
                            ui::device_list::placeholder(
                                ui::device_list::PlaceholderState::EmptyResults,
                            )
                        } else {
                            visual_check_device_list(app)
                        }
                    } else if app.is_refreshing_networks {
                        ui::device_list::placeholder(
                            ui::device_list::PlaceholderState::RefreshingNetworks {
                                spinner_frame: app.spinner_frame(),
                            },
                        )
                    } else {
                        ui::device_list::placeholder(ui::device_list::PlaceholderState::Idle)
                    };

                    let detail_state = if app.is_scanning && app.devices.is_empty() {
                        ui::device_detail::DetailState::Scanning {
                            spinner_frame: app.spinner_frame(),
                            progress: app.scan_progress,
                        }
                    } else if app.is_refreshing_networks && !app.has_scanned {
                        ui::device_detail::DetailState::RefreshingNetworks {
                            spinner_frame: app.spinner_frame(),
                        }
                    } else if !app.has_scanned {
                        ui::device_detail::DetailState::Idle
                    } else if app.devices.is_empty() {
                        ui::device_detail::DetailState::EmptyResults
                    } else if let Some(device) = app.selected_device() {
                        let status_text = device_detail_status(app, device);
                        let active_launcher_key =
                            app.active_quick_connect_launcher_for_device(device.ip.as_str());

                        ui::device_detail::DetailState::Selected(SelectedDetailState {
                            device,
                            status_text,
                            active_launcher_key,
                            on_shell: Some(Message::ConnectShell(device.ip.clone())),
                            on_vscode: Some(Message::ConnectVSCode(device.ip.clone())),
                            on_vnc: Some(Message::ConnectVNC(device.ip.clone())),
                            on_mobaxterm: Some(Message::ConnectMobaXterm(device.ip.clone())),
                            on_docker: Some(Message::ConnectDocker(device.ip.clone())),
                            on_rustdesk: Some(Message::ConnectRustDesk(device.ip.clone())),
                            on_close: Some(Message::CloseDetail),
                        })
                    } else {
                        ui::device_detail::DetailState::NoSelection
                    };
                    let show_detail_panel =
                        matches!(&detail_state, ui::device_detail::DetailState::Selected(_));
                    let detail_panel: Option<Element<'_, Message>> = if show_detail_panel {
                        Some(ui::device_detail::view(detail_state))
                    } else {
                        None
                    };
                    let list_panel = container(list_content)
                        .width(Fill)
                        .height(Fill)
                        .padding([0, 4]);
                    let right_body: Element<'_, Message> = if let Some(detail_panel) = detail_panel
                    {
                        row![list_panel, divider(), detail_panel]
                            .spacing(0)
                            .width(Fill)
                            .height(Fill)
                            .align_y(Alignment::Start)
                            .into()
                    } else {
                        list_panel.into()
                    };

                    let right_panel_body = column![result_header, horizontal_divider(), right_body];

                    let right_panel = container(right_panel_body)
                        .width(Fill)
                        .height(Fill)
                        .padding(RIGHT_PANEL_PADDING)
                        .style(ui::theme::styles::card_panel);

                    let content_body: Element<'_, Message> = row![left_column, right_panel]
                        .spacing(CONTENT_SPACING)
                        .height(Fill)
                        .align_y(Alignment::Start)
                        .into();

                    let content = container(content_body)
                        .width(Fill)
                        .height(Fill)
                        .padding(CONTENT_PADDING);

                    let shell = container(column![header, content].height(Fill).spacing(0))
                        .width(Fill)
                        .height(Fill)
                        .clip(true)
                        .style(ui::theme::styles::window_shell);
                    let shell: Element<'_, Message> = if WINDOW_PADDING > 0.0 {
                        container(shell)
                            .width(Fill)
                            .height(Fill)
                            .padding(WINDOW_PADDING)
                            .clip(true)
                            .style(ui::theme::styles::window_backdrop)
                            .into()
                    } else {
                        shell.into()
                    };
                    let shell: Element<'_, Message> = if app.is_window_maximized
                        || !platform::window::uses_custom_resize_overlay()
                    {
                        shell
                    } else {
                        stack([shell, window_resize_overlay()])
                            .width(Fill)
                            .height(Fill)
                            .into()
                    };

                    match app.active_modal.as_ref() {
                        Some(ActiveModal::HelpGuide) => ui::widgets::modal::overlay(
                            shell,
                            ui::modals::help::view(ui::modals::help::HelpGuideProps {
                                on_close: Message::CloseHelpModal,
                                on_open_github: Message::OpenGitHub,
                                show_rustdesk_section: app.help_modal_show_rustdesk,
                                on_show_basic: Message::ShowHelpGuideBasic,
                                on_show_rustdesk: Message::ShowHelpGuideRustDesk,
                            }),
                            Message::CloseHelpModal,
                            536.0,
                        ),
                        Some(ActiveModal::CredentialManagement) => ui::widgets::modal::overlay(
                            shell,
                            ui::modals::cred_mgmt::view(
                                ui::modals::cred_mgmt::CredentialManagementProps {
                                    credentials: &app.credentials,
                                    editing_username: app.editing_credential_username.as_deref(),
                                    username: &app.new_credential_username,
                                    password: &app.new_credential_password,
                                    on_username_input: Message::SetNewCredentialUsername,
                                    on_password_input: Message::SetNewCredentialPassword,
                                    on_edit: Message::StartEditCredential,
                                    on_cancel_edit: app
                                        .editing_credential_username
                                        .as_ref()
                                        .map(|_| Message::CancelEditCredential),
                                    on_save: app.save_credential_message(),
                                    on_remove: Message::RemoveCredential,
                                    on_close: Message::CloseCredModal,
                                },
                            ),
                            Message::CloseCredModal,
                            420.0,
                        ),
                        Some(ActiveModal::DockerSelect(state)) => ui::widgets::modal::overlay(
                            shell,
                            ui::modals::docker_select::view(
                                ui::modals::docker_select::DockerSelectProps {
                                    containers: &state.containers,
                                    selected_container_id: state.selected_container_id.as_deref(),
                                    on_select: Message::SelectContainer,
                                    on_close: Message::CloseDockerModal,
                                    on_connect: state
                                        .selected_container_id
                                        .as_ref()
                                        .map(|_| Message::AttachSelectedContainer),
                                },
                            ),
                            Message::CloseDockerModal,
                            560.0,
                        ),
                        None => shell,
                    }
                },
            )
        },
    )
}

fn visual_check_device_list(app: &ShellApp) -> Element<'_, Message> {
    ui::device_list::view(
        &app.devices,
        app.selected_device_id.as_deref(),
        Message::SelectDevice,
    )
}

fn credential_dropdown_affordance(app: &ShellApp) -> Element<'_, Message> {
    let on_toggle = app.user_dropdown_toggle_message();
    let is_open = app.user_dropdown_open;
    let is_disabled = on_toggle.is_none();
    let chevron = if is_open {
        Glyph::ChevronUp
    } else {
        Glyph::ChevronDown
    };

    let mut affordance = button(
        container(icons::themed_centered(
            chevron,
            icons::DROPDOWN_CHEVRON_SLOT,
            icons::DROPDOWN_CHEVRON_GLYPH,
            if is_disabled {
                credential_dropdown_icon_disabled_tone
            } else {
                credential_dropdown_icon_tone
            },
        ))
        .width(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Fill)
    .height(Fill)
    .padding([10, 12])
    .style(move |theme: &Theme, status| {
        ui::theme::styles::dropdown_trigger(theme, status, is_open)
    });

    if let Some(message) = on_toggle {
        affordance = affordance.on_press(message);
    }

    container(affordance).width(Fill).height(Fill).into()
}

fn credential_dropdown_icon_tone(theme: &Theme) -> iced::Color {
    let palette = ui::theme::colors::palette(theme);

    if palette.card == ui::theme::colors::DARK.card {
        palette.text
    } else {
        ui::theme::colors::rgb(0x4B, 0x55, 0x63)
    }
}

fn credential_dropdown_icon_disabled_tone(theme: &Theme) -> iced::Color {
    let palette = ui::theme::colors::palette(theme);
    let mut tone = credential_dropdown_icon_tone(theme);
    tone.a = if palette.card == ui::theme::colors::DARK.card {
        0.52
    } else {
        0.44
    };
    tone
}

fn device_detail_status(app: &ShellApp, device: &Device) -> String {
    if active_connection_device_ip(app) == Some(device.ip.as_str()) {
        return app
            .connection_status
            .clone()
            .unwrap_or_else(|| String::from("正在准备连接"));
    }

    match device.status {
        DeviceStatus::Untested => {
            String::from("凭据尚未检测；填写 SSH 用户名后，扫描结束会自动检测，也可手动重试。")
        }
        DeviceStatus::Ready => {
            String::from("SSH 凭据检测成功；外部工具的免密前置会在启动连接时单独校验。")
        }
        DeviceStatus::Denied => {
            String::from("检测结果为错误（用户名明显错误或认证失败）；仍可直接发起快速连接。")
        }
        DeviceStatus::Error => String::from(
            "检测结果为异常（仅用户名或网络抖动时可能无法稳定判定）；仍可直接发起快速连接。",
        ),
    }
}

fn active_connection_device_ip(app: &ShellApp) -> Option<&str> {
    app.pending_tool_action
        .as_ref()
        .map(|action| match action {
            PendingToolAction::Direct { context, .. } => context.device_ip.as_str(),
            PendingToolAction::DockerAttach { context, .. } => context.device_ip.as_str(),
        })
        .or_else(|| {
            app.pending_docker_context
                .as_ref()
                .map(|context| context.device_ip.as_str())
        })
}

fn divider<'a>() -> Element<'a, Message> {
    container(text(""))
        .width(1)
        .height(Fill)
        .style(|theme: &Theme| {
            let palette = ui::theme::colors::palette(theme);

            container::Style::default().background(palette.border)
        })
        .into()
}

fn horizontal_divider<'a>() -> Element<'a, Message> {
    container(text(""))
        .width(Fill)
        .height(1)
        .style(|theme: &Theme| {
            let palette = ui::theme::colors::palette(theme);

            container::Style::default().background(palette.border)
        })
        .into()
}

fn describe_network(
    network: &NetworkInterface,
    all_networks: &[NetworkInterface],
) -> DropdownEntry {
    let (glyph, kind_label) = describe_network_kind(network);
    let mut details = vec![format!("网段 {} · {}", network.ip_range, kind_label)];

    if needs_interface_disambiguation(network, all_networks, kind_label) {
        details.push(format!("接口 {}", network.id));
    }

    DropdownEntry {
        glyph,
        title: network.name.clone(),
        details,
    }
}

fn describe_credential(credential: &Credential) -> DropdownEntry {
    DropdownEntry {
        glyph: Glyph::KeyRound,
        title: credential.username.clone(),
        details: Vec::new(),
    }
}

fn describe_network_kind(network: &NetworkInterface) -> (Glyph, &'static str) {
    let fingerprint = format!("{} {}", network.name, network.id).to_ascii_lowercase();

    match network.iface_type {
        InterfaceType::Wifi => (Glyph::Wifi, "Wi-Fi"),
        InterfaceType::Ethernet => (Glyph::Ethernet, "以太网"),
        InterfaceType::Docker => {
            classify_virtual_network(&fingerprint).unwrap_or((Glyph::Docker, "虚拟网络"))
        }
        InterfaceType::Other => {
            classify_virtual_network(&fingerprint).unwrap_or((Glyph::Network, "网络接口"))
        }
    }
}

fn needs_interface_disambiguation(
    network: &NetworkInterface,
    all_networks: &[NetworkInterface],
    kind_label: &str,
) -> bool {
    all_networks.iter().any(|candidate| {
        candidate.id != network.id
            && candidate.name == network.name
            && candidate.ip_range == network.ip_range
            && describe_network_kind(candidate).1 == kind_label
    })
}

fn classify_virtual_network(fingerprint: &str) -> Option<(Glyph, &'static str)> {
    if fingerprint.contains("vmware") || fingerprint.contains("vmnet") {
        Some((Glyph::Docker, "VMware 虚拟网卡"))
    } else if fingerprint.contains("docker") {
        Some((Glyph::Docker, "Docker 虚拟网卡"))
    } else if fingerprint.contains("vethernet")
        || fingerprint.contains("hyper-v")
        || fingerprint.contains("hyperv")
        || fingerprint.contains("wsl")
    {
        Some((Glyph::Docker, "Windows 虚拟网卡"))
    } else if fingerprint.contains("virtualbox") || fingerprint.contains("vbox") {
        Some((Glyph::Docker, "VirtualBox 虚拟网卡"))
    } else if fingerprint.contains("tailscale")
        || fingerprint.contains("zerotier")
        || fingerprint.contains("vpn")
        || fingerprint.contains("tun")
        || fingerprint.contains("tap")
    {
        Some((Glyph::Network, "隧道网络接口"))
    } else if fingerprint.contains("virtual") || fingerprint.contains("bridge") {
        Some((Glyph::Docker, "虚拟网络"))
    } else {
        None
    }
}
