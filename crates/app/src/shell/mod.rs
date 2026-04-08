use std::time::Duration;

mod bootstrap;
mod notice;
mod selectors;
mod state;
mod subscription;
pub(crate) mod tasks {
    #[path = "connect.rs"]
    mod connect_impl;
    #[path = "scan.rs"]
    mod scan_impl;
    #[path = "verify.rs"]
    mod verify_impl;

    pub(crate) mod connect {
        pub(in super::super) const RUSTDESK_DIRECT_IP_PORT: u16 =
            super::connect_impl::RUSTDESK_DIRECT_IP_PORT;

        pub(in super::super) async fn probe_rustdesk_direct_ip_port(
            device_ip: &str,
        ) -> Result<(), String> {
            super::connect_impl::probe_rustdesk_direct_ip_port(device_ip).await
        }

        pub(in super::super) async fn execute_launch_action(
            action: super::super::PendingToolAction,
            tool_path: std::path::PathBuf,
        ) -> Result<crate::message::ConnectNotice, String> {
            super::connect_impl::execute_launch_action(action, tool_path).await
        }

        pub(in super::super) async fn prepare_ssh_launch_auth(
            context: &super::super::LaunchContext,
            consumer: ssh_core::ssh::auth::LaunchAuthConsumer,
        ) -> Result<ssh_core::ssh::auth::LaunchAuthPreparation, String> {
            super::connect_impl::prepare_ssh_launch_auth(context, consumer).await
        }

        pub(in super::super) fn shell_connect_notice(
            preparation: &ssh_core::ssh::auth::LaunchAuthPreparation,
        ) -> crate::message::ConnectNotice {
            super::connect_impl::shell_connect_notice(preparation)
        }
    }

    pub(crate) mod scan {
        pub(in super::super) fn spawn_scan_task(
            network: ssh_core::network::NetworkInterface,
            cancel_token: tokio_util::sync::CancellationToken,
            session_id: u64,
        ) -> (iced::Task<crate::message::Message>, iced::task::Handle) {
            super::scan_impl::spawn_scan_task(network, cancel_token, session_id)
        }

        pub(in super::super) fn spawn_ssh_probe_task(
            network: ssh_core::network::NetworkInterface,
            candidate_ips: Vec<String>,
            cancel_token: tokio_util::sync::CancellationToken,
            session_id: u64,
        ) -> (iced::Task<crate::message::Message>, iced::task::Handle) {
            super::scan_impl::spawn_ssh_probe_task(network, candidate_ips, cancel_token, session_id)
        }
    }

    pub(crate) mod verify {
        pub(in super::super) fn spawn_verify_task(
            devices: Vec<ssh_core::scanner::Device>,
            username: String,
            password: Option<String>,
            session_id: u64,
        ) -> iced::Task<crate::message::Message> {
            super::verify_impl::spawn_verify_task(devices, username, password, session_id)
        }
    }
}
pub(crate) mod update {
    #[path = "connect.rs"]
    mod connect_impl;
    #[path = "credential.rs"]
    mod credential_impl;
    #[path = "modal.rs"]
    mod modal_impl;
    #[path = "network.rs"]
    mod network_impl;
    #[path = "scan.rs"]
    mod scan_impl;
    #[path = "visual_check.rs"]
    mod visual_check_impl;
    #[path = "window.rs"]
    mod window_impl;

    pub(crate) mod connect {
        pub(in super::super) fn handle_connect_shell(
            app: &mut super::super::ShellApp,
            device_ip: String,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_connect_shell(app, device_ip)
        }

        pub(in super::super) fn handle_connect_vscode(
            app: &mut super::super::ShellApp,
            device_ip: String,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_connect_vscode(app, device_ip)
        }

        pub(in super::super) fn handle_connect_mobaxterm(
            app: &mut super::super::ShellApp,
            device_ip: String,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_connect_mobaxterm(app, device_ip)
        }

        pub(in super::super) fn handle_connect_vnc(
            app: &mut super::super::ShellApp,
            device_ip: String,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_connect_vnc(app, device_ip)
        }

        pub(in super::super) fn handle_connect_docker(
            app: &mut super::super::ShellApp,
            device_ip: String,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_connect_docker(app, device_ip)
        }

        pub(in super::super) fn handle_connect_rustdesk(
            app: &mut super::super::ShellApp,
            device_ip: String,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_connect_rustdesk(app, device_ip)
        }

        pub(in super::super) fn handle_rustdesk_probe_finished(
            app: &mut super::super::ShellApp,
            device_ip: String,
            password: Option<String>,
            result: Result<(), String>,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_rustdesk_probe_finished(app, device_ip, password, result)
        }

        pub(in super::super) fn handle_request_tool_path(
            app: &mut super::super::ShellApp,
            tool: ssh_core::credential::store::ToolKind,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_request_tool_path(app, tool)
        }

        pub(in super::super) fn handle_tool_path_picked(
            app: &mut super::super::ShellApp,
            tool: ssh_core::credential::store::ToolKind,
            selected_path: Option<std::path::PathBuf>,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_tool_path_picked(app, tool, selected_path)
        }

        pub(in super::super) fn handle_tool_path_pick_failed(
            app: &mut super::super::ShellApp,
            tool: ssh_core::credential::store::ToolKind,
            error: String,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_tool_path_pick_failed(app, tool, error)
        }

        pub(in super::super) fn handle_docker_containers_loaded(
            app: &mut super::super::ShellApp,
            containers: Vec<ssh_core::docker::Container>,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_docker_containers_loaded(app, containers)
        }

        pub(in super::super) fn handle_docker_containers_load_failed(
            app: &mut super::super::ShellApp,
            message: String,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_docker_containers_load_failed(app, message)
        }

        pub(in super::super) fn handle_attach_selected_container(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_attach_selected_container(app)
        }

        pub(in super::super) fn handle_connect_result(
            app: &mut super::super::ShellApp,
            result: Result<crate::message::ConnectNotice, String>,
        ) -> iced::Task<crate::message::Message> {
            super::connect_impl::handle_connect_result(app, result)
        }
    }

    pub(crate) mod credential {
        pub(in super::super) fn handle_user_dropdown_opened(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_user_dropdown_opened(app)
        }

        pub(in super::super) fn handle_user_dropdown_closed(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_user_dropdown_closed(app)
        }

        pub(in super::super) fn handle_set_username(
            app: &mut super::super::ShellApp,
            value: String,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_set_username(app, value)
        }

        pub(in super::super) fn handle_select_user(
            app: &mut super::super::ShellApp,
            username: String,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_select_user(app, username)
        }

        pub(in super::super) fn handle_set_password(
            app: &mut super::super::ShellApp,
            password: String,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_set_password(app, password)
        }

        pub(in super::super) fn handle_toggle_vnc(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_toggle_vnc(app)
        }

        pub(in super::super) fn handle_set_vnc_user(
            app: &mut super::super::ShellApp,
            value: String,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_set_vnc_user(app, value)
        }

        pub(in super::super) fn handle_set_vnc_password(
            app: &mut super::super::ShellApp,
            value: String,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_set_vnc_password(app, value)
        }

        pub(in super::super) fn handle_set_new_credential_username(
            app: &mut super::super::ShellApp,
            value: String,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_set_new_credential_username(app, value)
        }

        pub(in super::super) fn handle_set_new_credential_password(
            app: &mut super::super::ShellApp,
            value: String,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_set_new_credential_password(app, value)
        }

        pub(in super::super) fn handle_start_edit_credential(
            app: &mut super::super::ShellApp,
            username: String,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_start_edit_credential(app, username)
        }

        pub(in super::super) fn handle_cancel_edit_credential(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_cancel_edit_credential(app)
        }

        pub(in super::super) fn handle_add_credential(
            app: &mut super::super::ShellApp,
            username: String,
            password: Option<String>,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_add_credential(app, username, password)
        }

        pub(in super::super) fn handle_update_credential_password(
            app: &mut super::super::ShellApp,
            username: String,
            password: String,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_update_credential_password(app, username, password)
        }

        pub(in super::super) fn handle_remove_credential(
            app: &mut super::super::ShellApp,
            id: String,
        ) -> iced::Task<crate::message::Message> {
            super::credential_impl::handle_remove_credential(app, id)
        }
    }

    pub(crate) mod modal {
        pub(in super::super) fn handle_open_help_modal(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::modal_impl::handle_open_help_modal(app)
        }

        pub(in super::super) fn handle_close_help_modal(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::modal_impl::handle_close_help_modal(app)
        }

        pub(in super::super) fn handle_show_help_guide_basic(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::modal_impl::handle_show_help_guide_basic(app)
        }

        pub(in super::super) fn handle_show_help_guide_rustdesk(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::modal_impl::handle_show_help_guide_rustdesk(app)
        }

        pub(in super::super) fn handle_open_cred_modal(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::modal_impl::handle_open_cred_modal(app)
        }

        pub(in super::super) fn handle_close_cred_modal(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::modal_impl::handle_close_cred_modal(app)
        }

        pub(in super::super) fn handle_select_container(
            app: &mut super::super::ShellApp,
            container_id: String,
        ) -> iced::Task<crate::message::Message> {
            super::modal_impl::handle_select_container(app, container_id)
        }

        pub(in super::super) fn handle_close_docker_modal(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::modal_impl::handle_close_docker_modal(app)
        }
    }

    pub(crate) mod network {
        pub(in super::super) fn handle_refresh_networks(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::network_impl::handle_refresh_networks(app)
        }

        pub(in super::super) fn handle_networks_refreshed(
            app: &mut super::super::ShellApp,
            networks: Vec<ssh_core::network::NetworkInterface>,
        ) -> iced::Task<crate::message::Message> {
            super::network_impl::handle_networks_refreshed(app, networks)
        }

        pub(in super::super) fn handle_select_network(
            app: &mut super::super::ShellApp,
            network_id: String,
        ) -> iced::Task<crate::message::Message> {
            super::network_impl::handle_select_network(app, network_id)
        }

        pub(in super::super) fn handle_network_dropdown_opened(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::network_impl::handle_network_dropdown_opened(app)
        }

        pub(in super::super) fn handle_network_dropdown_closed(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::network_impl::handle_network_dropdown_closed(app)
        }

        pub(in super::super) fn handle_select_device(
            app: &mut super::super::ShellApp,
            device_id: String,
        ) -> iced::Task<crate::message::Message> {
            super::network_impl::handle_select_device(app, device_id)
        }

        pub(in super::super) fn handle_close_detail(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::network_impl::handle_close_detail(app)
        }
    }

    pub(crate) mod scan {
        pub(in super::super) fn handle_start_scan(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::scan_impl::handle_start_scan(app)
        }

        pub(in super::super) fn handle_scan_progress(
            app: &mut super::super::ShellApp,
            session_id: u64,
            scanned: usize,
            total: usize,
        ) -> iced::Task<crate::message::Message> {
            super::scan_impl::handle_scan_progress(app, session_id, scanned, total)
        }

        pub(in super::super) fn handle_scan_online_dataset_ready(
            app: &mut super::super::ShellApp,
            session_id: u64,
            evidence_by_ip: std::collections::HashMap<String, ssh_core::scanner::NeighborEvidence>,
        ) -> iced::Task<crate::message::Message> {
            super::scan_impl::handle_scan_online_dataset_ready(app, session_id, evidence_by_ip)
        }

        pub(in super::super) fn handle_scan_device_discovered(
            app: &mut super::super::ShellApp,
            session_id: u64,
            device: ssh_core::scanner::LayeredScanDevice,
        ) -> iced::Task<crate::message::Message> {
            super::scan_impl::handle_scan_device_discovered(app, session_id, device)
        }

        pub(in super::super) fn handle_scan_finished(
            app: &mut super::super::ShellApp,
            session_id: u64,
        ) -> iced::Task<crate::message::Message> {
            super::scan_impl::handle_scan_finished(app, session_id)
        }

        pub(in super::super) fn handle_scan_ssh_probe_finished(
            app: &mut super::super::ShellApp,
            session_id: u64,
            report: ssh_core::scanner::TcpProbeReport,
        ) -> iced::Task<crate::message::Message> {
            super::scan_impl::handle_scan_ssh_probe_finished(app, session_id, report)
        }

        pub(in super::super) fn handle_cancel_scan(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::scan_impl::handle_cancel_scan(app)
        }

        pub(in super::super) fn handle_start_verify(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::scan_impl::handle_start_verify(app)
        }

        pub(in super::super) fn handle_verify_result(
            app: &mut super::super::ShellApp,
            session_id: u64,
            ip: String,
            status: ssh_core::scanner::DeviceStatus,
        ) -> iced::Task<crate::message::Message> {
            super::scan_impl::handle_verify_result(app, session_id, ip, status)
        }

        pub(in super::super) fn handle_verify_complete(
            app: &mut super::super::ShellApp,
            session_id: u64,
        ) -> iced::Task<crate::message::Message> {
            super::scan_impl::handle_verify_complete(app, session_id)
        }

        pub(in super::super) fn can_start_verify(app: &super::super::ShellApp) -> bool {
            super::scan_impl::can_start_verify(app)
        }

        pub(in super::super) fn reset_verify_runtime(app: &mut super::super::ShellApp) {
            super::scan_impl::reset_verify_runtime(app);
        }

        pub(in super::super) fn rebuild_visible_devices_from_online(
            app: &mut super::super::ShellApp,
        ) {
            super::scan_impl::rebuild_visible_devices_from_online(app);
        }

        pub(in super::super) fn preferred_scan_result_filter(
            app: &super::super::ShellApp,
        ) -> super::super::ScanResultFilter {
            super::scan_impl::preferred_scan_result_filter(app)
        }

        pub(in super::super) fn cancel_scan_runtime(app: &mut super::super::ShellApp) {
            super::scan_impl::cancel_scan_runtime(app);
        }
    }

    pub(crate) mod visual_check {
        pub(in super::super) fn initialize_visual_check(
            app: &mut super::super::ShellApp,
            config: crate::visual_check::VisualCheckConfig,
        ) {
            super::visual_check_impl::initialize_visual_check(app, config);
        }

        pub(in super::super) fn handle_window_ready(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::visual_check_impl::handle_window_ready(app)
        }

        pub(in super::super) fn handle_tick(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::visual_check_impl::handle_tick(app)
        }

        pub(in super::super) fn capture_scene(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::visual_check_impl::capture_scene(app)
        }

        pub(in super::super) fn save_screenshot(
            app: &super::super::ShellApp,
            screenshot: &iced::window::Screenshot,
        ) -> Result<std::path::PathBuf, String> {
            super::visual_check_impl::save_screenshot(app, screenshot)
        }

        pub(in super::super) fn advance_or_exit(
            app: &mut super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::visual_check_impl::advance_or_exit(app)
        }

        pub(in super::super) fn close_window(
            app: &super::super::ShellApp,
        ) -> iced::Task<crate::message::Message> {
            super::visual_check_impl::close_window(app)
        }

        pub(in super::super) fn visual_check_scene_settling_frames(
            scene: crate::visual_check::VisualScene,
        ) -> u8 {
            super::visual_check_impl::visual_check_scene_settling_frames(scene)
        }
    }

    pub(crate) mod window {
        pub(in super::super) fn handle_window_ready(
            app: &mut super::super::ShellApp,
            window_id: iced::window::Id,
        ) -> iced::Task<crate::message::Message> {
            super::window_impl::handle_window_ready(app, window_id)
        }

        pub(in super::super) fn handle_window_resized(
            app: &mut super::super::ShellApp,
            window_id: iced::window::Id,
        ) -> iced::Task<crate::message::Message> {
            super::window_impl::handle_window_resized(app, window_id)
        }

        pub(in super::super) fn handle_window_maximized_changed(
            app: &mut super::super::ShellApp,
            is_maximized: bool,
        ) -> iced::Task<crate::message::Message> {
            super::window_impl::handle_window_maximized_changed(app, is_maximized)
        }

        pub(in super::super) fn handle_window_action(
            app: &mut super::super::ShellApp,
            action: platform::window::WindowAction,
        ) -> iced::Task<crate::message::Message> {
            super::window_impl::handle_window_action(app, action)
        }
    }
}
pub(crate) mod view {
    pub(super) mod dropdown_metrics;
    pub(super) mod filters;
    pub(super) mod header_status;
    #[path = "layout.rs"]
    mod layout_impl;
    pub(super) mod resize_overlay;

    pub(super) fn view(app: &super::ShellApp) -> iced::Element<'_, crate::message::Message> {
        layout_impl::view(app)
    }
}

use crate::message::Message;
use crate::visual_check::VisualCheckConfig;

use iced::{Element, Task, Theme};
use ssh_core::credential::{self, Credential};
use ssh_core::scanner::{DeviceStatus, SshPortProbeStatus};
use ssh_core::ssh::key_mgmt;

use state::ScanResultFilter;
pub use state::ShellApp;
use state::*;

const SPINNER_FRAMES: [&str; 12] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"];
const CONNECT_NOTICE_DISMISS_DELAY: Duration = Duration::from_secs(3);

impl ShellApp {
    pub fn title(&self) -> String {
        String::from("LANScanner")
    }

    pub fn theme(&self) -> Theme {
        self.theme_mode.theme()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleTheme => {
                self.close_overlays();
                self.theme_mode = self.theme_mode.toggle();
                Task::none()
            }
            Message::OpenHelpModal => update::modal::handle_open_help_modal(self),
            Message::CloseHelpModal => update::modal::handle_close_help_modal(self),
            Message::ShowHelpGuideBasic => update::modal::handle_show_help_guide_basic(self),
            Message::ShowHelpGuideRustDesk => update::modal::handle_show_help_guide_rustdesk(self),
            Message::OpenGitHub => {
                open_url("https://github.com/ChickmagnetL/LANScanner");
                Task::none()
            }
            Message::OpenCredModal => update::modal::handle_open_cred_modal(self),
            Message::WindowReady(window_id) => update::window::handle_window_ready(self, window_id),
            Message::WindowResized(window_id) => {
                update::window::handle_window_resized(self, window_id)
            }
            Message::WindowMaximizedChanged(is_maximized) => {
                update::window::handle_window_maximized_changed(self, is_maximized)
            }
            Message::WindowAction(action) => update::window::handle_window_action(self, action),
            Message::RefreshNetworks => update::network::handle_refresh_networks(self),
            Message::NetworksRefreshed(networks) => {
                update::network::handle_networks_refreshed(self, networks)
            }
            Message::SelectNetwork(network_id) => {
                update::network::handle_select_network(self, network_id)
            }
            Message::StartScan => update::scan::handle_start_scan(self),
            Message::ScanProgress {
                session_id,
                scanned,
                total,
            } => update::scan::handle_scan_progress(self, session_id, scanned, total),
            Message::ScanOnlineDatasetReady {
                session_id,
                evidence_by_ip,
            } => update::scan::handle_scan_online_dataset_ready(self, session_id, evidence_by_ip),
            Message::ScanDeviceDiscovered { session_id, device } => {
                update::scan::handle_scan_device_discovered(self, session_id, device)
            }
            Message::ScanFinished { session_id } => {
                update::scan::handle_scan_finished(self, session_id)
            }
            Message::ScanSshProbeFinished { session_id, report } => {
                update::scan::handle_scan_ssh_probe_finished(self, session_id, report)
            }
            Message::CancelScan => update::scan::handle_cancel_scan(self),
            Message::SelectDevice(device_id) => {
                update::network::handle_select_device(self, device_id)
            }
            Message::CloseDetail => update::network::handle_close_detail(self),
            Message::NetworkDropdownOpened => update::network::handle_network_dropdown_opened(self),
            Message::NetworkDropdownClosed => update::network::handle_network_dropdown_closed(self),
            Message::UserDropdownOpened => update::credential::handle_user_dropdown_opened(self),
            Message::UserDropdownClosed => update::credential::handle_user_dropdown_closed(self),
            Message::SetUsername(value) => update::credential::handle_set_username(self, value),
            Message::SelectUser(username) => update::credential::handle_select_user(self, username),
            Message::SetPassword(password) => {
                update::credential::handle_set_password(self, password)
            }
            Message::ShowAllOnlineResults => {
                if self.scan_result_filter != ScanResultFilter::AllOnline {
                    self.scan_result_filter = ScanResultFilter::AllOnline;
                    update::scan::rebuild_visible_devices_from_online(self);
                }
                Task::none()
            }
            Message::ShowSshReadyResults => {
                if self.scan_result_filter != ScanResultFilter::SshReady {
                    self.scan_result_filter = ScanResultFilter::SshReady;
                    update::scan::rebuild_visible_devices_from_online(self);
                }
                Task::none()
            }
            Message::ToggleVnc => update::credential::handle_toggle_vnc(self),
            Message::SetVncUser(value) => update::credential::handle_set_vnc_user(self, value),
            Message::SetVncPassword(value) => {
                update::credential::handle_set_vnc_password(self, value)
            }
            Message::CloseCredModal => update::modal::handle_close_cred_modal(self),
            Message::SetNewCredentialUsername(value) => {
                update::credential::handle_set_new_credential_username(self, value)
            }
            Message::SetNewCredentialPassword(value) => {
                update::credential::handle_set_new_credential_password(self, value)
            }
            Message::StartEditCredential(username) => {
                update::credential::handle_start_edit_credential(self, username)
            }
            Message::CancelEditCredential => {
                update::credential::handle_cancel_edit_credential(self)
            }
            Message::AddCredential(username, password) => {
                update::credential::handle_add_credential(self, username, password)
            }
            Message::UpdateCredentialPassword(username, password) => {
                update::credential::handle_update_credential_password(self, username, password)
            }
            Message::RemoveCredential(id) => update::credential::handle_remove_credential(self, id),
            Message::StartVerify => update::scan::handle_start_verify(self),
            Message::VerifyResult {
                session_id,
                ip,
                status,
            } => update::scan::handle_verify_result(self, session_id, ip, status),
            Message::VerifyComplete { session_id } => {
                update::scan::handle_verify_complete(self, session_id)
            }
            Message::ConnectShell(device_ip) => {
                update::connect::handle_connect_shell(self, device_ip)
            }
            Message::ConnectVSCode(device_ip) => {
                update::connect::handle_connect_vscode(self, device_ip)
            }
            Message::ConnectMobaXterm(device_ip) => {
                update::connect::handle_connect_mobaxterm(self, device_ip)
            }
            Message::ConnectVNC(device_ip) => update::connect::handle_connect_vnc(self, device_ip),
            Message::ConnectDocker(device_ip) => {
                update::connect::handle_connect_docker(self, device_ip)
            }
            Message::ConnectRustDesk(device_ip) => {
                update::connect::handle_connect_rustdesk(self, device_ip)
            }
            Message::RustDeskProbeFinished {
                device_ip,
                password,
                result,
            } => update::connect::handle_rustdesk_probe_finished(self, device_ip, password, result),
            Message::ConnectResult(_tool, result) => {
                update::connect::handle_connect_result(self, result)
            }
            Message::DismissNotice(version) => {
                if self.notice.is_some() && version == self.notice_version {
                    self.clear_notice();
                }
                Task::none()
            }
            Message::RequestToolPath(tool) => update::connect::handle_request_tool_path(self, tool),
            Message::ToolPathPicked(tool, selected_path) => {
                update::connect::handle_tool_path_picked(self, tool, selected_path)
            }
            Message::ToolPathPickFailed(tool, error) => {
                update::connect::handle_tool_path_pick_failed(self, tool, error)
            }
            Message::DockerContainersLoaded(containers) => {
                update::connect::handle_docker_containers_loaded(self, containers)
            }
            Message::DockerContainersLoadFailed(message) => {
                update::connect::handle_docker_containers_load_failed(self, message)
            }
            Message::SelectContainer(container_id) => {
                update::modal::handle_select_container(self, container_id)
            }
            Message::AttachSelectedContainer => {
                update::connect::handle_attach_selected_container(self)
            }
            Message::CloseDockerModal => update::modal::handle_close_docker_modal(self),
            Message::VisualCheckFrameTick => update::visual_check::handle_tick(self),
            Message::VisualCheckCapture => update::visual_check::capture_scene(self),
            Message::VisualCheckCaptured(screenshot) => {
                match update::visual_check::save_screenshot(self, &screenshot) {
                    Ok(path) => {
                        eprintln!("[visual-check] screenshot saved: {}", path.display());
                        update::visual_check::advance_or_exit(self)
                    }
                    Err(error) => {
                        eprintln!("[ERROR] visual check screenshot failed: {error}");
                        update::visual_check::close_window(self)
                    }
                }
            }
            Message::VisualCheckFailed(error) => {
                eprintln!("[ERROR] visual check failed: {error}");
                update::visual_check::close_window(self)
            }
            Message::Tick => {
                self.spinner_phase = (self.spinner_phase + 1) % SPINNER_FRAMES.len();
                Task::none()
            }
            Message::Noop => Task::none(),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        view::view(self)
    }

    fn initialize_visual_check(&mut self, config: VisualCheckConfig) {
        update::visual_check::initialize_visual_check(self, config);
    }

    fn replace_credentials(
        &mut self,
        credentials: Vec<Credential>,
        preferred_username: Option<String>,
        preferred_password: Option<String>,
    ) {
        let previous_verify_input = self.current_verify_input_signature();
        let previous_selected_username = self.selected_username.clone();

        self.credentials = credentials;

        if let Some(username) = preferred_username {
            self.ssh_username = username.clone();
            self.selected_username = credential::find_by_username(&self.credentials, &username)
                .map(|_| username.clone());
            let stored_password = credential::find_by_username(&self.credentials, &username)
                .and_then(|credential| credential.password.clone());
            self.password = preferred_password.or(stored_password).unwrap_or_default();
            if self.current_verify_input_signature() != previous_verify_input {
                self.clear_verification_state_for_credential_change();
            }
            return;
        }

        self.sync_selected_username_from_input();

        if let Some(username) = self.selected_username.as_deref()
            && previous_selected_username.as_deref() == Some(username)
            && let Some(stored_password) = credential::find_by_username(&self.credentials, username)
                .and_then(|credential| credential.password.clone())
        {
            self.password = stored_password;
        }

        if self.current_verify_input_signature() != previous_verify_input {
            self.clear_verification_state_for_credential_change();
        }
    }

    fn reset_credential_form(&mut self) {
        self.editing_credential_username = None;
        self.new_credential_username.clear();
        self.new_credential_password.clear();
    }

    fn apply_selected_username(&mut self, username: String) {
        let current_password = self.password.clone();
        let next_password = credential::find_by_username(&self.credentials, &username)
            .and_then(|credential| credential.password.clone())
            .unwrap_or_else(|| {
                if self.selected_username.as_deref() == Some(username.as_str()) {
                    current_password
                } else {
                    String::new()
                }
            });

        self.ssh_username = username.clone();
        self.selected_username = Some(username);
        self.password = next_password;
    }

    fn clear_verification_state_for_credential_change(&mut self) {
        update::scan::reset_verify_runtime(self);
        for layered in &mut self.online_devices {
            if layered.ssh_port_status == SshPortProbeStatus::Open
                && layered.device.status != DeviceStatus::Untested
            {
                layered.device.status = DeviceStatus::Untested;
            }
        }
        update::scan::rebuild_visible_devices_from_online(self);
    }

    fn spinner_frame(&self) -> &'static str {
        SPINNER_FRAMES[self.spinner_phase % SPINNER_FRAMES.len()]
    }

    fn network_dropdown_toggle_message(&self) -> Option<Message> {
        (!self.networks.is_empty()
            && !self.is_scanning
            && !self.is_refreshing_networks
            && !self.is_verifying
            && !self.is_connecting
            && self.active_modal.is_none())
        .then_some(if self.network_dropdown_open {
            Message::NetworkDropdownClosed
        } else {
            Message::NetworkDropdownOpened
        })
    }

    fn user_dropdown_toggle_message(&self) -> Option<Message> {
        (!self.credentials.is_empty()
            && !self.is_verifying
            && !self.is_connecting
            && self.active_modal.is_none())
        .then_some(if self.user_dropdown_open {
            Message::UserDropdownClosed
        } else {
            Message::UserDropdownOpened
        })
    }

    fn can_start_verify(&self) -> bool {
        update::scan::can_start_verify(self)
    }

    fn close_overlays(&mut self) {
        self.network_dropdown_open = false;
        self.user_dropdown_open = false;
    }

    fn save_credential_message(&self) -> Option<Message> {
        let username = self.new_credential_username.trim();
        let password = self.new_credential_password.trim();

        if let Some(editing_username) = self.editing_credential_username.as_deref() {
            return (!password.is_empty()).then(|| {
                Message::UpdateCredentialPassword(editing_username.to_owned(), password.to_owned())
            });
        }

        if username.is_empty()
            || credential::find_by_username(&self.credentials, username).is_some()
        {
            return None;
        }

        Some(Message::AddCredential(
            username.to_owned(),
            (!password.is_empty()).then_some(password.to_owned()),
        ))
    }

    fn handle_ssh_intent_updated(&mut self) -> Task<Message> {
        Task::none()
    }

    fn cancel_scan_runtime(&mut self) {
        update::scan::cancel_scan_runtime(self);
    }

    fn rebuild_visible_devices_from_online(&mut self) {
        update::scan::rebuild_visible_devices_from_online(self);
    }

    fn preferred_scan_result_filter(&self) -> ScanResultFilter {
        update::scan::preferred_scan_result_filter(self)
    }
}

impl Drop for ShellApp {
    fn drop(&mut self) {
        if let Err(error) = key_mgmt::cleanup_external_temp_keys_on_shutdown() {
            eprintln!("[ERROR] cleanup external key temp dirs on shutdown failed: {error}");
        }
    }
}

fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        let url_wide: Vec<u16> = OsStr::new(url).encode_wide().chain(Some(0)).collect();
        let verb: Vec<u16> = OsStr::new("open").encode_wide().chain(Some(0)).collect();
        unsafe {
            windows_sys::Win32::UI::Shell::ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                url_wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
            );
        }
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
