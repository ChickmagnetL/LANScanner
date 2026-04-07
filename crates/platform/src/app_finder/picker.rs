use std::path::PathBuf;

use ssh_core::credential::store::ToolKind;

#[cfg(windows)]
use tokio::process::Command as TokioCommand;

#[cfg(windows)]
use crate::process;

#[cfg(windows)]
use super::powershell::decode_windows_powershell_output;

pub(super) async fn pick_tool_path(tool: ToolKind) -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    {
        let title = format!("选择 {} 可执行文件", tool.label());
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             $dialog = New-Object System.Windows.Forms.OpenFileDialog; \
             $dialog.Title = '{}'; \
             $dialog.Filter = 'Executable (*.exe)|*.exe|All files (*.*)|*.*'; \
             if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {{ Write-Output $dialog.FileName }}",
            title.replace('\'', "''"),
        );
        let mut command = TokioCommand::new("powershell");
        command.args(["-NoProfile", "-STA", "-Command", &script]);
        process::hide_console_window_tokio(&mut command);
        let output = command
            .output()
            .await
            .map_err(|error| format!("无法打开 {} 路径选择器: {error}", tool.label()))?;

        if !output.status.success() {
            let stderr = decode_windows_powershell_output(&output.stderr);
            return Err(format!(
                "{} 路径选择器执行失败: {}",
                tool.label(),
                stderr.trim()
            ));
        }

        let selected = decode_windows_powershell_output(&output.stdout)
            .trim()
            .to_owned();
        if selected.is_empty() {
            Ok(None)
        } else {
            Ok(Some(PathBuf::from(selected)))
        }
    }

    #[cfg(not(windows))]
    {
        Err(super::non_windows_path_boundary_notice(tool))
    }
}
