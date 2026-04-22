# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-04-22

### Added

- Linux one-click build script (`tools/build/linux.sh`).
- Linux AppImage packaging.
- Wayland/X11 split custom chrome implementation.
- System locale detection for startup language.

### Fixed

- Removed shadows and fixed titlebar/corners on Linux.
- Restored X11 window shaping after Wayland split.
- Normalized titlebar logo font weight and offset on Linux.

### Known Issues

- The four corners of the window in Wayland cannot be rounded; only right angles are displayed.
- There is no native application for mobaxterm on Linux, and no alternative application capable of quickly connecting in the mobaxterm position has been found yet

## [0.1.0] - 2026-04-07

### Added

- Initial public open-source release of the Rust + iced LANScanner desktop application.
- Workspace structure with `crates/app`, `crates/core`, `crates/platform`, and `crates/ui`.
- LAN device discovery, SSH verification, credential management, and key handling flows.
- External launcher integration for tools such as VSCode, MobaXterm, VNC Viewer, and RustDesk.
- Windows build entrypoint under `tools/build/windows.ps1`.
