use ssh_core::credential::{self, Credential};
use ssh_core::network::{self, NetworkInterface};
use ssh_core::scanner::Device;

use super::{ShellApp, VerifyCredentialInput};

impl ShellApp {
    pub(super) fn selected_network(&self) -> Option<&NetworkInterface> {
        network::select_by_id(&self.networks, self.selected_network_id.as_deref())
    }

    pub(super) fn selected_device(&self) -> Option<&Device> {
        self.selected_device_id
            .as_deref()
            .and_then(|selected_id| self.devices.iter().find(|device| device.id == selected_id))
    }

    pub(super) fn selected_credential(&self) -> Option<&Credential> {
        self.selected_username
            .as_deref()
            .and_then(|username| credential::find_by_username(&self.credentials, username))
    }

    pub(super) fn normalized_ssh_username(&self) -> Option<String> {
        let username = self.ssh_username.trim();
        (!username.is_empty()).then(|| username.to_owned())
    }

    pub(super) fn normalized_ssh_password(&self) -> Option<String> {
        let password = self.password.trim();
        (!password.is_empty()).then(|| password.to_owned())
    }

    pub(super) fn current_verify_input_signature(&self) -> (Option<String>, Option<String>) {
        (
            self.normalized_ssh_username(),
            self.normalized_ssh_password(),
        )
    }

    pub(super) fn verify_credential_input(&self) -> VerifyCredentialInput {
        match (
            self.normalized_ssh_username(),
            self.normalized_ssh_password(),
        ) {
            (Some(username), Some(password)) => {
                VerifyCredentialInput::UsernamePassword { username, password }
            }
            (Some(username), None) => VerifyCredentialInput::UsernameOnly { username },
            (None, Some(_)) => VerifyCredentialInput::PasswordOnly,
            (None, None) => VerifyCredentialInput::Empty,
        }
    }

    pub(super) fn resolve_verify_credentials(&self) -> Option<(String, Option<String>)> {
        match self.verify_credential_input() {
            VerifyCredentialInput::UsernameOnly { username } => Some((username, None)),
            VerifyCredentialInput::UsernamePassword { username, password } => {
                Some((username, Some(password)))
            }
            VerifyCredentialInput::PasswordOnly | VerifyCredentialInput::Empty => None,
        }
    }

    pub(super) fn sync_selected_username_from_input(&mut self) {
        self.selected_username = self
            .normalized_ssh_username()
            .filter(|username| credential::find_by_username(&self.credentials, username).is_some());
    }

    pub(super) fn has_verify_inputs(&self) -> bool {
        self.resolve_verify_credentials().is_some()
    }

    pub(super) fn normalized_rustdesk_password(&self) -> Option<String> {
        if !self.vnc_enabled {
            return None;
        }

        let password = self.vnc_password.trim();
        (!password.is_empty()).then_some(password.to_owned())
    }
}
