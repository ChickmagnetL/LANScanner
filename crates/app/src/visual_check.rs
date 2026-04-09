use std::path::PathBuf;

use ssh_core::credential::Credential;
use ssh_core::docker::Container;
use ssh_core::network::{InterfaceType, NetworkInterface};
use ssh_core::scanner::{Device, DeviceIdentityKind, DeviceStatus, DeviceType};
use ui::theme::AppLanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualScene {
    Idle,
    DarkMode,
    Scanning,
    Verifying,
    SelectedDevice,
    NetworkDropdown,
    UserDropdown,
    RustDeskCredential,
    HelpModal,
    HelpModalDark,
    DockerModal,
    CredentialModal,
    CredentialEditing,
}

impl VisualScene {
    pub const MINIMAL_SET: [Self; 13] = [
        Self::Idle,
        Self::DarkMode,
        Self::Scanning,
        Self::Verifying,
        Self::SelectedDevice,
        Self::NetworkDropdown,
        Self::UserDropdown,
        Self::RustDeskCredential,
        Self::HelpModal,
        Self::HelpModalDark,
        Self::DockerModal,
        Self::CredentialModal,
        Self::CredentialEditing,
    ];

    pub fn is_help(self: &Self) -> bool {
        matches!(self, Self::HelpModal | Self::HelpModalDark)
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "idle" => Some(Self::Idle),
            "dark-mode" => Some(Self::DarkMode),
            "scanning" => Some(Self::Scanning),
            "verifying" => Some(Self::Verifying),
            "selected-device" => Some(Self::SelectedDevice),
            "network-dropdown" => Some(Self::NetworkDropdown),
            "user-dropdown" => Some(Self::UserDropdown),
            "rustdesk-credential" => Some(Self::RustDeskCredential),
            "help-modal" => Some(Self::HelpModal),
            "help-modal-dark" => Some(Self::HelpModalDark),
            "docker-modal" => Some(Self::DockerModal),
            "credential-modal" => Some(Self::CredentialModal),
            "credential-editing" => Some(Self::CredentialEditing),
            _ => None,
        }
    }

    pub fn file_name(self) -> &'static str {
        match self {
            Self::Idle => "idle.png",
            Self::DarkMode => "dark-mode.png",
            Self::Scanning => "scanning.png",
            Self::Verifying => "verifying.png",
            Self::SelectedDevice => "selected-device.png",
            Self::NetworkDropdown => "network-dropdown.png",
            Self::UserDropdown => "user-dropdown.png",
            Self::RustDeskCredential => "rustdesk-credential.png",
            Self::HelpModal => "help-modal.png",
            Self::HelpModalDark => "help-modal-dark.png",
            Self::DockerModal => "docker-modal.png",
            Self::CredentialModal => "credential-modal.png",
            Self::CredentialEditing => "credential-editing.png",
        }
    }
}

#[derive(Debug, Clone)]
pub struct VisualCheckConfig {
    pub scenes: Vec<VisualScene>,
    pub output_dir: PathBuf,
}

impl VisualCheckConfig {
    pub fn needs_large_window(&self) -> bool {
        self.scenes.iter().any(VisualScene::is_help)
    }

    pub fn from_args(scene_arg: &str, output_dir: Option<PathBuf>) -> Result<Self, String> {
        let scenes = if scene_arg.eq_ignore_ascii_case("all") {
            VisualScene::MINIMAL_SET.to_vec()
        } else {
            let scene = VisualScene::parse(scene_arg).ok_or_else(|| {
                format!(
                    "invalid scene `{scene_arg}`; expected idle|dark-mode|scanning|verifying|selected-device|network-dropdown|user-dropdown|rustdesk-credential|help-modal|help-modal-dark|docker-modal|credential-modal|credential-editing|all"
                )
            })?;
            vec![scene]
        };

        Ok(Self {
            scenes,
            output_dir: output_dir.unwrap_or_else(default_output_dir),
        })
    }
}

#[derive(Debug, Clone)]
pub enum VisualModalPreset {
    None,
    Help {
        show_rustdesk_section: bool,
    },
    Credential,
    Docker {
        containers: Vec<Container>,
        selected_container_id: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct VisualScenePreset {
    pub language: AppLanguage,
    pub dark_mode: bool,
    pub networks: Vec<NetworkInterface>,
    pub selected_network_id: Option<String>,
    pub network_dropdown_open: bool,
    pub user_dropdown_open: bool,
    pub is_scanning: bool,
    pub is_refreshing_networks: bool,
    pub is_verifying: bool,
    pub has_scanned: bool,
    pub scan_progress: Option<(usize, usize)>,
    pub devices: Vec<Device>,
    pub selected_device_id: Option<String>,
    pub verify_progress: Option<(usize, usize)>,
    pub selected_username: Option<String>,
    pub ssh_username: String,
    pub password: String,
    pub vnc_enabled: bool,
    pub vnc_user: String,
    pub vnc_password: String,
    pub modal: VisualModalPreset,
    pub editing_credential_username: Option<String>,
    pub new_credential_username: String,
    pub new_credential_password: String,
    pub connection_status: Option<String>,
    pub notice: Option<String>,
}

pub fn scene_preset(scene: VisualScene, credentials: &[Credential]) -> VisualScenePreset {
    let language = scene_language(scene);
    let base_networks = sample_networks(language);
    let base_selected_network_id = base_networks.first().map(|network| network.id.clone());
    let default_selected_username = preferred_username(credentials, "root");
    let default_ssh_username = default_selected_username.clone().unwrap_or_default();
    let default_password = default_selected_username
        .as_deref()
        .and_then(|username| password_for(credentials, username))
        .unwrap_or_else(|| String::from("demo-password"));

    match scene {
        VisualScene::Idle => VisualScenePreset {
            language,
            dark_mode: false,
            networks: base_networks.clone(),
            selected_network_id: base_selected_network_id.clone(),
            network_dropdown_open: false,
            user_dropdown_open: false,
            is_scanning: false,
            is_refreshing_networks: false,
            is_verifying: false,
            has_scanned: false,
            scan_progress: None,
            devices: Vec::new(),
            selected_device_id: None,
            verify_progress: None,
            selected_username: None,
            ssh_username: String::new(),
            password: String::new(),
            vnc_enabled: false,
            vnc_user: String::new(),
            vnc_password: String::new(),
            modal: VisualModalPreset::None,
            editing_credential_username: None,
            new_credential_username: String::new(),
            new_credential_password: String::new(),
            connection_status: None,
            notice: None,
        },
        VisualScene::DarkMode => VisualScenePreset {
            language,
            dark_mode: true,
            networks: base_networks.clone(),
            selected_network_id: base_selected_network_id.clone(),
            network_dropdown_open: false,
            user_dropdown_open: false,
            is_scanning: false,
            is_refreshing_networks: false,
            is_verifying: false,
            has_scanned: false,
            scan_progress: None,
            devices: Vec::new(),
            selected_device_id: None,
            verify_progress: None,
            selected_username: None,
            ssh_username: String::new(),
            password: String::new(),
            vnc_enabled: true,
            vnc_user: String::from("viewer"),
            vnc_password: String::new(),
            modal: VisualModalPreset::None,
            editing_credential_username: None,
            new_credential_username: String::new(),
            new_credential_password: String::new(),
            connection_status: None,
            notice: None,
        },
        VisualScene::Scanning => VisualScenePreset {
            language,
            dark_mode: false,
            networks: base_networks.clone(),
            selected_network_id: base_selected_network_id.clone(),
            network_dropdown_open: false,
            user_dropdown_open: false,
            is_scanning: true,
            is_refreshing_networks: false,
            is_verifying: false,
            has_scanned: false,
            scan_progress: Some((26, 128)),
            devices: Vec::new(),
            selected_device_id: None,
            verify_progress: None,
            selected_username: default_selected_username.clone(),
            ssh_username: default_ssh_username.clone(),
            password: default_password.clone(),
            vnc_enabled: false,
            vnc_user: String::new(),
            vnc_password: String::new(),
            modal: VisualModalPreset::None,
            editing_credential_username: None,
            new_credential_username: String::new(),
            new_credential_password: String::new(),
            connection_status: None,
            notice: None,
        },
        VisualScene::Verifying => VisualScenePreset {
            language,
            dark_mode: false,
            networks: base_networks.clone(),
            selected_network_id: base_selected_network_id.clone(),
            network_dropdown_open: false,
            user_dropdown_open: false,
            is_scanning: false,
            is_refreshing_networks: false,
            is_verifying: true,
            has_scanned: false,
            scan_progress: None,
            devices: Vec::new(),
            selected_device_id: None,
            verify_progress: Some((1, 3)),
            selected_username: default_selected_username.clone(),
            ssh_username: default_ssh_username.clone(),
            password: default_password.clone(),
            vnc_enabled: false,
            vnc_user: String::new(),
            vnc_password: String::new(),
            modal: VisualModalPreset::None,
            editing_credential_username: None,
            new_credential_username: String::new(),
            new_credential_password: String::new(),
            connection_status: None,
            notice: Some(match language {
                AppLanguage::Chinese => String::from("扫描完成后自动检测SSH凭证中"),
                AppLanguage::English => {
                    String::from("Scan complete. Automatically verifying SSH credentials")
                }
            }),
        },
        VisualScene::SelectedDevice => {
            let devices = sample_devices();
            let selected_device_id = devices
                .iter()
                .find(|device| device.status == DeviceStatus::Ready)
                .or_else(|| devices.first())
                .map(|device| device.id.clone());

            VisualScenePreset {
                language,
                dark_mode: false,
                networks: base_networks.clone(),
                selected_network_id: base_selected_network_id.clone(),
                network_dropdown_open: false,
                user_dropdown_open: false,
                is_scanning: false,
                is_refreshing_networks: false,
                is_verifying: false,
                has_scanned: true,
                scan_progress: None,
                devices,
                selected_device_id,
                verify_progress: None,
                selected_username: default_selected_username.clone(),
                ssh_username: default_ssh_username.clone(),
                password: default_password.clone(),
                vnc_enabled: true,
                vnc_user: String::from("vnc-operator"),
                vnc_password: String::new(),
                modal: VisualModalPreset::None,
                editing_credential_username: None,
                new_credential_username: String::new(),
                new_credential_password: String::new(),
                connection_status: None,
                notice: None,
            }
        }
        VisualScene::NetworkDropdown => VisualScenePreset {
            language,
            dark_mode: false,
            networks: base_networks.clone(),
            selected_network_id: base_selected_network_id.clone(),
            network_dropdown_open: true,
            user_dropdown_open: false,
            is_scanning: false,
            is_refreshing_networks: false,
            is_verifying: false,
            has_scanned: false,
            scan_progress: None,
            devices: Vec::new(),
            selected_device_id: None,
            verify_progress: None,
            selected_username: default_selected_username.clone(),
            ssh_username: default_ssh_username.clone(),
            password: default_password.clone(),
            vnc_enabled: false,
            vnc_user: String::new(),
            vnc_password: String::new(),
            modal: VisualModalPreset::None,
            editing_credential_username: None,
            new_credential_username: String::new(),
            new_credential_password: String::new(),
            connection_status: None,
            notice: None,
        },
        VisualScene::UserDropdown => {
            let selected_username = preferred_username(credentials, "pi");
            let ssh_username = selected_username.clone().unwrap_or_default();
            let password = selected_username
                .as_deref()
                .and_then(|username| password_for(credentials, username))
                .unwrap_or_default();

            VisualScenePreset {
                language,
                dark_mode: false,
                networks: base_networks.clone(),
                selected_network_id: base_selected_network_id.clone(),
                network_dropdown_open: false,
                user_dropdown_open: true,
                is_scanning: false,
                is_refreshing_networks: false,
                is_verifying: false,
                has_scanned: false,
                scan_progress: None,
                devices: Vec::new(),
                selected_device_id: None,
                verify_progress: None,
                selected_username,
                ssh_username,
                password,
                vnc_enabled: false,
                vnc_user: String::new(),
                vnc_password: String::new(),
                modal: VisualModalPreset::None,
                editing_credential_username: None,
                new_credential_username: String::new(),
                new_credential_password: String::new(),
                connection_status: None,
                notice: None,
            }
        }
        VisualScene::RustDeskCredential => {
            let selected_username = preferred_username(credentials, "admin")
                .or_else(|| preferred_username(credentials, "pi"));
            let ssh_username = selected_username.clone().unwrap_or_default();

            VisualScenePreset {
                language,
                dark_mode: false,
                networks: base_networks.clone(),
                selected_network_id: base_selected_network_id.clone(),
                network_dropdown_open: false,
                user_dropdown_open: false,
                is_scanning: false,
                is_refreshing_networks: false,
                is_verifying: false,
                has_scanned: false,
                scan_progress: None,
                devices: Vec::new(),
                selected_device_id: None,
                verify_progress: None,
                selected_username,
                ssh_username,
                password: String::new(),
                vnc_enabled: true,
                vnc_user: String::new(),
                vnc_password: String::new(),
                modal: VisualModalPreset::None,
                editing_credential_username: None,
                new_credential_username: String::new(),
                new_credential_password: String::new(),
                connection_status: None,
                notice: None,
            }
        }
        VisualScene::HelpModal | VisualScene::HelpModalDark => {
            let devices = sample_devices();
            let selected_device_id = devices
                .iter()
                .find(|device| device.status == DeviceStatus::Ready)
                .or_else(|| devices.first())
                .map(|device| device.id.clone());

            VisualScenePreset {
                language,
                dark_mode: matches!(scene, VisualScene::HelpModalDark),
                networks: base_networks.clone(),
                selected_network_id: base_selected_network_id.clone(),
                network_dropdown_open: false,
                user_dropdown_open: false,
                is_scanning: false,
                is_refreshing_networks: false,
                is_verifying: false,
                has_scanned: true,
                scan_progress: None,
                devices,
                selected_device_id,
                verify_progress: None,
                selected_username: default_selected_username.clone(),
                ssh_username: default_ssh_username.clone(),
                password: default_password.clone(),
                vnc_enabled: false,
                vnc_user: String::new(),
                vnc_password: String::from("optional-pass"),
                modal: VisualModalPreset::Help {
                    show_rustdesk_section: true,
                },
                editing_credential_username: None,
                new_credential_username: String::new(),
                new_credential_password: String::new(),
                connection_status: None,
                notice: None,
            }
        }
        VisualScene::DockerModal => {
            let devices = sample_devices();
            let selected_device_id = devices
                .iter()
                .find(|device| device.status == DeviceStatus::Ready)
                .or_else(|| devices.first())
                .map(|device| device.id.clone());
            let containers = sample_containers(language);

            VisualScenePreset {
                language,
                dark_mode: false,
                networks: base_networks.clone(),
                selected_network_id: base_selected_network_id.clone(),
                network_dropdown_open: false,
                user_dropdown_open: false,
                is_scanning: false,
                is_refreshing_networks: false,
                is_verifying: false,
                has_scanned: true,
                scan_progress: None,
                devices,
                selected_device_id,
                verify_progress: None,
                selected_username: default_selected_username.clone(),
                ssh_username: default_ssh_username.clone(),
                password: default_password.clone(),
                vnc_enabled: false,
                vnc_user: String::new(),
                vnc_password: String::new(),
                modal: VisualModalPreset::Docker {
                    selected_container_id: containers.first().map(|container| container.id.clone()),
                    containers,
                },
                editing_credential_username: None,
                new_credential_username: String::new(),
                new_credential_password: String::new(),
                connection_status: None,
                notice: None,
            }
        }
        VisualScene::CredentialModal => VisualScenePreset {
            language,
            dark_mode: false,
            networks: base_networks.clone(),
            selected_network_id: base_selected_network_id.clone(),
            network_dropdown_open: false,
            user_dropdown_open: false,
            is_scanning: false,
            is_refreshing_networks: false,
            is_verifying: false,
            has_scanned: false,
            scan_progress: None,
            devices: Vec::new(),
            selected_device_id: None,
            verify_progress: None,
            selected_username: None,
            ssh_username: String::new(),
            password: String::new(),
            vnc_enabled: false,
            vnc_user: String::new(),
            vnc_password: String::new(),
            modal: VisualModalPreset::Credential,
            editing_credential_username: None,
            new_credential_username: String::new(),
            new_credential_password: String::new(),
            connection_status: None,
            notice: None,
        },
        VisualScene::CredentialEditing => {
            let editing_username =
                preferred_username(credentials, "pi").or_else(|| default_selected_username.clone());
            let new_credential_username = editing_username.clone().unwrap_or_default();
            let ssh_username = default_selected_username.clone().unwrap_or_default();

            VisualScenePreset {
                language,
                dark_mode: false,
                networks: base_networks.clone(),
                selected_network_id: base_selected_network_id,
                network_dropdown_open: false,
                user_dropdown_open: false,
                is_scanning: false,
                is_refreshing_networks: false,
                is_verifying: false,
                has_scanned: false,
                scan_progress: None,
                devices: Vec::new(),
                selected_device_id: None,
                verify_progress: None,
                selected_username: default_selected_username,
                ssh_username,
                password: String::new(),
                vnc_enabled: false,
                vnc_user: String::new(),
                vnc_password: String::new(),
                modal: VisualModalPreset::Credential,
                editing_credential_username: editing_username,
                new_credential_username,
                new_credential_password: String::from("new-password"),
                connection_status: None,
                notice: None,
            }
        }
    }
}

fn scene_language(scene: VisualScene) -> AppLanguage {
    match scene {
        VisualScene::Verifying
        | VisualScene::SelectedDevice
        | VisualScene::NetworkDropdown
        | VisualScene::UserDropdown
        | VisualScene::RustDeskCredential
        | VisualScene::HelpModal
        | VisualScene::HelpModalDark
        | VisualScene::DockerModal
        | VisualScene::CredentialModal
        | VisualScene::CredentialEditing => AppLanguage::English,
        _ => AppLanguage::Chinese,
    }
}

fn preferred_username(credentials: &[Credential], preferred: &str) -> Option<String> {
    credentials
        .iter()
        .find(|credential| credential.username == preferred)
        .map(|credential| credential.username.clone())
        .or_else(|| {
            credentials
                .first()
                .map(|credential| credential.username.clone())
        })
        .or_else(|| Some(preferred.to_owned()))
}

fn password_for(credentials: &[Credential], username: &str) -> Option<String> {
    credentials
        .iter()
        .find(|credential| credential.username == username)
        .and_then(|credential| credential.password.clone())
        .or_else(|| Some(String::from("demo-password")))
}

pub fn default_output_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    cwd.join(".visual-check")
}

fn sample_networks(language: AppLanguage) -> Vec<NetworkInterface> {
    vec![
        NetworkInterface {
            id: String::from("wifi0"),
            name: String::from("Wi-Fi (Home)"),
            ip_range: String::from("192.168.1.0/24"),
            iface_type: InterfaceType::Wifi,
            local_ip: String::from("192.168.1.88"),
        },
        NetworkInterface {
            id: String::from("lan0"),
            name: match language {
                AppLanguage::Chinese => String::from("以太网 (Office)"),
                AppLanguage::English => String::from("Ethernet (Office)"),
            },
            ip_range: String::from("10.0.0.0/24"),
            iface_type: InterfaceType::Ethernet,
            local_ip: String::from("10.0.0.23"),
        },
        NetworkInterface {
            id: String::from("docker0"),
            name: String::from("Docker Bridge"),
            ip_range: String::from("172.17.0.0/16"),
            iface_type: InterfaceType::Docker,
            local_ip: String::from("172.17.0.1"),
        },
    ]
}

fn sample_devices() -> Vec<Device> {
    vec![
        Device {
            id: String::from("192.168.31.12"),
            name: String::from("Raspberry Pi"),
            ip: String::from("192.168.31.12"),
            identity_kind: DeviceIdentityKind::RaspberryPi,
            device_type: DeviceType::Server,
            status: DeviceStatus::Ready,
        },
        Device {
            id: String::from("192.168.31.28"),
            name: String::from("NVIDIA Jetson"),
            ip: String::from("192.168.31.28"),
            identity_kind: DeviceIdentityKind::Jetson,
            device_type: DeviceType::Server,
            status: DeviceStatus::Untested,
        },
        Device {
            id: String::from("192.168.31.44"),
            name: String::from("Computer"),
            ip: String::from("192.168.31.44"),
            identity_kind: DeviceIdentityKind::Computer,
            device_type: DeviceType::Desktop,
            status: DeviceStatus::Denied,
        },
        Device {
            id: String::from("192.168.31.57"),
            name: String::from("Unknown Device"),
            ip: String::from("192.168.31.57"),
            identity_kind: DeviceIdentityKind::Unknown,
            device_type: DeviceType::Desktop,
            status: DeviceStatus::Untested,
        },
    ]
}

fn sample_containers(language: AppLanguage) -> Vec<Container> {
    vec![
        Container {
            id: String::from("b8f4d1f0a0aa"),
            name: String::from("vision-api"),
            image: String::from("ghcr.io/acme/vision-api:main"),
            status: match language {
                AppLanguage::Chinese => String::from("运行中 3 小时"),
                AppLanguage::English => String::from("Up 3 hours"),
            },
            is_running: true,
        },
        Container {
            id: String::from("91de5ab2d8c4"),
            name: String::from("etl-worker"),
            image: String::from("ghcr.io/acme/etl-worker:latest"),
            status: match language {
                AppLanguage::Chinese => String::from("20 分钟前退出（0）"),
                AppLanguage::English => String::from("Exited (0) 20 minutes ago"),
            },
            is_running: false,
        },
    ]
}
