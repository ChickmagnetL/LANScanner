use iced::{Size, Task, window};

const WINDOW_ICON_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../app/assets/lanscanner.ico"
));

#[derive(Debug, Clone, Copy)]
pub enum WindowAction {
    Drag,
    Resize(window::Direction),
    Minimize,
    ToggleMaximize,
    Close,
}

pub fn settings() -> window::Settings {
    window::Settings {
        decorations: use_native_decorations(),
        resizable: true,
        size: Size::new(1050.0, 730.0),
        min_size: Some(Size::new(1050.0, 730.0)),
        transparent: uses_transparent_surface(),
        icon: load_window_icon(),
        platform_specific: platform_specific_settings(),
        ..window::Settings::default()
    }
}

fn use_native_decorations() -> bool {
    false
}

fn platform_specific_settings() -> window::settings::PlatformSpecific {
    #[allow(unused_mut)]
    let mut settings = window::settings::PlatformSpecific::default();
    #[cfg(target_os = "windows")]
    {
        settings.undecorated_shadow = false;
        settings.corner_preference = window::settings::platform::CornerPreference::DoNotRound;
    }
    settings
}

pub fn uses_transparent_surface() -> bool {
    true
}

pub fn uses_custom_resize_overlay() -> bool {
    true
}

fn load_window_icon() -> Option<window::Icon> {
    window::icon::from_file_data(WINDOW_ICON_BYTES, None).ok()
}

pub fn perform<Message>(window_id: window::Id, action: WindowAction) -> Task<Message> {
    match action {
        WindowAction::Drag => drag_window(window_id),
        WindowAction::Resize(direction) => drag_resize_window(window_id, direction),
        WindowAction::Minimize => window::minimize(window_id, true),
        WindowAction::ToggleMaximize => window::toggle_maximize(window_id),
        WindowAction::Close => window::close(window_id),
    }
}

fn drag_window<Message>(window_id: window::Id) -> Task<Message> {
    window::drag(window_id)
}

fn drag_resize_window<Message>(
    window_id: window::Id,
    direction: window::Direction,
) -> Task<Message> {
    window::drag_resize(window_id, direction)
}
