use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use iced::{Size, Task, window};
use ssh_core::credential;
use ssh_core::network;
use ssh_core::scanner::{LayeredScanDevice, SshPortProbeStatus};
use ui::theme::ThemeMode;

use crate::message::Message;
use crate::visual_check::{
    VisualCheckConfig, VisualModalPreset, VisualScene, VisualScenePreset, scene_preset,
};

use super::super::{
    ActiveModal, DockerModalState, Notice, NoticeTone, ScanResultFilter, ShellApp,
    VisualCheckRuntime, VisualCheckStage,
};

const VISUAL_CHECK_WINDOW_WIDTH: f32 = 1050.0;
const VISUAL_CHECK_WINDOW_HEIGHT: f32 = 730.0;
const VISUAL_CHECK_HELP_WINDOW_HEIGHT: f32 = 1100.0;
const VISUAL_CHECK_SETTLING_FRAMES: u8 = 2;
const VISUAL_CHECK_HELP_SETTLING_FRAMES: u8 = 4;
const VISUAL_CHECK_RESIZE_FALLBACK_FRAMES: u8 = 3;
const VISUAL_CHECK_HELP_RESIZE_FALLBACK_FRAMES: u8 = 6;

pub(super) fn initialize_visual_check(app: &mut ShellApp, config: VisualCheckConfig) {
    let mut scenes = config.scenes.into_iter();
    let Some(first_scene) = scenes.next() else {
        return;
    };

    app.visual_check = Some(VisualCheckRuntime {
        output_dir: config.output_dir,
        current_scene: first_scene,
        pending_scenes: scenes.collect::<VecDeque<_>>(),
        stage: VisualCheckStage::WaitingForWindow,
    });
    apply_visual_scene(app, first_scene);
}

pub(super) fn handle_window_ready(app: &mut ShellApp) -> Task<Message> {
    let Some(scene) = app
        .visual_check
        .as_ref()
        .map(|runtime| runtime.current_scene)
    else {
        return Task::none();
    };
    if app.window_id.is_some() {
        prepare_visual_check_scene(app, None, scene)
    } else {
        Task::none()
    }
}

pub(super) fn handle_tick(app: &mut ShellApp) -> Task<Message> {
    let Some(runtime) = app.visual_check.as_mut() else {
        return Task::none();
    };

    match runtime.stage {
        VisualCheckStage::WaitingForWindow => Task::none(),
        VisualCheckStage::WaitingForResize(remaining) => {
            if remaining > 1 {
                runtime.stage = VisualCheckStage::WaitingForResize(remaining - 1);
            } else {
                runtime.stage = VisualCheckStage::Settling(visual_check_scene_settling_frames(
                    runtime.current_scene,
                ));
            }
            Task::none()
        }
        VisualCheckStage::Settling(remaining) => {
            if remaining > 1 {
                runtime.stage = VisualCheckStage::Settling(remaining - 1);
                Task::none()
            } else {
                runtime.stage = VisualCheckStage::Capturing;
                Task::done(Message::VisualCheckCapture)
            }
        }
        VisualCheckStage::Capturing => Task::none(),
    }
}

pub(super) fn capture_scene(app: &mut ShellApp) -> Task<Message> {
    let Some(window_id) = app.window_id else {
        return Task::done(Message::VisualCheckFailed(String::from(
            "window is not ready",
        )));
    };

    window::screenshot(window_id).map(Message::VisualCheckCaptured)
}

pub(super) fn save_screenshot(
    app: &ShellApp,
    screenshot: &window::Screenshot,
) -> Result<PathBuf, String> {
    let Some(runtime) = app.visual_check.as_ref() else {
        return Err(String::from("visual check runtime is not initialized"));
    };

    std::fs::create_dir_all(&runtime.output_dir)
        .map_err(|error| format!("failed to create output directory: {error}"))?;
    let output_path = runtime.output_dir.join(runtime.current_scene.file_name());
    write_screenshot_png(screenshot, &output_path)?;

    Ok(output_path)
}

pub(super) fn advance_or_exit(app: &mut ShellApp) -> Task<Message> {
    let next_scene = if let Some(runtime) = app.visual_check.as_mut() {
        runtime
            .pending_scenes
            .pop_front()
            .map(|next| (runtime.current_scene, next))
    } else {
        None
    };

    if let Some((previous_scene, scene)) = next_scene {
        if let Some(runtime) = app.visual_check.as_mut() {
            runtime.current_scene = scene;
        }
        apply_visual_scene(app, scene);
        prepare_visual_check_scene(app, Some(previous_scene), scene)
    } else {
        close_window(app)
    }
}

pub(super) fn close_window(app: &ShellApp) -> Task<Message> {
    app.window_id.map_or_else(Task::none, window::close)
}

pub(super) fn visual_check_scene_settling_frames(scene: VisualScene) -> u8 {
    if visual_check_scene_is_help(scene) {
        VISUAL_CHECK_HELP_SETTLING_FRAMES
    } else {
        VISUAL_CHECK_SETTLING_FRAMES
    }
}

fn visual_check_scene_is_help(scene: VisualScene) -> bool {
    matches!(scene, VisualScene::HelpModal | VisualScene::HelpModalDark)
}

fn visual_check_scene_window_size(scene: VisualScene) -> Size {
    if visual_check_scene_is_help(scene) {
        Size::new(VISUAL_CHECK_WINDOW_WIDTH, VISUAL_CHECK_HELP_WINDOW_HEIGHT)
    } else {
        Size::new(VISUAL_CHECK_WINDOW_WIDTH, VISUAL_CHECK_WINDOW_HEIGHT)
    }
}

fn visual_check_scene_resize_fallback_frames(scene: VisualScene) -> u8 {
    if visual_check_scene_is_help(scene) {
        VISUAL_CHECK_HELP_RESIZE_FALLBACK_FRAMES
    } else {
        VISUAL_CHECK_RESIZE_FALLBACK_FRAMES
    }
}

fn visual_check_scene_needs_resize(
    previous_scene: Option<VisualScene>,
    next_scene: VisualScene,
) -> bool {
    previous_scene.is_none_or(|previous| {
        visual_check_scene_is_help(previous) != visual_check_scene_is_help(next_scene)
    }) && visual_check_scene_is_help(next_scene)
        || previous_scene.is_some_and(|previous| {
            visual_check_scene_is_help(previous) && !visual_check_scene_is_help(next_scene)
        })
}

fn prepare_visual_check_scene(
    app: &mut ShellApp,
    previous_scene: Option<VisualScene>,
    scene: VisualScene,
) -> Task<Message> {
    let needs_resize =
        app.window_id.is_some() && visual_check_scene_needs_resize(previous_scene, scene);

    if let Some(runtime) = app.visual_check.as_mut() {
        runtime.stage = if needs_resize {
            VisualCheckStage::WaitingForResize(visual_check_scene_resize_fallback_frames(scene))
        } else {
            VisualCheckStage::Settling(visual_check_scene_settling_frames(scene))
        };
    }

    if needs_resize {
        resize_window_for_scene(app, scene)
    } else {
        Task::none()
    }
}

fn resize_window_for_scene(app: &ShellApp, scene: VisualScene) -> Task<Message> {
    app.window_id.map_or_else(Task::none, |window_id| {
        window::resize(window_id, visual_check_scene_window_size(scene))
    })
}

fn apply_visual_scene(app: &mut ShellApp, scene: VisualScene) {
    let preset = scene_preset(scene, &app.credentials);
    apply_visual_scene_preset(app, preset);
}

fn apply_visual_scene_preset(app: &mut ShellApp, preset: VisualScenePreset) {
    app.close_overlays();
    app.cancel_scan_runtime();

    app.networks = preset.networks;
    app.networks_signature = network::signature(&app.networks);
    app.app_language = preset.language;
    app.theme_mode = if preset.dark_mode {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    app.selected_network_id = preset
        .selected_network_id
        .filter(|network_id| app.networks.iter().any(|network| network.id == *network_id))
        .or_else(|| app.networks.first().map(|network| network.id.clone()));
    app.is_refreshing_networks = preset.is_refreshing_networks;
    app.is_scanning = preset.is_scanning;
    app.has_scanned = preset.has_scanned;
    app.scan_progress = preset.scan_progress;
    app.verify_progress = preset.verify_progress;
    app.scan_session_id = app.scan_session_id.wrapping_add(1);
    app.scan_phase = None;
    app.scan_auto_verify_enabled = false;
    app.scan_cancel_token = None;
    app.scan_task_handle = None;
    app.verify_inflight_ips.clear();
    app.verified_ips.clear();
    app.verify_enqueued_count = 0;
    app.verify_completed_count = 0;
    app.online_evidence_by_ip.clear();
    app.online_devices = preset
        .devices
        .iter()
        .cloned()
        .map(|device| LayeredScanDevice {
            device,
            ssh_port_status: SshPortProbeStatus::Unchecked,
        })
        .collect();
    app.scan_result_filter = ScanResultFilter::AllOnline;
    app.rebuild_visible_devices_from_online();
    app.selected_device_id = preset
        .selected_device_id
        .filter(|device_id| app.devices.iter().any(|device| device.id == *device_id));

    app.ssh_username = preset.ssh_username;
    app.selected_username = app
        .normalized_ssh_username()
        .filter(|username| credential::find_by_username(&app.credentials, username).is_some())
        .or_else(|| {
            preset.selected_username.filter(|username| {
                credential::find_by_username(&app.credentials, username).is_some()
            })
        });

    if let Some(username) = app.selected_username.as_ref()
        && app.ssh_username.trim().is_empty()
    {
        app.ssh_username = username.clone();
    }
    app.password = preset.password;
    app.vnc_enabled = preset.vnc_enabled;
    app.vnc_user = preset.vnc_user;
    app.vnc_password = preset.vnc_password;
    app.scan_result_filter = app.preferred_scan_result_filter();
    app.rebuild_visible_devices_from_online();

    app.is_verifying = preset.is_verifying;
    app.is_connecting = false;
    app.pending_tool_action = None;
    app.pending_docker_context = None;
    app.active_quick_connect = None;
    app.connection_status = preset.connection_status;
    app.notice = preset.notice.map(|message| Notice {
        tone: NoticeTone::Warning,
        message,
    });
    app.network_dropdown_open = false;
    app.user_dropdown_open = false;
    app.active_modal = match preset.modal {
        VisualModalPreset::None => None,
        VisualModalPreset::Help {
            show_rustdesk_section,
        } => {
            app.help_modal_show_rustdesk = show_rustdesk_section;
            Some(ActiveModal::HelpGuide)
        }
        VisualModalPreset::Credential => Some(ActiveModal::CredentialManagement),
        VisualModalPreset::Docker {
            containers,
            selected_container_id,
        } => Some(ActiveModal::DockerSelect(DockerModalState {
            selected_container_id: selected_container_id
                .filter(|container_id| containers.iter().any(|item| item.id == *container_id))
                .or_else(|| containers.first().map(|container| container.id.clone())),
            containers,
        })),
    };
    app.editing_credential_username = preset
        .editing_credential_username
        .filter(|username| credential::find_by_username(&app.credentials, username).is_some());
    app.new_credential_username = preset.new_credential_username;
    app.new_credential_password = preset.new_credential_password;
    app.network_dropdown_open =
        preset.network_dropdown_open && app.active_modal.is_none() && !app.networks.is_empty();
    app.user_dropdown_open =
        preset.user_dropdown_open && app.active_modal.is_none() && !app.credentials.is_empty();
    app.spinner_phase = 0;
}

fn write_screenshot_png(screenshot: &window::Screenshot, output_path: &Path) -> Result<(), String> {
    let width = screenshot.size.width;
    let height = screenshot.size.height;
    let expected_len = width as usize * height as usize * 4;
    let bytes = screenshot.rgba.to_vec();

    if bytes.len() != expected_len {
        return Err(format!(
            "invalid screenshot data length: expected {expected_len}, got {}",
            bytes.len()
        ));
    }

    let image = image::RgbaImage::from_raw(width, height, bytes)
        .ok_or_else(|| String::from("failed to construct RGBA image from screenshot bytes"))?;
    image
        .save(output_path)
        .map_err(|error| format!("failed to write PNG {}: {error}", output_path.display()))
}
