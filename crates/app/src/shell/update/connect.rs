use std::path::PathBuf;

use iced::Task;
use platform::app_finder;
use ssh_core::credential::store::{self, ToolKind};
use ssh_core::docker::{self, Container};
use ssh_core::scanner::{Device, DeviceStatus};
use ssh_core::ssh::auth::LaunchAuthConsumer;

use crate::message::{ConnectNoticeTone, Message};

use super::super::tasks::connect as connect_tasks;
use super::super::{
    ActiveModal, DockerModalState, LaunchContext, Notice, NoticeTone, PendingToolAction, ShellApp,
    VncCredentialFieldSource, VncCredentialSource,
};

pub(super) fn handle_connect_shell(app: &mut ShellApp, device_ip: String) -> Task<Message> {
    if app.is_connecting {
        Task::none()
    } else {
        start_shell_launch(app, device_ip)
    }
}

pub(super) fn handle_connect_vscode(app: &mut ShellApp, device_ip: String) -> Task<Message> {
    if app.is_connecting {
        Task::none()
    } else {
        start_direct_launch(app, ToolKind::Vscode, device_ip)
    }
}

pub(super) fn handle_connect_mobaxterm(app: &mut ShellApp, device_ip: String) -> Task<Message> {
    if app.is_connecting {
        Task::none()
    } else {
        start_direct_launch(app, ToolKind::Mobaxterm, device_ip)
    }
}

pub(super) fn handle_connect_vnc(app: &mut ShellApp, device_ip: String) -> Task<Message> {
    if app.is_connecting {
        Task::none()
    } else {
        start_direct_launch(app, ToolKind::VncViewer, device_ip)
    }
}

pub(super) fn handle_connect_docker(app: &mut ShellApp, device_ip: String) -> Task<Message> {
    if app.is_connecting {
        Task::none()
    } else {
        start_docker_flow(app, device_ip)
    }
}

pub(super) fn handle_connect_rustdesk(app: &mut ShellApp, device_ip: String) -> Task<Message> {
    if app.is_connecting {
        Task::none()
    } else {
        start_rustdesk_launch(app, device_ip)
    }
}

pub(super) fn handle_rustdesk_probe_finished(
    app: &mut ShellApp,
    device_ip: String,
    password: Option<String>,
    result: Result<(), String>,
) -> Task<Message> {
    match result {
        Ok(()) => continue_rustdesk_launch(app, device_ip, password),
        Err(error) => {
            app.is_connecting = false;
            app.connection_status = None;
            app.pending_tool_action = None;
            app.clear_active_quick_connect();
            app.set_quick_connect_notice(Notice {
                tone: NoticeTone::Error,
                message: error,
            })
        }
    }
}

pub(super) fn handle_request_tool_path(app: &mut ShellApp, tool: ToolKind) -> Task<Message> {
    if app.visual_check.is_some() {
        return Task::none();
    }

    Task::perform(
        app_finder::pick_tool_path(tool),
        move |result| match result {
            Ok(path) => Message::ToolPathPicked(tool, path),
            Err(error) => Message::ToolPathPickFailed(tool, error),
        },
    )
}

pub(super) fn handle_tool_path_picked(
    app: &mut ShellApp,
    tool: ToolKind,
    selected_path: Option<PathBuf>,
) -> Task<Message> {
    let Some(path) = selected_path else {
        app.is_connecting = false;
        app.connection_status = None;
        app.pending_tool_action = None;
        app.pending_docker_context = None;
        app.clear_active_quick_connect();
        return app.set_quick_connect_notice(Notice {
            tone: NoticeTone::Error,
            message: format!("已取消选择 {} 路径", tool.label()),
        });
    };

    let mut quick_connect_notice_task = None;
    match store::save_app_path(tool, Some(path.as_path())) {
        Ok(config) => {
            app.app_paths = config.app_paths.clone();
        }
        Err(error) => {
            quick_connect_notice_task = Some(app.set_quick_connect_notice(Notice {
                tone: NoticeTone::Error,
                message: format!("保存 {} 路径失败: {error}", tool.label()),
            }));
        }
    }

    let Some(action) = app.pending_tool_action.take() else {
        app.is_connecting = false;
        app.connection_status = None;
        app.pending_docker_context = None;
        app.clear_active_quick_connect();
        return quick_connect_notice_task.unwrap_or_else(Task::none);
    };

    let launch_task = perform_launch(app, action, path);
    if let Some(notice_task) = quick_connect_notice_task {
        Task::batch([notice_task, launch_task])
    } else {
        launch_task
    }
}

pub(super) fn handle_tool_path_pick_failed(
    app: &mut ShellApp,
    tool: ToolKind,
    error: String,
) -> Task<Message> {
    app.is_connecting = false;
    app.connection_status = None;
    app.pending_tool_action = None;
    app.pending_docker_context = None;
    app.clear_active_quick_connect();
    let supports_picker = app_finder::supports_native_tool_picker();
    app.set_quick_connect_notice(Notice {
        tone: if supports_picker {
            NoticeTone::Error
        } else {
            NoticeTone::Warning
        },
        message: if supports_picker {
            format!("选择 {} 路径失败: {error}", tool.label())
        } else {
            app_finder::unresolved_tool_path_notice(tool)
        },
    })
}

pub(super) fn handle_docker_containers_loaded(
    app: &mut ShellApp,
    containers: Vec<Container>,
) -> Task<Message> {
    app.is_connecting = false;
    app.connection_status = None;
    app.clear_active_quick_connect();
    let Some(_context) = &app.pending_docker_context else {
        return Task::none();
    };

    app.active_modal = Some(ActiveModal::DockerSelect(DockerModalState {
        selected_container_id: containers.first().map(|container| container.id.clone()),
        containers,
    }));
    Task::none()
}

pub(super) fn handle_docker_containers_load_failed(
    app: &mut ShellApp,
    message: String,
) -> Task<Message> {
    app.is_connecting = false;
    app.connection_status = None;
    app.pending_docker_context = None;
    app.clear_active_quick_connect();
    app.set_quick_connect_notice(Notice {
        tone: NoticeTone::Error,
        message,
    })
}

pub(super) fn handle_attach_selected_container(app: &mut ShellApp) -> Task<Message> {
    if app.visual_check.is_some() {
        return Task::none();
    }

    let Some(context) = app.pending_docker_context.clone() else {
        return Task::none();
    };
    let Some(ActiveModal::DockerSelect(state)) = &app.active_modal else {
        return Task::none();
    };
    let Some(selected_id) = state.selected_container_id.as_deref() else {
        return Task::none();
    };
    let Some(container) = state
        .containers
        .iter()
        .find(|container| container.id == selected_id)
        .cloned()
    else {
        return Task::none();
    };

    app.active_modal = None;
    app.set_active_quick_connect(context.device_ip.clone(), "docker");
    request_launch(app, PendingToolAction::DockerAttach { context, container })
}

pub(super) fn handle_connect_result(
    app: &mut ShellApp,
    result: Result<crate::message::ConnectNotice, String>,
) -> Task<Message> {
    app.is_connecting = false;
    app.connection_status = None;
    app.pending_tool_action = None;
    app.pending_docker_context = None;
    app.clear_active_quick_connect();

    app.set_quick_connect_notice(match result {
        Ok(notice) => Notice {
            tone: match notice.tone {
                ConnectNoticeTone::Success => NoticeTone::Success,
                ConnectNoticeTone::Warning => NoticeTone::Warning,
            },
            message: notice.message,
        },
        Err(error) => Notice {
            tone: NoticeTone::Error,
            message: error,
        },
    })
}

pub(super) fn active_connection_device_ip(app: &ShellApp) -> Option<&str> {
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

pub(super) fn device_detail_status(app: &ShellApp, device: &Device) -> String {
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

fn start_shell_launch(app: &mut ShellApp, device_ip: String) -> Task<Message> {
    if app.visual_check.is_some() {
        return Task::none();
    }

    app.clear_notice();
    app.close_overlays();

    let context = match build_launch_context(app, &device_ip) {
        Ok(context) => context,
        Err(message) => {
            app.clear_active_quick_connect();
            return app.set_quick_connect_notice(Notice {
                tone: NoticeTone::Error,
                message,
            });
        }
    };

    app.set_active_quick_connect(device_ip.clone(), "shell");
    app.is_connecting = true;
    app.connection_status = Some(String::from("正在启动终端连接"));

    Task::perform(
        async move {
            let auth_preparation =
                connect_tasks::prepare_ssh_launch_auth(&context, LaunchAuthConsumer::Shell).await?;
            platform::launcher::launch_shell_ssh(
                &context.device_ip,
                &context.username,
                &auth_preparation,
            )
            .map_err(|error| format!("终端连接启动失败: {error}"))?;
            Ok(connect_tasks::shell_connect_notice(&auth_preparation))
        },
        |result| Message::ConnectResult(ToolKind::Vscode, result),
    )
}

fn start_direct_launch(app: &mut ShellApp, tool: ToolKind, device_ip: String) -> Task<Message> {
    if app.visual_check.is_some() {
        return Task::none();
    }

    app.clear_notice();
    app.close_overlays();

    let context = match build_launch_context(app, &device_ip) {
        Ok(context) => context,
        Err(message) => {
            app.clear_active_quick_connect();
            return app.set_quick_connect_notice(Notice {
                tone: NoticeTone::Error,
                message,
            });
        }
    };

    app.set_active_quick_connect(device_ip, ShellApp::quick_connect_launcher_for_tool(tool));
    request_launch(app, PendingToolAction::Direct { tool, context })
}

fn start_rustdesk_launch(app: &mut ShellApp, device_ip: String) -> Task<Message> {
    if app.visual_check.is_some() {
        return Task::none();
    }

    app.clear_notice();
    app.close_overlays();
    app.pending_tool_action = None;
    app.pending_docker_context = None;
    app.set_active_quick_connect(device_ip.clone(), "rustdesk");
    app.is_connecting = true;
    app.connection_status = Some(format!(
        "正在探测 RustDesk Direct IP 端口 {}",
        connect_tasks::RUSTDESK_DIRECT_IP_PORT
    ));

    let password = app.normalized_rustdesk_password();
    Task::perform(
        async move {
            let result = connect_tasks::probe_rustdesk_direct_ip_port(&device_ip).await;
            (device_ip, password, result)
        },
        |(device_ip, password, result)| Message::RustDeskProbeFinished {
            device_ip,
            password,
            result,
        },
    )
}

fn continue_rustdesk_launch(
    app: &mut ShellApp,
    device_ip: String,
    password: Option<String>,
) -> Task<Message> {
    let has_password = password.is_some();
    app.set_active_quick_connect(device_ip.clone(), "rustdesk");
    request_launch(
        app,
        PendingToolAction::Direct {
            tool: ToolKind::RustDesk,
            context: LaunchContext {
                device_ip,
                username: String::new(),
                password: None,
                vnc_requested: has_password,
                vnc_username: None,
                vnc_password: password,
                vnc_username_source: VncCredentialFieldSource::Unavailable,
                vnc_password_source: if has_password {
                    VncCredentialFieldSource::Dedicated
                } else {
                    VncCredentialFieldSource::Unavailable
                },
                vnc_source: if has_password {
                    VncCredentialSource::Dedicated
                } else {
                    VncCredentialSource::Unavailable
                },
            },
        },
    )
}

fn start_docker_flow(app: &mut ShellApp, device_ip: String) -> Task<Message> {
    if app.visual_check.is_some() {
        return Task::none();
    }

    app.clear_notice();
    app.close_overlays();

    let context = match build_launch_context(app, &device_ip) {
        Ok(context) => context,
        Err(message) => {
            app.clear_active_quick_connect();
            return app.set_quick_connect_notice(Notice {
                tone: NoticeTone::Error,
                message,
            });
        }
    };

    app.set_active_quick_connect(device_ip, "docker");
    app.is_connecting = true;
    app.connection_status = Some(String::from("正在读取远程 Docker 容器列表"));
    app.pending_docker_context = Some(context.clone());

    Task::perform(
        async move {
            docker::list_containers(
                &context.device_ip,
                &context.username,
                context.password.as_deref(),
            )
            .await
        },
        |result| match result {
            Ok(containers) => Message::DockerContainersLoaded(containers),
            Err(error) => Message::DockerContainersLoadFailed(error.to_string()),
        },
    )
}

fn build_launch_context(app: &ShellApp, device_ip: &str) -> Result<LaunchContext, String> {
    let username = app
        .normalized_ssh_username()
        .ok_or_else(|| String::from("请先填写 SSH 用户名"))?;
    let password = (!app.password.trim().is_empty()).then_some(app.password.clone());
    let (vnc_username, vnc_username_source) = if app.vnc_enabled {
        resolve_vnc_value(app.vnc_user.trim(), Some(username.clone()))
    } else {
        (None, VncCredentialFieldSource::Unavailable)
    };
    let (vnc_password, vnc_password_source) = if app.vnc_enabled {
        resolve_vnc_value(app.vnc_password.trim(), password.clone())
    } else {
        (None, VncCredentialFieldSource::Unavailable)
    };
    let vnc_source = summarize_vnc_source(vnc_username_source, vnc_password_source);

    Ok(LaunchContext {
        device_ip: device_ip.to_owned(),
        username,
        password,
        vnc_requested: app.vnc_enabled,
        vnc_username,
        vnc_password,
        vnc_username_source,
        vnc_password_source,
        vnc_source,
    })
}

fn resolve_vnc_value(
    value: &str,
    fallback: Option<String>,
) -> (Option<String>, VncCredentialFieldSource) {
    let value = value.trim();
    if !value.is_empty() {
        return (Some(value.to_owned()), VncCredentialFieldSource::Dedicated);
    }

    if let Some(fallback) = fallback.filter(|candidate| !candidate.trim().is_empty()) {
        return (Some(fallback), VncCredentialFieldSource::SshFallback);
    }

    (None, VncCredentialFieldSource::Unavailable)
}

fn summarize_vnc_source(
    username_source: VncCredentialFieldSource,
    password_source: VncCredentialFieldSource,
) -> VncCredentialSource {
    if username_source == VncCredentialFieldSource::Dedicated
        || password_source == VncCredentialFieldSource::Dedicated
    {
        VncCredentialSource::Dedicated
    } else if username_source == VncCredentialFieldSource::SshFallback
        || password_source == VncCredentialFieldSource::SshFallback
    {
        VncCredentialSource::SshFallback
    } else {
        VncCredentialSource::Unavailable
    }
}

fn request_launch(app: &mut ShellApp, action: PendingToolAction) -> Task<Message> {
    if app.visual_check.is_some() {
        return Task::none();
    }

    let tool = action.tool_kind();
    app.is_connecting = true;
    app.connection_status = Some(action.status_message());

    if let Some(path) = resolve_tool_path(app, tool) {
        perform_launch(app, action, path)
    } else if app_finder::supports_native_tool_picker() {
        app.pending_tool_action = Some(action);
        Task::done(Message::RequestToolPath(tool))
    } else {
        app.is_connecting = false;
        app.connection_status = None;
        app.pending_tool_action = None;
        app.pending_docker_context = None;
        app.clear_active_quick_connect();
        app.set_quick_connect_notice(Notice {
            tone: NoticeTone::Warning,
            message: app_finder::unresolved_tool_path_notice(tool),
        })
    }
}

fn resolve_tool_path(app: &ShellApp, tool: ToolKind) -> Option<PathBuf> {
    app.app_paths
        .path_buf_for(tool)
        .filter(|path| path.is_file())
        .or_else(|| app_finder::find_tool(tool))
}

fn perform_launch(
    app: &mut ShellApp,
    action: PendingToolAction,
    tool_path: PathBuf,
) -> Task<Message> {
    let tool = action.tool_kind();
    app.pending_tool_action = Some(action.clone());
    app.connection_status = Some(action.status_message());

    Task::perform(
        async move { connect_tasks::execute_launch_action(action, tool_path).await },
        move |result| Message::ConnectResult(tool, result),
    )
}
