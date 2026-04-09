use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use iced::task::Handle;
use ssh_core::credential::Credential;
use ssh_core::credential::store::{AppPaths, ToolKind};
use ssh_core::docker::Container;
use ssh_core::network::NetworkInterface;
use ssh_core::scanner::{Device, LayeredScanDevice, NeighborEvidence};
use tokio_util::sync::CancellationToken;
use ui::theme::{AppLanguage, ThemeMode};

use crate::visual_check::VisualScene;

pub(super) enum ActiveModal {
    HelpGuide,
    CredentialManagement,
    DockerSelect(DockerModalState),
}

pub(super) struct DockerModalState {
    pub(super) containers: Vec<Container>,
    pub(super) selected_container_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct ActiveQuickConnectState {
    pub(super) device_ip: String,
    pub(super) launcher_key: &'static str,
}

#[derive(Debug, Clone)]
pub(super) struct LaunchContext {
    pub(super) device_ip: String,
    pub(super) username: String,
    pub(super) password: Option<String>,
    pub(super) vnc_requested: bool,
    pub(super) vnc_username: Option<String>,
    pub(super) vnc_password: Option<String>,
    pub(super) vnc_username_source: VncCredentialFieldSource,
    pub(super) vnc_password_source: VncCredentialFieldSource,
    pub(super) vnc_source: VncCredentialSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VncCredentialFieldSource {
    Dedicated,
    SshFallback,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VncCredentialSource {
    Dedicated,
    SshFallback,
    Unavailable,
}

impl LaunchContext {
    pub(super) fn vnc_resolution_message(&self) -> Option<String> {
        self.vnc_resolution_message_for_language(AppLanguage::Chinese)
    }

    pub(super) fn vnc_resolution_message_for_language(
        &self,
        language: AppLanguage,
    ) -> Option<String> {
        if !self.vnc_requested {
            return None;
        }

        let mut notes = Vec::new();

        if self.vnc_username_source == VncCredentialFieldSource::SshFallback {
            notes.push(match language {
                AppLanguage::Chinese => String::from("VNC 用户名未单独填写，已回退 SSH 用户名"),
                AppLanguage::English => String::from(
                    "VNC username not provided separately; falling back to SSH username",
                ),
            });
        }
        if self.vnc_username_source == VncCredentialFieldSource::Unavailable {
            notes.push(match language {
                AppLanguage::Chinese => String::from("当前没有可用的 VNC 用户名"),
                AppLanguage::English => String::from("No VNC username is currently available"),
            });
        }
        if self.vnc_password_source == VncCredentialFieldSource::SshFallback {
            notes.push(match language {
                AppLanguage::Chinese => String::from("VNC 密码未单独填写，已回退 SSH 密码"),
                AppLanguage::English => String::from(
                    "VNC password not provided separately; falling back to SSH password",
                ),
            });
        }
        if self.vnc_password_source == VncCredentialFieldSource::Unavailable {
            notes.push(match language {
                AppLanguage::Chinese => {
                    String::from("当前没有可用的 VNC 密码，后续如需认证请手动输入")
                }
                AppLanguage::English => {
                    String::from("No VNC password is currently available; enter it manually if authentication is required")
                }
            });
        }
        if self.vnc_source == VncCredentialSource::Unavailable && notes.is_empty() {
            notes.push(match language {
                AppLanguage::Chinese => String::from("当前没有可直接复用的 VNC 凭据"),
                AppLanguage::English => {
                    String::from("There are no reusable VNC credentials available right now")
                }
            });
        }

        let delimiter = match language {
            AppLanguage::Chinese => "；",
            AppLanguage::English => "; ",
        };

        (!notes.is_empty()).then(|| notes.join(delimiter))
    }
}

#[derive(Debug, Clone)]
pub(super) enum PendingToolAction {
    Direct {
        tool: ToolKind,
        context: LaunchContext,
    },
    DockerAttach {
        context: LaunchContext,
        container: Container,
    },
}

impl PendingToolAction {
    pub(super) fn tool_kind(&self) -> ToolKind {
        match self {
            Self::Direct { tool, .. } => *tool,
            Self::DockerAttach { .. } => ToolKind::Vscode,
        }
    }

    pub(super) fn status_message(&self) -> String {
        self.status_message_for_language(AppLanguage::Chinese)
    }

    pub(super) fn status_message_for_language(&self, language: AppLanguage) -> String {
        match self {
            Self::Direct { tool, .. } => match language {
                AppLanguage::Chinese => format!("正在启动 {} 连接", tool.label()),
                AppLanguage::English => format!("Launching {} connection", tool.label()),
            },
            Self::DockerAttach { container, .. } => match language {
                AppLanguage::Chinese => format!("正在准备 Docker 容器 {}", container.name),
                AppLanguage::English => {
                    format!("Preparing Docker container {}", container.name)
                }
            },
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct Notice {
    pub(super) tone: NoticeTone,
    pub(super) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NoticeTone {
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub(super) enum VerifyCredentialInput {
    Empty,
    PasswordOnly,
    UsernameOnly { username: String },
    UsernamePassword { username: String, password: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScanResultFilter {
    AllOnline,
    SshReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScanPhase {
    DiscoverOnline,
    ProbeSsh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VisualCheckStage {
    WaitingForWindow,
    WaitingForResize(u8),
    Settling(u8),
    Capturing,
}

#[derive(Debug, Clone)]
pub(super) struct VisualCheckRuntime {
    pub(super) output_dir: PathBuf,
    pub(super) current_scene: VisualScene,
    pub(super) pending_scenes: VecDeque<VisualScene>,
    pub(super) stage: VisualCheckStage,
}

pub struct ShellApp {
    pub(super) network_dropdown_open: bool,
    pub(super) user_dropdown_open: bool,
    pub(super) networks: Vec<NetworkInterface>,
    pub(super) selected_network_id: Option<String>,
    pub(super) is_scanning: bool,
    pub(super) is_refreshing_networks: bool,
    pub(super) online_devices: Vec<LayeredScanDevice>,
    pub(super) online_evidence_by_ip: HashMap<String, NeighborEvidence>,
    pub(super) devices: Vec<Device>,
    pub(super) scan_result_filter: ScanResultFilter,
    pub(super) selected_device_id: Option<String>,
    pub(super) has_scanned: bool,
    pub(super) scan_progress: Option<(usize, usize)>,
    pub(super) verify_progress: Option<(usize, usize)>,
    pub(super) scan_session_id: u64,
    pub(super) scan_phase: Option<ScanPhase>,
    pub(super) scan_auto_verify_enabled: bool,
    pub(super) scan_cancel_token: Option<CancellationToken>,
    pub(super) scan_task_handle: Option<Handle>,
    pub(super) verify_inflight_ips: HashSet<String>,
    pub(super) verified_ips: HashSet<String>,
    pub(super) verify_enqueued_count: usize,
    pub(super) verify_completed_count: usize,
    pub(super) networks_signature: u64,
    pub(super) spinner_phase: usize,
    pub(super) app_language: AppLanguage,
    pub(super) theme_mode: ThemeMode,
    pub(super) window_id: Option<iced::window::Id>,
    pub(super) is_window_maximized: bool,
    pub(super) credentials: Vec<Credential>,
    pub(super) app_paths: AppPaths,
    pub(super) ssh_username: String,
    pub(super) selected_username: Option<String>,
    pub(super) password: String,
    pub(super) vnc_enabled: bool,
    pub(super) vnc_user: String,
    pub(super) vnc_password: String,
    pub(super) active_modal: Option<ActiveModal>,
    pub(super) help_modal_show_rustdesk: bool,
    pub(super) editing_credential_username: Option<String>,
    pub(super) new_credential_username: String,
    pub(super) new_credential_password: String,
    pub(super) is_verifying: bool,
    pub(super) is_connecting: bool,
    pub(super) pending_tool_action: Option<PendingToolAction>,
    pub(super) pending_docker_context: Option<LaunchContext>,
    pub(super) active_quick_connect: Option<ActiveQuickConnectState>,
    pub(super) connection_status: Option<String>,
    pub(super) notice: Option<Notice>,
    pub(super) notice_version: u64,
    pub(super) visual_check: Option<VisualCheckRuntime>,
}
