use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use ssh_core::ssh::auth::LaunchAuthPreparation;

use crate::process;

use super::ssh_target::prepare_vscode_target;
use super::types::LaunchError;

pub fn launch_vscode_ssh(
    vscode_path: &Path,
    ip: &str,
    user: &str,
    auth: &LaunchAuthPreparation,
) -> Result<(), LaunchError> {
    let target = prepare_vscode_target(ip, user, auth)?;
    let home_dir = if user == "root" {
        String::from("/root")
    } else {
        format!("/home/{user}")
    };
    let uri = format!("vscode-remote://ssh-remote+{target}{home_dir}");

    launch_vscode_uri(vscode_path, &uri)
}

pub fn launch_vscode_devcontainer(
    vscode_path: &Path,
    ip: &str,
    user: &str,
    folder_uri: &str,
    auth: &LaunchAuthPreparation,
) -> Result<(), LaunchError> {
    let _ = prepare_vscode_target(ip, user, auth)?;
    launch_vscode_uri(vscode_path, folder_uri)
}

fn launch_vscode_uri(vscode_path: &Path, uri: &str) -> Result<(), LaunchError> {
    let mut last_error: Option<io::Error> = None;

    for executable in vscode_executable_candidates(vscode_path) {
        match spawn_vscode_with_executable(&executable, uri) {
            Ok(()) => return Ok(()),
            Err(error) if should_retry_vscode_spawn(&error) => {
                last_error = Some(error);
            }
            Err(error) => return Err(LaunchError::Io(error)),
        }
    }

    if let Some(error) = last_error {
        Err(LaunchError::Io(error))
    } else {
        Err(LaunchError::Unsupported(String::from(
            "VS Code 启动入口不可用，请重新选择有效的 VS Code 路径",
        )))
    }
}

fn spawn_vscode_with_executable(executable: &Path, uri: &str) -> io::Result<()> {
    #[cfg(windows)]
    if should_launch_via_cmd(executable) {
        return spawn_vscode_via_cmd(executable, uri);
    }

    #[cfg(target_os = "macos")]
    if is_macos_app_bundle_path(executable) {
        return spawn_vscode_via_open(executable, uri);
    }

    let mut command = Command::new(executable);
    command.arg("--folder-uri").arg(uri);
    spawn_command(&mut command)
}

#[cfg(windows)]
fn spawn_vscode_via_cmd(executable: &Path, uri: &str) -> io::Result<()> {
    let mut command = Command::new("cmd");
    command
        .arg("/d")
        .arg("/c")
        .arg(executable)
        .arg("--folder-uri")
        .arg(uri);
    process::hide_console_window(&mut command);
    command.spawn().map(|_| ())
}

#[cfg(windows)]
fn should_launch_via_cmd(executable: &Path) -> bool {
    let extension = executable
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if matches!(extension.as_deref(), Some("cmd" | "bat")) {
        return true;
    }

    extension.is_none()
        && executable
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("code"))
}

fn should_retry_vscode_spawn(error: &io::Error) -> bool {
    #[cfg(windows)]
    {
        error.kind() == io::ErrorKind::NotFound || matches!(error.raw_os_error(), Some(193 | 2 | 3))
    }

    #[cfg(target_os = "macos")]
    {
        matches!(
            error.kind(),
            io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
        )
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = error;
        false
    }
}

fn vscode_executable_candidates(vscode_path: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    push_unique_path(&mut candidates, preferred_vscode_executable(vscode_path));
    #[cfg(windows)]
    if let Some(wrapper) = vscode_cmd_wrapper_candidate(vscode_path) {
        push_unique_path(&mut candidates, wrapper);
    }
    push_unique_path(&mut candidates, vscode_path.to_path_buf());
    candidates
}

fn push_unique_path(candidates: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

#[cfg(windows)]
fn preferred_vscode_executable(vscode_path: &Path) -> PathBuf {
    if let Some(candidate) = vscode_code_exe_candidate(vscode_path) {
        return candidate;
    }

    vscode_path.to_path_buf()
}

#[cfg(windows)]
fn vscode_code_exe_candidate(vscode_path: &Path) -> Option<PathBuf> {
    if !is_vscode_wrapper_path(vscode_path) {
        return None;
    }

    let Some(bin_dir) = vscode_path.parent() else {
        return None;
    };
    let Some(install_root) = bin_dir.parent() else {
        return None;
    };
    let candidate = install_root.join("Code.exe");
    candidate.is_file().then_some(candidate)
}

#[cfg(windows)]
fn vscode_cmd_wrapper_candidate(vscode_path: &Path) -> Option<PathBuf> {
    let file_name = vscode_path.file_name()?.to_str()?.to_ascii_lowercase();
    if file_name != "code.exe" {
        return None;
    }

    let install_root = vscode_path.parent()?;
    let bin_dir = install_root.join("bin");
    if !bin_dir.is_dir() {
        return None;
    }

    for candidate_name in ["code.cmd", "code"] {
        let candidate = bin_dir.join(candidate_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn preferred_vscode_executable(vscode_path: &Path) -> PathBuf {
    vscode_path.to_path_buf()
}

#[cfg(target_os = "macos")]
fn preferred_vscode_executable(vscode_path: &Path) -> PathBuf {
    vscode_bundle_cli_candidate(vscode_path).unwrap_or_else(|| vscode_path.to_path_buf())
}

#[cfg(windows)]
fn is_vscode_wrapper_path(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let file_name = file_name.to_ascii_lowercase();
    if matches!(extension.as_deref(), Some("cmd" | "bat")) {
        return file_name == "code.cmd" || file_name == "code.bat";
    }

    extension.is_none() && file_name == "code"
}

#[cfg(target_os = "macos")]
fn vscode_bundle_cli_candidate(vscode_path: &Path) -> Option<PathBuf> {
    if !is_macos_app_bundle_path(vscode_path) {
        return None;
    }

    let candidate = vscode_path.join("Contents/Resources/app/bin/code");
    candidate.is_file().then_some(candidate)
}

#[cfg(target_os = "macos")]
fn is_macos_app_bundle_path(path: &Path) -> bool {
    path.is_dir()
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("app"))
}

#[cfg(target_os = "macos")]
fn spawn_vscode_via_open(application_path: &Path, uri: &str) -> io::Result<()> {
    let mut command = Command::new("open");
    command
        .arg("-a")
        .arg(application_path)
        .arg("--args")
        .arg("--folder-uri")
        .arg(uri);
    spawn_command(&mut command)
}

fn spawn_command(command: &mut Command) -> io::Result<()> {
    process::hide_console_window(command);
    command.spawn().map(|_| ())
}
