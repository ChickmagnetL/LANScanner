use iced::Task;
use ssh_core::credential::{self, store};

use crate::message::Message;

use super::super::ShellApp;
use super::super::state::ActiveModal;

pub(super) fn handle_open_cred_modal(app: &mut ShellApp) -> Task<Message> {
    if app.is_verifying || app.is_connecting {
        return Task::none();
    }

    app.close_overlays();
    app.reset_credential_form();
    app.active_modal = Some(ActiveModal::CredentialManagement);
    Task::none()
}

pub(super) fn handle_user_dropdown_opened(app: &mut ShellApp) -> Task<Message> {
    if app.user_dropdown_toggle_message().is_some() {
        app.network_dropdown_open = false;
        app.user_dropdown_open = true;
    }

    Task::none()
}

pub(super) fn handle_user_dropdown_closed(app: &mut ShellApp) -> Task<Message> {
    app.user_dropdown_open = false;
    Task::none()
}

pub(super) fn handle_set_username(app: &mut ShellApp, value: String) -> Task<Message> {
    let previous_verify_input = app.current_verify_input_signature();
    if !app.is_verifying && !app.is_connecting {
        let previous_username = app.normalized_ssh_username();
        app.ssh_username = value;
        app.sync_selected_username_from_input();

        if let Some(username) = app.selected_username.as_deref()
            && previous_username.as_deref() != Some(username)
        {
            app.password = credential::find_by_username(&app.credentials, username)
                .and_then(|credential| credential.password.clone())
                .unwrap_or_default();
        }
    }
    if app.current_verify_input_signature() != previous_verify_input {
        app.clear_verification_state_for_credential_change();
    }

    app.handle_ssh_intent_updated()
}

pub(super) fn handle_select_user(app: &mut ShellApp, username: String) -> Task<Message> {
    let previous_verify_input = app.current_verify_input_signature();
    app.apply_selected_username(username);
    app.user_dropdown_open = false;
    if app.current_verify_input_signature() != previous_verify_input {
        app.clear_verification_state_for_credential_change();
    }

    app.handle_ssh_intent_updated()
}

pub(super) fn handle_set_password(app: &mut ShellApp, password: String) -> Task<Message> {
    let previous_verify_input = app.current_verify_input_signature();
    if !app.is_verifying && !app.is_connecting {
        app.password = password;
    }
    if app.current_verify_input_signature() != previous_verify_input {
        app.clear_verification_state_for_credential_change();
    }

    app.handle_ssh_intent_updated()
}

pub(super) fn handle_toggle_vnc(app: &mut ShellApp) -> Task<Message> {
    if !app.is_verifying && !app.is_connecting {
        app.vnc_enabled = !app.vnc_enabled;
    }

    Task::none()
}

pub(super) fn handle_set_vnc_user(app: &mut ShellApp, value: String) -> Task<Message> {
    if !app.is_verifying && !app.is_connecting {
        app.vnc_user = value;
    }

    Task::none()
}

pub(super) fn handle_set_vnc_password(app: &mut ShellApp, value: String) -> Task<Message> {
    if !app.is_verifying && !app.is_connecting {
        app.vnc_password = value;
    }

    Task::none()
}

pub(super) fn handle_close_cred_modal(app: &mut ShellApp) -> Task<Message> {
    if matches!(app.active_modal, Some(ActiveModal::CredentialManagement)) {
        app.active_modal = None;
    }
    app.reset_credential_form();
    Task::none()
}

pub(super) fn handle_set_new_credential_username(
    app: &mut ShellApp,
    value: String,
) -> Task<Message> {
    app.new_credential_username = value;
    Task::none()
}

pub(super) fn handle_set_new_credential_password(
    app: &mut ShellApp,
    value: String,
) -> Task<Message> {
    app.new_credential_password = value;
    Task::none()
}

pub(super) fn handle_start_edit_credential(app: &mut ShellApp, username: String) -> Task<Message> {
    if credential::find_by_username(&app.credentials, &username).is_none() {
        return Task::none();
    }

    app.editing_credential_username = Some(username.clone());
    app.new_credential_username = username;
    app.new_credential_password.clear();
    Task::none()
}

pub(super) fn handle_cancel_edit_credential(app: &mut ShellApp) -> Task<Message> {
    app.reset_credential_form();
    Task::none()
}

pub(super) fn handle_add_credential(
    app: &mut ShellApp,
    username: String,
    password: Option<String>,
) -> Task<Message> {
    match store::add_credential(&username, password.as_deref()) {
        Ok(config) => {
            app.app_paths = config.app_paths.clone();
            app.replace_credentials(
                credential::credentials_from_config(&config),
                Some(username),
                password,
            );
            app.reset_credential_form();
        }
        Err(error) => {
            eprintln!("[ERROR] credential save failed: {error}");
        }
    }

    Task::none()
}

pub(super) fn handle_update_credential_password(
    app: &mut ShellApp,
    username: String,
    password: String,
) -> Task<Message> {
    match store::update_credential_password(&username, &password) {
        Ok(config) => {
            app.app_paths = config.app_paths.clone();
            app.replace_credentials(
                credential::credentials_from_config(&config),
                Some(username),
                Some(password),
            );
            app.reset_credential_form();
        }
        Err(error) => {
            eprintln!("[ERROR] credential password update failed: {error}");
        }
    }

    Task::none()
}

pub(super) fn handle_remove_credential(app: &mut ShellApp, id: String) -> Task<Message> {
    match store::remove_credential(&id) {
        Ok(config) => {
            let removed_username = credential::username_from_id(&id).to_owned();
            app.app_paths = config.app_paths.clone();

            app.replace_credentials(credential::credentials_from_config(&config), None, None);

            if app.editing_credential_username.as_deref() == Some(removed_username.as_str()) {
                app.reset_credential_form();
            }
        }
        Err(error) => {
            eprintln!("[ERROR] credential delete failed: {error}");
        }
    }

    Task::none()
}
