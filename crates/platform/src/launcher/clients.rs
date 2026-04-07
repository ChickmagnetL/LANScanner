use std::path::Path;
use std::process::Command;

use ssh_core::ssh::auth::LaunchAuthPreparation;

use crate::process;

use super::types::{LaunchError, VncLaunchOutcome};

pub fn launch_mobaxterm_ssh(
    moba_path: &Path,
    ip: &str,
    user: &str,
    password: Option<&str>,
    auth: &LaunchAuthPreparation,
) -> Result<(), LaunchError> {
    let _ = password;
    let mut command = Command::new(moba_path);

    match auth {
        LaunchAuthPreparation::KeyReady { key_path, .. } => {
            let ssh_command = build_mobaxterm_key_command(ip, user, key_path);
            command.arg("-newtab").arg(ssh_command);
        }
        LaunchAuthPreparation::PasswordFallback { .. } => {
            command.arg("-newtab").arg(format!("ssh {user}@{ip}"));
        }
        LaunchAuthPreparation::HardFailure { reason } => {
            return Err(LaunchError::Unsupported(reason.clone()));
        }
    }

    process::hide_console_window(&mut command);
    command.spawn()?;
    Ok(())
}

pub fn launch_vncviewer(
    vnc_path: &Path,
    ip: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<VncLaunchOutcome, LaunchError> {
    let mut command = Command::new(vnc_path);
    command.arg("-UseAddressBook").arg(ip);
    process::hide_console_window(&mut command);
    command.spawn()?;

    let mut omitted_fields = Vec::new();
    if username.is_some_and(|value| !value.trim().is_empty()) {
        omitted_fields.push("用户名");
    }
    if password.is_some_and(|value| !value.trim().is_empty()) {
        omitted_fields.push("密码");
    }

    let warning = if omitted_fields.is_empty() {
        None
    } else {
        Some(format!(
            "当前 VNC Viewer 启动链路未安全自动带入{}，已仅启动客户端，请在客户端中手动输入。",
            omitted_fields.join("和")
        ))
    };

    Ok(VncLaunchOutcome { warning })
}

pub fn launch_rustdesk(
    rustdesk_path: &Path,
    target: &str,
    password: Option<&str>,
) -> Result<(), LaunchError> {
    let password = password.map(str::trim).filter(|value| !value.is_empty());

    let mut command = Command::new(rustdesk_path);
    command.arg("--connect").arg(target);
    if let Some(password) = password {
        command.arg("--password").arg(password);
    }
    process::hide_console_window(&mut command);
    command.spawn()?;
    Ok(())
}

fn build_mobaxterm_key_command(ip: &str, user: &str, key_path: &Path) -> String {
    let target = format!("{user}@{ip}");
    let key_argument = mobaxterm_key_argument(key_path);
    format!(
        "ssh -i {key_argument} -o IdentitiesOnly=yes -o PreferredAuthentications=publickey,password -o HostKeyAlgorithms=+ssh-rsa -o PubkeyAcceptedAlgorithms=+ssh-rsa -o PubkeyAcceptedKeyTypes=+ssh-rsa {target}"
    )
}

#[cfg(windows)]
fn mobaxterm_key_argument(key_path: &Path) -> String {
    let raw = key_path.to_string_lossy().replace('\\', "/");
    let converted = if raw.len() >= 3 && raw.as_bytes().get(1) == Some(&b':') {
        let drive = raw
            .chars()
            .next()
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or('c');
        format!("/drives/{drive}{}", &raw[2..])
    } else {
        raw
    };

    if converted.contains(' ') {
        format!("\"{converted}\"")
    } else {
        converted
    }
}

#[cfg(not(windows))]
fn mobaxterm_key_argument(key_path: &Path) -> String {
    let raw = key_path.to_string_lossy().to_string();
    if raw.contains(' ') {
        format!("\"{raw}\"")
    } else {
        raw
    }
}
