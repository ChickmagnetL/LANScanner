# Build Tooling

This directory contains platform-specific build entrypoints.

Current layout:

- `windows.ps1`: native Windows build flow
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

Current Linux build behavior:

- runs only on Linux `x86_64` hosts and builds `x86_64-unknown-linux-gnu`
- prefers a project-local cargo toolchain when present, otherwise reuses host `cargo`; if `cargo` is missing it can bootstrap Rust into `.build-tools/linux-gnu/local-tools/`
- stores intermediate artifacts under `.build-tools/linux-gnu/build/`
- refreshes `release/linux/` on each run and keeps only `LANScanner`

Linux usage:

1. Open a Linux shell in `tools/build/`.
2. Ensure `cc` is available on `PATH`. If `cargo` is not already installed, keep `curl` or `wget` available so the script can bootstrap Rust locally.
3. Run the build script:

   ```bash
   chmod +x ./linux.sh
   ./linux.sh
   ```

4. After a successful build, the final executable is written to `release/linux/LANScanner`.
5. `release/linux/LANScanner` is a Linux binary. Run it in Linux (including WSL2), not as a native Windows executable.

Project-local directories used by the Linux build flow:

- `.build-tools/linux-gnu/local-tools/`: optional project-local Rust toolchain bootstrap cache and downloads
- `.build-tools/linux-gnu/build/`: Cargo target dir and intermediate build artifacts
- `release/linux/`: final `LANScanner` only
