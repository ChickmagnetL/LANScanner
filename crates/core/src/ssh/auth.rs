use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, AuthResult, Handle};
use russh::keys::{self, Algorithm, HashAlg, key::PrivateKeyWithHashAlg, known_hosts};
use russh::{ChannelMsg, Disconnect, Error as RusshError, MethodKind, MethodSet};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time;

use crate::credential::store;
use crate::scanner::{Device, DeviceStatus};

use super::{config, key_mgmt};

pub const VERIFY_TIMEOUT: Duration = Duration::from_millis(1500);
pub const VERIFY_CONCURRENCY: usize = 8;
pub const LAUNCH_AUTH_TIMEOUT: Duration = Duration::from_secs(10);

const SSH_PORT: u16 = 22;
const OPENSSH_NOT_FOUND_MESSAGE: &str = "本机未安装可用的 OpenSSH 客户端（ssh），已尝试 PATH 与 Windows 系统路径，无法验证 key-only 可用性";
#[cfg(windows)]
const CREATE_NO_WINDOW_FLAG: u32 = 0x0800_0000;

#[derive(Debug)]
pub enum SshError {
    Io(io::Error),
    Timeout(Duration),
    Unsupported(&'static str),
    Connection(String),
    HostKey(String),
    Key(String),
    Protocol(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_status: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyReadySource {
    Existing,
    Installed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchAuthConsumer {
    VscodeLike,
    Mobaxterm,
    Shell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchAuthPreparation {
    KeyReady {
        key_path: PathBuf,
        source: KeyReadySource,
    },
    PasswordFallback {
        reason: String,
    },
    HardFailure {
        reason: String,
    },
}

impl LaunchAuthPreparation {
    pub fn key_path(&self) -> Option<&Path> {
        match self {
            Self::KeyReady { key_path, .. } => Some(key_path.as_path()),
            Self::PasswordFallback { .. } | Self::HardFailure { .. } => None,
        }
    }

    pub fn host_target(&self, ip: &str, user: &str) -> String {
        match self {
            Self::KeyReady { .. } => super::config::host_alias(ip, user),
            Self::PasswordFallback { .. } => super::config::host_alias(ip, user),
            Self::HardFailure { .. } => ip.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthDeniedReason {
    PasswordRejected,
    PublicKeyRejected,
}

impl AuthDeniedReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::PasswordRejected => "password rejected",
            Self::PublicKeyRejected => "public key rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthOutcome {
    Authenticated,
    Denied(AuthDeniedReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanVerifyMode {
    UsernamePassword,
    UsernameOnlyBestEffort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UsernameOnlyProbeOutcome {
    Authenticated,
    LikelyRejected,
    Ambiguous,
}

impl fmt::Display for SshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Timeout(timeout) => write!(f, "ssh verification timed out after {timeout:?}"),
            Self::Unsupported(reason) => write!(f, "{reason}"),
            Self::Connection(reason) => write!(f, "{reason}"),
            Self::HostKey(reason) => write!(f, "{reason}"),
            Self::Key(reason) => write!(f, "{reason}"),
            Self::Protocol(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for SshError {}

impl From<io::Error> for SshError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<RusshError> for SshError {
    fn from(error: RusshError) -> Self {
        Self::Protocol(error.to_string())
    }
}

pub async fn verify_ssh(
    ip: &str,
    username: &str,
    password: Option<&str>,
    timeout: Duration,
) -> Result<bool, SshError> {
    match verify_auth_outcome(ip, username, password, timeout).await? {
        AuthOutcome::Authenticated => Ok(true),
        AuthOutcome::Denied(_) => Ok(false),
    }
}

pub async fn verify_ssh_key_path(
    ip: &str,
    username: &str,
    timeout: Duration,
) -> Result<Option<PathBuf>, SshError> {
    time::timeout(timeout, async move {
        let mut client = connect_client(ip, timeout).await?;
        let result = authenticate_with_key(&mut client, username).await;
        let _ = disconnect_client(client, "key verification complete").await;
        result
    })
    .await
    .map_err(|_| SshError::Timeout(timeout))?
}

pub async fn execute_remote_command(
    ip: &str,
    username: &str,
    password: Option<&str>,
    timeout: Duration,
    command: &str,
) -> Result<CommandOutput, SshError> {
    time::timeout(timeout, async move {
        let mut client = connect_authenticated_client(ip, username, password, timeout).await?;
        let result = run_command(&mut client, command).await;
        let _ = disconnect_client(client, "remote command completed").await;
        result
    })
    .await
    .map_err(|_| SshError::Timeout(timeout))?
}

pub async fn prepare_launch_auth(
    ip: &str,
    username: &str,
    password: Option<&str>,
    timeout: Duration,
) -> LaunchAuthPreparation {
    prepare_launch_auth_for_consumer(
        ip,
        username,
        password,
        LaunchAuthConsumer::VscodeLike,
        timeout,
    )
    .await
}

pub async fn prepare_launch_auth_for_consumer(
    ip: &str,
    username: &str,
    password: Option<&str>,
    consumer: LaunchAuthConsumer,
    timeout: Duration,
) -> LaunchAuthPreparation {
    let password = password.map(str::trim).filter(|value| !value.is_empty());
    let mut existing_key_issue: Option<String> = None;

    let existing_key_paths = match default_private_key_paths() {
        Ok(paths) => paths,
        Err(error) => {
            if password.is_none() {
                return LaunchAuthPreparation::HardFailure {
                    reason: format!("读取本地 SSH 密钥列表失败，且没有密码可回退：{error}"),
                };
            }

            Vec::new()
        }
    };

    if !existing_key_paths.is_empty() {
        match verify_existing_keys_for_launch(ip, username, consumer, timeout, &existing_key_paths)
            .await
        {
            ExistingKeyProbeOutcome::Ready(key_path) => {
                return LaunchAuthPreparation::KeyReady {
                    key_path,
                    source: KeyReadySource::Existing,
                };
            }
            ExistingKeyProbeOutcome::AllFailed { reason } => {
                if password.is_none() {
                    return LaunchAuthPreparation::HardFailure { reason };
                }
                existing_key_issue = Some(reason);
            }
            ExistingKeyProbeOutcome::Incomplete { reason } => {
                return LaunchAuthPreparation::HardFailure {
                    reason: format!(
                        "{reason}；本轮既有 SSH 密钥探测未完成，已阻止提前进入托管密钥 bootstrap，请稍后重试"
                    ),
                };
            }
        }
    } else if password.is_none() {
        return LaunchAuthPreparation::HardFailure {
            reason: String::from("当前没有可用 SSH 密钥，且未提供密码，无法建立连接"),
        };
    }

    let Some(password) = password else {
        return LaunchAuthPreparation::HardFailure {
            reason: String::from("当前没有可用的密码回退路径，无法建立连接"),
        };
    };

    let key_path = match key_mgmt::ensure_managed_keypair() {
        Ok(path) => path,
        Err(error) => {
            return password_fallback_with_known_hosts_repair(
                ip,
                username,
                consumer,
                existing_key_issue.as_deref(),
                format!("准备 LANScanner 托管密钥失败，当前改用密码连接：{error}"),
            );
        }
    };

    match key_mgmt::install_public_key(ip, username, Some(password), &key_path, timeout).await {
        Ok(()) => match verify_specific_ssh_key(ip, username, timeout, &key_path).await {
            Ok(true) => {
                match verify_key_for_external_openssh(ip, username, consumer, timeout, &key_path) {
                    Ok(prepared_key_path) => LaunchAuthPreparation::KeyReady {
                        key_path: prepared_key_path,
                        source: KeyReadySource::Installed,
                    },
                    Err(error) => password_fallback_with_known_hosts_repair(
                        ip,
                        username,
                        consumer,
                        existing_key_issue.as_deref(),
                        format!(
                            "已安装 LANScanner 托管公钥，但{}外部免密预检失败，当前改用密码连接：{error}",
                            launch_auth_consumer_label(consumer)
                        ),
                    ),
                }
            }
            Ok(false) => password_fallback_with_known_hosts_repair(
                ip,
                username,
                consumer,
                existing_key_issue.as_deref(),
                String::from("已尝试安装公钥，但远端未接受免密登录，当前改用密码连接"),
            ),
            Err(error) => password_fallback_with_known_hosts_repair(
                ip,
                username,
                consumer,
                existing_key_issue.as_deref(),
                format!("已尝试安装公钥，但免密验证失败，当前改用密码连接：{error}"),
            ),
        },
        Err(error) => password_fallback_with_known_hosts_repair(
            ip,
            username,
            consumer,
            existing_key_issue.as_deref(),
            format!("已尝试安装公钥但失败，当前改用密码连接：{error}"),
        ),
    }
}

pub async fn verify_devices<F>(
    devices: &[Device],
    username: &str,
    password: Option<&str>,
    concurrency: usize,
    mut on_result: F,
) -> Vec<(String, DeviceStatus)>
where
    F: FnMut(&str, DeviceStatus),
{
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut join_set = JoinSet::new();
    let username = username.to_owned();
    let password = password
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let mode = resolve_scan_verify_mode(password.as_deref());

    for device in devices.iter().cloned() {
        let semaphore = Arc::clone(&semaphore);
        let username = username.clone();
        let password = password.clone();
        let mode = mode;

        join_set.spawn(async move {
            let _permit = semaphore.acquire_owned().await.ok();
            let status =
                verify_device_status_for_scan(&device.ip, &username, password.as_deref(), mode)
                    .await;

            (device.ip, status)
        });
    }

    let mut ordered = Vec::with_capacity(devices.len());

    while let Some(result) = join_set.join_next().await {
        if let Ok((ip, status)) = result {
            on_result(&ip, status);
            ordered.push((ip, status));
        }
    }

    ordered.sort_by(|left, right| left.0.cmp(&right.0));
    ordered
}

fn resolve_scan_verify_mode(password: Option<&str>) -> ScanVerifyMode {
    if password.is_some() {
        ScanVerifyMode::UsernamePassword
    } else {
        ScanVerifyMode::UsernameOnlyBestEffort
    }
}

async fn verify_device_status_for_scan(
    ip: &str,
    username: &str,
    password: Option<&str>,
    mode: ScanVerifyMode,
) -> DeviceStatus {
    match mode {
        ScanVerifyMode::UsernamePassword => map_verify_result_to_device_status(
            verify_auth_outcome(ip, username, password, VERIFY_TIMEOUT).await,
            ip,
            username,
        ),
        ScanVerifyMode::UsernameOnlyBestEffort => map_username_only_probe_status(
            verify_username_only_probe(ip, username, VERIFY_TIMEOUT).await,
            ip,
            username,
        ),
    }
}

fn client_config(timeout: Duration) -> client::Config {
    client::Config {
        inactivity_timeout: Some(timeout),
        keepalive_interval: Some(Duration::from_secs(1).min(timeout)),
        keepalive_max: 1,
        nodelay: true,
        ..client::Config::default()
    }
}

async fn connect_authenticated_client(
    ip: &str,
    username: &str,
    password: Option<&str>,
    timeout: Duration,
) -> Result<Handle<VerificationClient>, SshError> {
    let password = password.map(str::trim).filter(|value| !value.is_empty());
    let mut client = connect_client(ip, timeout).await?;

    match authenticate_client(&mut client, username, password).await? {
        AuthOutcome::Authenticated => Ok(client),
        AuthOutcome::Denied(reason) => {
            let _ = disconnect_client(client, "authentication failed").await;
            Err(SshError::Connection(format!(
                "ssh authentication rejected for {username}@{ip}: {}",
                reason.as_str()
            )))
        }
    }
}

async fn connect_client(
    ip: &str,
    timeout: Duration,
) -> Result<Handle<VerificationClient>, SshError> {
    let known_hosts_path = store::known_hosts_path().map_err(|error| {
        SshError::HostKey(format!("unable to prepare ssh known_hosts path: {error}"))
    })?;
    let config = Arc::new(client_config(timeout));
    let handler = VerificationClient::new(ip, SSH_PORT, known_hosts_path);

    client::connect(config, format!("{ip}:{SSH_PORT}"), handler).await
}

async fn disconnect_client(
    client: Handle<VerificationClient>,
    reason: &str,
) -> Result<(), SshError> {
    let _ = client
        .disconnect(Disconnect::ByApplication, reason, "en-US")
        .await;
    let _ = client.await;
    Ok(())
}

#[derive(Debug, Clone)]
struct VerificationClient {
    host: String,
    port: u16,
    known_hosts_path: PathBuf,
}

impl VerificationClient {
    fn new(host: &str, port: u16, known_hosts_path: PathBuf) -> Self {
        Self {
            host: host.to_owned(),
            port,
            known_hosts_path,
        }
    }
}

impl client::Handler for VerificationClient {
    type Error = SshError;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        match known_hosts::check_known_hosts_path(
            &self.host,
            self.port,
            server_public_key,
            &self.known_hosts_path,
        ) {
            Ok(true) => Ok(true),
            Ok(false) => {
                known_hosts::learn_known_hosts_path(
                    &self.host,
                    self.port,
                    server_public_key,
                    &self.known_hosts_path,
                )
                .map_err(|error| {
                    SshError::HostKey(format!(
                        "unable to record host key for {}:{}: {error}",
                        self.host, self.port
                    ))
                })?;

                Ok(true)
            }
            Err(error) => {
                config::remove_known_host_from_file(&self.known_hosts_path, &self.host).map_err(
                    |repair_error| {
                        SshError::HostKey(format!(
                            "app-private known_hosts changed-host-key auto-repair failed for {}:{}: original check error: {error}; cleanup error: {repair_error}",
                            self.host, self.port
                        ))
                    },
                )?;

                known_hosts::learn_known_hosts_path(
                    &self.host,
                    self.port,
                    server_public_key,
                    &self.known_hosts_path,
                )
                .map_err(|repair_error| {
                    SshError::HostKey(format!(
                        "app-private known_hosts changed-host-key auto-repair failed for {}:{}: original check error: {error}; relearn error: {repair_error}",
                        self.host, self.port
                    ))
                })?;

                Ok(true)
            }
        }
    }
}

async fn authenticate_with_password(
    client: &mut Handle<VerificationClient>,
    username: &str,
    password: &str,
) -> Result<bool, SshError> {
    let result = client.authenticate_password(username, password).await?;
    Ok(result.success())
}

async fn verify_username_only_probe(
    ip: &str,
    username: &str,
    timeout: Duration,
) -> Result<UsernameOnlyProbeOutcome, SshError> {
    time::timeout(timeout, async move {
        let mut client = connect_client(ip, timeout).await?;
        let result = probe_username_only_authentication(&mut client, username).await;
        let _ = disconnect_client(client, "username-only probe complete").await;
        result
    })
    .await
    .map_err(|_| SshError::Timeout(timeout))?
}

async fn probe_username_only_authentication(
    client: &mut Handle<VerificationClient>,
    username: &str,
) -> Result<UsernameOnlyProbeOutcome, SshError> {
    match client.authenticate_none(username).await {
        Ok(AuthResult::Success) => Ok(UsernameOnlyProbeOutcome::Authenticated),
        Ok(AuthResult::Failure {
            remaining_methods,
            partial_success,
        }) => {
            if partial_success {
                return Ok(UsernameOnlyProbeOutcome::Ambiguous);
            }

            Ok(if looks_like_explicit_rejection(&remaining_methods) {
                UsernameOnlyProbeOutcome::LikelyRejected
            } else {
                UsernameOnlyProbeOutcome::Ambiguous
            })
        }
        Err(error) => Err(SshError::from(error)),
    }
}

fn looks_like_explicit_rejection(remaining_methods: &MethodSet) -> bool {
    remaining_methods.is_empty()
        || !remaining_methods.contains(&MethodKind::Password)
            && !remaining_methods.contains(&MethodKind::PublicKey)
            && !remaining_methods.contains(&MethodKind::KeyboardInteractive)
}

fn map_username_only_probe_status(
    result: Result<UsernameOnlyProbeOutcome, SshError>,
    ip: &str,
    username: &str,
) -> DeviceStatus {
    match result {
        Ok(UsernameOnlyProbeOutcome::Authenticated) => DeviceStatus::Ready,
        Ok(UsernameOnlyProbeOutcome::LikelyRejected) => DeviceStatus::Denied,
        Ok(UsernameOnlyProbeOutcome::Ambiguous) => DeviceStatus::Error,
        Err(error) => {
            let detail = error.to_string();
            if looks_like_username_rejection_error(&detail) {
                DeviceStatus::Denied
            } else {
                eprintln!(
                    "[WARN] ssh username-only probe degraded for {ip} as {username}: {detail}"
                );
                DeviceStatus::Error
            }
        }
    }
}

fn looks_like_username_rejection_error(detail: &str) -> bool {
    let lower = detail.to_ascii_lowercase();
    [
        "invalid user",
        "unknown user",
        "no such user",
        "userauth fail",
        "authentication failed",
        "permission denied",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
}

async fn authenticate_with_key(
    client: &mut Handle<VerificationClient>,
    username: &str,
) -> Result<Option<PathBuf>, SshError> {
    let key_paths = default_private_key_paths()?;
    if key_paths.is_empty() {
        return Err(SshError::Key(String::from(
            "no SSH private keys were found under ~/.ssh",
        )));
    }

    let preferred_rsa_hash = client.best_supported_rsa_hash().await?;
    let mut saw_auth_failure = false;
    let mut first_load_error: Option<SshError> = None;

    for key_path in key_paths {
        match load_private_key(&key_path, preferred_rsa_hash) {
            Ok(key) => match client.authenticate_publickey(username, key).await? {
                russh::client::AuthResult::Success => return Ok(Some(key_path)),
                russh::client::AuthResult::Failure { .. } => {
                    saw_auth_failure = true;
                }
            },
            Err(error) => {
                if first_load_error.is_none() {
                    first_load_error = Some(error);
                }
            }
        }
    }

    if saw_auth_failure {
        return Ok(None);
    }

    Err(first_load_error.unwrap_or_else(|| {
        SshError::Key(String::from(
            "no SSH private keys could be loaded for verification",
        ))
    }))
}

async fn verify_auth_outcome(
    ip: &str,
    username: &str,
    password: Option<&str>,
    timeout: Duration,
) -> Result<AuthOutcome, SshError> {
    let password = password.map(str::trim).filter(|value| !value.is_empty());

    time::timeout(timeout, async move {
        let mut client = connect_client(ip, timeout).await?;
        let result = authenticate_client(&mut client, username, password).await;
        let _ = disconnect_client(client, "credential verification complete").await;
        result
    })
    .await
    .map_err(|_| SshError::Timeout(timeout))?
}

async fn authenticate_client(
    client: &mut Handle<VerificationClient>,
    username: &str,
    password: Option<&str>,
) -> Result<AuthOutcome, SshError> {
    if let Some(password) = password {
        let authenticated = authenticate_with_password(client, username, password).await?;
        return Ok(if authenticated {
            AuthOutcome::Authenticated
        } else {
            AuthOutcome::Denied(AuthDeniedReason::PasswordRejected)
        });
    }

    let key_path = authenticate_with_key(client, username).await?;
    Ok(match key_path {
        Some(_) => AuthOutcome::Authenticated,
        None => AuthOutcome::Denied(AuthDeniedReason::PublicKeyRejected),
    })
}

fn map_verify_result_to_device_status(
    result: Result<AuthOutcome, SshError>,
    ip: &str,
    username: &str,
) -> DeviceStatus {
    match result {
        Ok(AuthOutcome::Authenticated) => DeviceStatus::Ready,
        Ok(AuthOutcome::Denied(_)) => DeviceStatus::Denied,
        Err(error) => {
            eprintln!("[ERROR] ssh verify failed for {ip} as {username}: {error}");
            DeviceStatus::Error
        }
    }
}

async fn verify_specific_ssh_key(
    ip: &str,
    username: &str,
    timeout: Duration,
    key_path: &Path,
) -> Result<bool, SshError> {
    time::timeout(timeout, async move {
        let mut client = connect_client(ip, timeout).await?;
        let result = authenticate_with_specific_key(&mut client, username, key_path).await;
        let _ = disconnect_client(client, "installed key verification complete").await;
        result
    })
    .await
    .map_err(|_| SshError::Timeout(timeout))?
}

async fn authenticate_with_specific_key(
    client: &mut Handle<VerificationClient>,
    username: &str,
    key_path: &Path,
) -> Result<bool, SshError> {
    let preferred_rsa_hash = client.best_supported_rsa_hash().await?;
    let key = load_private_key(key_path, preferred_rsa_hash)?;

    match client.authenticate_publickey(username, key).await? {
        russh::client::AuthResult::Success => Ok(true),
        russh::client::AuthResult::Failure { .. } => Ok(false),
    }
}

async fn run_command(
    client: &mut Handle<VerificationClient>,
    command: &str,
) -> Result<CommandOutput, SshError> {
    let mut channel = client.channel_open_session().await?;
    channel.exec(true, command.as_bytes()).await?;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_status = 0_i32;

    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Data { data } => stdout.extend_from_slice(&data),
            ChannelMsg::ExtendedData { data, .. } => stderr.extend_from_slice(&data),
            ChannelMsg::ExitStatus {
                exit_status: status,
            } => {
                exit_status = status as i32;
            }
            ChannelMsg::ExitSignal {
                signal_name,
                error_message,
                ..
            } => {
                return Err(SshError::Connection(format!(
                    "remote command terminated by signal {signal_name:?}: {error_message}"
                )));
            }
            _ => {}
        }
    }

    Ok(CommandOutput {
        stdout: String::from_utf8_lossy(&stdout).trim().to_owned(),
        stderr: String::from_utf8_lossy(&stderr).trim().to_owned(),
        exit_status,
    })
}

fn load_private_key(
    key_path: &Path,
    preferred_rsa_hash: Option<Option<HashAlg>>,
) -> Result<PrivateKeyWithHashAlg, SshError> {
    let private_key = keys::load_secret_key(key_path, None).map_err(|error| {
        SshError::Key(format!(
            "unable to load SSH private key {}: {error}",
            key_path.display()
        ))
    })?;
    let hash_alg = match private_key.algorithm() {
        Algorithm::Rsa { .. } => preferred_rsa_hash.unwrap_or(Some(HashAlg::Sha512)),
        _ => None,
    };

    Ok(PrivateKeyWithHashAlg::new(Arc::new(private_key), hash_alg))
}

fn default_private_key_paths() -> Result<Vec<PathBuf>, SshError> {
    key_mgmt::candidate_private_key_paths()
        .map_err(|error| SshError::Key(error.to_string()))
        .map(|paths| paths.into_iter().filter(|path| path.is_file()).collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExistingKeyProbeOutcome {
    Ready(PathBuf),
    AllFailed { reason: String },
    Incomplete { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalOpenSshFailureKind {
    HostKeyMismatch,
    PublicKeyRejected,
    PrivateKeyProblem,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalOpenSshFailure {
    kind: LocalOpenSshFailureKind,
    detail: String,
}

#[derive(Debug)]
enum LocalOpenSshValidationError {
    Execution(SshError),
    Failure(LocalOpenSshFailure),
}

async fn verify_existing_keys_for_launch(
    ip: &str,
    username: &str,
    consumer: LaunchAuthConsumer,
    timeout: Duration,
    key_paths: &[PathBuf],
) -> ExistingKeyProbeOutcome {
    let deadline = time::Instant::now() + timeout;
    let mut attempts = Vec::with_capacity(key_paths.len());
    let mut exhausted_by_timeout = false;

    for key_path in key_paths {
        let key_label = display_key_label(key_path);
        let Some(remaining) = deadline.checked_duration_since(time::Instant::now()) else {
            exhausted_by_timeout = true;
            break;
        };
        if remaining.is_zero() {
            exhausted_by_timeout = true;
            break;
        }

        match verify_specific_ssh_key(ip, username, remaining, key_path).await {
            Ok(true) => {
                match verify_key_for_external_openssh(ip, username, consumer, remaining, key_path) {
                    Ok(prepared_key_path) => {
                        return ExistingKeyProbeOutcome::Ready(prepared_key_path);
                    }
                    Err(error) => attempts.push(format!(
                        "{key_label} 可通过 russh 认证，但{}验证失败：{}",
                        launch_auth_consumer_label(consumer),
                        compact_user_message(&error.to_string(), 96)
                    )),
                }
            }
            Ok(false) => attempts.push(format!("{key_label} 远端未接受该密钥认证")),
            Err(error) => attempts.push(format!(
                "{key_label} 验证失败：{}",
                compact_user_message(&error.to_string(), 96)
            )),
        }
    }

    classify_existing_key_probe_outcome(key_paths, &attempts, exhausted_by_timeout)
}

fn classify_existing_key_probe_outcome(
    key_paths: &[PathBuf],
    attempts: &[String],
    exhausted_by_timeout: bool,
) -> ExistingKeyProbeOutcome {
    let reason = summarize_existing_key_attempts(key_paths, attempts, exhausted_by_timeout);
    if exhausted_by_timeout && attempts.len() < key_paths.len() {
        ExistingKeyProbeOutcome::Incomplete { reason }
    } else {
        ExistingKeyProbeOutcome::AllFailed { reason }
    }
}

fn summarize_existing_key_attempts(
    key_paths: &[PathBuf],
    attempts: &[String],
    exhausted_by_timeout: bool,
) -> String {
    if attempts.is_empty() {
        if exhausted_by_timeout {
            return String::from("尝试本地 SSH 密钥时超时，尚未找到可用于外部工具免密的密钥");
        }
        return String::from("检测到本地 SSH 密钥，但没有任何密钥可用于外部工具免密连接");
    }

    let visible_attempts = attempts.iter().take(2).cloned().collect::<Vec<_>>();
    let mut reason = format!(
        "已尝试本地已有 SSH 密钥（{}/{}）：{}",
        attempts.len(),
        key_paths.len(),
        visible_attempts.join("；")
    );
    let omitted = attempts.len().saturating_sub(visible_attempts.len());
    if omitted > 0 {
        reason.push_str(&format!("；其余 {omitted} 项已省略"));
    }
    if exhausted_by_timeout && attempts.len() < key_paths.len() {
        reason.push_str("；其余密钥因总超时限制未继续尝试");
    }

    reason
}

fn verify_key_for_external_openssh(
    ip: &str,
    username: &str,
    consumer: LaunchAuthConsumer,
    timeout: Duration,
    key_path: &Path,
) -> Result<PathBuf, SshError> {
    let prepared_key =
        key_mgmt::prepare_private_key_for_external_use(key_path).map_err(|error| {
            SshError::Connection(format!(
                "修复本地 SSH 私钥权限失败：{}",
                compact_user_message(&error.to_string(), 120)
            ))
        })?;
    let prepared_key_path = prepared_key.path.clone();
    let verification_result = match consumer {
        LaunchAuthConsumer::Mobaxterm => {
            verify_local_openssh_explicit_key_mode(ip, username, &prepared_key_path, timeout, true)
        }
        LaunchAuthConsumer::VscodeLike | LaunchAuthConsumer::Shell => {
            let alias = config::host_alias(ip, username);
            let rollback = match config::update_ssh_config_with_rollback(
                &alias,
                ip,
                username,
                &prepared_key_path,
            ) {
                Ok(rollback) => rollback,
                Err(error) => {
                    let cleanup_error = cleanup_prepared_external_key(&prepared_key).err();
                    return Err(assemble_external_validation_error(
                        SshError::Connection(format!("更新 SSH alias 配置失败：{error}")),
                        None,
                        cleanup_error,
                    ));
                }
            };

            let verification = verify_vscode_like_key_ready(ip, &alias, timeout);
            if let Err(error) = verification {
                let mapped = local_openssh_validation_to_ssh_error(error);
                let rollback_error = rollback.rollback().err();
                let cleanup_error = cleanup_prepared_external_key(&prepared_key).err();
                return Err(assemble_external_validation_error(
                    mapped,
                    rollback_error,
                    cleanup_error,
                ));
            }

            Ok(())
        }
    };

    match verification_result {
        Ok(()) => Ok(prepared_key_path),
        Err(validation_error) => {
            let mapped = local_openssh_validation_to_ssh_error(validation_error);
            let cleanup_error = cleanup_prepared_external_key(&prepared_key).err();
            Err(assemble_external_validation_error(
                mapped,
                None,
                cleanup_error,
            ))
        }
    }
}

fn verify_vscode_like_key_ready(
    ip: &str,
    alias: &str,
    timeout: Duration,
) -> Result<(), LocalOpenSshValidationError> {
    let first = verify_local_openssh_batch_mode(alias, timeout, true);
    match first {
        Ok(()) => Ok(()),
        Err(LocalOpenSshValidationError::Failure(failure))
            if failure.kind == LocalOpenSshFailureKind::HostKeyMismatch =>
        {
            config::repair_known_host_mismatch(ip, alias).map_err(|error| {
                LocalOpenSshValidationError::Execution(SshError::Connection(format!(
                    "检测到 system known_hosts 主机指纹冲突，但修复冲突条目失败：{error}"
                )))
            })?;
            verify_local_openssh_batch_mode(alias, timeout, true)
        }
        Err(error) => Err(error),
    }
}

fn verify_local_openssh_batch_mode(
    alias: &str,
    timeout: Duration,
    accept_new: bool,
) -> Result<(), LocalOpenSshValidationError> {
    let config_path = store::system_ssh_config_path().map_err(|error| {
        LocalOpenSshValidationError::Execution(SshError::Connection(format!(
            "读取系统 SSH config 路径失败：{error}"
        )))
    })?;
    let connect_timeout = timeout.as_secs().max(1);
    let mut args = vec![
        OsString::from("-F"),
        config_path.into_os_string(),
        OsString::from("-o"),
        OsString::from("BatchMode=yes"),
        OsString::from("-o"),
        OsString::from("PasswordAuthentication=no"),
        OsString::from("-o"),
        OsString::from("PreferredAuthentications=publickey"),
        OsString::from("-o"),
        OsString::from(format!("ConnectTimeout={connect_timeout}")),
        OsString::from(alias),
        OsString::from("exit"),
    ];
    if accept_new {
        args.splice(
            6..6,
            [
                OsString::from("-o"),
                OsString::from("StrictHostKeyChecking=accept-new"),
            ],
        );
    }
    verify_local_openssh_command(&args)
}

fn verify_local_openssh_explicit_key_mode(
    ip: &str,
    username: &str,
    key_path: &Path,
    timeout: Duration,
    accept_new: bool,
) -> Result<(), LocalOpenSshValidationError> {
    let connect_timeout = timeout.as_secs().max(1);
    let target = format!("{username}@{ip}");
    let mut args = vec![
        OsString::from("-o"),
        OsString::from("BatchMode=yes"),
        OsString::from("-o"),
        OsString::from("PasswordAuthentication=no"),
        OsString::from("-o"),
        OsString::from("PreferredAuthentications=publickey"),
        OsString::from("-o"),
        OsString::from("PubkeyAuthentication=yes"),
        OsString::from("-o"),
        OsString::from("IdentitiesOnly=yes"),
        OsString::from("-o"),
        OsString::from("HostKeyAlgorithms=+ssh-rsa"),
        OsString::from("-o"),
        OsString::from("PubkeyAcceptedAlgorithms=+ssh-rsa"),
        OsString::from("-o"),
        OsString::from("PubkeyAcceptedKeyTypes=+ssh-rsa"),
        OsString::from("-o"),
        OsString::from(format!("ConnectTimeout={connect_timeout}")),
        OsString::from("-i"),
        key_path.as_os_str().to_os_string(),
        OsString::from(target),
        OsString::from("exit"),
    ];
    if accept_new {
        args.splice(
            12..12,
            [
                OsString::from("-o"),
                OsString::from("StrictHostKeyChecking=accept-new"),
            ],
        );
    }
    verify_local_openssh_command(&args)
}

fn verify_local_openssh_command(args: &[OsString]) -> Result<(), LocalOpenSshValidationError> {
    let output = run_local_openssh_command(args).map_err(LocalOpenSshValidationError::Execution)?;
    if output.status.success() {
        return Ok(());
    }

    let detail = command_failure_reason(output.status.code(), &output.stderr, &output.stdout);
    Err(LocalOpenSshValidationError::Failure(
        classify_local_openssh_failure(&detail),
    ))
}

fn run_local_openssh_command(args: &[OsString]) -> Result<std::process::Output, SshError> {
    let mut first_failure_output: Option<std::process::Output> = None;
    let mut first_io_error: Option<io::Error> = None;

    for executable in local_openssh_candidates() {
        let mut command = Command::new(&executable);
        command.args(args);
        configure_windows_hidden_process(&mut command);
        match command.output() {
            Ok(output) if output.status.success() => return Ok(output),
            Ok(output) => {
                if first_failure_output.is_none() {
                    first_failure_output = Some(output);
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

    if let Some(output) = first_failure_output {
        return Ok(output);
    }
    if let Some(error) = first_io_error {
        return Err(SshError::Io(error));
    }

    Err(SshError::Unsupported(OPENSSH_NOT_FOUND_MESSAGE))
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

fn local_openssh_candidates() -> Vec<PathBuf> {
    #[cfg(windows)]
    let mut candidates = Vec::new();
    #[cfg(not(windows))]
    let candidates = vec![PathBuf::from("ssh")];

    #[cfg(windows)]
    {
        push_windows_openssh_candidate(&mut candidates, "WINDIR");
        push_windows_openssh_candidate(&mut candidates, "SystemRoot");
        if !candidates.iter().any(|path| path == Path::new("ssh")) {
            candidates.push(PathBuf::from("ssh"));
        }
    }

    candidates
}

#[cfg(windows)]
fn push_windows_openssh_candidate(candidates: &mut Vec<PathBuf>, env_key: &str) {
    let Some(root) = std::env::var_os(env_key) else {
        return;
    };
    let path = PathBuf::from(root)
        .join("System32")
        .join("OpenSSH")
        .join("ssh.exe");
    if path.is_file() && !candidates.iter().any(|existing| existing == &path) {
        candidates.push(path);
    }
}

fn command_failure_reason(code: Option<i32>, stderr: &[u8], stdout: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(stdout).trim().to_owned();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else if let Some(code) = code {
        format!("exit code {code}")
    } else {
        String::from("terminated by signal")
    };

    normalize_whitespace(&detail)
}

fn summarize_local_openssh_failure(detail: &str) -> String {
    let normalized = normalize_whitespace(detail);
    match local_openssh_failure_kind(&normalized) {
        LocalOpenSshFailureKind::HostKeyMismatch => {
            String::from("系统 known_hosts 主机指纹冲突（host key mismatch）")
        }
        LocalOpenSshFailureKind::PublicKeyRejected => String::from("远端未接受当前 SSH 公钥"),
        LocalOpenSshFailureKind::PrivateKeyProblem => {
            if let Some(path) = extract_quoted_path(
                &normalized,
                &[
                    "Load key '",
                    "Load key \"",
                    "Permissions for '",
                    "Permissions for \"",
                ],
            ) {
                return format!("Windows 私钥权限或格式异常，OpenSSH 已拒绝加载：{path}");
            }
            String::from("Windows 私钥权限或格式异常，OpenSSH 已拒绝加载")
        }
        LocalOpenSshFailureKind::Other => compact_user_message(&normalized, 120),
    }
}

fn local_openssh_failure_kind(detail: &str) -> LocalOpenSshFailureKind {
    let lowered = detail.to_ascii_lowercase();

    if lowered.contains("host key verification failed")
        || lowered.contains("remote host identification has changed")
        || lowered.contains("offending key in")
    {
        return LocalOpenSshFailureKind::HostKeyMismatch;
    }
    if is_public_key_rejected_error(&lowered) {
        return LocalOpenSshFailureKind::PublicKeyRejected;
    }
    if lowered.contains("unprotected private key file")
        || lowered.contains("bad permissions")
        || lowered.contains("are too open")
        || lowered.contains("load key")
        || lowered.contains("invalid format")
        || lowered.contains("error in libcrypto")
        || lowered.contains("no such identity")
    {
        return LocalOpenSshFailureKind::PrivateKeyProblem;
    }

    LocalOpenSshFailureKind::Other
}

fn is_public_key_rejected_error(lowered: &str) -> bool {
    let has_publickey_token = lowered.contains("publickey");
    let has_permission_denied = lowered.contains("permission denied");
    let has_methods_continue = lowered.contains("authentications that can continue");
    let has_auth_methods_exhausted = lowered.contains("no more authentication methods to try")
        || lowered.contains("no supported authentication methods available")
        || lowered.contains("all configured authentication methods failed");

    (has_permission_denied || has_methods_continue || has_auth_methods_exhausted)
        && has_publickey_token
}

fn classify_local_openssh_failure(detail: &str) -> LocalOpenSshFailure {
    let normalized = normalize_whitespace(detail);
    LocalOpenSshFailure {
        kind: local_openssh_failure_kind(&normalized),
        detail: summarize_local_openssh_failure(&normalized),
    }
}

fn local_openssh_validation_to_ssh_error(error: LocalOpenSshValidationError) -> SshError {
    match error {
        LocalOpenSshValidationError::Execution(error) => error,
        LocalOpenSshValidationError::Failure(failure) => SshError::Connection(format!(
            "本地 OpenSSH 验证失败（{}）：{}",
            local_openssh_failure_kind_label(failure.kind),
            failure.detail
        )),
    }
}

fn local_openssh_failure_kind_label(kind: LocalOpenSshFailureKind) -> &'static str {
    match kind {
        LocalOpenSshFailureKind::HostKeyMismatch => "host key mismatch",
        LocalOpenSshFailureKind::PublicKeyRejected => "public key rejected",
        LocalOpenSshFailureKind::PrivateKeyProblem => "private key problem",
        LocalOpenSshFailureKind::Other => "other",
    }
}

fn display_key_label(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn compact_user_message(message: &str, max_chars: usize) -> String {
    let normalized = normalize_whitespace(message);
    let mut shortened = String::new();

    for (count, ch) in normalized.chars().enumerate() {
        if count == max_chars {
            shortened.push('…');
            return shortened;
        }
        shortened.push(ch);
    }

    shortened
}

fn normalize_whitespace(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_quoted_path(message: &str, prefixes: &[&str]) -> Option<String> {
    prefixes.iter().find_map(|prefix| {
        let (_, rest) = message.split_once(prefix)?;
        let quote = prefix.chars().last()?;
        let end = rest.find(quote)?;
        Some(rest[..end].to_owned())
    })
}

fn merge_fallback_reason(existing_issue: Option<&str>, current_reason: String) -> String {
    match existing_issue {
        Some(issue) => format!("{issue}；{current_reason}"),
        None => current_reason,
    }
}

fn password_fallback_with_known_hosts_repair(
    ip: &str,
    username: &str,
    consumer: LaunchAuthConsumer,
    existing_issue: Option<&str>,
    current_reason: String,
) -> LaunchAuthPreparation {
    let merged_reason = merge_fallback_reason(existing_issue, current_reason);
    let reason = maybe_repair_system_known_hosts_for_password_fallback(
        merged_reason,
        ip,
        username,
        consumer,
    );

    LaunchAuthPreparation::PasswordFallback { reason }
}

fn maybe_repair_system_known_hosts_for_password_fallback(
    reason: String,
    ip: &str,
    username: &str,
    consumer: LaunchAuthConsumer,
) -> String {
    if !matches!(
        consumer,
        LaunchAuthConsumer::VscodeLike | LaunchAuthConsumer::Shell
    ) {
        return reason;
    }

    let alias = config::host_alias(ip, username);
    match config::repair_known_host_mismatch(ip, &alias) {
        Ok(()) => reason,
        Err(error) => append_system_known_hosts_repair_failure_reason(reason, &error),
    }
}

fn append_system_known_hosts_repair_failure_reason(
    reason: String,
    error: &config::ConfigError,
) -> String {
    format!(
        "{reason}；并且 system known_hosts 主机指纹修复失败（{error}），当前可能仍无法进入密码输入阶段，请先修复该主机的 known_hosts 信任状态"
    )
}

fn assemble_external_validation_error(
    primary: SshError,
    rollback_error: Option<config::ConfigError>,
    cleanup_error: Option<SshError>,
) -> SshError {
    if rollback_error.is_none() && cleanup_error.is_none() {
        return primary;
    }

    let mut detail = primary.to_string();
    if let Some(error) = rollback_error {
        detail.push_str(&format!("；并且回滚 SSH alias 配置失败：{error}"));
    }
    if let Some(error) = cleanup_error {
        detail.push_str(&format!("；并且{}", error));
    }

    SshError::Connection(detail)
}

fn cleanup_prepared_external_key(
    prepared_key: &key_mgmt::PreparedPrivateKeyForExternalUse,
) -> Result<(), SshError> {
    key_mgmt::cleanup_prepared_private_key(&prepared_key.path, prepared_key.cleanup_on_failure)
        .map_err(|error| {
            SshError::Connection(format!(
                "清理本次连接临时私钥副本失败（{}）：{}",
                prepared_key.path.display(),
                compact_user_message(&error.to_string(), 120)
            ))
        })
}

fn launch_auth_consumer_label(consumer: LaunchAuthConsumer) -> &'static str {
    match consumer {
        LaunchAuthConsumer::VscodeLike => "系统 OpenSSH",
        LaunchAuthConsumer::Mobaxterm => "MobaXterm 显式 ssh -i 路径",
        LaunchAuthConsumer::Shell => "系统 OpenSSH 终端",
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::scanner::DeviceStatus;

    use std::path::PathBuf;

    use super::{
        AuthDeniedReason, AuthOutcome, ExistingKeyProbeOutcome, LaunchAuthConsumer,
        LocalOpenSshFailureKind, SshError, append_system_known_hosts_repair_failure_reason,
        classify_existing_key_probe_outcome, config, local_openssh_failure_kind,
        map_verify_result_to_device_status, maybe_repair_system_known_hosts_for_password_fallback,
        summarize_existing_key_attempts, summarize_local_openssh_failure,
    };

    #[test]
    fn summarize_existing_key_attempts_includes_counts_and_details() {
        let keys = vec![
            PathBuf::from("/tmp/.ssh/lanscanner_id_rsa"),
            PathBuf::from("/tmp/.ssh/id_ed25519"),
        ];
        let attempts = vec![
            String::from("/tmp/.ssh/lanscanner_id_rsa 远端未接受该密钥认证"),
            String::from("/tmp/.ssh/id_ed25519 验证失败：permission denied"),
        ];

        let reason = summarize_existing_key_attempts(&keys, &attempts, false);
        assert!(reason.contains("（2/2）"));
        assert!(reason.contains("lanscanner_id_rsa 远端未接受该密钥认证"));
        assert!(reason.contains("id_ed25519 验证失败"));
    }

    #[test]
    fn summarize_existing_key_attempts_marks_timeout_when_unfinished() {
        let keys = vec![
            PathBuf::from("/tmp/.ssh/lanscanner_id_rsa"),
            PathBuf::from("/tmp/.ssh/id_ed25519"),
            PathBuf::from("/tmp/.ssh/id_rsa"),
        ];
        let attempts = vec![String::from(
            "/tmp/.ssh/lanscanner_id_rsa 远端未接受该密钥认证",
        )];

        let reason = summarize_existing_key_attempts(&keys, &attempts, true);
        assert!(reason.contains("（1/3）"));
        assert!(reason.contains("其余密钥因总超时限制未继续尝试"));
    }

    #[test]
    fn summarize_existing_key_attempts_omits_long_tail() {
        let keys = vec![
            PathBuf::from("/tmp/.ssh/key_alpha"),
            PathBuf::from("/tmp/.ssh/key_beta"),
            PathBuf::from("/tmp/.ssh/key_gamma"),
        ];
        let attempts = vec![
            String::from("key_alpha 远端未接受该密钥认证"),
            String::from("key_beta 远端未接受该密钥认证"),
            String::from("key_gamma 远端未接受该密钥认证"),
        ];

        let reason = summarize_existing_key_attempts(&keys, &attempts, false);
        assert!(reason.contains("其余 1 项已省略"));
        assert!(!reason.contains("key_gamma 远端未接受该密钥认证"));
    }

    #[test]
    fn summarize_local_openssh_failure_collapses_bad_permissions_banner() {
        let reason = summarize_local_openssh_failure(
            "Bad permissions. Try removing permissions for user: UNKNOWN\\UNKNOWN on file C:/Users/leo/.ssh/lanscanner_id_rsa.\n@@@@@@@@ WARNING: UNPROTECTED PRIVATE KEY FILE! @@@@@@@@\nPermissions for 'C:/Users/leo/.ssh/lanscanner_id_rsa' are too open.\nLoad key \"C:/Users/leo/.ssh/lanscanner_id_rsa\": bad permissions",
        );

        assert_eq!(
            reason,
            "Windows 私钥权限或格式异常，OpenSSH 已拒绝加载：C:/Users/leo/.ssh/lanscanner_id_rsa"
        );
    }

    #[test]
    fn local_openssh_failure_kind_detects_host_key_mismatch() {
        let kind = local_openssh_failure_kind(
            "WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED! Host key verification failed.",
        );
        assert_eq!(kind, LocalOpenSshFailureKind::HostKeyMismatch);
    }

    #[test]
    fn local_openssh_failure_kind_detects_public_key_rejected() {
        let kind = local_openssh_failure_kind("Permission denied (publickey,password).");
        assert_eq!(kind, LocalOpenSshFailureKind::PublicKeyRejected);
    }

    #[test]
    fn local_openssh_failure_kind_detects_public_key_rejected_with_extra_methods() {
        let kind = local_openssh_failure_kind(
            "root@10.0.0.8: Permission denied (publickey,gssapi-keyex,gssapi-with-mic,password).",
        );
        assert_eq!(kind, LocalOpenSshFailureKind::PublicKeyRejected);
    }

    #[test]
    fn local_openssh_failure_kind_detects_public_key_rejected_when_methods_exhausted() {
        let kind = local_openssh_failure_kind(
            "No more authentication methods to try. Permission denied (publickey).",
        );
        assert_eq!(kind, LocalOpenSshFailureKind::PublicKeyRejected);
    }

    #[test]
    fn classify_existing_key_probe_outcome_marks_incomplete_when_keys_untried() {
        let keys = vec![
            PathBuf::from("/tmp/.ssh/lanscanner_id_rsa"),
            PathBuf::from("/tmp/.ssh/id_ed25519"),
            PathBuf::from("/tmp/.ssh/id_rsa"),
        ];
        let attempts = vec![String::from(
            "/tmp/.ssh/lanscanner_id_rsa 远端未接受该密钥认证",
        )];

        let outcome = classify_existing_key_probe_outcome(&keys, &attempts, true);
        assert!(matches!(
            outcome,
            ExistingKeyProbeOutcome::Incomplete { .. }
        ));
    }

    #[test]
    fn classify_existing_key_probe_outcome_marks_all_failed_after_full_attempts() {
        let keys = vec![
            PathBuf::from("/tmp/.ssh/lanscanner_id_rsa"),
            PathBuf::from("/tmp/.ssh/id_ed25519"),
        ];
        let attempts = vec![
            String::from("/tmp/.ssh/lanscanner_id_rsa 远端未接受该密钥认证"),
            String::from("/tmp/.ssh/id_ed25519 验证失败：permission denied"),
        ];

        let outcome = classify_existing_key_probe_outcome(&keys, &attempts, true);
        assert!(matches!(outcome, ExistingKeyProbeOutcome::AllFailed { .. }));
    }

    #[test]
    fn verify_result_maps_password_rejected_to_denied() {
        let status = map_verify_result_to_device_status(
            Ok(AuthOutcome::Denied(AuthDeniedReason::PasswordRejected)),
            "192.168.1.5",
            "root",
        );
        assert_eq!(status, DeviceStatus::Denied);
    }

    #[test]
    fn verify_result_maps_public_key_rejected_to_denied() {
        let status = map_verify_result_to_device_status(
            Ok(AuthOutcome::Denied(AuthDeniedReason::PublicKeyRejected)),
            "192.168.1.6",
            "root",
        );
        assert_eq!(status, DeviceStatus::Denied);
    }

    #[test]
    fn verify_result_maps_timeout_to_error() {
        let status = map_verify_result_to_device_status(
            Err(SshError::Timeout(Duration::from_secs(2))),
            "192.168.1.7",
            "root",
        );
        assert_eq!(status, DeviceStatus::Error);
    }

    #[test]
    fn append_system_known_hosts_repair_failure_reason_includes_warning_detail() {
        let reason = append_system_known_hosts_repair_failure_reason(
            String::from("已回退到密码模式"),
            &config::ConfigError::Command(String::from("ssh-keygen -R failed")),
        );

        assert!(reason.contains("已回退到密码模式"));
        assert!(reason.contains("system known_hosts 主机指纹修复失败"));
        assert!(reason.contains("ssh-keygen -R failed"));
        assert!(reason.contains("可能仍无法进入密码输入阶段"));
    }

    #[test]
    fn maybe_repair_system_known_hosts_skips_non_vscode_consumer() {
        let reason = maybe_repair_system_known_hosts_for_password_fallback(
            String::from("fallback"),
            "192.168.1.8",
            "root",
            LaunchAuthConsumer::Mobaxterm,
        );

        assert_eq!(reason, "fallback");
    }
}
