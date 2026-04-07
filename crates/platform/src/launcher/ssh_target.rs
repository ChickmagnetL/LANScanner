use ssh_core::ssh::{auth::LaunchAuthPreparation, config};

use super::types::LaunchError;

pub(super) fn prepare_vscode_target(
    ip: &str,
    user: &str,
    auth: &LaunchAuthPreparation,
) -> Result<String, LaunchError> {
    prepare_openssh_target(ip, user, auth)
}

pub(super) fn prepare_shell_target(
    ip: &str,
    user: &str,
    auth: &LaunchAuthPreparation,
) -> Result<String, LaunchError> {
    prepare_openssh_target(ip, user, auth)
}

fn prepare_openssh_target(
    ip: &str,
    user: &str,
    auth: &LaunchAuthPreparation,
) -> Result<String, LaunchError> {
    let alias = config::host_alias(ip, user);
    match auth {
        LaunchAuthPreparation::KeyReady { key_path, .. } => {
            config::update_ssh_config(&alias, ip, user, key_path)?;
            Ok(alias)
        }
        LaunchAuthPreparation::PasswordFallback { .. } => {
            config::update_ssh_config_for_password_fallback(&alias, ip, user)?;
            Ok(alias)
        }
        LaunchAuthPreparation::HardFailure { reason } => {
            Err(LaunchError::Unsupported(reason.clone()))
        }
    }
}
