use iced::{Size, Task, window};

const WINDOW_ICON_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../app/assets/lanscanner.ico"
));

#[cfg(target_os = "linux")]
use x11rb::protocol::shape::{ConnectionExt as _, SK, SO};
#[cfg(target_os = "linux")]
use x11rb::protocol::xproto::{self, ClipOrdering, ConnectionExt as _};

#[cfg(target_os = "linux")]
const LINUX_X11_CORNER_RADIUS: u16 = 14;

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinuxStartupBackend {
    Wayland,
    X11,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxCustomChrome {
    X11Active,
    WaylandActive,
    Fallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxWindowBackend {
    Wayland,
    X11Xcb,
    X11Xlib,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinuxWindowRuntime {
    pub backend: LinuxWindowBackend,
    pub is_maximized: Option<bool>,
    pub sync_error: Option<String>,
}

impl LinuxWindowRuntime {
    pub fn custom_chrome(&self) -> LinuxCustomChrome {
        #[cfg(target_os = "linux")]
        {
            linux_custom_chrome_from_window_backend(self.backend)
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = self;
            LinuxCustomChrome::Fallback
        }
    }
}

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
    #[cfg(target_os = "macos")]
    {
        true
    }

    #[cfg(target_os = "linux")]
    {
        linux_startup_custom_chrome().uses_native_decorations()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        false
    }
}

fn platform_specific_settings() -> window::settings::PlatformSpecific {
    #[allow(unused_mut)]
    let mut settings = window::settings::PlatformSpecific::default();
    #[cfg(target_os = "macos")]
    {
        settings.title_hidden = true;
        settings.titlebar_transparent = true;
        settings.fullsize_content_view = true;
    }
    #[cfg(target_os = "windows")]
    {
        settings.undecorated_shadow = false;
        settings.corner_preference = window::settings::platform::CornerPreference::DoNotRound;
    }
    settings
}

pub fn uses_transparent_surface() -> bool {
    #[cfg(target_os = "macos")]
    {
        false
    }

    #[cfg(target_os = "linux")]
    {
        // `transparent` is fixed at window creation time, so Linux can only use
        // startup environment hints here. Exact backend confirmation happens
        // later via raw display/window handles once the window exists.
        linux_startup_custom_chrome().uses_transparent_surface()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        true
    }
}

pub fn uses_custom_titlebar() -> bool {
    !cfg!(target_os = "macos")
}

pub fn uses_custom_resize_overlay() -> bool {
    !cfg!(target_os = "macos")
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

#[cfg(target_os = "linux")]
pub fn query_or_sync_linux_window_runtime(window_id: window::Id) -> Task<LinuxWindowRuntime> {
    window::run(window_id, |window| {
        query_or_sync_linux_window_runtime_from_handles(
            window.display_handle().ok().map(|handle| handle.as_raw()),
            window.window_handle().ok().map(|handle| handle.as_raw()),
        )
    })
}

#[cfg(target_os = "linux")]
fn linux_startup_backend() -> LinuxStartupBackend {
    classify_linux_startup_backend(
        std::env::var_os("WAYLAND_DISPLAY").is_some_and(|value| !value.is_empty()),
        std::env::var_os("DISPLAY").is_some_and(|value| !value.is_empty()),
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
    )
}

#[cfg(target_os = "linux")]
pub fn linux_startup_custom_chrome() -> LinuxCustomChrome {
    linux_custom_chrome_from_startup_backend(linux_startup_backend())
}

#[cfg(target_os = "linux")]
fn classify_linux_startup_backend(
    has_wayland_display: bool,
    has_x_display: bool,
    session_type: Option<&str>,
) -> LinuxStartupBackend {
    if session_type.is_some_and(|value| value.eq_ignore_ascii_case("wayland")) {
        LinuxStartupBackend::Wayland
    } else if session_type.is_some_and(|value| value.eq_ignore_ascii_case("x11")) {
        LinuxStartupBackend::X11
    } else if has_wayland_display {
        LinuxStartupBackend::Wayland
    } else if has_x_display {
        LinuxStartupBackend::X11
    } else {
        LinuxStartupBackend::Unknown
    }
}

#[cfg(target_os = "linux")]
fn linux_custom_chrome_from_startup_backend(backend: LinuxStartupBackend) -> LinuxCustomChrome {
    match backend {
        LinuxStartupBackend::X11 => LinuxCustomChrome::X11Active,
        LinuxStartupBackend::Wayland => LinuxCustomChrome::WaylandActive,
        LinuxStartupBackend::Unknown => LinuxCustomChrome::Fallback,
    }
}

#[cfg(target_os = "linux")]
fn linux_custom_chrome_from_window_backend(backend: LinuxWindowBackend) -> LinuxCustomChrome {
    match backend {
        LinuxWindowBackend::X11Xcb | LinuxWindowBackend::X11Xlib => LinuxCustomChrome::X11Active,
        LinuxWindowBackend::Wayland => LinuxCustomChrome::WaylandActive,
        LinuxWindowBackend::Unknown => LinuxCustomChrome::Fallback,
    }
}

impl LinuxCustomChrome {
    fn uses_native_decorations(self) -> bool {
        matches!(self, Self::Fallback)
    }

    fn uses_transparent_surface(self) -> bool {
        // Fallback B: Never request a transparent surface for Wayland, because wgpu
        // often falls back to an opaque format on many compositors, causing a black box.
        // We still request transparency for X11 since we use XShape to clip it properly.
        matches!(self, Self::X11Active)
    }
}

#[cfg(target_os = "linux")]
fn classify_linux_window_backend(
    display_handle: Option<window::raw_window_handle::RawDisplayHandle>,
    window_handle: Option<window::raw_window_handle::RawWindowHandle>,
) -> LinuxWindowBackend {
    let display_backend = display_handle.and_then(linux_backend_from_display_handle);
    let window_backend = window_handle.and_then(linux_backend_from_window_handle);

    match (display_backend, window_backend) {
        (Some(display), Some(window)) if display == window => display,
        (Some(_), Some(_)) => LinuxWindowBackend::Unknown,
        (Some(backend), None) | (None, Some(backend)) => backend,
        (None, None) => LinuxWindowBackend::Unknown,
    }
}

#[cfg(target_os = "linux")]
fn linux_backend_from_display_handle(
    handle: window::raw_window_handle::RawDisplayHandle,
) -> Option<LinuxWindowBackend> {
    match handle {
        window::raw_window_handle::RawDisplayHandle::Wayland(_) => {
            Some(LinuxWindowBackend::Wayland)
        }
        window::raw_window_handle::RawDisplayHandle::Xcb(_) => Some(LinuxWindowBackend::X11Xcb),
        window::raw_window_handle::RawDisplayHandle::Xlib(_) => Some(LinuxWindowBackend::X11Xlib),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn linux_backend_from_window_handle(
    handle: window::raw_window_handle::RawWindowHandle,
) -> Option<LinuxWindowBackend> {
    match handle {
        window::raw_window_handle::RawWindowHandle::Wayland(_) => Some(LinuxWindowBackend::Wayland),
        window::raw_window_handle::RawWindowHandle::Xcb(_) => Some(LinuxWindowBackend::X11Xcb),
        window::raw_window_handle::RawWindowHandle::Xlib(_) => Some(LinuxWindowBackend::X11Xlib),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn query_or_sync_linux_window_runtime_from_handles(
    display_handle: Option<window::raw_window_handle::RawDisplayHandle>,
    window_handle: Option<window::raw_window_handle::RawWindowHandle>,
) -> LinuxWindowRuntime {
    let backend = classify_linux_window_backend(display_handle, window_handle);

    match backend {
        LinuxWindowBackend::X11Xcb | LinuxWindowBackend::X11Xlib => {
            match sync_x11_window_runtime(window_handle) {
                Ok(is_maximized) => LinuxWindowRuntime {
                    backend,
                    is_maximized: Some(is_maximized),
                    sync_error: None,
                },
                Err(error) => LinuxWindowRuntime {
                    backend,
                    is_maximized: None,
                    sync_error: Some(error.to_string()),
                },
            }
        }
        LinuxWindowBackend::Wayland | LinuxWindowBackend::Unknown => LinuxWindowRuntime {
            backend,
            is_maximized: None,
            sync_error: None,
        },
    }
}

#[cfg(target_os = "linux")]
fn sync_x11_window_runtime(
    window_handle: Option<window::raw_window_handle::RawWindowHandle>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let Some(window) = window_handle.and_then(x11_window_from_handle) else {
        return Err(
            std::io::Error::other("missing X11 window handle for Linux runtime sync").into(),
        );
    };

    let (connection, _) = x11rb::connect(None)?;
    let is_maximized = x11_window_is_maximized(&connection, window)?;
    apply_x11_window_shape(&connection, window, is_maximized)?;
    Ok(is_maximized)
}

#[cfg(target_os = "linux")]
fn x11_window_from_handle(
    handle: window::raw_window_handle::RawWindowHandle,
) -> Option<xproto::Window> {
    match handle {
        window::raw_window_handle::RawWindowHandle::Xcb(h) => Some(h.window.get()),
        window::raw_window_handle::RawWindowHandle::Xlib(h) => Some(h.window as xproto::Window),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn x11_window_is_maximized<C: x11rb::connection::Connection>(
    connection: &C,
    window: xproto::Window,
) -> Result<bool, Box<dyn std::error::Error>> {
    let net_wm_state = x11_intern_atom(connection, b"_NET_WM_STATE")?;
    let max_horz = x11_intern_atom(connection, b"_NET_WM_STATE_MAXIMIZED_HORZ")?;
    let max_vert = x11_intern_atom(connection, b"_NET_WM_STATE_MAXIMIZED_VERT")?;
    let reply = connection
        .get_property(
            false,
            window,
            net_wm_state,
            xproto::AtomEnum::ATOM,
            0,
            u32::MAX,
        )?
        .reply()?;
    let Some(states) = reply.value32() else {
        return Ok(false);
    };

    let mut has_horz = false;
    let mut has_vert = false;

    for state in states {
        has_horz |= state == max_horz;
        has_vert |= state == max_vert;
    }

    Ok(has_horz && has_vert)
}

#[cfg(target_os = "linux")]
fn x11_intern_atom<C: x11rb::connection::Connection>(
    connection: &C,
    name: &[u8],
) -> Result<xproto::Atom, Box<dyn std::error::Error>> {
    Ok(connection.intern_atom(false, name)?.reply()?.atom)
}

#[cfg(target_os = "linux")]
fn apply_x11_window_shape<C: x11rb::connection::Connection>(
    connection: &C,
    window: xproto::Window,
    is_maximized: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let geometry = connection.get_geometry(window)?.reply()?;
    let rectangles = x11_window_shape_rectangles(geometry.width, geometry.height, is_maximized);

    for kind in [SK::BOUNDING, SK::INPUT] {
        connection
            .shape_rectangles(
                SO::SET,
                kind,
                ClipOrdering::UNSORTED,
                window,
                0,
                0,
                &rectangles,
            )?
            .check()?;
    }

    connection.flush()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn x11_window_shape_rectangles(
    width: u16,
    height: u16,
    is_maximized: bool,
) -> Vec<xproto::Rectangle> {
    if is_maximized || width <= 1 || height <= 1 {
        return vec![rectangle(0, 0, width, height)];
    }

    let radius = LINUX_X11_CORNER_RADIUS.min(width / 2).min(height / 2);
    if radius <= 1 {
        return vec![rectangle(0, 0, width, height)];
    }

    let top_rows = compact_corner_rows(radius);
    let mut rectangles = Vec::with_capacity(top_rows.len() * 2 + 1);
    let mut y = 0_u16;

    for &(inset, rows) in &top_rows {
        let row_width = width.saturating_sub(inset.saturating_mul(2));
        if row_width > 0 {
            rectangles.push(rectangle(inset, y, row_width, rows));
        }
        y += rows;
    }

    let middle_rows = height.saturating_sub(y.saturating_mul(2));
    if middle_rows > 0 {
        rectangles.push(rectangle(0, y, width, middle_rows));
    }
    y += middle_rows;

    for &(inset, rows) in top_rows.iter().rev() {
        let row_width = width.saturating_sub(inset.saturating_mul(2));
        if row_width > 0 {
            rectangles.push(rectangle(inset, y, row_width, rows));
        }
        y += rows;
    }

    rectangles
}

#[cfg(target_os = "linux")]
fn compact_corner_rows(radius: u16) -> Vec<(u16, u16)> {
    let mut rows: Vec<(u16, u16)> = Vec::with_capacity(radius as usize);
    let mut last_inset = radius;

    for y in 0..radius {
        let x = (radius as f32
            - (radius as f32 * radius as f32 - (radius - 1 - y) as f32 * (radius - 1 - y) as f32)
                .sqrt())
        .round() as u16;
        if x == last_inset && !rows.is_empty() {
            rows.last_mut().unwrap().1 += 1;
        } else {
            rows.push((x, 1));
            last_inset = x;
        }
    }
    rows
}

#[cfg(target_os = "linux")]
fn rectangle(x: u16, y: u16, width: u16, height: u16) -> xproto::Rectangle {
    xproto::Rectangle {
        x: x as i16,
        y: y as i16,
        width,
        height,
    }
}
