#[cfg(target_os = "linux")]
use std::time::Duration;

use iced::{Task, window};
use platform::window as platform_window;

use crate::message::Message;

use super::super::{ShellApp, VisualCheckStage};
use super::visual_check;

#[cfg(target_os = "linux")]
const LINUX_WINDOW_SYNC_RETRY_DELAY: Duration = Duration::from_millis(120);
#[cfg(target_os = "linux")]
const LINUX_WINDOW_SYNC_MAX_ATTEMPTS: u8 = 12;

pub(super) fn handle_window_ready(
    app: &mut ShellApp,
    window_id: iced::window::Id,
) -> Task<Message> {
    app.window_id = Some(window_id);
    app.linux_window_backend = None;
    let visual_check_task = visual_check::handle_window_ready(app);
    Task::batch([
        window::is_maximized(window_id).map(Message::WindowMaximizedChanged),
        apply_dwm_border_none(window_id),
        sync_linux_window_runtime(window_id),
        schedule_linux_window_sync_retry(0),
        visual_check_task,
    ])
}

pub(super) fn handle_linux_window_runtime_resolved(
    app: &mut ShellApp,
    runtime: platform_window::LinuxWindowRuntime,
) -> Task<Message> {
    if let Some(error) = runtime.sync_error.as_deref() {
        eprintln!("[WARN] linux window runtime sync failed: {error}");
    }

    if let Some(is_maximized) = runtime.is_maximized {
        app.is_window_maximized = is_maximized;
    }

    app.linux_window_backend = Some(runtime);

    Task::none()
}

pub(super) fn handle_linux_window_sync_retry(app: &ShellApp, attempt: u8) -> Task<Message> {
    #[cfg(target_os = "linux")]
    {
        let Some(window_id) = app.window_id else {
            return Task::none();
        };

        let mut tasks = Vec::new();
        if should_sync_linux_window_runtime(app.linux_window_backend.as_ref()) {
            tasks.push(sync_linux_window_runtime(window_id));
        }

        if should_retry_linux_window_sync(app.linux_window_backend.as_ref(), attempt) {
            tasks.push(schedule_linux_window_sync_retry(attempt + 1));
        }

        return if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        };
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (app, attempt);
        Task::none()
    }
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

        Task::batch([
            window::is_maximized(window_id).map(Message::WindowMaximizedChanged),
            sync_linux_window_runtime_if_needed(app, window_id),
        ])
    } else {
        Task::none()
    }
}

pub(super) fn handle_window_maximized_changed(
    app: &mut ShellApp,
    is_maximized: bool,
) -> Task<Message> {
    if !matches!(
        app.linux_custom_chrome(),
        platform_window::LinuxCustomChrome::X11Active
    ) {
        app.is_window_maximized = is_maximized;
    }

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

#[cfg(target_os = "linux")]
fn sync_linux_window_runtime(window_id: iced::window::Id) -> Task<Message> {
    platform_window::query_or_sync_linux_window_runtime(window_id)
        .map(Message::LinuxWindowRuntimeResolved)
}

#[cfg(not(target_os = "linux"))]
fn sync_linux_window_runtime(_window_id: iced::window::Id) -> Task<Message> {
    Task::none()
}

#[cfg(target_os = "linux")]
fn sync_linux_window_runtime_if_needed(
    app: &ShellApp,
    window_id: iced::window::Id,
) -> Task<Message> {
    if should_sync_linux_window_runtime(app.linux_window_backend.as_ref()) {
        sync_linux_window_runtime(window_id)
    } else {
        Task::none()
    }
}

#[cfg(target_os = "linux")]
fn schedule_linux_window_sync_retry(attempt: u8) -> Task<Message> {
    Task::perform(
        async move {
            tokio::time::sleep(LINUX_WINDOW_SYNC_RETRY_DELAY).await;
            attempt
        },
        Message::LinuxWindowSyncRetry,
    )
}

#[cfg(not(target_os = "linux"))]
fn schedule_linux_window_sync_retry(_attempt: u8) -> Task<Message> {
    Task::none()
}

#[cfg(target_os = "linux")]
fn should_sync_linux_window_runtime(runtime: Option<&platform_window::LinuxWindowRuntime>) -> bool {
    matches!(
        runtime.map(|runtime| runtime.backend),
        None | Some(platform_window::LinuxWindowBackend::Unknown)
            | Some(platform_window::LinuxWindowBackend::X11Xcb)
            | Some(platform_window::LinuxWindowBackend::X11Xlib)
    )
}

#[cfg(target_os = "linux")]
fn should_retry_linux_window_sync(
    runtime: Option<&platform_window::LinuxWindowRuntime>,
    attempt: u8,
) -> bool {
    if attempt + 1 >= LINUX_WINDOW_SYNC_MAX_ATTEMPTS {
        return false;
    }

    should_sync_linux_window_runtime(runtime)
}

#[cfg(not(target_os = "linux"))]
fn should_sync_linux_window_runtime(
    _runtime: Option<&platform_window::LinuxWindowRuntime>,
) -> bool {
    false
}

#[cfg(not(target_os = "linux"))]
fn should_retry_linux_window_sync(
    _runtime: Option<&platform_window::LinuxWindowRuntime>,
    _attempt: u8,
) -> bool {
    false
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use platform::window::{LinuxWindowBackend, LinuxWindowRuntime};

    use super::{
        LINUX_WINDOW_SYNC_MAX_ATTEMPTS, should_retry_linux_window_sync,
        should_sync_linux_window_runtime,
    };

    fn runtime(backend: LinuxWindowBackend) -> LinuxWindowRuntime {
        LinuxWindowRuntime {
            backend,
            is_maximized: None,
            sync_error: None,
        }
    }

    #[test]
    fn linux_window_sync_retries_unknown_and_x11_until_budget_exhausted() {
        let unknown = runtime(LinuxWindowBackend::Unknown);
        let x11_xlib = runtime(LinuxWindowBackend::X11Xlib);
        let x11_xcb = runtime(LinuxWindowBackend::X11Xcb);

        assert!(should_retry_linux_window_sync(None, 0));
        assert!(should_retry_linux_window_sync(Some(&unknown), 0,));
        assert!(should_retry_linux_window_sync(
            Some(&x11_xlib),
            LINUX_WINDOW_SYNC_MAX_ATTEMPTS - 2,
        ));
        assert!(!should_retry_linux_window_sync(
            Some(&x11_xcb),
            LINUX_WINDOW_SYNC_MAX_ATTEMPTS - 1,
        ));
    }

    #[test]
    fn linux_window_sync_stops_after_confirmed_wayland_backend() {
        let wayland = runtime(LinuxWindowBackend::Wayland);

        assert!(!should_sync_linux_window_runtime(Some(&wayland)));
        assert!(!should_retry_linux_window_sync(Some(&wayland), 0));
    }

    #[test]
    fn linux_window_sync_runs_until_runtime_enters_unknown_or_x11_states() {
        let unknown = runtime(LinuxWindowBackend::Unknown);
        let x11_xcb = runtime(LinuxWindowBackend::X11Xcb);
        let x11_xlib = runtime(LinuxWindowBackend::X11Xlib);

        assert!(should_sync_linux_window_runtime(None));
        assert!(should_sync_linux_window_runtime(Some(&unknown)));
        assert!(should_sync_linux_window_runtime(Some(&x11_xcb)));
        assert!(should_sync_linux_window_runtime(Some(&x11_xlib)));
    }
}
