use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::credential::store;

#[cfg(windows)]
const CREATE_NO_WINDOW_FLAG: u32 = 0x0800_0000;

#[derive(Debug)]
pub enum ConfigError {
    Io(io::Error),
    Store(store::StoreError),
    Command(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Store(error) => write!(f, "{error}"),
            Self::Command(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<store::StoreError> for ConfigError {
    fn from(error: store::StoreError) -> Self {
        Self::Store(error)
    }
}

#[derive(Debug)]
pub struct SshConfigRollback {
    config_path: PathBuf,
    previous_content: Option<String>,
}

impl SshConfigRollback {
    pub fn rollback(self) -> Result<(), ConfigError> {
        match self.previous_content {
            Some(content) => {
                fs::write(&self.config_path, content)?;
            }
            None => match fs::remove_file(&self.config_path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(ConfigError::Io(error)),
            },
        }

        Ok(())
    }
}

pub fn host_alias(ip: &str, user: &str) -> String {
    let user = user
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || value == '-' || value == '_' {
                value
            } else {
                '-'
            }
        })
        .collect::<String>();
    let ip = ip
        .chars()
        .map(|value| if value == '.' { '-' } else { value })
        .collect::<String>();

    format!("{user}-{ip}")
}

pub fn update_ssh_config(
    host: &str,
    ip: &str,
    user: &str,
    key_path: &Path,
) -> Result<(), ConfigError> {
    let block = build_managed_key_host_block(host, ip, user, key_path);
    let _rollback_guard = update_ssh_config_block_with_rollback(host, &block)?;
    Ok(())
}

pub fn update_ssh_config_with_rollback(
    host: &str,
    ip: &str,
    user: &str,
    key_path: &Path,
) -> Result<SshConfigRollback, ConfigError> {
    let block = build_managed_key_host_block(host, ip, user, key_path);
    update_ssh_config_block_with_rollback(host, &block)
}

pub fn update_ssh_config_for_password_fallback(
    host: &str,
    ip: &str,
    user: &str,
) -> Result<(), ConfigError> {
    let block = build_password_fallback_host_block(host, ip, user);
    let _rollback_guard = update_ssh_config_block_with_rollback(host, &block)?;
    Ok(())
}

pub fn remove_known_host(host: &str) -> Result<(), ConfigError> {
    let path = store::system_known_hosts_path()?;
    remove_known_host_from_file(&path, host)?;

    Ok(())
}

pub fn repair_known_host_mismatch(ip: &str, alias: &str) -> Result<(), ConfigError> {
    let path = store::system_known_hosts_path()?;
    let mut targets = vec![ip.to_owned(), format!("[{ip}]:22"), alias.to_owned()];
    targets.sort();
    targets.dedup();
    for target in targets {
        remove_known_host_from_file(&path, &target)?;
    }
    Ok(())
}

fn upsert_host_block(existing: &str, host: &str, block: &str) -> String {
    let mut result = String::new();
    let mut lines = existing.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("Host ") {
            let matches_target = rest.split_whitespace().any(|value| value == host);
            if matches_target {
                while let Some(next) = lines.peek() {
                    if next.trim_start().starts_with("Host ") {
                        break;
                    }
                    lines.next();
                }
                continue;
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    if !result.trim().is_empty() && !result.ends_with("\n\n") {
        result.push('\n');
    }

    result.push_str(block);
    result
}

fn update_ssh_config_with_rollback_at_path(
    config_path: &Path,
    host: &str,
    block: &str,
) -> Result<SshConfigRollback, ConfigError> {
    let previous_content = match fs::read_to_string(config_path) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(ConfigError::Io(error)),
    };
    let existing = previous_content.as_deref().unwrap_or_default();
    let updated = upsert_host_block(existing, host, block);
    fs::write(config_path, updated)?;

    Ok(SshConfigRollback {
        config_path: config_path.to_path_buf(),
        previous_content,
    })
}

pub fn remove_known_host_from_file(path: &Path, host: &str) -> Result<(), ConfigError> {
    if !path.exists() {
        return Ok(());
    }

    let original = fs::read_to_string(path).unwrap_or_default();
    let filtered = original
        .lines()
        .filter(|line| !known_host_matches(line, host))
        .collect::<Vec<_>>()
        .join("\n");

    if filtered != original {
        let mut content = filtered;
        if !content.is_empty() {
            content.push('\n');
        }
        fs::write(path, content)?;
    }

    run_ssh_keygen_remove(path, host)?;
    Ok(())
}

fn run_ssh_keygen_remove(path: &Path, host: &str) -> Result<(), ConfigError> {
    let mut first_failure_message: Option<String> = None;
    let mut first_io_error: Option<io::Error> = None;

    for executable in ssh_keygen_candidates() {
        let mut command = Command::new(&executable);
        command.arg("-R").arg(host).arg("-f").arg(path);
        configure_windows_hidden_process(&mut command);
        let output = command.output();

        match output {
            Ok(result) if result.status.success() => return Ok(()),
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                if stderr.contains("No such file") || stderr.contains("not found in") {
                    return Ok(());
                }
                if first_failure_message.is_none() {
                    first_failure_message = Some(command_failure_message(
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

    if let Some(message) = first_failure_message {
        return Err(ConfigError::Command(message));
    }
    if let Some(error) = first_io_error {
        return Err(ConfigError::Io(error));
    }

    Ok(())
}

fn command_failure_message(code: Option<i32>, stderr: &[u8], stdout: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(stdout).trim().to_owned();
    if !stderr.is_empty() {
        return stderr;
    }
    if !stdout.is_empty() {
        return stdout;
    }
    if let Some(code) = code {
        return format!("exit code {code}");
    }

    String::from("terminated by signal")
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

fn known_host_matches(line: &str, host: &str) -> bool {
    let Some(field) = line.split_whitespace().next() else {
        return false;
    };
    let bracketed = format!("[{host}]:22");

    field
        .split(',')
        .any(|entry| entry == host || entry == bracketed)
}

fn update_ssh_config_block_with_rollback(
    host: &str,
    block: &str,
) -> Result<SshConfigRollback, ConfigError> {
    let config_path = store::system_ssh_config_path()?;
    update_ssh_config_with_rollback_at_path(&config_path, host, block)
}

fn build_managed_key_host_block(host: &str, ip: &str, user: &str, key_path: &Path) -> String {
    let mut block = format!("Host {host}\n  HostName {ip}\n  Port 22\n  User {user}\n");

    if host == host_alias(ip, user) {
        block.push_str(&format!("  IdentityFile {}\n", ssh_path(key_path)));
        block.push_str("  IdentitiesOnly yes\n");
        block.push_str("  PreferredAuthentications publickey,password\n");
        block.push_str("  PubkeyAuthentication yes\n");
        block.push_str("  HostKeyAlgorithms +ssh-rsa\n");
        block.push_str("  PubkeyAcceptedAlgorithms +ssh-rsa\n");
        block.push_str("  PubkeyAcceptedKeyTypes +ssh-rsa\n");
    }

    block
}

fn build_password_fallback_host_block(host: &str, ip: &str, user: &str) -> String {
    let mut block = format!("Host {host}\n  HostName {ip}\n  Port 22\n  User {user}\n");
    block.push_str("  PreferredAuthentications password,publickey\n");
    block.push_str("  PubkeyAuthentication yes\n");
    block.push_str("  HostKeyAlgorithms +ssh-rsa\n");
    block.push_str("  PubkeyAcceptedAlgorithms +ssh-rsa\n");
    block.push_str("  PubkeyAcceptedKeyTypes +ssh-rsa\n");
    block
}

fn ssh_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::{env, fs};

    use super::{
        build_managed_key_host_block, build_password_fallback_host_block, host_alias,
        remove_known_host_from_file, update_ssh_config_with_rollback_at_path, upsert_host_block,
    };

    #[test]
    fn generates_stable_host_alias() {
        assert_eq!(host_alias("192.168.1.5", "root"), "root-192-168-1-5");
    }

    #[test]
    fn replaces_existing_host_block() {
        let existing = "Host other\n  HostName 1.1.1.1\n\nHost target\n  HostName old\n";
        let updated = upsert_host_block(existing, "target", "Host target\n  HostName new\n");

        assert!(updated.contains("Host other"));
        assert!(updated.contains("Host target\n  HostName new"));
        assert!(!updated.contains("HostName old"));
    }

    #[test]
    fn managed_host_block_writes_key_only_options_without_custom_known_hosts() {
        let block = build_managed_key_host_block(
            "root-192-168-1-5",
            "192.168.1.5",
            "root",
            Path::new("/home/test/.ssh/lanscanner_id_rsa"),
        );

        assert!(block.contains("IdentityFile /home/test/.ssh/lanscanner_id_rsa"));
        assert!(block.contains("Port 22"));
        assert!(block.contains("IdentitiesOnly yes"));
        assert!(block.contains("PreferredAuthentications publickey,password"));
        assert!(block.contains("PubkeyAcceptedAlgorithms +ssh-rsa"));
        assert!(block.contains("PubkeyAcceptedKeyTypes +ssh-rsa"));
        assert!(!block.contains("UserKnownHostsFile"));
    }

    #[test]
    fn non_lanscanner_host_block_does_not_write_key_only_options() {
        let block = build_managed_key_host_block(
            "example-host",
            "192.168.1.5",
            "root",
            Path::new("/home/test/.ssh/id_ed25519"),
        );

        assert!(block.starts_with("Host example-host"));
        assert!(!block.contains("IdentityFile"));
        assert!(!block.contains("IdentitiesOnly"));
    }

    #[test]
    fn rollback_restores_previous_alias_block() {
        let config_path = temp_config_path("restore-existing");
        let original = "Host root-192-168-1-5\n  HostName 192.168.1.5\n  User root\n  IdentityFile /home/test/.ssh/old\n  IdentitiesOnly yes\n";
        fs::write(&config_path, original).expect("write original ssh config");

        let rollback = update_ssh_config_with_rollback_at_path(
            &config_path,
            "root-192-168-1-5",
            "Host root-192-168-1-5\n  HostName 192.168.1.5\n  User root\n  IdentityFile /home/test/.ssh/new\n",
        )
        .expect("update config");

        let updated = fs::read_to_string(&config_path).expect("read updated config");
        assert!(updated.contains("IdentityFile /home/test/.ssh/new"));
        assert!(!updated.contains("IdentityFile /home/test/.ssh/old"));

        rollback.rollback().expect("rollback config");
        let restored = fs::read_to_string(&config_path).expect("read restored config");
        assert_eq!(restored, original);
        let _ = fs::remove_file(&config_path);
    }

    #[test]
    fn rollback_removes_new_alias_when_config_did_not_exist() {
        let config_path = temp_config_path("remove-new");
        let _ = fs::remove_file(&config_path);

        let rollback = update_ssh_config_with_rollback_at_path(
            &config_path,
            "root-192-168-1-6",
            "Host root-192-168-1-6\n  HostName 192.168.1.6\n  User root\n  IdentityFile /home/test/.ssh/managed\n",
        )
        .expect("write new alias block");

        let content = fs::read_to_string(&config_path).expect("read created config");
        assert!(content.contains("Host root-192-168-1-6"));
        assert!(content.contains("IdentityFile /home/test/.ssh/managed"));

        rollback.rollback().expect("rollback created alias");
        assert!(!config_path.exists());
    }

    #[test]
    fn password_fallback_host_block_does_not_write_identityfile_or_identities_only() {
        let block = build_password_fallback_host_block("root-192-168-1-5", "192.168.1.5", "root");

        assert!(block.contains("Host root-192-168-1-5"));
        assert!(block.contains("PreferredAuthentications password,publickey"));
        assert!(!block.contains("IdentityFile"));
        assert!(!block.contains("IdentitiesOnly"));
        assert!(!block.contains("UserKnownHostsFile"));
    }

    #[test]
    fn remove_known_host_from_file_removes_plain_host_and_keeps_others() {
        let known_hosts_path = temp_known_hosts_path("remove-plain");
        let original = "10.0.0.8 ssh-ed25519 AAAAplain\nother-host ssh-ed25519 AAAAother\n";
        fs::write(&known_hosts_path, original).expect("write known_hosts");

        remove_known_host_from_file(&known_hosts_path, "10.0.0.8")
            .expect("remove plain host entry");

        let updated = fs::read_to_string(&known_hosts_path).expect("read known_hosts");
        assert!(!updated.contains("10.0.0.8 ssh-ed25519"));
        assert!(updated.contains("other-host ssh-ed25519 AAAAother"));

        let _ = fs::remove_file(&known_hosts_path);
    }

    #[test]
    fn remove_known_host_from_file_removes_bracketed_host_and_keeps_others() {
        let known_hosts_path = temp_known_hosts_path("remove-bracketed");
        let original = "[10.0.0.8]:22 ssh-ed25519 AAAAbracketed\nexample ssh-ed25519 AAAAkeep\n";
        fs::write(&known_hosts_path, original).expect("write known_hosts");

        remove_known_host_from_file(&known_hosts_path, "10.0.0.8")
            .expect("remove bracketed host entry");

        let updated = fs::read_to_string(&known_hosts_path).expect("read known_hosts");
        assert!(!updated.contains("[10.0.0.8]:22 ssh-ed25519"));
        assert!(updated.contains("example ssh-ed25519 AAAAkeep"));

        let _ = fs::remove_file(&known_hosts_path);
    }

    fn temp_config_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("valid system time")
            .as_nanos();
        env::temp_dir().join(format!("lanscanner-{name}-{nonce}.config"))
    }

    fn temp_known_hosts_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("valid system time")
            .as_nanos();
        env::temp_dir().join(format!("lanscanner-{name}-{nonce}.known_hosts"))
    }
}
