use std::env;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::process::Command as StdCommand;

#[cfg(windows)]
use winreg::{RegKey, enums::HKEY_CURRENT_USER, enums::HKEY_LOCAL_MACHINE};

#[cfg(windows)]
use crate::process;

#[cfg(windows)]
use super::powershell::decode_windows_powershell_output;

pub(super) fn find_tool_with(
    path_candidates: &[&str],
    registry_candidates: &[&str],
    shortcut_candidates: &[&str],
    common_paths: &[&str],
) -> Option<PathBuf> {
    search_path(path_candidates)
        .or_else(|| search_registry(registry_candidates))
        .or_else(|| search_start_menu_shortcuts(shortcut_candidates))
        .or_else(|| search_common_paths(common_paths))
}

pub(super) fn is_launchable_tool_path(path: &Path) -> bool {
    path.is_file() || is_macos_app_bundle(path)
}

fn search_path(candidates: &[&str]) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let path_exts = windows_path_exts();

    for directory in env::split_paths(&path) {
        for candidate in candidates {
            let joined = directory.join(candidate);
            if is_launchable_tool_path(&joined) {
                return Some(joined);
            }

            if Path::new(candidate).extension().is_none() {
                for extension in &path_exts {
                    let joined = directory.join(format!("{candidate}{extension}"));
                    if is_launchable_tool_path(&joined) {
                        return Some(joined);
                    }
                }
            }
        }
    }

    None
}

#[cfg(windows)]
fn search_registry(candidates: &[&str]) -> Option<PathBuf> {
    for root in [
        RegKey::predef(HKEY_CURRENT_USER),
        RegKey::predef(HKEY_LOCAL_MACHINE),
    ] {
        for candidate in candidates {
            let key_path =
                format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{candidate}");
            let Ok(key) = root.open_subkey(&key_path) else {
                continue;
            };
            let Ok(value) = key.get_value::<String, _>("") else {
                continue;
            };

            let path = PathBuf::from(value);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(not(windows))]
fn search_registry(_candidates: &[&str]) -> Option<PathBuf> {
    None
}

#[cfg(windows)]
fn search_start_menu_shortcuts(candidates: &[&str]) -> Option<PathBuf> {
    if candidates.is_empty() {
        return None;
    }

    let names = candidates
        .iter()
        .map(|name| format!("'{}'", name.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ");
    let script = format!(
        r#"
$names = @({names})
$startMenuPaths = @(
    "$env:APPDATA\Microsoft\Windows\Start Menu\Programs",
    "$env:ProgramData\Microsoft\Windows\Start Menu\Programs"
)

$shell = New-Object -ComObject WScript.Shell
foreach ($startMenuPath in $startMenuPaths) {{
    if (-not (Test-Path $startMenuPath)) {{ continue }}
    $links = Get-ChildItem -Path $startMenuPath -Recurse -Filter *.lnk -ErrorAction SilentlyContinue
    foreach ($link in $links) {{
        try {{
            $shortcut = $shell.CreateShortcut($link.FullName)
            $target = $shortcut.TargetPath
            if (-not $target -or -not (Test-Path $target)) {{ continue }}
            $targetName = [System.IO.Path]::GetFileName($target)
            $linkName = $link.BaseName

            foreach ($name in $names) {{
                if ($targetName -like "*$name*" -or $linkName -like "*$name*") {{
                    Write-Output $target
                    exit 0
                }}
            }}
        }} catch {{
            continue
        }}
    }}
}}
"#
    );
    let mut command = StdCommand::new("powershell");
    command.args(["-NoProfile", "-Command", &script]);
    process::hide_console_window(&mut command);
    let output = command.output().ok()?;

    if !output.status.success() {
        return None;
    }

    decode_windows_powershell_output(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_file())
}

#[cfg(not(windows))]
fn search_start_menu_shortcuts(_candidates: &[&str]) -> Option<PathBuf> {
    None
}

fn search_common_paths(candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .map(|candidate| expand_common_path(candidate))
        .find(|path| is_launchable_tool_path(path))
}

fn expand_common_path(value: &str) -> PathBuf {
    #[cfg(not(windows))]
    if let Some(stripped) = value.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }

    PathBuf::from(expand_windows_env(value))
}

fn expand_windows_env(value: &str) -> String {
    let mut expanded = value.to_owned();
    for key in ["LOCALAPPDATA", "PROGRAMFILES", "PROGRAMFILES(X86)"] {
        let token = format!("%{key}%");
        if let Some(env_value) = env::var_os(key) {
            expanded = expanded.replace(&token, &env_value.to_string_lossy());
        }
    }

    expanded
}

fn windows_path_exts() -> Vec<String> {
    env::var_os("PATHEXT")
        .map(|value| {
            value
                .to_string_lossy()
                .split(';')
                .filter(|segment| !segment.trim().is_empty())
                .map(|segment| segment.to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .filter(|extensions| !extensions.is_empty())
        .unwrap_or_else(|| {
            vec![
                String::from(".exe"),
                String::from(".cmd"),
                String::from(".bat"),
            ]
        })
}

#[cfg(target_os = "macos")]
fn is_macos_app_bundle(path: &Path) -> bool {
    path.is_dir()
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("app"))
}

#[cfg(not(target_os = "macos"))]
fn is_macos_app_bundle(_path: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::is_launchable_tool_path;

    #[test]
    fn launchable_tool_path_accepts_regular_files() {
        let path = unique_temp_path("tool.bin");
        fs::write(&path, "binary").unwrap();

        assert!(is_launchable_tool_path(&path));

        cleanup_path(&path);
    }

    #[test]
    fn launchable_tool_path_rejects_plain_directories() {
        let path = unique_temp_path("plain-directory");
        fs::create_dir_all(&path).unwrap();

        assert!(!is_launchable_tool_path(&path));

        cleanup_path(&path);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchable_tool_path_accepts_app_bundles() {
        let path = unique_temp_path("Example.app");
        fs::create_dir_all(&path).unwrap();

        assert!(is_launchable_tool_path(&path));

        cleanup_path(&path);
    }

    fn unique_temp_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("lanscanner-platform-{nonce}-{name}"))
    }

    fn cleanup_path(path: &PathBuf) {
        if path.is_dir() {
            let _ = fs::remove_dir_all(path);
        } else {
            let _ = fs::remove_file(path);
        }
    }
}
