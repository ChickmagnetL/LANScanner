#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod app;
mod cli;
mod message;
mod visual_check;

use app::ShellApp;
use cli::LaunchMode;
use iced::{Size, Theme};

#[cfg(target_os = "windows")]
const APP_USER_MODEL_ID: &str = "com.lanscanner.desktop";

const BUNDLED_CJK_FONT_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/NotoSansSC-ProjectSubset-Regular.ttf"
));

fn main() -> iced::Result {
    configure_windows_app_id();

    let launch_mode = match cli::parse_launch_mode_from_env() {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("[ERROR] {error}");
            std::process::exit(2);
        }
    };

    match launch_mode {
        LaunchMode::Normal => iced::application(ShellApp::boot, ShellApp::update, ShellApp::view)
            .title(ShellApp::title)
            .subscription(ShellApp::subscription)
            .theme(ShellApp::theme)
            .style(app_style)
            .font(BUNDLED_CJK_FONT_BYTES)
            .default_font(ui::theme::fonts::body())
            .window(platform::window::settings())
            .centered()
            .run(),
        LaunchMode::VisualCheck(config) => {
            let window_settings = {
                let mut s = platform::window::settings();
                if config.needs_large_window() {
                    s.size = Size::new(1050.0, 1100.0);
                }
                s
            };
            iced::application(
                move || ShellApp::boot_visual_check(config.clone()),
                ShellApp::update,
                ShellApp::view,
            )
            .title(ShellApp::title)
            .subscription(ShellApp::subscription)
            .theme(ShellApp::theme)
            .style(app_style)
            .font(BUNDLED_CJK_FONT_BYTES)
            .default_font(ui::theme::fonts::body())
            .window(window_settings)
            .centered()
            .run()
        }
    }
}

#[cfg(target_os = "windows")]
fn configure_windows_app_id() {
    use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let app_id: Vec<u16> = APP_USER_MODEL_ID
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let result = unsafe { SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr()) };
    if result < 0 {
        eprintln!(
            "[WARN] SetCurrentProcessExplicitAppUserModelID failed: 0x{:08X}",
            result as u32
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn configure_windows_app_id() {}

fn app_style(_state: &ShellApp, theme: &Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: if platform::window::uses_transparent_surface() {
            iced::Color::TRANSPARENT
        } else {
            ui::theme::colors::palette(theme).canvas
        },
        text_color: ui::theme::colors::palette(theme).text,
    }
}
