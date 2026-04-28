# Build Tooling

This directory contains platform-specific build entrypoints.

Current layout:

- `windows.ps1`: native Windows build flow
- `macos.sh`: native macOS build flow
- `linux.sh`: native Linux build flow

Build scripts should:

- validate the host platform before continuing
- bootstrap project-local Rust tooling where practical
- keep build caches and intermediate artifacts inside the repository
- place final distributable outputs under `release/<platform>/`

Current Windows build behavior:

- runs only on Windows and builds `x86_64-pc-windows-msvc`
- supports using the current MSVC shell, importing an installed Build Tools environment, or installing Build Tools via `winget`
- stores project-local tooling and intermediate artifacts under `.build-tools/windows-msvc/`
- refreshes `release/windows/` on each run and keeps only `LANScanner.exe`

Windows usage:

1. Open PowerShell in `tools/build/`.
2. If PowerShell blocks `windows.ps1` because it is not digitally signed, allow script execution for the current shell session only:

   ```powershell
   Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
   ```

   This change is temporary and applies only to the current PowerShell window.
3. Run the build script:

   ```powershell
   .\windows.ps1
   ```

4. Follow the MSVC setup prompt shown by the script. After a successful build, the final executable is written to `release/windows/LANScanner.exe`.

Project-local directories used by the Windows build flow:

- `.build-tools/windows-msvc/local-tools/`: project-local Rust toolchain, Cargo home, and downloaded installers
- `.build-tools/windows-msvc/build/`: Cargo target dir and intermediate build artifacts
- `release/windows/`: final `LANScanner.exe` only

Current macOS build behavior:

- runs only on macOS `x86_64` and `arm64` hosts and builds the matching native Apple target
- prefers a project-local cargo toolchain when present, otherwise reuses host `cargo`; if `cargo` is missing it can bootstrap Rust into `.build-tools/macos/local-tools/`
- derives a temporary `.icns` file from `crates/app/assets/lanscanner.ico`, renders a light drag-to-Applications background into a hidden `.background/` DMG asset, stages the app beside an `Applications` symlink inside a branded `LANScanner-macos.dmg`, and applies a best-effort custom icon to the final DMG file
- stores intermediate artifacts under `.build-tools/macos/build/`
- refreshes `release/macos/` on each run and writes only `LANScanner-macos.dmg`

macOS usage:

1. Open Terminal in `tools/build/`.
2. Ensure Xcode Command Line Tools, `python3`, `cc`, `xcrun`, and the standard macOS packaging tools (`sips`, `iconutil`, `plutil`, `hdiutil`, `osascript`) are available on `PATH`. Keep `curl` or `wget` available on the first run when host `cargo` is unavailable so the script can bootstrap a project-local Rust toolchain. Finder scripting must also be permitted for the current terminal session so the script can save the DMG window layout. If `SetFile` is available through Xcode tooling, the script also uses it as a best-effort way to apply a custom volume icon.
3. Run the build script:

   ```bash
   chmod +x ./macos.sh
   ./macos.sh
   ```

4. After a successful build, the macOS release artifact is written to `release/macos/LANScanner-macos.dmg`.
5. On the build host, Finder should show `LANScanner-macos.dmg` with the LANScanner icon when the best-effort outer-DMG icon write succeeds.
6. Opening the DMG shows a light Finder background with an arrow and install hint, `LANScanner.app` on the left, and an `Applications` shortcut on the right.
7. Drag `LANScanner.app` onto `Applications` to install the app in the standard macOS way.
8. The generated disk image contains an unsigned app bundle and is not notarized; it is intended for local testing on the same macOS host.

Project-local directories used by the macOS build flow:

- `.build-tools/macos/local-tools/`: optional project-local Rust toolchain bootstrap cache and downloads
- `.build-tools/macos/build/`: Cargo target dir, temporary iconset assets, the generated DMG background asset, DMG staging assets, and other packaging artifacts
- `release/macos/`: final `LANScanner-macos.dmg`

Current Linux build behavior:

- runs only on Linux `x86_64` hosts and builds `x86_64-unknown-linux-gnu`
- prefers a project-local cargo toolchain when present, otherwise reuses host `cargo`; if `cargo` is missing it can bootstrap Rust into `.build-tools/linux-gnu/local-tools/`
- bootstraps `linuxdeploy` into `.build-tools/linux-gnu/local-tools/linuxdeploy/` when needed and packages an AppImage
- stores intermediate artifacts under `.build-tools/linux-gnu/build/`
- refreshes `release/linux/` on each run and writes only `LANScanner-x86_64.AppImage`

Linux usage:

1. Open a Linux shell in `tools/build/`.
2. Ensure `cc` and `python3` are available on `PATH`. Keep `curl` or `wget` available on the first run so the script can bootstrap missing local tooling such as Rust or `linuxdeploy`.
3. Run the build script:

   ```bash
   chmod +x ./linux.sh
   ./linux.sh
   ```

4. After a successful build, the Linux release artifact is written to `release/linux/LANScanner-x86_64.AppImage`.
5. Run the artifact in Linux (including WSL2), not as a native Windows executable.

Project-local directories used by the Linux build flow:

- `.build-tools/linux-gnu/local-tools/`: optional project-local Rust toolchain bootstrap cache and downloads
- `.build-tools/linux-gnu/local-tools/linuxdeploy/`: cached AppImage packaging tool
- `.build-tools/linux-gnu/build/`: Cargo target dir and intermediate build artifacts
- `release/linux/`: final `LANScanner-x86_64.AppImage`
