use std::collections::HashMap;

use iced::Task;
use ssh_core::scanner::{
    Device, DeviceStatus, LayeredScanDevice, NeighborEvidence, SshPortProbeStatus, TcpProbeReport,
    build_layered_scan_devices_from_probe_report, compare_devices_by_ip, sort_devices_by_ip,
};
use tokio_util::sync::CancellationToken;
use ui::theme::AppLanguage;

use crate::message::Message;

use super::super::ShellApp;
use super::super::state::{Notice, NoticeTone, ScanPhase, ScanResultFilter, VerifyCredentialInput};
use super::super::tasks::{scan as scan_tasks, verify as verify_tasks};

pub(super) fn handle_start_scan(app: &mut ShellApp) -> Task<Message> {
    if app.visual_check.is_some() || app.is_verifying || app.is_connecting {
        return Task::none();
    }

    let Some(network) = app.selected_network().cloned() else {
        return Task::none();
    };

    app.close_overlays();
    let session_id = advance_scan_session(app);
    cancel_scan_runtime(app);
    reset_verify_runtime(app);
    app.is_scanning = true;
    app.has_scanned = false;
    app.scan_progress = Some((0, 0));
    app.scan_phase = Some(ScanPhase::DiscoverOnline);
    app.scan_auto_verify_enabled = matches!(
        app.verify_credential_input(),
        VerifyCredentialInput::UsernameOnly { .. } | VerifyCredentialInput::UsernamePassword { .. }
    );
    app.online_devices.clear();
    app.online_evidence_by_ip.clear();
    app.devices.clear();
    app.scan_result_filter = preferred_scan_result_filter(app);
    app.selected_device_id = None;

    let cancel_token = CancellationToken::new();
    let (task, handle) = scan_tasks::spawn_scan_task(network, cancel_token.clone(), session_id);
    app.scan_cancel_token = Some(cancel_token);
    app.scan_task_handle = Some(handle);

    task
}

pub(super) fn handle_scan_progress(
    app: &mut ShellApp,
    session_id: u64,
    scanned: usize,
    total: usize,
) -> Task<Message> {
    if !is_current_scan_session(app, session_id) {
        return Task::none();
    }
    app.scan_progress = Some((scanned, total));
    Task::none()
}

pub(super) fn handle_scan_online_dataset_ready(
    app: &mut ShellApp,
    session_id: u64,
    evidence_by_ip: HashMap<String, NeighborEvidence>,
) -> Task<Message> {
    if !is_current_scan_session(app, session_id) {
        return Task::none();
    }
    app.online_evidence_by_ip = evidence_by_ip;
    Task::none()
}

pub(super) fn handle_scan_device_discovered(
    app: &mut ShellApp,
    session_id: u64,
    device: LayeredScanDevice,
) -> Task<Message> {
    if !is_current_scan_session(app, session_id) {
        return Task::none();
    }

    let verify_candidate = device.device.clone();
    let is_ssh_ready = device.ssh_port_status == SshPortProbeStatus::Open;
    let inserted = upsert_layered_scan_device(app, device);
    if inserted {
        app.has_scanned = true;
    }

    if should_auto_verify_while_scanning(app) && is_ssh_ready {
        return enqueue_verify_devices(app, vec![verify_candidate], false)
            .unwrap_or_else(Task::none);
    }

    Task::none()
}

pub(super) fn handle_scan_finished(app: &mut ShellApp, session_id: u64) -> Task<Message> {
    if !is_current_scan_session(app, session_id) {
        return Task::none();
    }

    app.scan_progress = None;
    app.scan_cancel_token = None;
    app.scan_task_handle = None;
    let finished_phase = app.scan_phase.take();

    if matches!(finished_phase, Some(ScanPhase::DiscoverOnline))
        && let Some(task) = start_ssh_probe_if_needed(app)
    {
        return task;
    }

    app.is_scanning = false;
    app.scan_auto_verify_enabled = false;
    app.has_scanned = true;
    sync_verify_runtime_flags(app);
    Task::none()
}

pub(super) fn handle_scan_ssh_probe_finished(
    app: &mut ShellApp,
    session_id: u64,
    report: TcpProbeReport,
) -> Task<Message> {
    if !is_current_scan_session(app, session_id) {
        return Task::none();
    }

    let online_ips = app
        .online_devices
        .iter()
        .map(|layered| layered.device.ip.clone())
        .collect::<Vec<_>>();
    let layered_devices = build_layered_scan_devices_from_probe_report(
        online_ips,
        &report,
        app.online_evidence_by_ip.clone(),
    );
    replace_online_devices(app, layered_devices.online_devices);
    app.has_scanned = true;

    if should_auto_verify_while_scanning(app) {
        return enqueue_verify_devices(app, ssh_ready_devices(app), false)
            .unwrap_or_else(Task::none);
    }

    Task::none()
}

pub(super) fn handle_cancel_scan(app: &mut ShellApp) -> Task<Message> {
    app.close_overlays();
    advance_scan_session(app);
    cancel_scan_runtime(app);
    reset_verify_runtime(app);
    app.is_scanning = false;
    app.scan_progress = None;
    app.scan_phase = None;
    app.scan_auto_verify_enabled = false;
    app.has_scanned = !app.online_devices.is_empty();
    Task::none()
}

pub(super) fn handle_start_verify(app: &mut ShellApp) -> Task<Message> {
    if can_start_verify(app) {
        start_verify_task(app)
    } else {
        Task::none()
    }
}

pub(super) fn handle_verify_result(
    app: &mut ShellApp,
    session_id: u64,
    ip: String,
    status: DeviceStatus,
) -> Task<Message> {
    if !is_current_scan_session(app, session_id) {
        return Task::none();
    }

    update_device_status(app, &ip, status);

    if app.verify_inflight_ips.remove(&ip) {
        app.verify_completed_count = app.verify_completed_count.saturating_add(1);
        app.verified_ips.insert(ip);
    }
    sync_verify_runtime_flags(app);

    Task::none()
}

pub(super) fn handle_verify_complete(app: &mut ShellApp, session_id: u64) -> Task<Message> {
    if !is_current_scan_session(app, session_id) {
        return Task::none();
    }
    sync_verify_runtime_flags(app);
    Task::none()
}

pub(super) fn can_start_verify(app: &ShellApp) -> bool {
    app.has_scanned
        && !ssh_ready_devices(app).is_empty()
        && !app.is_verifying
        && !app.is_connecting
        && app.has_verify_inputs()
}

pub(super) fn start_verify_task(app: &mut ShellApp) -> Task<Message> {
    let Some((username, password)) = app.resolve_verify_credentials() else {
        app.set_notice(Notice {
            tone: NoticeTone::Warning,
            message: verify_warning_message(app.app_language, app.verify_credential_input()),
        });
        return Task::none();
    };
    let verify_devices = ssh_ready_devices(app);

    if verify_devices.is_empty() {
        app.set_notice(Notice {
            tone: NoticeTone::Warning,
            message: match app.app_language {
                AppLanguage::Chinese => {
                    String::from("当前没有可验证的 SSH 设备，请先等待 TCP 22 探测完成。")
                }
                AppLanguage::English => String::from(
                    "There are no SSH-ready devices to verify yet. Wait for the TCP 22 probe to finish first.",
                ),
            },
        });
        return Task::none();
    }

    app.user_dropdown_open = false;
    reset_device_statuses(app);
    app.verify_inflight_ips = verify_devices
        .iter()
        .map(|device| device.ip.clone())
        .collect();
    app.verified_ips.clear();
    app.verify_enqueued_count = app.verify_inflight_ips.len();
    app.verify_completed_count = 0;
    sync_verify_runtime_flags(app);
    let session_id = app.scan_session_id;

    verify_tasks::spawn_verify_task(verify_devices, username, password, session_id)
}

pub(super) fn advance_scan_session(app: &mut ShellApp) -> u64 {
    app.scan_session_id = app.scan_session_id.wrapping_add(1);
    app.scan_session_id
}

pub(super) fn is_current_scan_session(app: &ShellApp, session_id: u64) -> bool {
    app.scan_session_id == session_id
}

fn verify_warning_message(language: AppLanguage, input: VerifyCredentialInput) -> String {
    match (language, input) {
        (AppLanguage::Chinese, VerifyCredentialInput::PasswordOnly) => String::from(
            "当前仅填写 SSH 密码；自动验证不会在 password-only 模式下触发，请补充 SSH 用户名。",
        ),
        (AppLanguage::English, VerifyCredentialInput::PasswordOnly) => String::from(
            "Only an SSH password is filled in. Automatic verification does not run in password-only mode, so add an SSH username first.",
        ),
        (AppLanguage::Chinese, VerifyCredentialInput::Empty)
        | (AppLanguage::Chinese, VerifyCredentialInput::UsernameOnly { .. })
        | (AppLanguage::Chinese, VerifyCredentialInput::UsernamePassword { .. }) => {
            String::from("请先填写 SSH 用户名，再执行凭证检测。")
        }
        (AppLanguage::English, VerifyCredentialInput::Empty)
        | (AppLanguage::English, VerifyCredentialInput::UsernameOnly { .. })
        | (AppLanguage::English, VerifyCredentialInput::UsernamePassword { .. }) => {
            String::from("Enter an SSH username before starting credential verification.")
        }
    }
}

pub(super) fn reset_verify_runtime(app: &mut ShellApp) {
    app.verify_inflight_ips.clear();
    app.verified_ips.clear();
    app.verify_enqueued_count = 0;
    app.verify_completed_count = 0;
    app.verify_progress = None;
    app.is_verifying = false;
}

pub(super) fn sync_verify_runtime_flags(app: &mut ShellApp) {
    app.is_verifying = !app.verify_inflight_ips.is_empty();
    if app.verify_enqueued_count == 0 {
        app.verify_progress = None;
        return;
    }

    let done = app.verify_completed_count.min(app.verify_enqueued_count);
    if !app.is_scanning && !app.is_verifying && done >= app.verify_enqueued_count {
        app.verify_progress = None;
    } else {
        app.verify_progress = Some((done, app.verify_enqueued_count));
    }
}

pub(super) fn should_auto_verify_while_scanning(app: &ShellApp) -> bool {
    if app.visual_check.is_some() || app.is_connecting || !app.scan_auto_verify_enabled {
        return false;
    }

    matches!(
        app.verify_credential_input(),
        VerifyCredentialInput::UsernameOnly { .. } | VerifyCredentialInput::UsernamePassword { .. }
    )
}

pub(super) fn enqueue_verify_devices(
    app: &mut ShellApp,
    devices: Vec<Device>,
    allow_reverify: bool,
) -> Option<Task<Message>> {
    let (username, password) = app.resolve_verify_credentials()?;

    let mut queued = Vec::new();
    for device in devices {
        if ssh_port_status_for_ip(app, device.ip.as_str()) != Some(SshPortProbeStatus::Open) {
            continue;
        }
        let ip = device.ip.clone();
        if app.verify_inflight_ips.contains(ip.as_str()) {
            continue;
        }
        if !allow_reverify && app.verified_ips.contains(ip.as_str()) {
            continue;
        }

        app.verify_inflight_ips.insert(ip);
        queued.push(device);
    }

    if queued.is_empty() {
        return None;
    }

    app.verify_enqueued_count = app.verify_enqueued_count.saturating_add(queued.len());
    sync_verify_runtime_flags(app);
    let session_id = app.scan_session_id;
    Some(verify_tasks::spawn_verify_task(
        queued, username, password, session_id,
    ))
}

pub(super) fn upsert_layered_scan_device(app: &mut ShellApp, device: LayeredScanDevice) -> bool {
    if let Some(existing) = app
        .online_devices
        .iter_mut()
        .find(|existing| existing.device.ip == device.device.ip)
    {
        let preserved_status = existing.device.status;
        *existing = device;
        if existing.device.status == DeviceStatus::Untested
            && preserved_status != DeviceStatus::Untested
        {
            existing.device.status = preserved_status;
        }
        rebuild_visible_devices_from_online(app);
        return false;
    }

    app.online_devices.push(device);
    app.online_devices
        .sort_by(|left, right| compare_devices_by_ip(&left.device, &right.device));
    rebuild_visible_devices_from_online(app);
    true
}

pub(super) fn replace_online_devices(app: &mut ShellApp, mut devices: Vec<LayeredScanDevice>) {
    let previous_status_by_ip = app
        .online_devices
        .iter()
        .map(|layered| (layered.device.ip.clone(), layered.device.status))
        .collect::<HashMap<_, _>>();
    for layered in &mut devices {
        if layered.device.status == DeviceStatus::Untested
            && let Some(status) = previous_status_by_ip.get(layered.device.ip.as_str())
        {
            layered.device.status = *status;
        }
    }
    devices.sort_by(|left, right| compare_devices_by_ip(&left.device, &right.device));
    app.online_devices = devices;
    rebuild_visible_devices_from_online(app);
}

pub(super) fn rebuild_visible_devices_from_online(app: &mut ShellApp) {
    let mut devices = app
        .online_devices
        .iter()
        .filter(|layered| match app.scan_result_filter {
            ScanResultFilter::AllOnline => true,
            ScanResultFilter::SshReady => layered.ssh_port_status == SshPortProbeStatus::Open,
        })
        .map(|layered| layered.device.clone())
        .collect::<Vec<_>>();
    sort_devices_by_ip(&mut devices);
    app.devices = devices;
    app.selected_device_id = app
        .selected_device_id
        .take()
        .filter(|selected| app.devices.iter().any(|device| &device.id == selected));
}

pub(super) fn preferred_scan_result_filter(app: &ShellApp) -> ScanResultFilter {
    if has_ssh_intent(app) {
        ScanResultFilter::SshReady
    } else {
        ScanResultFilter::AllOnline
    }
}

pub(super) fn ssh_ready_devices(app: &ShellApp) -> Vec<Device> {
    app.online_devices
        .iter()
        .filter(|layered| layered.ssh_port_status == SshPortProbeStatus::Open)
        .map(|layered| layered.device.clone())
        .collect()
}

pub(super) fn pending_ssh_probe_ips(app: &ShellApp) -> Vec<String> {
    app.online_devices
        .iter()
        .filter(|layered| layered.ssh_port_status == SshPortProbeStatus::Unchecked)
        .map(|layered| layered.device.ip.clone())
        .collect()
}

pub(super) fn start_ssh_probe_if_needed(app: &mut ShellApp) -> Option<Task<Message>> {
    if app.visual_check.is_some() || app.is_connecting {
        return None;
    }
    if app.is_scanning && matches!(app.scan_phase, Some(ScanPhase::DiscoverOnline)) {
        return None;
    }
    if matches!(app.scan_phase, Some(ScanPhase::ProbeSsh)) {
        return None;
    }

    let network = app.selected_network().cloned()?;
    let pending_ips = pending_ssh_probe_ips(app);
    if pending_ips.is_empty() {
        return None;
    }

    cancel_scan_runtime(app);
    app.is_scanning = true;
    app.scan_phase = Some(ScanPhase::ProbeSsh);
    app.scan_progress = Some((0, pending_ips.len()));
    let cancel_token = CancellationToken::new();
    let (task, handle) = scan_tasks::spawn_ssh_probe_task(
        network,
        pending_ips,
        cancel_token.clone(),
        app.scan_session_id,
    );
    app.scan_cancel_token = Some(cancel_token);
    app.scan_task_handle = Some(handle);
    Some(task)
}

pub(super) fn cancel_scan_runtime(app: &mut ShellApp) {
    if let Some(cancel_token) = app.scan_cancel_token.take() {
        cancel_token.cancel();
    }

    if let Some(handle) = app.scan_task_handle.take() {
        handle.abort();
    }
}

pub(super) fn reset_device_statuses(app: &mut ShellApp) {
    for layered in &mut app.online_devices {
        layered.device.status = DeviceStatus::Untested;
    }
    rebuild_visible_devices_from_online(app);
}

pub(super) fn update_device_status(app: &mut ShellApp, ip: &str, status: DeviceStatus) {
    if let Some(layered) = app
        .online_devices
        .iter_mut()
        .find(|layered| layered.device.ip == ip)
    {
        layered.device.status = status;
    }
    rebuild_visible_devices_from_online(app);
}

fn has_ssh_intent(app: &ShellApp) -> bool {
    !matches!(app.verify_credential_input(), VerifyCredentialInput::Empty)
}

fn ssh_port_status_for_ip(app: &ShellApp, ip: &str) -> Option<SshPortProbeStatus> {
    app.online_devices
        .iter()
        .find(|layered| layered.device.ip == ip)
        .map(|layered| layered.ssh_port_status)
}
