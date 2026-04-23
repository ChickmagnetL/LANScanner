mod discovery;
mod picker;
mod powershell;
#[cfg(windows)]
mod vscode_windows;

use std::path::{Path, PathBuf};

use ssh_core::credential::store::ToolKind;

pub fn find_tool(tool: ToolKind) -> Option<PathBuf> {
    match tool {
        ToolKind::Vscode => find_vscode(),
        ToolKind::Mobaxterm => find_mobaxterm(),
        ToolKind::VncViewer => find_vncviewer(),
        ToolKind::RustDesk => find_rustdesk(),
    }
}

pub fn supports_native_tool_picker() -> bool {
    cfg!(windows)
}

pub fn is_launchable_tool_path(path: &Path) -> bool {
    discovery::is_launchable_tool_path(path)
}

pub fn unresolved_tool_path_notice(tool: ToolKind) -> String {
    #[cfg(windows)]
    {
        format!("未找到 {} 可执行文件，请选择安装路径后重试。", tool.label())
    }

    #[cfg(not(windows))]
    {
        non_windows_path_boundary_notice(tool)
    }
}

#[cfg(target_os = "macos")]
pub(super) fn non_windows_path_boundary_notice(tool: ToolKind) -> String {
    format!(
        "未找到 {} 可执行文件。macOS 目前支持已保存路径、PATH 自动发现，以及 /Applications 或 ~/Applications 下常见 .app 应用包自动发现，暂不提供原生路径选择器。请将 {} 加入 PATH，或在 ~/.lanscanner/config.json 的 app_paths 中预先配置可执行文件或 .app 路径后重试。",
        tool.label(),
        tool.label(),
    )
}

#[cfg(all(not(windows), not(target_os = "macos")))]
pub(super) fn non_windows_path_boundary_notice(tool: ToolKind) -> String {
    format!(
        "未找到 {} 可执行文件。Linux/macOS MVP 仅支持已保存路径或 PATH 自动发现，暂不提供原生路径选择器。请将 {} 加入 PATH，或在 ~/.lanscanner/config.json 的 app_paths 中预先配置路径后重试。",
        tool.label(),
        tool.label(),
    )
}

pub fn find_vscode() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        discovery::find_tool_with(
            &["Code.exe", "code.cmd", "code"],
            &["Code.exe", "code.cmd"],
            &["Visual Studio Code", "Code", "Code.exe", "code.cmd"],
            &[
                r"%LOCALAPPDATA%\Programs\Microsoft VS Code\Code.exe",
                r"%PROGRAMFILES%\Microsoft VS Code\Code.exe",
                r"%LOCALAPPDATA%\Programs\Microsoft VS Code\bin\code.cmd",
                r"%PROGRAMFILES%\Microsoft VS Code\bin\code.cmd",
                r"%PROGRAMFILES(X86)%\Microsoft VS Code\bin\code.cmd",
            ],
        )
        .and_then(vscode_windows::normalize_windows_vscode_path)
    }

    #[cfg(target_os = "macos")]
    {
        discovery::find_tool_with(
            &["code"],
            &[],
            &[],
            &[
                "/Applications/Visual Studio Code.app",
                "~/Applications/Visual Studio Code.app",
            ],
        )
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        discovery::find_tool_with(&["code"], &[], &[], &[])
    }
}

pub fn find_mobaxterm() -> Option<PathBuf> {
    discovery::find_tool_with(
        &["MobaXterm", "MobaXterm.exe", "MobaXterm_Personal.exe"],
        &["MobaXterm.exe", "MobaXterm_Personal.exe", "MobaXterm"],
        &["MobaXterm", "MobaXterm.exe", "MobaXterm_Personal.exe"],
        &[
            r"%PROGRAMFILES%\Mobatek\MobaXterm\MobaXterm.exe",
            r"%PROGRAMFILES(X86)%\Mobatek\MobaXterm\MobaXterm.exe",
            r"%LOCALAPPDATA%\Programs\Mobatek\MobaXterm\MobaXterm.exe",
            r"C:\Program Files\Mobatek\MobaXterm Personal Edition\MobaXterm_Personal.exe",
            r"C:\Program Files (x86)\Mobatek\MobaXterm Personal Edition\MobaXterm_Personal.exe",
            r"%PROGRAMFILES%\Mobatek\MobaXterm Personal Edition\MobaXterm_Personal.exe",
            r"%PROGRAMFILES(X86)%\Mobatek\MobaXterm Personal Edition\MobaXterm_Personal.exe",
        ],
    )
}

pub fn find_vncviewer() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        discovery::find_tool_with(
            &["vncviewer"],
            &[],
            &[],
            &[
                "/Applications/VNC Viewer.app",
                "~/Applications/VNC Viewer.app",
            ],
        )
    }

    #[cfg(not(target_os = "macos"))]
    {
        discovery::find_tool_with(
            &["VNC Viewer", "vncviewer", "vncviewer.exe"],
            &["vncviewer.exe", "VNCViewer.exe"],
            &["VNC Viewer", "vncviewer", "vncviewer.exe"],
            &[
                r"%PROGRAMFILES%\RealVNC\VNC Viewer\vncviewer.exe",
                r"%PROGRAMFILES(X86)%\RealVNC\VNC Viewer\vncviewer.exe",
                r"%PROGRAMFILES%\TightVNC\vncviewer.exe",
                r"%PROGRAMFILES(X86)%\TightVNC\vncviewer.exe",
                r"%PROGRAMFILES%\UltraVNC\vncviewer.exe",
                r"%PROGRAMFILES(X86)%\UltraVNC\vncviewer.exe",
                r"%LOCALAPPDATA%\Programs\RealVNC\VNC Viewer\vncviewer.exe",
            ],
        )
    }
}

pub fn find_rustdesk() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        discovery::find_tool_with(
            &["RustDesk", "RustDesk.exe", "rustdesk", "rustdesk.exe"],
            &["RustDesk.exe", "rustdesk.exe"],
            &["RustDesk", "RustDesk.exe", "rustdesk"],
            &[
                r"%PROGRAMFILES%\RustDesk\RustDesk.exe",
                r"%PROGRAMFILES(X86)%\RustDesk\RustDesk.exe",
                r"%LOCALAPPDATA%\Programs\RustDesk\RustDesk.exe",
                r"%LOCALAPPDATA%\RustDesk\RustDesk.exe",
            ],
        )
    }

    #[cfg(target_os = "macos")]
    {
        discovery::find_tool_with(
            &["rustdesk"],
            &[],
            &[],
            &["/Applications/RustDesk.app", "~/Applications/RustDesk.app"],
        )
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        discovery::find_tool_with(&["rustdesk"], &[], &[], &[])
    }
}

pub async fn pick_tool_path(tool: ToolKind) -> Result<Option<PathBuf>, String> {
    picker::pick_tool_path(tool).await
}
