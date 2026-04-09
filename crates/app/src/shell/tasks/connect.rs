use std::path::PathBuf;
use std::time::Duration;

use platform::launcher;
use ssh_core::credential::store::ToolKind;
use ssh_core::docker;
use ssh_core::ssh::auth::{self, KeyReadySource, LaunchAuthConsumer, LaunchAuthPreparation};
use ui::theme::AppLanguage;

use crate::message::{ConnectNotice, ConnectNoticeTone};

use super::super::{LaunchContext, PendingToolAction};

pub(super) const RUSTDESK_DIRECT_IP_PORT: u16 = 21118;
const RUSTDESK_DIRECT_IP_PROBE_TIMEOUT: Duration = Duration::from_secs(3);

pub(super) async fn probe_rustdesk_direct_ip_port(device_ip: &str) -> Result<(), String> {
    probe_rustdesk_direct_ip_port_for_language(device_ip, AppLanguage::Chinese).await
}

async fn probe_rustdesk_direct_ip_port_for_language(
    device_ip: &str,
    language: AppLanguage,
) -> Result<(), String> {
    let endpoint = format!("{device_ip}:{RUSTDESK_DIRECT_IP_PORT}");
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
            device_ip,
            &error.to_string(),
            language,
        )),
        Err(_) => Err(rustdesk_probe_failure_message(
            device_ip,
            localized(language, "连接超时", "connection timed out"),
            language,
        )),
    }
}

pub(super) async fn execute_launch_action(
    action: PendingToolAction,
    tool_path: PathBuf,
) -> Result<ConnectNotice, String> {
    execute_launch_action_for_language(action, tool_path, AppLanguage::Chinese).await
}

async fn execute_launch_action_for_language(
    action: PendingToolAction,
    tool_path: PathBuf,
    language: AppLanguage,
) -> Result<ConnectNotice, String> {
    match action {
        PendingToolAction::Direct { tool, context } => match tool {
            ToolKind::Vscode => {
                let auth_preparation =
                    prepare_ssh_launch_auth(&context, LaunchAuthConsumer::VscodeLike).await?;
                launcher::launch_vscode_ssh(
                    &tool_path,
                    &context.device_ip,
                    &context.username,
                    &auth_preparation,
                )
                .map_err(|error| match language {
                    AppLanguage::Chinese => format!("启动 VS Code 失败: {error}"),
                    AppLanguage::English => format!("Failed to launch VS Code: {error}"),
                })?;

                Ok(auth_connect_notice(language, "VS Code", &auth_preparation))
            }
            ToolKind::Mobaxterm => {
                let auth_preparation =
                    prepare_ssh_launch_auth(&context, LaunchAuthConsumer::Mobaxterm).await?;
                launcher::launch_mobaxterm_ssh(
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

                Ok(mobaxterm_connect_notice(language, &auth_preparation))
            }
            ToolKind::VncViewer => {
                let launch_outcome = launcher::launch_vncviewer(
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
                launcher::launch_rustdesk(&tool_path, &context.device_ip, rustdesk_password)
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
                prepare_ssh_launch_auth(&context, LaunchAuthConsumer::VscodeLike).await?;

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
                AppLanguage::English => {
                    format!("Failed to prepare the Docker attach URI: {error}")
                }
            })?;

            launcher::launch_vscode_devcontainer(
                &tool_path,
                &context.device_ip,
                &context.username,
                &uri,
                &auth_preparation,
            )
            .map_err(|error| match language {
                AppLanguage::Chinese => format!("启动 VS Code Docker attach 失败: {error}"),
                AppLanguage::English => {
                    format!("Failed to launch VS Code Docker attach: {error}")
                }
            })?;

            Ok(auth_connect_notice(
                language,
                &match language {
                    AppLanguage::Chinese => format!("VS Code Docker attach（{}）", container.name),
                    AppLanguage::English => {
                        format!("VS Code Docker attach ({})", container.name)
                    }
                },
                &auth_preparation,
            ))
        }
    }
}

pub(super) async fn prepare_ssh_launch_auth(
    context: &LaunchContext,
    consumer: LaunchAuthConsumer,
) -> Result<LaunchAuthPreparation, String> {
    let preparation = auth::prepare_launch_auth_for_consumer(
        &context.device_ip,
        &context.username,
        context.password.as_deref(),
        consumer,
        auth::LAUNCH_AUTH_TIMEOUT,
    )
    .await;

    match preparation {
        LaunchAuthPreparation::HardFailure { reason } => Err(reason),
        other => Ok(other),
    }
}

pub(super) fn shell_connect_notice(preparation: &LaunchAuthPreparation) -> ConnectNotice {
    shell_connect_notice_for_language(preparation, AppLanguage::Chinese)
}

fn shell_connect_notice_for_language(
    preparation: &LaunchAuthPreparation,
    language: AppLanguage,
) -> ConnectNotice {
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

fn rustdesk_probe_failure_message(device_ip: &str, detail: &str, language: AppLanguage) -> String {
    match language {
        AppLanguage::Chinese => format!(
            "RustDesk 默认 Direct IP 端口 {RUSTDESK_DIRECT_IP_PORT} 不可达：{device_ip}:{RUSTDESK_DIRECT_IP_PORT}（{detail}）。请检查：目标是否已安装 RustDesk、RustDesk 是否正在运行、是否已开启 Direct IP、防火墙是否拦截端口 {RUSTDESK_DIRECT_IP_PORT}；若目标使用非默认端口，当前版本暂不支持。"
        ),
        AppLanguage::English => format!(
            "RustDesk's default Direct IP port {RUSTDESK_DIRECT_IP_PORT} is unreachable at {device_ip}:{RUSTDESK_DIRECT_IP_PORT} ({detail}). Check whether RustDesk is installed on the target, whether the service is running, whether Direct IP is enabled, and whether a firewall is blocking port {RUSTDESK_DIRECT_IP_PORT}. Non-default ports are not supported in the current version."
        ),
    }
}

fn auth_connect_notice(
    language: AppLanguage,
    action: &str,
    preparation: &LaunchAuthPreparation,
) -> ConnectNotice {
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

fn mobaxterm_connect_notice(
    language: AppLanguage,
    preparation: &LaunchAuthPreparation,
) -> ConnectNotice {
    match preparation {
        LaunchAuthPreparation::KeyReady { .. } => success_connect_notice(localized(
            language,
            "MobaXterm 已启动",
            "Launched MobaXterm",
        )),
        other => auth_connect_notice(language, "MobaXterm", other),
    }
}

fn success_connect_notice(message: impl Into<String>) -> ConnectNotice {
    ConnectNotice {
        tone: ConnectNoticeTone::Success,
        message: message.into(),
    }
}

fn warning_connect_notice(message: impl Into<String>) -> ConnectNotice {
    ConnectNotice {
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
