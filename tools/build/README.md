# Build Tooling

This directory contains platform-specific build entrypoints.

Current layout:

- `windows.ps1`: native Windows build flow

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

Project-local directories used by the Windows build flow:

- `.build-tools/windows-msvc/local-tools/`: project-local Rust toolchain, Cargo home, and downloaded installers
- `.build-tools/windows-msvc/build/`: Cargo target dir and intermediate build artifacts
- `release/windows/`: final `LANScanner.exe` only
