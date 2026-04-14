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
- derives a temporary `.icns` file from `crates/app/assets/lanscanner.ico` and packages a runnable `LANScanner.app`
- stores intermediate artifacts under `.build-tools/macos/build/`
- refreshes `release/macos/` on each run and writes only `LANScanner.app`

macOS usage:

1. Open Terminal in `tools/build/`.
2. Ensure Xcode Command Line Tools, `python3`, and the standard macOS packaging tools (`sips`, `iconutil`, `plutil`) are available on `PATH`. Keep `curl` or `wget` available on the first run so the script can bootstrap missing local Rust tooling if needed.
3. Run the build script:

   ```bash
   chmod +x ./macos.sh
   ./macos.sh
   ```

4. After a successful build, the macOS release artifact is written to `release/macos/LANScanner.app`.
5. The generated app bundle is unsigned and not notarized; it is intended for local testing on the same macOS host.

Project-local directories used by the macOS build flow:

- `.build-tools/macos/local-tools/`: optional project-local Rust toolchain bootstrap cache and downloads
- `.build-tools/macos/build/`: Cargo target dir, temporary iconset assets, and intermediate build artifacts
- `release/macos/`: final `LANScanner.app`

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
