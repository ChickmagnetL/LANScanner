use std::path::{Path, PathBuf};

pub(super) fn normalize_windows_vscode_path(path: PathBuf) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_str()?.to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);

    if extension.as_deref() == Some("exe") {
        return Some(path);
    }

    if matches!(extension.as_deref(), Some("cmd" | "bat")) {
        return Some(prefer_windows_vscode_code_exe(&path).unwrap_or(path));
    }

    if extension.is_none() && file_name == "code" {
        if let Some(executable) = prefer_windows_vscode_code_exe(&path) {
            return Some(executable);
        }

        let cmd_candidate = path.with_extension("cmd");
        if cmd_candidate.is_file() {
            return Some(cmd_candidate);
        }
    }

    Some(path)
}

fn prefer_windows_vscode_code_exe(path: &Path) -> Option<PathBuf> {
    let bin_dir = path.parent()?;
    let install_root = bin_dir.parent()?;
    let code_exe = install_root.join("Code.exe");
    code_exe.is_file().then_some(code_exe)
}
