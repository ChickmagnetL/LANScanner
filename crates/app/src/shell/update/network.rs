use iced::Task;
use ssh_core::network::{self, NetworkInterface};

use crate::message::Message;

use super::super::ShellApp;

pub(super) fn handle_refresh_networks(app: &mut ShellApp) -> Task<Message> {
    if app.visual_check.is_some()
        || app.is_refreshing_networks
        || app.is_verifying
        || app.is_connecting
    {
        return Task::none();
    }

    app.close_overlays();
    app.is_refreshing_networks = true;

    Task::perform(network::detect_interfaces(), Message::NetworksRefreshed)
}

pub(super) fn handle_networks_refreshed(
    app: &mut ShellApp,
    networks: Vec<NetworkInterface>,
) -> Task<Message> {
    app.is_refreshing_networks = false;
    apply_networks(app, networks);
    Task::none()
}

pub(super) fn handle_select_network(app: &mut ShellApp, network_id: String) -> Task<Message> {
    app.selected_network_id = Some(network_id);
    app.network_dropdown_open = false;
    Task::none()
}

pub(super) fn handle_network_dropdown_opened(app: &mut ShellApp) -> Task<Message> {
    if app.network_dropdown_toggle_message().is_some() {
        app.user_dropdown_open = false;
        app.network_dropdown_open = true;
    }

    Task::none()
}

pub(super) fn handle_network_dropdown_closed(app: &mut ShellApp) -> Task<Message> {
    app.network_dropdown_open = false;
    Task::none()
}

pub(super) fn handle_select_device(app: &mut ShellApp, device_id: String) -> Task<Message> {
    if app.devices.iter().any(|device| device.id == device_id) {
        app.selected_device_id = Some(device_id);
    }

    Task::none()
}

pub(super) fn handle_close_detail(app: &mut ShellApp) -> Task<Message> {
    app.selected_device_id = None;
    Task::none()
}

pub(super) fn apply_networks(app: &mut ShellApp, networks: Vec<NetworkInterface>) {
    let signature = network::signature(&networks);

    if signature == app.networks_signature && networks == app.networks {
        return;
    }

    let selected = app
        .selected_network_id
        .as_deref()
        .and_then(|selected_id| networks.iter().find(|iface| iface.id == selected_id))
        .cloned()
        .or_else(|| networks.first().cloned());

    app.network_dropdown_open = false;
    app.networks_signature = signature;
    app.selected_network_id = selected.as_ref().map(|network| network.id.clone());
    app.networks = networks;
}
