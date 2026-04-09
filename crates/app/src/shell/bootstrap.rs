use std::collections::{HashMap, HashSet};

use iced::Task;
use ssh_core::credential::store::{self, AppConfig};
use ssh_core::credential::{self, Credential};
use ssh_core::ssh::key_mgmt;
use ui::theme::{AppLanguage, ThemeMode};

use crate::message::Message;
use crate::visual_check::VisualCheckConfig;

use super::{ScanResultFilter, ShellApp};

impl ShellApp {
    pub fn boot() -> (Self, Task<Message>) {
        platform::network::ensure_registered();
        if let Err(error) = key_mgmt::cleanup_external_temp_keys_on_startup() {
            eprintln!("[ERROR] cleanup stale external key temp dirs failed: {error}");
        }

        let (config, credentials, selected_username, password) = Self::load_bootstrap_state();
        let app = Self::new_state(config, credentials, selected_username, password);

        (app, Task::done(Message::RefreshNetworks))
    }

    pub fn boot_visual_check(config: VisualCheckConfig) -> (Self, Task<Message>) {
        platform::network::ensure_registered();
        if let Err(error) = key_mgmt::cleanup_external_temp_keys_on_startup() {
            eprintln!("[ERROR] cleanup stale external key temp dirs failed: {error}");
        }

        let (bootstrap_config, credentials, selected_username, password) =
            Self::load_bootstrap_state();
        let mut app = Self::new_state(bootstrap_config, credentials, selected_username, password);
        app.initialize_visual_check(config);

        (app, Task::none())
    }

    fn load_bootstrap_state() -> (AppConfig, Vec<Credential>, Option<String>, String) {
        let config = store::load_config().unwrap_or_else(|error| {
            eprintln!("[ERROR] config bootstrap failed: {error}");
            AppConfig::default()
        });
        let credentials = credential::credentials_from_config(&config);

        (config, credentials, None, String::new())
    }

    fn new_state(
        config: AppConfig,
        credentials: Vec<Credential>,
        selected_username: Option<String>,
        password: String,
    ) -> Self {
        let app_language = match platform::locale::detect_system_language() {
            platform::locale::SystemLanguage::Chinese => AppLanguage::Chinese,
            platform::locale::SystemLanguage::Other => AppLanguage::English,
        };

        Self {
            network_dropdown_open: false,
            user_dropdown_open: false,
            networks: Vec::new(),
            selected_network_id: None,
            is_scanning: false,
            is_refreshing_networks: false,
            online_devices: Vec::new(),
            online_evidence_by_ip: HashMap::new(),
            devices: Vec::new(),
            scan_result_filter: ScanResultFilter::AllOnline,
            selected_device_id: None,
            has_scanned: false,
            scan_progress: None,
            verify_progress: None,
            scan_session_id: 0,
            scan_phase: None,
            scan_auto_verify_enabled: false,
            scan_cancel_token: None,
            scan_task_handle: None,
            verify_inflight_ips: HashSet::new(),
            verified_ips: HashSet::new(),
            verify_enqueued_count: 0,
            verify_completed_count: 0,
            networks_signature: 0,
            spinner_phase: 0,
            app_language,
            theme_mode: ThemeMode::Light,
            window_id: None,
            is_window_maximized: false,
            credentials,
            app_paths: config.app_paths.clone(),
            ssh_username: selected_username.clone().unwrap_or_default(),
            selected_username,
            password,
            vnc_enabled: false,
            vnc_user: String::new(),
            vnc_password: String::new(),
            active_modal: None,
            help_modal_show_rustdesk: false,
            editing_credential_username: None,
            new_credential_username: String::new(),
            new_credential_password: String::new(),
            is_verifying: false,
            is_connecting: false,
            pending_tool_action: None,
            pending_docker_context: None,
            active_quick_connect: None,
            connection_status: None,
            notice: None,
            notice_version: 0,
            visual_check: None,
        }
    }
}
