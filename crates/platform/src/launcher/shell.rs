use std::io;
use std::process::Command;

use ssh_core::ssh::auth::LaunchAuthPreparation;

use super::ssh_target::prepare_shell_target;
use super::types::LaunchError;

pub fn launch_shell_ssh(
    ip: &str,
    user: &str,
    auth: &LaunchAuthPreparation,
) -> Result<(), LaunchError> {
    let target = prepare_shell_target(ip, user, auth)?;
    launch_terminal_with_ssh_target(&target)
}

#[cfg(windows)]
fn launch_terminal_with_ssh_target(target: &str) -> Result<(), LaunchError> {
    let candidates = ["Windows Terminal", "cmd", "PowerShell"];
    let mut last_error: Option<io::Error> = None;

    match spawn_windows_terminal_with_ssh_target(target) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => last_error = Some(error),
    }
    match spawn_cmd_terminal_with_ssh_target(target) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => last_error = Some(error),
    }
    match spawn_powershell_terminal_with_ssh_target(target) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => last_error = Some(error),
    }

    if let Some(error) = last_error {
        return Err(LaunchError::Io(error));
    }

    Err(LaunchError::Unsupported(format!(
        "未找到可用终端程序（已尝试 {}），无法启动 SSH Shell",
        candidates.join("、")
    )))
}

#[cfg(windows)]
fn spawn_windows_terminal_with_ssh_target(target: &str) -> io::Result<()> {
    let mut command = Command::new("wt");
    command
        .arg("-w")
        .arg("0")
        .arg("new-tab")
        .arg("ssh")
        .arg(target);
    command.spawn().map(|_| ())
}

#[cfg(windows)]
fn spawn_cmd_terminal_with_ssh_target(target: &str) -> io::Result<()> {
    let mut command = Command::new("cmd");
    command.arg("/d").arg("/k").arg(format!("ssh {target}"));
    command.spawn().map(|_| ())
}

#[cfg(windows)]
fn spawn_powershell_terminal_with_ssh_target(target: &str) -> io::Result<()> {
    let mut command = Command::new("powershell");
    command
        .arg("-NoExit")
        .arg("-Command")
        .arg(format!("ssh {target}"));
    command.spawn().map(|_| ())
}

#[cfg(target_os = "macos")]
fn launch_terminal_with_ssh_target(target: &str) -> Result<(), LaunchError> {
    let ssh_command = format!("ssh {target}");
    let escaped = escape_applescript_string(&ssh_command);
    let mut command = Command::new("osascript");
    command
        .arg("-e")
        .arg(format!(
            "tell application \"Terminal\" to do script \"{escaped}\""
        ))
        .arg("-e")
        .arg("tell application \"Terminal\" to activate");
    match command.spawn() {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(LaunchError::Unsupported(
            String::from("未找到 osascript，无法通过 macOS Terminal 启动 SSH Shell"),
        )),
        Err(error) => Err(LaunchError::Io(error)),
    }
}

#[cfg(target_os = "macos")]
fn escape_applescript_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
        .replace('\r', " ")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn launch_terminal_with_ssh_target(target: &str) -> Result<(), LaunchError> {
    let candidates = [
        "x-terminal-emulator",
        "gnome-terminal",
        "konsole",
        "xfce4-terminal",
        "xterm",
        "alacritty",
    ];
    let mut last_error: Option<io::Error> = None;

    match spawn_x_terminal_emulator_with_ssh_target(target) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => last_error = Some(error),
    }
    match spawn_gnome_terminal_with_ssh_target(target) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => last_error = Some(error),
    }
    match spawn_konsole_with_ssh_target(target) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => last_error = Some(error),
    }
    match spawn_xfce4_terminal_with_ssh_target(target) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => last_error = Some(error),
    }
    match spawn_xterm_with_ssh_target(target) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => last_error = Some(error),
    }
    match spawn_alacritty_with_ssh_target(target) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => last_error = Some(error),
    }

    if let Some(error) = last_error {
        return Err(LaunchError::Io(error));
    }

    Err(LaunchError::Unsupported(format!(
        "未找到可用终端程序（已尝试 {}），无法启动 SSH Shell。Linux MVP 仅支持通过这些终端启动，请确认终端已安装且在 PATH 中。",
        candidates.join("、")
    )))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_x_terminal_emulator_with_ssh_target(target: &str) -> io::Result<()> {
    let mut command = Command::new("x-terminal-emulator");
    command.arg("-e").arg("ssh").arg(target);
    command.spawn().map(|_| ())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_gnome_terminal_with_ssh_target(target: &str) -> io::Result<()> {
    let mut command = Command::new("gnome-terminal");
    command.arg("--").arg("ssh").arg(target);
    command.spawn().map(|_| ())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_konsole_with_ssh_target(target: &str) -> io::Result<()> {
    let mut command = Command::new("konsole");
    command.arg("-e").arg("ssh").arg(target);
    command.spawn().map(|_| ())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_xfce4_terminal_with_ssh_target(target: &str) -> io::Result<()> {
    let mut command = Command::new("xfce4-terminal");
    command.arg("-x").arg("ssh").arg(target);
    command.spawn().map(|_| ())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_xterm_with_ssh_target(target: &str) -> io::Result<()> {
    let mut command = Command::new("xterm");
    command.arg("-e").arg("ssh").arg(target);
    command.spawn().map(|_| ())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn spawn_alacritty_with_ssh_target(target: &str) -> io::Result<()> {
    let mut command = Command::new("alacritty");
    command.arg("-e").arg("ssh").arg(target);
    command.spawn().map(|_| ())
}
