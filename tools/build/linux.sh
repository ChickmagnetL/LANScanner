#!/usr/bin/env bash

set -euo pipefail

TARGET_TRIPLE="x86_64-unknown-linux-gnu"
ARTIFACT_NAME="LANScanner"
APPIMAGE_ID="lanscanner"
APPIMAGE_NAME="LANScanner-x86_64.AppImage"
LINUXDEPLOY_CHANNEL="${LINUXDEPLOY_CHANNEL:-continuous}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

BUILD_TOOLS_ROOT="${PROJECT_ROOT}/.build-tools/linux-gnu"
LOCAL_TOOLS_ROOT="${BUILD_TOOLS_ROOT}/local-tools"
DOWNLOADS_DIR="${LOCAL_TOOLS_ROOT}/downloads"
LOCAL_CARGO_HOME="${LOCAL_TOOLS_ROOT}/cargo"
LOCAL_RUSTUP_HOME="${LOCAL_TOOLS_ROOT}/rustup"
LINUXDEPLOY_ROOT="${LOCAL_TOOLS_ROOT}/linuxdeploy"
LINUXDEPLOY_BIN="${LINUXDEPLOY_ROOT}/linuxdeploy-x86_64.AppImage"
BUILD_ROOT="${BUILD_TOOLS_ROOT}/build"
CARGO_TARGET_DIR="${BUILD_ROOT}/target"
RELEASE_DIR="${PROJECT_ROOT}/release/linux"
APPDIR_PATH="${BUILD_ROOT}/AppDir"
APPIMAGE_RESOURCES_BUILD_DIR="${BUILD_ROOT}/appimage-resources"
ARTIFACT_PATH="${CARGO_TARGET_DIR}/${TARGET_TRIPLE}/release/${ARTIFACT_NAME}"
RELEASE_APPIMAGE_PATH="${RELEASE_DIR}/${APPIMAGE_NAME}"
APPIMAGE_RESOURCES_DIR="${SCRIPT_DIR}/appimage"
DESKTOP_FILE_PATH="${APPIMAGE_RESOURCES_DIR}/${APPIMAGE_ID}.desktop"
ICON_SOURCE_PATH="${PROJECT_ROOT}/crates/app/assets/lanscanner.ico"
ICON_FILE_PATH="${APPIMAGE_RESOURCES_BUILD_DIR}/${APPIMAGE_ID}.png"
BUILD_CARGO_HOME="${CARGO_HOME:-${LOCAL_CARGO_HOME}}"

CLEAN=0
CARGO_BIN=""

write_step() {
    printf '[LANScanner] %s\n' "$1"
}

write_warn() {
    printf '[LANScanner][warn] %s\n' "$1" >&2
}

fail() {
    printf '[LANScanner][error] %s\n' "$1" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Usage: ./linux.sh [--clean] [--help]

Options:
  --clean  Remove the Linux build cache and release directory before building.
  --help   Show this help text.
EOF
}

ensure_dir() {
    mkdir -p "$1"
}

require_file() {
    local path="$1"
    local description="$2"

    if [[ ! -f "${path}" ]]; then
        fail "Missing ${description}: ${path}"
    fi
}

download_file() {
    local url="$1"
    local destination="$2"

    if command -v curl >/dev/null 2>&1; then
        curl --fail --location --silent --show-error "$url" -o "$destination"
        return 0
    fi

    if command -v wget >/dev/null 2>&1; then
        wget -qO "$destination" "$url"
        return 0
    fi

    fail "Neither curl nor wget is available. Install one of them so the script can bootstrap Linux build tooling such as rustup or linuxdeploy."
}

bootstrap_local_rust_toolchain() {
    local installer_path="${DOWNLOADS_DIR}/rustup-init.sh"

    ensure_dir "${DOWNLOADS_DIR}"
    ensure_dir "${LOCAL_CARGO_HOME}"
    ensure_dir "${LOCAL_RUSTUP_HOME}"

    if [[ ! -f "${installer_path}" ]]; then
        write_step "Downloading rustup-init.sh into the project-local tools cache"
        download_file "https://sh.rustup.rs" "${installer_path}"
    fi

    write_step "Installing a project-local Rust toolchain"
    chmod +x "${installer_path}"
    env \
        CARGO_HOME="${LOCAL_CARGO_HOME}" \
        RUSTUP_HOME="${LOCAL_RUSTUP_HOME}" \
        sh "${installer_path}" -y --no-modify-path --profile minimal --default-toolchain stable
}

resolve_cargo() {
    local local_cargo="${LOCAL_CARGO_HOME}/bin/cargo"

    if [[ -x "${local_cargo}" ]]; then
        write_step "Using project-local cargo from ${local_cargo}"
        CARGO_BIN="${local_cargo}"
        return 0
    fi

    if command -v cargo >/dev/null 2>&1; then
        local host_cargo
        host_cargo="$(command -v cargo)"
        write_step "Using host cargo from ${host_cargo}; build outputs stay under .build-tools/linux-gnu/"
        CARGO_BIN="${host_cargo}"
        return 0
    fi

    bootstrap_local_rust_toolchain

    if [[ -x "${local_cargo}" ]]; then
        write_step "Using newly bootstrapped project-local cargo from ${local_cargo}"
        CARGO_BIN="${local_cargo}"
        return 0
    fi

    fail "cargo is still unavailable after bootstrap."
}

run_checked() {
    local cargo_bin="$1"
    shift
    "${cargo_bin}" "$@"
}

extract_appimage_icon() {
    if ! command -v python3 >/dev/null 2>&1; then
        fail "python3 is required to extract the AppImage icon from ${ICON_SOURCE_PATH}"
    fi

    ensure_dir "${APPIMAGE_RESOURCES_BUILD_DIR}"

    python3 - "${ICON_SOURCE_PATH}" "${ICON_FILE_PATH}" <<'PY'
import struct
import sys
from pathlib import Path

source = Path(sys.argv[1])
destination = Path(sys.argv[2])
data = source.read_bytes()

if len(data) < 6:
    raise SystemExit(f"ICO file is too small: {source}")

reserved, kind, count = struct.unpack_from("<HHH", data, 0)
if reserved != 0 or kind != 1 or count <= 0:
    raise SystemExit(f"Unsupported ICO header in {source}")

best = None
for index in range(count):
    offset = 6 + index * 16
    width, height, _, _, _, bpp, size, image_offset = struct.unpack_from("<BBBBHHII", data, offset)
    width = 256 if width == 0 else width
    height = 256 if height == 0 else height
    payload = data[image_offset:image_offset + size]
    if payload.startswith(b"\x89PNG\r\n\x1a\n"):
        score = width * height
        if best is None or score > best[0]:
            best = (score, payload)

if best is None:
    raise SystemExit(f"No embedded PNG image found in {source}")

destination.write_bytes(best[1])
PY
}

resolve_linuxdeploy() {
    if [[ -x "${LINUXDEPLOY_BIN}" ]]; then
        write_step "Using cached linuxdeploy from ${LINUXDEPLOY_BIN}"
        return 0
    fi

    ensure_dir "${LINUXDEPLOY_ROOT}"

    write_step "Downloading linuxdeploy AppImage into the project-local tools cache"
    download_file \
        "https://github.com/linuxdeploy/linuxdeploy/releases/download/${LINUXDEPLOY_CHANNEL}/linuxdeploy-x86_64.AppImage" \
        "${LINUXDEPLOY_BIN}"
    chmod +x "${LINUXDEPLOY_BIN}"
}

run_linuxdeploy() {
    env APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 "${LINUXDEPLOY_BIN}" "$@"
}

normalize_generated_appimage() {
    shopt -s nullglob
    local appimages=("${RELEASE_DIR}"/*.AppImage)
    shopt -u nullglob

    if [[ "${#appimages[@]}" -ne 1 ]]; then
        fail "Expected exactly one generated AppImage in ${RELEASE_DIR}, found ${#appimages[@]}"
    fi

    if [[ "${appimages[0]}" != "${RELEASE_APPIMAGE_PATH}" ]]; then
        mv -f "${appimages[0]}" "${RELEASE_APPIMAGE_PATH}"
    fi

    chmod +x "${RELEASE_APPIMAGE_PATH}"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --clean)
            CLEAN=1
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            fail "Unknown argument: $1"
            ;;
    esac
    shift
done

if [[ "$(uname -s)" != "Linux" ]]; then
    fail "tools/build/linux.sh must be run on Linux."
fi

if [[ "$(uname -m)" != "x86_64" ]]; then
    fail "tools/build/linux.sh currently supports only x86_64 Linux hosts."
fi

if ! command -v cc >/dev/null 2>&1; then
    fail "A Linux C compiler is required. Install gcc or clang and make sure 'cc' is on PATH."
fi

if ! command -v pkg-config >/dev/null 2>&1; then
    write_warn "pkg-config is not installed. Native dependencies may fail to compile."
fi

ensure_dir "${BUILD_TOOLS_ROOT}"
ensure_dir "${DOWNLOADS_DIR}"
ensure_dir "${BUILD_CARGO_HOME}"
ensure_dir "${BUILD_ROOT}"
ensure_dir "${RELEASE_DIR}"
require_file "${DESKTOP_FILE_PATH}" "AppImage desktop file"
require_file "${ICON_SOURCE_PATH}" "Windows ICO app icon"

if [[ "${CLEAN}" -eq 1 ]]; then
    write_step "Cleaning project-local Linux build and release directories"
    rm -rf "${BUILD_ROOT}" "${RELEASE_DIR}"
    ensure_dir "${BUILD_ROOT}"
    ensure_dir "${RELEASE_DIR}"
fi

export CARGO_TARGET_DIR
export CARGO_HOME="${BUILD_CARGO_HOME}"
write_step "CARGO_TARGET_DIR=${CARGO_TARGET_DIR}"
write_step "CARGO_HOME=${CARGO_HOME}"

resolve_cargo
resolve_linuxdeploy

write_step "Building ${ARTIFACT_NAME} for ${TARGET_TRIPLE}"
run_checked "${CARGO_BIN}" build \
    --manifest-path "${PROJECT_ROOT}/Cargo.toml" \
    --locked \
    --release \
    --target "${TARGET_TRIPLE}" \
    -p lanscanner-app

if [[ ! -f "${ARTIFACT_PATH}" ]]; then
    fail "Expected Linux artifact not found: ${ARTIFACT_PATH}"
fi

write_step "Refreshing release/linux staging area"
rm -rf "${RELEASE_DIR}"
ensure_dir "${RELEASE_DIR}"
rm -rf "${APPDIR_PATH}"
ensure_dir "${APPDIR_PATH}"
rm -rf "${APPIMAGE_RESOURCES_BUILD_DIR}"

write_step "Extracting AppImage icon from ${ICON_SOURCE_PATH}"
extract_appimage_icon

write_step "Packaging ${APPIMAGE_NAME} with linuxdeploy"
(
    cd "${RELEASE_DIR}"
    run_linuxdeploy \
        --appdir "${APPDIR_PATH}" \
        --executable "${ARTIFACT_PATH}" \
        --desktop-file "${DESKTOP_FILE_PATH}" \
        --icon-file "${ICON_FILE_PATH}" \
        --output appimage
)

normalize_generated_appimage

write_step "Build completed"
printf 'AppImage: %s\n' "${RELEASE_APPIMAGE_PATH}"
printf 'Note: run Linux artifacts inside Linux/WSL2, not as native Windows executables.\n'
