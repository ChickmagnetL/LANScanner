use iced::{Task, window};
use platform::window as platform_window;

use crate::message::Message;

use super::super::{ShellApp, VisualCheckStage};
use super::visual_check;

pub(super) fn handle_window_ready(
    app: &mut ShellApp,
    window_id: iced::window::Id,
) -> Task<Message> {
    app.window_id = Some(window_id);
    let visual_check_task = visual_check::handle_window_ready(app);
    Task::batch([
        window::is_maximized(window_id).map(Message::WindowMaximizedChanged),
        apply_dwm_border_none(window_id),
        visual_check_task,
    ])
}

pub(super) fn handle_window_resized(
    app: &mut ShellApp,
    window_id: iced::window::Id,
) -> Task<Message> {
    if app.window_id == Some(window_id) {
        if let Some(runtime) = app.visual_check.as_mut()
            && matches!(runtime.stage, VisualCheckStage::WaitingForResize(_))
        {
            runtime.stage = VisualCheckStage::Settling(
                visual_check::visual_check_scene_settling_frames(runtime.current_scene),
            );
        }

        window::is_maximized(window_id).map(Message::WindowMaximizedChanged)
    } else {
        Task::none()
    }
}

pub(super) fn handle_window_maximized_changed(
    app: &mut ShellApp,
    is_maximized: bool,
) -> Task<Message> {
    app.is_window_maximized = is_maximized;
    Task::none()
}

pub(super) fn handle_window_action(
    app: &mut ShellApp,
    action: platform_window::WindowAction,
) -> Task<Message> {
    app.close_overlays();
    app.window_id.map_or_else(Task::none, |window_id| {
        platform_window::perform(window_id, action)
    })
}

#[cfg(target_os = "windows")]
fn apply_dwm_border_none(window_id: iced::window::Id) -> Task<Message> {
    use iced::window::raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DwmSetWindowAttribute,
    };

    window::run(window_id, |w| {
        let Ok(handle) = w.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(h) = handle.as_raw() else {
            return;
        };
        let hwnd = h.hwnd.get() as *mut core::ffi::c_void;
        let color: u32 = DWMWA_COLOR_NONE;
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR as u32,
                &color as *const u32 as *const core::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    })
    .map(|_| Message::Noop)
}

#[cfg(not(target_os = "windows"))]
fn apply_dwm_border_none(_window_id: iced::window::Id) -> Task<Message> {
    Task::none()
}
