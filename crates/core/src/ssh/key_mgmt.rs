#[cfg(windows)]
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::fs;
#[cfg(windows)]
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
#[cfg(windows)]
use std::time::{SystemTime, UNIX_EPOCH};

use russh::keys::ssh_key::LineEnding;
use russh::keys::ssh_key::rand_core::OsRng;
use russh::keys::{self, Algorithm, PrivateKey};

use crate::credential::store;

use super::auth::{self, SshError};

const LANSCANNER_MANAGED_KEY: &str = "lanscanner_id_rsa";
const LANSCANNER_LEGACY_MANAGED_ED25519_KEY: &str = "lanscanner_id_ed25519";
const LANSCANNER_LEGACY_MANAGED_PRESERVE_PREFIX: &str = "lanscanner_id_ed25519_legacy";
#[cfg(windows)]
const EXTERNAL_TEMP_KEY_STALE_TTL_SECS: u64 = 12 * 60 * 60;
#[cfg(windows)]
const EXTERNAL_TEMP_KEY_ROOT_DIR: &str = "lanscanner";
#[cfg(windows)]
const CREATE_NO_WINDOW_FLAG: u32 = 0x0800_0000;
const DEFAULT_PRIVATE_KEYS: [&str; 7] = [
    "id_ed25519",
    "id_ecdsa",
    "id_ecdsa_sk",
    "id_ed25519_sk",
    "id_rsa",
    "identity",
    "id_dsa",
];

#[derive(Debug)]
pub enum KeyError {
    Io(io::Error),
    Store(store::StoreError),
    Ssh(SshError),
    Key(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPrivateKeyForExternalUse {
    pub path: PathBuf,
    pub cleanup_on_failure: bool,
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Store(error) => write!(f, "{error}"),
            Self::Ssh(error) => write!(f, "{error}"),
            Self::Key(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for KeyError {}

impl From<io::Error> for KeyError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<store::StoreError> for KeyError {
    fn from(error: store::StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<SshError> for KeyError {
    fn from(error: SshError) -> Self {
        Self::Ssh(error)
    }
}

pub fn candidate_private_key_paths() -> Result<Vec<PathBuf>, KeyError> {
    let ssh_dir = store::system_ssh_dir()?;
    Ok(build_candidate_private_key_paths(&ssh_dir))
}

pub fn managed_private_key_path() -> Result<PathBuf, KeyError> {
    Ok(store::system_ssh_dir()?.join(LANSCANNER_MANAGED_KEY))
}

pub fn ensure_managed_keypair() -> Result<PathBuf, KeyError> {
    let path = managed_private_key_path()?;
    migrate_non_rsa_managed_key_if_needed(&path)?;
    if !path.is_file() {
        generate_keypair(&path)?;
    }

    ensure_private_key_permissions(&path)?;
    ensure_public_key_file(&path)?;
    Ok(path)
}

pub fn prepare_private_key_for_external_use(
    path: &Path,
) -> Result<PreparedPrivateKeyForExternalUse, KeyError> {
    if is_app_managed_key(path) {
        ensure_private_key_permissions(path)?;
        ensure_public_key_file(path)?;
        return Ok(PreparedPrivateKeyForExternalUse {
            path: path.to_path_buf(),
            cleanup_on_failure: false,
        });
    }

    #[cfg(windows)]
    {
        return prepare_windows_user_key_for_external_use(path);
    }

    #[cfg(not(windows))]
    {
        Ok(PreparedPrivateKeyForExternalUse {
            path: path.to_path_buf(),
            cleanup_on_failure: false,
        })
    }
}

pub fn cleanup_prepared_private_key(path: &Path, cleanup_on_failure: bool) -> Result<(), KeyError> {
    if !cleanup_on_failure {
        return Ok(());
    }

    #[cfg(windows)]
    {
        if !is_external_temp_key_path(path) {
            return Ok(());
        }
        cleanup_external_key_artifacts(path)?;
    }

    #[cfg(not(windows))]
    let _ = (path, cleanup_on_failure);

    Ok(())
}

pub fn cleanup_external_temp_keys_on_startup() -> Result<(), KeyError> {
    #[cfg(windows)]
    {
        cleanup_stale_external_temp_keys()?;
    }

    Ok(())
}

pub fn cleanup_external_temp_keys_on_shutdown() -> Result<(), KeyError> {
    #[cfg(windows)]
    {
        cleanup_current_process_external_temp_key_dir()?;
    }

    Ok(())
}

fn build_candidate_private_key_paths(ssh_dir: &Path) -> Vec<PathBuf> {
    let preserved = preserved_legacy_managed_key_paths(ssh_dir);
    let mut candidates = Vec::with_capacity(DEFAULT_PRIVATE_KEYS.len() + 2 + preserved.len());
    candidates.push(ssh_dir.join(LANSCANNER_MANAGED_KEY));
    candidates.push(ssh_dir.join(LANSCANNER_LEGACY_MANAGED_ED25519_KEY));
    candidates.extend(preserved);

    for name in DEFAULT_PRIVATE_KEYS {
        let path = ssh_dir.join(name);
        if !candidates.iter().any(|existing| existing == &path) {
            candidates.push(path);
        }
    }

    candidates
}

fn is_app_managed_key(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };

    file_name == LANSCANNER_MANAGED_KEY
        || file_name == LANSCANNER_LEGACY_MANAGED_ED25519_KEY
        || file_name.starts_with(LANSCANNER_LEGACY_MANAGED_PRESERVE_PREFIX)
}

fn preserved_legacy_managed_key_paths(ssh_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let Ok(entries) = fs::read_dir(ssh_dir) else {
        return paths;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if file_name.starts_with(LANSCANNER_LEGACY_MANAGED_PRESERVE_PREFIX)
            && !file_name.ends_with(".pub")
        {
            paths.push(path);
        }
    }

    paths.sort();
    paths
}

pub fn find_default_key() -> Option<PathBuf> {
    candidate_private_key_paths()
        .ok()
        .and_then(|paths| paths.into_iter().find(|path| path.is_file()))
}

pub fn generate_keypair(path: &Path) -> Result<(), KeyError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if generate_keypair_via_ssh_keygen(path).is_ok() {
        ensure_private_key_permissions(path)?;
        ensure_public_key_file(path)?;
        return Ok(());
    }

    let private_key = PrivateKey::random(&mut OsRng, key_algorithm_for_path(path))
        .map_err(|error| KeyError::Key(error.to_string()))?;
    private_key
        .write_openssh_file(path, LineEnding::LF)
        .map_err(|error| KeyError::Key(error.to_string()))?;
    private_key
        .public_key()
        .write_openssh_file(&path.with_extension("pub"))
        .map_err(|error| KeyError::Key(error.to_string()))?;

    ensure_private_key_permissions(path)?;
    ensure_public_key_file(path)?;
    Ok(())
}

fn generate_keypair_via_ssh_keygen(path: &Path) -> Result<(), KeyError> {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(path.with_extension("pub"));

    let (key_type, bits) = match key_algorithm_for_path(path) {
        Algorithm::Rsa { .. } => ("rsa", Some("4096")),
        _ => ("ed25519", None),
    };
    let mut first_failure_reason: Option<String> = None;
    let mut first_io_error: Option<io::Error> = None;

    for executable in ssh_keygen_candidates() {
        let mut command = Command::new(&executable);
        command
            .arg("-t")
            .arg(key_type)
            .arg("-N")
            .arg("")
            .arg("-q")
            .arg("-f")
            .arg(path);
        if let Some(bits) = bits {
            command.arg("-b").arg(bits);
        }
        configure_windows_hidden_process(&mut command);

        match command.output() {
            Ok(result) if result.status.success() && path.is_file() => return Ok(()),
            Ok(result) => {
                if first_failure_reason.is_none() {
                    first_failure_reason = Some(command_failure_reason(
                        result.status.code(),
                        &result.stderr,
                        &result.stdout,
                    ));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                if first_io_error.is_none() {
                    first_io_error = Some(error);
                }
            }
        }
    }

    if let Some(reason) = first_failure_reason {
        return Err(KeyError::Key(format!(
            "ssh-keygen 生成密钥失败（{}）：{}",
            path.display(),
            reason
        )));
    }
    if let Some(error) = first_io_error {
        return Err(KeyError::Io(error));
    }

    Err(KeyError::Key(String::from(
        "ssh-keygen 不可用（已尝试 PATH 与 Windows OpenSSH 系统路径）",
    )))
}

fn ssh_keygen_candidates() -> Vec<PathBuf> {
    #[cfg(windows)]
    let mut candidates = Vec::new();
    #[cfg(not(windows))]
    let candidates = vec![PathBuf::from("ssh-keygen")];

    #[cfg(windows)]
    {
        push_windows_openssh_keygen_candidate(&mut candidates, "WINDIR");
        push_windows_openssh_keygen_candidate(&mut candidates, "SystemRoot");
        if !candidates
            .iter()
            .any(|candidate| candidate == Path::new("ssh-keygen"))
        {
            candidates.push(PathBuf::from("ssh-keygen"));
        }
    }

    candidates
}

#[cfg(windows)]
fn push_windows_openssh_keygen_candidate(candidates: &mut Vec<PathBuf>, env_key: &str) {
    let Some(root) = std::env::var_os(env_key) else {
        return;
    };
    let path = PathBuf::from(root)
        .join("System32")
        .join("OpenSSH")
        .join("ssh-keygen.exe");
    if path.is_file() && !candidates.iter().any(|existing| existing == &path) {
        candidates.push(path);
    }
}

fn key_algorithm_for_path(path: &Path) -> Algorithm {
    let is_rsa_path = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase().contains("rsa"))
        .unwrap_or(false);

    if is_rsa_path {
        Algorithm::Rsa { hash: None }
    } else {
        Algorithm::Ed25519
    }
}

fn ensure_public_key_file(path: &Path) -> Result<(), KeyError> {
    let public_key_path = path.with_extension("pub");
    if public_key_path.is_file() {
        return Ok(());
    }

    let private_key = keys::load_secret_key(path, None).map_err(|error| {
        KeyError::Key(format!(
            "unable to load SSH private key {}: {error}",
            path.display()
        ))
    })?;
    private_key
        .public_key()
        .write_openssh_file(&public_key_path)
        .map_err(|error| KeyError::Key(error.to_string()))?;

    Ok(())
}

#[cfg(unix)]
fn ensure_private_key_permissions(path: &Path) -> Result<(), KeyError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(windows)]
fn ensure_private_key_permissions(path: &Path) -> Result<(), KeyError> {
    if !path.is_file() {
        return Err(KeyError::Key(format!(
            "SSH private key not found: {}",
            path.display()
        )));
    }

    match rewrite_private_key_acl_with_powershell(path) {
        Ok(()) => Ok(()),
        Err(powershell_error) => fallback_private_key_acl_with_icacls(path).map_err(|error| {
            KeyError::Key(format!(
                "PowerShell ACL 修复失败：{powershell_error}；icacls 回退也失败：{error}"
            ))
        }),
    }
}

#[cfg(not(any(unix, windows)))]
fn ensure_private_key_permissions(_path: &Path) -> Result<(), KeyError> {
    Ok(())
}

#[cfg(windows)]
fn run_icacls(path: &Path, args: &[&str]) -> Result<(), KeyError> {
    let mut command = Command::new("icacls");
    command.arg(path).args(args);
    configure_windows_hidden_process(&mut command);
    let output = command.output();

    match output {
        Ok(result) if result.status.success() => Ok(()),
        Ok(result) => Err(KeyError::Key(format!(
            "icacls {} 失败：{}",
            args.join(" "),
            command_failure_reason(result.status.code(), &result.stderr, &result.stdout)
        ))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(KeyError::Key(String::from(
            "icacls 不可用，无法修复 Windows 私钥 ACL 权限",
        ))),
        Err(error) => Err(KeyError::Io(error)),
    }
}

fn command_failure_reason(code: Option<i32>, stderr: &[u8], stdout: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(stdout).trim().to_owned();
    if !stderr.is_empty() {
        return stderr.replace('\n', " ");
    }
    if !stdout.is_empty() {
        return stdout.replace('\n', " ");
    }
    if let Some(code) = code {
        return format!("exit code {code}");
    }

    String::from("terminated by signal")
}

#[cfg(windows)]
fn rewrite_private_key_acl_with_powershell(path: &Path) -> Result<(), String> {
    let path = path.to_string_lossy().replace('\'', "''");
    let command = format!(
        concat!(
            "$ErrorActionPreference='Stop';",
            "$path='{path}';",
            "$userSid=[System.Security.Principal.WindowsIdentity]::GetCurrent().User;",
            "$acl=New-Object System.Security.AccessControl.FileSecurity;",
            "$acl.SetOwner($userSid);",
            "$acl.SetAccessRuleProtection($true,$false);",
            "$rule=New-Object System.Security.AccessControl.FileSystemAccessRule($userSid,'FullControl','Allow');",
            "$acl.SetAccessRule($rule);",
            "Set-Acl -LiteralPath $path -AclObject $acl;"
        ),
        path = path,
    );
    let mut powershell = Command::new("powershell");
    powershell
        .arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(command);
    configure_windows_hidden_process(&mut powershell);
    let output = powershell.output();

    match output {
        Ok(result) if result.status.success() => Ok(()),
        Ok(result) => Err(command_failure_reason(
            result.status.code(),
            &result.stderr,
            &result.stdout,
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(String::from("powershell 不可用"))
        }
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(windows)]
fn fallback_private_key_acl_with_icacls(path: &Path) -> Result<(), KeyError> {
    run_icacls(path, &["/reset"])?;
    run_icacls(path, &["/inheritance:r"])?;

    if let Some(account) = current_windows_account() {
        let grant = format!("{account}:F");
        run_icacls(path, &["/grant:r", grant.as_str()])?;
        run_icacls(path, &["/setowner", account.as_str()])?;
    }

    Ok(())
}

#[cfg(windows)]
fn current_windows_account() -> Option<String> {
    let username = std::env::var("USERNAME")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())?;
    let domain = std::env::var("USERDOMAIN")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    Some(match domain {
        Some(domain) => format!(r"{domain}\{username}"),
        None => username,
    })
}

fn migrate_non_rsa_managed_key_if_needed(managed_rsa_path: &Path) -> Result<(), KeyError> {
    if !managed_rsa_path.is_file() {
        return Ok(());
    }

    let private_key = keys::load_secret_key(managed_rsa_path, None).map_err(|error| {
        KeyError::Key(format!(
            "unable to inspect managed SSH private key {}: {error}",
            managed_rsa_path.display()
        ))
    })?;
    if matches!(private_key.algorithm(), Algorithm::Rsa { .. }) {
        return Ok(());
    }

    let target_path = choose_legacy_managed_preserve_path(managed_rsa_path)?;
    preserve_legacy_managed_key(managed_rsa_path, &target_path)?;
    Ok(())
}

fn choose_legacy_managed_preserve_path(managed_rsa_path: &Path) -> Result<PathBuf, KeyError> {
    let ssh_dir = managed_rsa_path.parent().ok_or_else(|| {
        KeyError::Key(format!(
            "managed SSH key path has no parent directory: {}",
            managed_rsa_path.display()
        ))
    })?;
    let preferred = ssh_dir.join(LANSCANNER_LEGACY_MANAGED_ED25519_KEY);
    if !preferred.exists() && !preferred.with_extension("pub").exists() {
        return Ok(preferred);
    }

    for index in 1..=1024 {
        let candidate = ssh_dir.join(format!(
            "{LANSCANNER_LEGACY_MANAGED_PRESERVE_PREFIX}_{index}"
        ));
        if !candidate.exists() && !candidate.with_extension("pub").exists() {
            return Ok(candidate);
        }
    }

    Err(KeyError::Key(String::from(
        "unable to reserve a safe path for legacy non-RSA managed key migration",
    )))
}

fn preserve_legacy_managed_key(source_path: &Path, target_path: &Path) -> Result<(), KeyError> {
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if target_path.exists() || target_path.with_extension("pub").exists() {
        return Err(KeyError::Key(format!(
            "legacy managed key migration target already exists: {}",
            target_path.display()
        )));
    }

    fs::copy(source_path, target_path)?;
    let source_pub_path = source_path.with_extension("pub");
    let target_pub_path = target_path.with_extension("pub");

    if source_pub_path.is_file()
        && let Err(error) = fs::copy(&source_pub_path, &target_pub_path)
    {
        let _ = fs::remove_file(target_path);
        return Err(KeyError::Io(error));
    }
    if let Err(error) = ensure_public_key_file(target_path) {
        let _ = fs::remove_file(target_path);
        let _ = fs::remove_file(&target_pub_path);
        return Err(error);
    }
    ensure_private_key_permissions(target_path)?;

    fs::remove_file(source_path)?;
    if source_pub_path.is_file() {
        let _ = fs::remove_file(source_pub_path);
    }

    Ok(())
}

#[cfg(windows)]
fn external_temp_key_root() -> PathBuf {
    std::env::temp_dir()
        .join(EXTERNAL_TEMP_KEY_ROOT_DIR)
        .join("external-keys")
}

#[cfg(windows)]
fn current_process_external_temp_key_dir() -> PathBuf {
    external_temp_key_root().join(std::process::id().to_string())
}

#[cfg(windows)]
fn is_external_temp_key_path(path: &Path) -> bool {
    path.starts_with(external_temp_key_root())
}

#[cfg(windows)]
fn prepare_windows_user_key_for_external_use(
    path: &Path,
) -> Result<PreparedPrivateKeyForExternalUse, KeyError> {
    if !path.is_file() {
        return Err(KeyError::Key(format!(
            "SSH private key not found: {}",
            path.display()
        )));
    }

    let temp_dir = current_process_external_temp_key_dir();
    fs::create_dir_all(&temp_dir)?;
    let temp_path = stable_external_temp_key_path(path, &temp_dir);
    if temp_path.is_file() {
        if validate_reusable_external_temp_key(&temp_path).is_ok() {
            return Ok(PreparedPrivateKeyForExternalUse {
                path: temp_path,
                cleanup_on_failure: false,
            });
        }
        cleanup_external_key_artifacts(&temp_path)?;
    }

    let staging_path = staging_external_temp_key_path(&temp_path);
    cleanup_external_key_artifacts(&staging_path)?;

    let mut committed_temp = false;
    let create_result = (|| -> Result<(), KeyError> {
        fs::copy(path, &staging_path)?;
        ensure_private_key_permissions(&staging_path)?;
        ensure_public_key_file(&staging_path)?;

        fs::rename(&staging_path, &temp_path)?;
        committed_temp = true;

        let staging_pub_path = staging_path.with_extension("pub");
        let temp_pub_path = temp_path.with_extension("pub");
        if staging_pub_path.is_file() {
            match fs::remove_file(&temp_pub_path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(KeyError::Io(error)),
            }
            fs::rename(&staging_pub_path, &temp_pub_path)?;
        }

        validate_reusable_external_temp_key(&temp_path)?;
        Ok(())
    })();

    if let Err(error) = create_result {
        let _ = cleanup_external_key_artifacts(&staging_path);
        if committed_temp {
            let _ = cleanup_external_key_artifacts(&temp_path);
        }
        return Err(error);
    }

    Ok(PreparedPrivateKeyForExternalUse {
        path: temp_path,
        cleanup_on_failure: true,
    })
}

#[cfg(windows)]
fn cleanup_stale_external_temp_keys() -> Result<(), KeyError> {
    let root = external_temp_key_root();
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(KeyError::Io(error)),
    };
    let current_pid = std::process::id();
    let now = SystemTime::now();

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(dir_name) = entry.file_name().to_str().map(|value| value.to_owned()) else {
            continue;
        };
        let should_remove = match dir_name.parse::<u32>() {
            Ok(pid) if pid == current_pid => false,
            Ok(pid) => !is_windows_process_alive(pid),
            Err(_) => {
                let metadata = match entry.metadata() {
                    Ok(metadata) => metadata,
                    Err(_) => continue,
                };
                let modified = metadata.modified().unwrap_or(now);
                now.duration_since(modified).unwrap_or_default()
                    >= Duration::from_secs(EXTERNAL_TEMP_KEY_STALE_TTL_SECS)
            }
        };
        if should_remove {
            let _ = fs::remove_dir_all(path);
        }
    }

    Ok(())
}

#[cfg(windows)]
fn cleanup_current_process_external_temp_key_dir() -> Result<(), KeyError> {
    match fs::remove_dir_all(current_process_external_temp_key_dir()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(KeyError::Io(error)),
    }
}

#[cfg(windows)]
fn stable_external_temp_key_path(source_key: &Path, temp_dir: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    source_key_identity(source_key).hash(&mut hasher);
    let digest = hasher.finish();
    let source_name = source_key
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("id_key");
    let safe_name = source_name
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || value == '.' || value == '-' || value == '_' {
                value
            } else {
                '-'
            }
        })
        .collect::<String>();

    temp_dir.join(format!("lanscanner_external_{digest:016x}_{safe_name}"))
}

#[cfg(windows)]
fn staging_external_temp_key_path(temp_path: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = temp_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("lanscanner_external_key");
    temp_path.with_file_name(format!("{file_name}.staging-{nonce}"))
}

#[cfg(windows)]
fn validate_reusable_external_temp_key(path: &Path) -> Result<(), KeyError> {
    ensure_private_key_permissions(path)?;
    keys::load_secret_key(path, None).map_err(|error| {
        KeyError::Key(format!(
            "unable to validate reusable SSH private key {}: {error}",
            path.display()
        ))
    })?;
    ensure_public_key_file(path)?;
    Ok(())
}

#[cfg(windows)]
fn cleanup_external_key_artifacts(path: &Path) -> Result<(), KeyError> {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(KeyError::Io(error)),
    }

    let pub_path = path.with_extension("pub");
    match fs::remove_file(pub_path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(KeyError::Io(error)),
    }

    Ok(())
}

#[cfg(windows)]
fn source_key_identity(path: &Path) -> String {
    match fs::canonicalize(path) {
        Ok(canonical) => canonical.to_string_lossy().to_ascii_lowercase(),
        Err(_) => path.to_string_lossy().to_ascii_lowercase(),
    }
}

#[cfg(windows)]
fn is_windows_process_alive(pid: u32) -> bool {
    let script = format!(
        "$p=Get-Process -Id {pid} -ErrorAction SilentlyContinue; if ($p) {{ Write-Output 'alive' }}"
    );
    let mut powershell = Command::new("powershell");
    powershell
        .arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(script);
    configure_windows_hidden_process(&mut powershell);
    let output = powershell.output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            stdout.trim().eq_ignore_ascii_case("alive")
        }
        Ok(_) => false,
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(_) => false,
    }
}

pub fn ensure_default_keypair() -> Result<PathBuf, KeyError> {
    if let Some(path) = find_default_key() {
        return Ok(path);
    }

    let path = store::system_ssh_dir()?.join("id_ed25519");
    generate_keypair(&path)?;
    Ok(path)
}

pub fn read_public_key(path: &Path) -> Result<String, KeyError> {
    let public_key = fs::read_to_string(path.with_extension("pub"))?;
    let public_key = public_key.trim();

    if public_key.is_empty() {
        return Err(KeyError::Key(String::from("SSH public key file is empty")));
    }

    Ok(public_key.to_owned())
}

pub async fn install_public_key(
    ip: &str,
    user: &str,
    password: Option<&str>,
    key_path: &Path,
    timeout: Duration,
) -> Result<(), KeyError> {
    let password = password.map(str::trim).filter(|value| !value.is_empty());
    let Some(password) = password else {
        return Ok(());
    };
    let public_key = read_public_key(key_path)?;
    let quoted_key = shell_quote(&public_key);
    let command = format!(
        "mkdir -p \"$HOME/.ssh\" && chmod 700 \"$HOME/.ssh\" && touch \"$HOME/.ssh/authorized_keys\" && chmod 600 \"$HOME/.ssh/authorized_keys\" && (grep -qxF {quoted_key} \"$HOME/.ssh/authorized_keys\" || printf '%s\\n' {quoted_key} >> \"$HOME/.ssh/authorized_keys\")"
    );
    let output = auth::execute_remote_command(ip, user, Some(password), timeout, &command).await?;

    if output.exit_status == 0 {
        Ok(())
    } else {
        Err(KeyError::Key(
            output
                .stderr
                .lines()
                .last()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("unable to install public key on remote host")
                .to_owned(),
        ))
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn configure_windows_hidden_process(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(CREATE_NO_WINDOW_FLAG);
    }

    #[cfg(not(windows))]
    {
        let _ = command;
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    use russh::keys::Algorithm;

    use super::{
        build_candidate_private_key_paths, generate_keypair, key_algorithm_for_path,
        migrate_non_rsa_managed_key_if_needed, prepare_private_key_for_external_use,
    };

    #[test]
    fn candidate_private_key_paths_prioritize_lanscanner_keys() {
        let ssh_dir = Path::new("/tmp/mock-ssh");
        let candidates = build_candidate_private_key_paths(ssh_dir);

        assert_eq!(candidates[0], ssh_dir.join("lanscanner_id_rsa"));
        assert_eq!(candidates[1], ssh_dir.join("lanscanner_id_ed25519"));
        assert!(candidates.contains(&ssh_dir.join("id_ed25519")));
        assert!(candidates.contains(&ssh_dir.join("id_rsa")));
    }

    #[test]
    fn key_generation_algorithm_matches_key_name() {
        assert!(matches!(
            key_algorithm_for_path(Path::new("/tmp/mock/.ssh/lanscanner_id_rsa")),
            Algorithm::Rsa { hash: None }
        ));
        assert_eq!(
            key_algorithm_for_path(Path::new("/tmp/mock/.ssh/id_ed25519")),
            Algorithm::Ed25519
        );
    }

    #[test]
    fn candidate_private_key_paths_include_preserved_legacy_managed_keys() {
        let temp_dir = temp_ssh_dir("legacy-candidates");
        let legacy_one = temp_dir.join("lanscanner_id_ed25519_legacy_1");
        let legacy_two = temp_dir.join("lanscanner_id_ed25519_legacy_2");
        fs::write(&legacy_one, "dummy").expect("write first legacy candidate");
        fs::write(&legacy_two, "dummy").expect("write second legacy candidate");

        let candidates = build_candidate_private_key_paths(&temp_dir);

        assert!(candidates.contains(&legacy_one));
        assert!(candidates.contains(&legacy_two));
    }

    #[test]
    fn migrate_non_rsa_managed_key_keeps_legacy_copy_when_ed25519_exists() {
        let temp_dir = temp_ssh_dir("migrate-non-rsa");
        let source_rsa = temp_dir.join("lanscanner_id_rsa");
        let staging_key = temp_dir.join("staging_ed25519");
        let existing_legacy = temp_dir.join("lanscanner_id_ed25519");

        generate_keypair(&staging_key).expect("generate staging ed25519");
        fs::rename(&staging_key, &source_rsa).expect("rename staging key to managed rsa slot");
        fs::rename(
            staging_key.with_extension("pub"),
            source_rsa.with_extension("pub"),
        )
        .expect("rename staging public key");
        generate_keypair(&existing_legacy).expect("generate existing legacy key");

        migrate_non_rsa_managed_key_if_needed(&source_rsa).expect("migrate non-rsa managed key");

        let preserved = temp_dir.join("lanscanner_id_ed25519_legacy_1");
        assert!(!source_rsa.exists());
        assert!(preserved.exists());
        assert!(preserved.with_extension("pub").exists());
        assert!(existing_legacy.exists());
    }

    #[test]
    fn prepare_private_key_for_external_use_keeps_user_key_path() {
        let temp_dir = temp_ssh_dir("prepare-user-key");
        let user_key = temp_dir.join("id_rsa");
        generate_keypair(&user_key).expect("generate user key");

        let prepared =
            prepare_private_key_for_external_use(&user_key).expect("prepare external user key");
        let prepared_path = prepared.path;
        #[cfg(not(windows))]
        assert_eq!(prepared_path, user_key);
        #[cfg(windows)]
        assert_ne!(prepared_path, user_key);
        assert!(prepared_path.is_file());
    }

    fn temp_ssh_dir(label: &str) -> std::path::PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lanscanner-key-mgmt-{label}-{timestamp}"));
        fs::create_dir_all(&dir).expect("create temp ssh dir");
        dir
    }
}
