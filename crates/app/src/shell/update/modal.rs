use iced::Task;

use crate::message::Message;

use super::super::{ActiveModal, ShellApp};

pub(super) fn handle_open_help_modal(app: &mut ShellApp) -> Task<Message> {
    app.close_overlays();
    app.help_modal_show_rustdesk = false;
    app.active_modal = Some(ActiveModal::HelpGuide);
    Task::none()
}

pub(super) fn handle_close_help_modal(app: &mut ShellApp) -> Task<Message> {
    if matches!(app.active_modal, Some(ActiveModal::HelpGuide)) {
        app.active_modal = None;
    }
    Task::none()
}

pub(super) fn handle_show_help_guide_basic(app: &mut ShellApp) -> Task<Message> {
    if matches!(app.active_modal, Some(ActiveModal::HelpGuide)) {
        app.help_modal_show_rustdesk = false;
    }
    Task::none()
}

pub(super) fn handle_show_help_guide_rustdesk(app: &mut ShellApp) -> Task<Message> {
    if matches!(app.active_modal, Some(ActiveModal::HelpGuide)) {
        app.help_modal_show_rustdesk = true;
    }
    Task::none()
}

pub(super) fn handle_open_cred_modal(app: &mut ShellApp) -> Task<Message> {
    if app.is_verifying || app.is_connecting {
        return Task::none();
    }

    app.close_overlays();
    app.reset_credential_form();
    app.active_modal = Some(ActiveModal::CredentialManagement);
    Task::none()
}

pub(super) fn handle_close_cred_modal(app: &mut ShellApp) -> Task<Message> {
    if matches!(app.active_modal, Some(ActiveModal::CredentialManagement)) {
        app.active_modal = None;
    }
    app.reset_credential_form();
    Task::none()
}

pub(super) fn handle_select_container(app: &mut ShellApp, container_id: String) -> Task<Message> {
    if let Some(ActiveModal::DockerSelect(state)) = &mut app.active_modal
        && state
            .containers
            .iter()
            .any(|container| container.id == container_id)
    {
        state.selected_container_id = Some(container_id);
    }

    Task::none()
}

pub(super) fn handle_close_docker_modal(app: &mut ShellApp) -> Task<Message> {
    if matches!(app.active_modal, Some(ActiveModal::DockerSelect(_))) {
        app.active_modal = None;
    }
    app.pending_docker_context = None;
    app.connection_status = None;
    app.clear_active_quick_connect();
    Task::none()
}
