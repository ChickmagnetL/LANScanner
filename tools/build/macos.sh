#!/usr/bin/env bash

set -euo pipefail

ARTIFACT_NAME="LANScanner"
APP_BUNDLE_NAME="LANScanner.app"
BUNDLE_IDENTIFIER="com.lanscanner.desktop"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

BUILD_TOOLS_ROOT="${PROJECT_ROOT}/.build-tools/macos"
LOCAL_TOOLS_ROOT="${BUILD_TOOLS_ROOT}/local-tools"
DOWNLOADS_DIR="${LOCAL_TOOLS_ROOT}/downloads"
LOCAL_CARGO_HOME="${LOCAL_TOOLS_ROOT}/cargo"
LOCAL_RUSTUP_HOME="${LOCAL_TOOLS_ROOT}/rustup"
BUILD_ROOT="${BUILD_TOOLS_ROOT}/build"
CARGO_TARGET_DIR="${BUILD_ROOT}/target"
RELEASE_DIR="${PROJECT_ROOT}/release/macos"
APP_BUNDLE_PATH="${RELEASE_DIR}/${APP_BUNDLE_NAME}"
CONTENTS_DIR="${APP_BUNDLE_PATH}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"
MACOS_RESOURCES_BUILD_DIR="${BUILD_ROOT}/macos-resources"
ICON_SOURCE_PATH="${PROJECT_ROOT}/crates/app/assets/lanscanner.ico"
BASE_ICON_PNG_PATH="${MACOS_RESOURCES_BUILD_DIR}/LANScanner.png"
ICONSET_DIR="${MACOS_RESOURCES_BUILD_DIR}/LANScanner.iconset"
APP_ICNS_PATH="${RESOURCES_DIR}/LANScanner.icns"
PLIST_PATH="${CONTENTS_DIR}/Info.plist"
BUILD_CARGO_HOME="${CARGO_HOME:-${LOCAL_CARGO_HOME}}"

CLEAN=0
CARGO_BIN=""
TARGET_TRIPLE=""
ARTIFACT_PATH=""
RUSTUP_ENV_MODE="host"

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
Usage: ./macos.sh [--clean] [--help]

Options:
  --clean  Remove the macOS build cache and release directory before building.
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

    fail "Neither curl nor wget is available. Install one of them so the script can bootstrap macOS build tooling when cargo is missing."
}

determine_target_triple() {
    case "$(uname -m)" in
        x86_64)
            TARGET_TRIPLE="x86_64-apple-darwin"
            ;;
        arm64|aarch64)
            TARGET_TRIPLE="aarch64-apple-darwin"
            ;;
        *)
            fail "tools/build/macos.sh currently supports only x86_64 and arm64 macOS hosts."
            ;;
    esac

    ARTIFACT_PATH="${CARGO_TARGET_DIR}/${TARGET_TRIPLE}/release/${ARTIFACT_NAME}"
}

bootstrap_local_rust_toolchain() {
    local installer_path="${DOWNLOADS_DIR}/rustup-init.sh"
    local rustup_bin="${LOCAL_CARGO_HOME}/bin/rustup"

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

    if [[ -x "${rustup_bin}" ]]; then
        env \
            CARGO_HOME="${LOCAL_CARGO_HOME}" \
            RUSTUP_HOME="${LOCAL_RUSTUP_HOME}" \
            "${rustup_bin}" target add "${TARGET_TRIPLE}" >/dev/null
    fi
}

resolve_cargo() {
    local local_cargo="${LOCAL_CARGO_HOME}/bin/cargo"

    if [[ -x "${local_cargo}" ]]; then
        write_step "Using project-local cargo from ${local_cargo}"
        CARGO_BIN="${local_cargo}"
        RUSTUP_ENV_MODE="local"
        return 0
    fi

    if command -v cargo >/dev/null 2>&1; then
        local host_cargo
        host_cargo="$(command -v cargo)"
        write_step "Using host cargo from ${host_cargo}; build outputs stay under .build-tools/macos/"
        CARGO_BIN="${host_cargo}"
        RUSTUP_ENV_MODE="host"
        return 0
    fi

    bootstrap_local_rust_toolchain

    if [[ -x "${local_cargo}" ]]; then
        write_step "Using newly bootstrapped project-local cargo from ${local_cargo}"
        CARGO_BIN="${local_cargo}"
        RUSTUP_ENV_MODE="local"
        return 0
    fi

    fail "cargo is still unavailable after bootstrap."
}

run_checked() {
    local cargo_bin="$1"
    shift
    "${cargo_bin}" "$@"
}

read_workspace_version() {
    local version
    version="$(
        awk '
            $0 ~ /^\[workspace\.package\]/ { in_section = 1; next }
            in_section && $0 ~ /^\[/ { in_section = 0 }
            in_section && $1 == "version" {
                gsub(/"/, "", $3)
                print $3
                exit
            }
        ' "${PROJECT_ROOT}/Cargo.toml"
    )"

    if [[ -z "${version}" ]]; then
        fail "Unable to read workspace version from ${PROJECT_ROOT}/Cargo.toml"
    fi

    printf '%s\n' "${version}"
}

extract_base_icon_png() {
    if ! command -v python3 >/dev/null 2>&1; then
        fail "python3 is required to extract the macOS app icon from ${ICON_SOURCE_PATH}"
    fi

    ensure_dir "${MACOS_RESOURCES_BUILD_DIR}"

    python3 - "${ICON_SOURCE_PATH}" "${BASE_ICON_PNG_PATH}" <<'PY'
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
    width, height, _, _, _, _, size, image_offset = struct.unpack_from("<BBBBHHII", data, offset)
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

create_icon_png() {
    local size="$1"
    local filename="$2"

    sips -s format png -z "${size}" "${size}" "${BASE_ICON_PNG_PATH}" --out "${ICONSET_DIR}/${filename}" >/dev/null
}

build_icns() {
    write_step "Extracting the bundled ICO icon for macOS packaging"
    extract_base_icon_png

    rm -rf "${ICONSET_DIR}"
    ensure_dir "${ICONSET_DIR}"

    write_step "Generating macOS iconset variants"
    create_icon_png 16 "icon_16x16.png"
    create_icon_png 32 "icon_16x16@2x.png"
    create_icon_png 32 "icon_32x32.png"
    create_icon_png 64 "icon_32x32@2x.png"
    create_icon_png 128 "icon_128x128.png"
    create_icon_png 256 "icon_128x128@2x.png"
    create_icon_png 256 "icon_256x256.png"
    create_icon_png 512 "icon_256x256@2x.png"
    create_icon_png 512 "icon_512x512.png"
    create_icon_png 1024 "icon_512x512@2x.png"

    write_step "Building ${APP_ICNS_PATH}"
    iconutil -c icns "${ICONSET_DIR}" -o "${APP_ICNS_PATH}"
}

write_info_plist() {
    local app_version="$1"

    cat > "${PLIST_PATH}" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>${ARTIFACT_NAME}</string>
    <key>CFBundleIconFile</key>
    <string>${ARTIFACT_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_IDENTIFIER}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${ARTIFACT_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${app_version}</string>
    <key>CFBundleVersion</key>
    <string>${app_version}</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

    plutil -lint "${PLIST_PATH}" >/dev/null
}

package_app_bundle() {
    local app_version="$1"

    write_step "Refreshing release/macos with ${APP_BUNDLE_NAME}"
    rm -rf "${RELEASE_DIR}"
    ensure_dir "${MACOS_DIR}"
    ensure_dir "${RESOURCES_DIR}"

    cp "${ARTIFACT_PATH}" "${MACOS_DIR}/${ARTIFACT_NAME}"
    chmod +x "${MACOS_DIR}/${ARTIFACT_NAME}"

    rm -rf "${MACOS_RESOURCES_BUILD_DIR}"
    build_icns
    write_info_plist "${app_version}"
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

if [[ "$(uname -s)" != "Darwin" ]]; then
    fail "tools/build/macos.sh must be run on macOS."
fi

if ! command -v xcrun >/dev/null 2>&1; then
    fail "xcrun is required. Install Xcode Command Line Tools and rerun the script."
fi

if ! command -v cc >/dev/null 2>&1; then
    fail "A macOS C compiler is required. Install Xcode Command Line Tools and make sure 'cc' is on PATH."
fi

for tool in sips iconutil plutil; do
    if ! command -v "${tool}" >/dev/null 2>&1; then
        fail "${tool} is required for macOS app packaging."
    fi
done

determine_target_triple

ensure_dir "${BUILD_TOOLS_ROOT}"
ensure_dir "${DOWNLOADS_DIR}"
ensure_dir "${BUILD_CARGO_HOME}"
ensure_dir "${BUILD_ROOT}"
ensure_dir "${RELEASE_DIR}"
require_file "${ICON_SOURCE_PATH}" "Windows ICO app icon"

if [[ "${CLEAN}" -eq 1 ]]; then
    write_step "Cleaning project-local macOS build and release directories"
    rm -rf "${BUILD_ROOT}" "${RELEASE_DIR}"
    ensure_dir "${BUILD_ROOT}"
    ensure_dir "${RELEASE_DIR}"
fi

export CARGO_TARGET_DIR
write_step "CARGO_TARGET_DIR=${CARGO_TARGET_DIR}"

resolve_cargo

if [[ "${RUSTUP_ENV_MODE}" == "local" ]]; then
    export CARGO_HOME="${LOCAL_CARGO_HOME}"
    export RUSTUP_HOME="${LOCAL_RUSTUP_HOME}"
    write_step "CARGO_HOME=${CARGO_HOME}"
    write_step "RUSTUP_HOME=${RUSTUP_HOME}"
else
    export CARGO_HOME="${BUILD_CARGO_HOME}"
    write_step "CARGO_HOME=${CARGO_HOME}"
fi

APP_VERSION="$(read_workspace_version)"

write_step "Building ${ARTIFACT_NAME} for ${TARGET_TRIPLE}"
run_checked "${CARGO_BIN}" build \
    --manifest-path "${PROJECT_ROOT}/Cargo.toml" \
    --locked \
    --release \
    --target "${TARGET_TRIPLE}" \
    -p lanscanner-app

if [[ ! -f "${ARTIFACT_PATH}" ]]; then
    fail "Expected macOS artifact not found: ${ARTIFACT_PATH}"
fi

package_app_bundle "${APP_VERSION}"

write_step "Build completed"
printf 'App bundle: %s\n' "${APP_BUNDLE_PATH}"
printf 'Note: the generated macOS app bundle is unsigned and not notarized.\n'
