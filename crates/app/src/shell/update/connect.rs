use std::path::PathBuf;
use std::time::Duration;

use iced::Task;
use platform::app_finder;
use ssh_core::credential::store::{self, ToolKind};
use ssh_core::docker::{self, Container};
use ssh_core::scanner::{Device, DeviceStatus};
use ssh_core::ssh::auth::{KeyReadySource, LaunchAuthConsumer, LaunchAuthPreparation};
use ui::theme::AppLanguage;

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
            message: tool_path_cancelled_message(app.app_language, tool),
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
                message: tool_path_save_failed_message(app.app_language, tool, &error.to_string()),
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
            tool_path_pick_failed_message(app.app_language, tool, &error)
        } else {
            unresolved_tool_path_message(app.app_language, tool)
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
            .unwrap_or_else(|| match app.app_language {
                AppLanguage::Chinese => String::from("正在准备连接"),
                AppLanguage::English => String::from("Preparing connection"),
            });
    }

    device_status_message(app.app_language, device.status)
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
    app.connection_status = Some(match app.app_language {
        AppLanguage::Chinese => String::from("正在启动终端连接"),
        AppLanguage::English => String::from("Launching shell connection"),
    });
    let language = app.app_language;

    Task::perform(
        async move {
            let auth_preparation =
                connect_tasks::prepare_ssh_launch_auth(&context, LaunchAuthConsumer::Shell).await?;
            platform::launcher::launch_shell_ssh(
                &context.device_ip,
                &context.username,
                &auth_preparation,
            )
            .map_err(|error| match language {
                AppLanguage::Chinese => format!("终端连接启动失败: {error}"),
                AppLanguage::English => format!("Failed to launch shell connection: {error}"),
            })?;
            Ok(shell_connect_notice_for_language(
                &auth_preparation,
                language,
            ))
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
        "{} {}",
        match app.app_language {
            AppLanguage::Chinese => "正在探测 RustDesk Direct IP 端口",
            AppLanguage::English => "Probing RustDesk Direct IP port",
        },
        connect_tasks::RUSTDESK_DIRECT_IP_PORT,
    ));

    let password = app.normalized_rustdesk_password();
    let language = app.app_language;
    Task::perform(
        async move {
            let result = probe_rustdesk_direct_ip_port_for_language(&device_ip, language).await;
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
    app.connection_status = Some(match app.app_language {
        AppLanguage::Chinese => String::from("正在读取远程 Docker 容器列表"),
        AppLanguage::English => String::from("Loading remote Docker containers"),
    });
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
        .ok_or_else(|| match app.app_language {
            AppLanguage::Chinese => String::from("请先填写 SSH 用户名"),
            AppLanguage::English => String::from("Enter an SSH username first"),
        })?;
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
    app.connection_status = Some(action.status_message_for_language(app.app_language));

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
            message: unresolved_tool_path_message(app.app_language, tool),
        })
    }
}

fn resolve_tool_path(app: &ShellApp, tool: ToolKind) -> Option<PathBuf> {
    app.app_paths
        .path_buf_for(tool)
        .filter(|path| platform::app_finder::is_launchable_tool_path(path))
        .or_else(|| app_finder::find_tool(tool))
}

fn perform_launch(
    app: &mut ShellApp,
    action: PendingToolAction,
    tool_path: PathBuf,
) -> Task<Message> {
    let tool = action.tool_kind();
    app.pending_tool_action = Some(action.clone());
    app.connection_status = Some(action.status_message_for_language(app.app_language));
    let language = app.app_language;

    Task::perform(
        async move { execute_launch_action_for_language(action, tool_path, language).await },
        move |result| Message::ConnectResult(tool, result),
    )
}

fn tool_path_cancelled_message(language: AppLanguage, tool: ToolKind) -> String {
    match language {
        AppLanguage::Chinese => format!("已取消选择 {} 路径", tool.label()),
        AppLanguage::English => format!("Cancelled selecting the path for {}", tool.label()),
    }
}

fn tool_path_save_failed_message(language: AppLanguage, tool: ToolKind, error: &str) -> String {
    match language {
        AppLanguage::Chinese => format!("保存 {} 路径失败: {error}", tool.label()),
        AppLanguage::English => format!("Failed to save the path for {}: {error}", tool.label()),
    }
}

fn tool_path_pick_failed_message(language: AppLanguage, tool: ToolKind, error: &str) -> String {
    match language {
        AppLanguage::Chinese => format!("选择 {} 路径失败: {error}", tool.label()),
        AppLanguage::English => {
            format!("Failed to choose the path for {}: {error}", tool.label())
        }
    }
}

fn unresolved_tool_path_message(language: AppLanguage, tool: ToolKind) -> String {
    match language {
        AppLanguage::Chinese => format!(
            "未找到 {} 路径。请先在系统中安装该工具，或手动选择其可执行文件。",
            tool.label()
        ),
        AppLanguage::English => format!(
            "No path is configured for {}. Install the tool first or choose its executable manually.",
            tool.label()
        ),
    }
}

fn device_status_message(language: AppLanguage, status: DeviceStatus) -> String {
    match (language, status) {
        (AppLanguage::Chinese, DeviceStatus::Untested) => {
            String::from("凭据尚未检测；填写 SSH 用户名后，扫描结束会自动检测，也可手动重试。")
        }
        (AppLanguage::English, DeviceStatus::Untested) => String::from(
            "Credentials have not been checked yet. Fill in an SSH username and the app will verify automatically after scanning, or you can retry manually.",
        ),
        (AppLanguage::Chinese, DeviceStatus::Ready) => {
            String::from("SSH 凭据检测成功；外部工具的免密前置会在启动连接时单独校验。")
        }
        (AppLanguage::English, DeviceStatus::Ready) => String::from(
            "SSH credential verification succeeded. Any passwordless launch preparation for external tools is checked separately when the connection starts.",
        ),
        (AppLanguage::Chinese, DeviceStatus::Denied) => {
            String::from("检测结果为错误（用户名明显错误或认证失败）；仍可直接发起快速连接。")
        }
        (AppLanguage::English, DeviceStatus::Denied) => String::from(
            "Verification failed, usually because the username is wrong or authentication was denied. You can still launch a quick connection directly.",
        ),
        (AppLanguage::Chinese, DeviceStatus::Error) => String::from(
            "检测结果为异常（仅用户名或网络抖动时可能无法稳定判定）；仍可直接发起快速连接。",
        ),
        (AppLanguage::English, DeviceStatus::Error) => String::from(
            "Verification ended in an indeterminate error, which can happen with username-only input or unstable network conditions. You can still launch a quick connection directly.",
        ),
    }
}

const RUSTDESK_DIRECT_IP_PROBE_TIMEOUT: Duration = Duration::from_secs(3);

async fn probe_rustdesk_direct_ip_port_for_language(
    device_ip: &str,
    language: AppLanguage,
) -> Result<(), String> {
    let endpoint = format!("{device_ip}:{}", connect_tasks::RUSTDESK_DIRECT_IP_PORT);
    let connect = tokio::time::timeout(
        RUSTDESK_DIRECT_IP_PROBE_TIMEOUT,
        tokio::net::TcpStream::connect(endpoint.as_str()),
    )
    .await;

    match connect {
        Ok(Ok(stream)) => {
            drop(stream);
            Ok(())
        }
        Ok(Err(error)) => Err(rustdesk_probe_failure_message(
            language,
            device_ip,
            &error.to_string(),
        )),
        Err(_) => Err(rustdesk_probe_failure_message(
            language,
            device_ip,
            localized(language, "连接超时", "connection timed out"),
        )),
    }
}

async fn execute_launch_action_for_language(
    action: PendingToolAction,
    tool_path: PathBuf,
    language: AppLanguage,
) -> Result<crate::message::ConnectNotice, String> {
    match action {
        PendingToolAction::Direct { tool, context } => match tool {
            ToolKind::Vscode => {
                let auth_preparation = connect_tasks::prepare_ssh_launch_auth(
                    &context,
                    LaunchAuthConsumer::VscodeLike,
                )
                .await?;
                platform::launcher::launch_vscode_ssh(
                    &tool_path,
                    &context.device_ip,
                    &context.username,
                    &auth_preparation,
                )
                .map_err(|error| match language {
                    AppLanguage::Chinese => format!("启动 VS Code 失败: {error}"),
                    AppLanguage::English => format!("Failed to launch VS Code: {error}"),
                })?;

                Ok(auth_connect_notice_for_language(
                    language,
                    "VS Code",
                    &auth_preparation,
                ))
            }
            ToolKind::Mobaxterm => {
                let auth_preparation =
                    connect_tasks::prepare_ssh_launch_auth(&context, LaunchAuthConsumer::Mobaxterm)
                        .await?;
                platform::launcher::launch_mobaxterm_ssh(
                    &tool_path,
                    &context.device_ip,
                    &context.username,
                    context.password.as_deref(),
                    &auth_preparation,
                )
                .map_err(|error| match language {
                    AppLanguage::Chinese => format!("启动 MobaXterm 失败: {error}"),
                    AppLanguage::English => format!("Failed to launch MobaXterm: {error}"),
                })?;

                Ok(mobaxterm_connect_notice_for_language(
                    language,
                    &auth_preparation,
                ))
            }
            ToolKind::VncViewer => {
                let launch_outcome = platform::launcher::launch_vncviewer(
                    &tool_path,
                    &context.device_ip,
                    context.vnc_username.as_deref(),
                    context.vnc_password.as_deref(),
                )
                .map_err(|error| match language {
                    AppLanguage::Chinese => format!("启动 VNC Viewer 失败: {error}"),
                    AppLanguage::English => format!("Failed to launch VNC Viewer: {error}"),
                })?;

                let mut messages = Vec::new();
                if let Some(message) = context.vnc_resolution_message_for_language(language) {
                    messages.push(message);
                }
                if let Some(warning) = launch_outcome.warning {
                    messages.push(warning);
                }

                if messages.is_empty() {
                    Ok(success_connect_notice(localized(
                        language,
                        "已启动 VNC Viewer",
                        "Launched VNC Viewer",
                    )))
                } else {
                    Ok(warning_connect_notice(localized(
                        language,
                        "已启动 VNC Viewer，部分凭据需手动输入",
                        "Launched VNC Viewer, but some credentials still need to be entered manually",
                    )))
                }
            }
            ToolKind::RustDesk => {
                let rustdesk_password = context.vnc_password.as_deref();
                platform::launcher::launch_rustdesk(
                    &tool_path,
                    &context.device_ip,
                    rustdesk_password,
                )
                .map_err(|error| match language {
                    AppLanguage::Chinese => format!("启动 RustDesk 失败: {error}"),
                    AppLanguage::English => format!("Failed to launch RustDesk: {error}"),
                })?;

                if rustdesk_password.is_some() {
                    Ok(success_connect_notice(localized(
                        language,
                        "已启动 RustDesk，已尝试带入连接密码",
                        "Launched RustDesk and attempted to pass the connection password",
                    )))
                } else {
                    Ok(warning_connect_notice(localized(
                        language,
                        "已启动 RustDesk，但未提供连接密码，请在 RustDesk 客户端中手动输入",
                        "Launched RustDesk, but no connection password was provided. Enter it manually in the RustDesk client.",
                    )))
                }
            }
        },
        PendingToolAction::DockerAttach { context, container } => {
            let auth_preparation =
                connect_tasks::prepare_ssh_launch_auth(&context, LaunchAuthConsumer::VscodeLike)
                    .await?;

            if !container.is_running {
                docker::restart_container(
                    &context.device_ip,
                    &context.username,
                    context.password.as_deref(),
                    &container.id,
                )
                .await
                .map_err(|error| match language {
                    AppLanguage::Chinese => format!("启动 Docker 容器失败: {error}"),
                    AppLanguage::English => {
                        format!("Failed to start the Docker container: {error}")
                    }
                })?;
            }

            let host_target = auth_preparation.host_target(&context.device_ip, &context.username);
            let uri = docker::prepare_devcontainer_uri(
                &host_target,
                &context.device_ip,
                &context.username,
                context.password.as_deref(),
                &container.id,
                &container.name,
            )
            .await
            .map_err(|error| match language {
                AppLanguage::Chinese => format!("准备 Docker attach URI 失败: {error}"),
                AppLanguage::English => format!("Failed to prepare the Docker attach URI: {error}"),
            })?;

            platform::launcher::launch_vscode_devcontainer(
                &tool_path,
                &context.device_ip,
                &context.username,
                &uri,
                &auth_preparation,
            )
            .map_err(|error| match language {
                AppLanguage::Chinese => format!("启动 VS Code Docker attach 失败: {error}"),
                AppLanguage::English => format!("Failed to launch VS Code Docker attach: {error}"),
            })?;

            let action = match language {
                AppLanguage::Chinese => format!("VS Code Docker attach（{}）", container.name),
                AppLanguage::English => format!("VS Code Docker attach ({})", container.name),
            };
            Ok(auth_connect_notice_for_language(
                language,
                action.as_str(),
                &auth_preparation,
            ))
        }
    }
}

fn shell_connect_notice_for_language(
    preparation: &LaunchAuthPreparation,
    language: AppLanguage,
) -> crate::message::ConnectNotice {
    match preparation {
        LaunchAuthPreparation::KeyReady { .. } => success_connect_notice(localized(
            language,
            "已启动终端连接",
            "Launched shell connection",
        )),
        LaunchAuthPreparation::PasswordFallback { .. } => warning_connect_notice(localized(
            language,
            "已打开终端，请按提示输入 SSH 密码",
            "Opened the terminal. Enter the SSH password when prompted.",
        )),
        LaunchAuthPreparation::HardFailure { reason } => warning_connect_notice(reason.clone()),
    }
}

fn auth_connect_notice_for_language(
    language: AppLanguage,
    action: &str,
    preparation: &LaunchAuthPreparation,
) -> crate::message::ConnectNotice {
    match preparation {
        LaunchAuthPreparation::KeyReady {
            source: KeyReadySource::Existing,
            ..
        } => success_connect_notice(match language {
            AppLanguage::Chinese => format!("已启动 {action}"),
            AppLanguage::English => format!("Launched {action}"),
        }),
        LaunchAuthPreparation::KeyReady {
            source: KeyReadySource::Installed,
            ..
        } => success_connect_notice(match language {
            AppLanguage::Chinese => format!("已完成免密准备并启动 {action}"),
            AppLanguage::English => format!("Prepared passwordless access and launched {action}"),
        }),
        LaunchAuthPreparation::PasswordFallback { reason } => {
            warning_connect_notice(match language {
                AppLanguage::Chinese => format!("已启动 {action}，请按提示输入密码（{reason}）"),
                AppLanguage::English => {
                    format!("Launched {action}. Enter the password when prompted ({reason})")
                }
            })
        }
        LaunchAuthPreparation::HardFailure { reason } => warning_connect_notice(reason.clone()),
    }
}

fn mobaxterm_connect_notice_for_language(
    language: AppLanguage,
    preparation: &LaunchAuthPreparation,
) -> crate::message::ConnectNotice {
    match preparation {
        LaunchAuthPreparation::KeyReady { .. } => success_connect_notice(localized(
            language,
            "MobaXterm 已启动",
            "Launched MobaXterm",
        )),
        other => auth_connect_notice_for_language(language, "MobaXterm", other),
    }
}

fn success_connect_notice(message: impl Into<String>) -> crate::message::ConnectNotice {
    crate::message::ConnectNotice {
        tone: ConnectNoticeTone::Success,
        message: message.into(),
    }
}

fn warning_connect_notice(message: impl Into<String>) -> crate::message::ConnectNotice {
    crate::message::ConnectNotice {
        tone: ConnectNoticeTone::Warning,
        message: message.into(),
    }
}

fn localized(language: AppLanguage, chinese: &'static str, english: &'static str) -> &'static str {
    match language {
        AppLanguage::Chinese => chinese,
        AppLanguage::English => english,
    }
}

fn rustdesk_probe_failure_message(language: AppLanguage, device_ip: &str, detail: &str) -> String {
    match language {
        AppLanguage::Chinese => format!(
            "RustDesk 默认 Direct IP 端口 {} 不可达：{}:{}（{}）。请检查：目标是否已安装 RustDesk、RustDesk 是否正在运行、是否已开启 Direct IP、防火墙是否拦截端口 {}；若目标使用非默认端口，当前版本暂不支持。",
            connect_tasks::RUSTDESK_DIRECT_IP_PORT,
            device_ip,
            connect_tasks::RUSTDESK_DIRECT_IP_PORT,
            detail,
            connect_tasks::RUSTDESK_DIRECT_IP_PORT,
        ),
        AppLanguage::English => format!(
            "RustDesk's default Direct IP port {} is unreachable at {}:{} ({}). Check whether RustDesk is installed on the target, whether the service is running, whether Direct IP is enabled, and whether a firewall is blocking port {}. Non-default ports are not supported in the current version.",
            connect_tasks::RUSTDESK_DIRECT_IP_PORT,
            device_ip,
            connect_tasks::RUSTDESK_DIRECT_IP_PORT,
            detail,
            connect_tasks::RUSTDESK_DIRECT_IP_PORT,
        ),
    }
}
