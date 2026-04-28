#!/usr/bin/env bash

set -euo pipefail

ARTIFACT_NAME="LANScanner"
APP_BUNDLE_NAME="LANScanner.app"
DMG_NAME="LANScanner-macos.dmg"
APPLICATIONS_LINK_NAME="Applications"
BUNDLE_IDENTIFIER="com.lanscanner.desktop"
DMG_VOLUME_NAME="LANScanner"
DMG_WINDOW_WIDTH=540
DMG_WINDOW_HEIGHT=320
DMG_ICON_SIZE=88
DMG_ICON_GAP=136
DMG_TITLE_FONT_SIZE=24
DMG_ICON_LABEL_ALLOWANCE=30
DMG_HELPER_SHELF_OFFSET=56
DMG_HELPER_SHELF_Y=$((DMG_WINDOW_HEIGHT + DMG_HELPER_SHELF_OFFSET))
DMG_HELPER_SHELF_START_X=48
DMG_HELPER_SHELF_STEP_X=96
DMG_ICON_GROUP_WIDTH=$(((DMG_ICON_SIZE * 2) + DMG_ICON_GAP))
DMG_LAYOUT_CENTER_X=$((DMG_WINDOW_WIDTH / 2))
DMG_APP_X=$(((DMG_WINDOW_WIDTH - DMG_ICON_GROUP_WIDTH) / 2))
DMG_APPLICATIONS_X=$((DMG_APP_X + DMG_ICON_SIZE + DMG_ICON_GAP))
# Center the visible install row first; arrow/text geometry is derived from it.
DMG_ITEM_Y=$(((DMG_WINDOW_HEIGHT - DMG_ICON_SIZE - DMG_ICON_LABEL_ALLOWANCE) / 2))
DMG_BACKGROUND_SCALE=2
DMG_BACKGROUND_NAME="LANScannerDMGBackground.png"
DMG_BACKGROUND_DIR_NAME=".background"
DMG_BACKGROUND_HFS_PATH="${DMG_BACKGROUND_DIR_NAME}:${DMG_BACKGROUND_NAME}"

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
APP_BUNDLE_PATH="${BUILD_ROOT}/${APP_BUNDLE_NAME}"
CONTENTS_DIR="${APP_BUNDLE_PATH}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"
MACOS_RESOURCES_BUILD_DIR="${BUILD_ROOT}/macos-resources"
DMG_STAGING_DIR="${BUILD_ROOT}/dmg-root"
DMG_RW_PATH="${BUILD_ROOT}/LANScanner-temp.dmg"
DMG_LAYOUT_MOUNT_POINT="/Volumes/${DMG_VOLUME_NAME}"
DMG_VERIFY_MOUNT_POINT="/Volumes/${DMG_VOLUME_NAME}"
DMG_LAYOUT_PERSISTENCE_MARKER_PATH="${BUILD_ROOT}/dmg-layout-persisted.marker"
RELEASE_DMG_PATH="${RELEASE_DIR}/${DMG_NAME}"
DMG_VOLUME_ICON_NAME=".VolumeIcon.icns"
DMG_VOLUME_ICON_PATH="${DMG_STAGING_DIR}/${DMG_VOLUME_ICON_NAME}"
DMG_BACKGROUND_STAGING_DIR="${DMG_STAGING_DIR}/${DMG_BACKGROUND_DIR_NAME}"
DMG_STAGED_BACKGROUND_PATH="${DMG_BACKGROUND_STAGING_DIR}/${DMG_BACKGROUND_NAME}"
ICON_SVG_SOURCE_PATH="${PROJECT_ROOT}/crates/app/assets/lanscanner_mac.svg"
ICON_ICO_SOURCE_PATH="${PROJECT_ROOT}/crates/app/assets/lanscanner.ico"
BASE_ICON_PNG_PATH="${MACOS_RESOURCES_BUILD_DIR}/LANScanner.png"
ICONSET_DIR="${MACOS_RESOURCES_BUILD_DIR}/LANScanner.iconset"
APP_ICNS_PATH="${RESOURCES_DIR}/LANScanner.icns"
DMG_BACKGROUND_PATH="${MACOS_RESOURCES_BUILD_DIR}/${DMG_BACKGROUND_NAME}"
DMG_BACKGROUND_RELATIVE_PATH="${DMG_BACKGROUND_DIR_NAME}/${DMG_BACKGROUND_NAME}"
DMG_MOUNTED_BACKGROUND_PATH="${DMG_LAYOUT_MOUNT_POINT}/${DMG_BACKGROUND_RELATIVE_PATH}"
DMG_VERIFY_BACKGROUND_PATH="${DMG_VERIFY_MOUNT_POINT}/${DMG_BACKGROUND_RELATIVE_PATH}"
DMG_DS_STORE_TEMP_NAME="$(basename "${DMG_RW_PATH}")"
PLIST_PATH="${CONTENTS_DIR}/Info.plist"
BUILD_CARGO_HOME="${CARGO_HOME:-${LOCAL_CARGO_HOME}}"

CLEAN=0
CARGO_BIN=""
TARGET_TRIPLE=""
ARTIFACT_PATH=""
RUSTUP_ENV_MODE="host"
SETFILE_BIN=""
DMG_VERIFY_DEVICE=""

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

cleanup_dmg_mount() {
    if [[ -n "${DMG_DEVICE:-}" ]]; then
        hdiutil detach "${DMG_DEVICE}" -quiet >/dev/null 2>&1 || true
        DMG_DEVICE=""
    fi
}

cleanup_verify_dmg_mount() {
    if [[ -n "${DMG_VERIFY_DEVICE:-}" ]]; then
        hdiutil detach "${DMG_VERIFY_DEVICE}" -quiet >/dev/null 2>&1 || true
        DMG_VERIFY_DEVICE=""
    fi
}

detach_stale_image_mounts() {
    local image_path="$1"
    local device

    if [[ -z "${image_path}" || ! -f "${image_path}" ]]; then
        return 0
    fi

    while IFS= read -r device; do
        if [[ -n "${device}" ]]; then
            hdiutil detach "${device}" -quiet >/dev/null 2>&1 || true
        fi
    done < <(
        hdiutil info | awk -v target="${image_path}" '
            /^image-path[[:space:]]*:/ {
                current = substr($0, index($0, ":") + 2)
                next
            }
            $1 ~ /^\/dev\/disk[0-9]+$/ && current == target {
                print $1
            }
        ' | sort -u
    )
}

detach_stale_dmg_mounts() {
    detach_stale_image_mounts "${DMG_RW_PATH}"
    detach_stale_image_mounts "${RELEASE_DMG_PATH}"
}

usage() {
    cat <<'EOF'
Usage: ./macos.sh [--clean] [--help]

Builds release/macos/LANScanner-macos.dmg as a standard drag-to-Applications
disk image with LANScanner.app on the left, an Applications shortcut on the
right, and a light Finder background with an arrow and install hint.

Options:
  --clean  Remove the macOS build cache and release directory before building.
  --help   Show this help text.

Requirements:
  macOS with Xcode Command Line Tools, python3, cc, xcrun, sips, iconutil,
  plutil, hdiutil, osascript, and Finder automation permission for the terminal
  running this script. Keep curl or wget available when host cargo is missing
  so the script can bootstrap a project-local Rust toolchain. SetFile is used
  for best-effort volume icon branding when available.
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

run_osascript() {
    osascript "$@" >/dev/null
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

create_icon_png() {
    local size="$1"
    local filename="$2"

    sips -s format png -z "${size}" "${size}" "${BASE_ICON_PNG_PATH}" --out "${ICONSET_DIR}/${filename}" >/dev/null
}

extract_base_icon_png_from_ico() {
    require_file "${ICON_ICO_SOURCE_PATH}" "fallback macOS ICO icon"
    ensure_dir "${MACOS_RESOURCES_BUILD_DIR}"

    python3 - "${ICON_ICO_SOURCE_PATH}" "${BASE_ICON_PNG_PATH}" <<'PY'
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

build_icns() {
    write_step "Generating the base macOS app icon from SVG"
    
    rm -rf "${ICONSET_DIR}"
    ensure_dir "${ICONSET_DIR}"
    ensure_dir "${MACOS_RESOURCES_BUILD_DIR}"
    
    if ! sips -s format png "${ICON_SVG_SOURCE_PATH}" --out "${BASE_ICON_PNG_PATH}" >/dev/null; then
        write_warn "Unable to rasterize ${ICON_SVG_SOURCE_PATH} with sips; falling back to the embedded PNG inside ${ICON_ICO_SOURCE_PATH}"
        extract_base_icon_png_from_ico
    fi

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
    if ! iconutil -c icns "${ICONSET_DIR}" -o "${APP_ICNS_PATH}"; then
        write_warn "iconutil rejected the generated iconset; retrying with a direct sips -> icns conversion"
        rm -f "${APP_ICNS_PATH}"

        if ! sips -s format icns "${BASE_ICON_PNG_PATH}" --out "${APP_ICNS_PATH}" >/dev/null; then
            rm -f "${APP_ICNS_PATH}"
            fail "Unable to generate the macOS packaging icon from ${BASE_ICON_PNG_PATH}"
        fi
    fi
}


create_dmg_background() {
    local destination_path="${1:-${DMG_BACKGROUND_PATH}}"
    local window_width="${2:-${DMG_WINDOW_WIDTH}}"
    local window_height="${3:-${DMG_WINDOW_HEIGHT}}"
    local app_x="${4:-${DMG_APP_X}}"
    local applications_x="${5:-${DMG_APPLICATIONS_X}}"
    local item_y="${6:-${DMG_ITEM_Y}}"
    local icon_size="${7:-${DMG_ICON_SIZE}}"
    local layout_center_x="${8:-${DMG_LAYOUT_CENTER_X}}"
    local title_font_size="${9:-${DMG_TITLE_FONT_SIZE}}"
    local background_scale="${10:-${DMG_BACKGROUND_SCALE}}"

    write_step "Rendering the DMG drag-to-Applications background"

    if ! osascript -l JavaScript - \
        "${destination_path}" \
        "${window_width}" \
        "${window_height}" \
        "${app_x}" \
        "${applications_x}" \
        "${item_y}" \
        "${icon_size}" \
        "${layout_center_x}" \
        "${title_font_size}" \
        "${background_scale}" <<'JXA' >/dev/null; then
ObjC.import('AppKit')
ObjC.import('Foundation')

function nsColor(red, green, blue, alpha) {
    return $.NSColor.colorWithCalibratedRedGreenBlueAlpha(red / 255, green / 255, blue / 255, alpha)
}

function topOriginToCanvasY(height, topY) {
    return height - topY
}

function drawCenteredText(text, centerX, topY, fontSize, red, green, blue, canvasHeight, scale) {
    const attributes = $({
        NSFont: $.NSFont.systemFontOfSize(fontSize * scale),
        NSForegroundColor: nsColor(red, green, blue, 1),
    })
    const string = $(text)
    const size = string.sizeWithAttributes(attributes)
    const y = topOriginToCanvasY(canvasHeight, topY * scale) - size.height
    string.drawAtPointWithAttributes($.NSMakePoint((centerX * scale) - (size.width / 2), y), attributes)
}

function run(argv) {
    const destination = argv[0]
    const width = Number(argv[1])
    const height = Number(argv[2])
    const appX = Number(argv[3])
    const applicationsX = Number(argv[4])
    const itemY = Number(argv[5])
    const iconSize = Number(argv[6])
    const layoutCenterX = Number(argv[7])
    const titleFontSize = Number(argv[8]) || 24
    const scale = Math.max(1, Number(argv[9]) || 1)
    const pixelWidth = Math.round(width * scale)
    const pixelHeight = Math.round(height * scale)

    const image = $.NSImage.alloc.initWithSize($.NSMakeSize(pixelWidth, pixelHeight))
    image.lockFocus

    nsColor(255, 255, 255, 1).setFill
    $.NSBezierPath.fillRect($.NSMakeRect(0, 0, pixelWidth, pixelHeight))

    nsColor(20, 116, 150, 1).setStroke
    nsColor(20, 116, 150, 1).setFill

    const iconRadius = Math.round(iconSize / 2)
    const appCenterX = appX + iconRadius
    const applicationsCenterX = applicationsX + iconRadius
    const iconCenterY = itemY + iconRadius
    const titleBandHeight = Math.max(titleFontSize + 8, Math.round(iconSize * 0.36))
    const titleToIconsGap = Math.max(18, Math.round(iconSize * 0.18))
    const textTopY = Math.max(24, itemY - titleBandHeight - titleToIconsGap)
    const arrowCenterY = iconCenterY
    const arrowInset = Math.max(16, Math.round(iconSize * 0.18))
    const arrowTipX = applicationsCenterX - iconRadius - arrowInset
    const arrowStartX = appCenterX + iconRadius + arrowInset
    const arrowLength = arrowTipX - arrowStartX
    if (arrowLength < 48) {
        throw new Error('DMG icon positions leave too little room for the install arrow')
    }

    const arrowY = topOriginToCanvasY(height, arrowCenterY) * scale
    const arrowHeadLength = Math.min(24, Math.max(18, Math.round(arrowLength * 0.28)))
    const arrowHeadHeight = Math.max(14, Math.round(iconSize * 0.18))
    const arrowShaftEndX = arrowTipX - arrowHeadLength
    const arrow = $.NSBezierPath.bezierPath
    arrow.setLineWidth(6 * scale)
    arrow.moveToPoint($.NSMakePoint(arrowStartX * scale, arrowY))
    arrow.lineToPoint($.NSMakePoint(arrowShaftEndX * scale, arrowY))
    arrow.stroke

    const head = $.NSBezierPath.bezierPath
    head.moveToPoint($.NSMakePoint(arrowTipX * scale, arrowY))
    head.lineToPoint($.NSMakePoint(arrowShaftEndX * scale, arrowY + (arrowHeadHeight * scale)))
    head.lineToPoint($.NSMakePoint(arrowShaftEndX * scale, arrowY - (arrowHeadHeight * scale)))
    head.closePath
    head.fill

    drawCenteredText('Drag to Applications', layoutCenterX, textTopY, titleFontSize, 35, 43, 52, pixelHeight, scale)

    image.unlockFocus

    const bitmap = $.NSBitmapImageRep.imageRepWithData(image.TIFFRepresentation)
    bitmap.setSize($.NSMakeSize(width, height))
    const png = bitmap.representationUsingTypeProperties($.NSBitmapImageFileTypePNG, $())
    if (!png.writeToFileAtomically(destination, true)) {
        throw new Error('Unable to write DMG background to ' + destination)
    }
}
JXA
        fail "Unable to render the DMG background at ${destination_path}"
    fi

    require_file "${destination_path}" "DMG background image"
}

write_info_plist() {
    local app_version="$1"
    local icon_plist_block=""

    if [[ -f "${APP_ICNS_PATH}" ]]; then
        icon_plist_block=$(cat <<EOF
    <key>CFBundleIconFile</key>
    <string>${ARTIFACT_NAME}</string>
EOF
)
    fi

    cat > "${PLIST_PATH}" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>${ARTIFACT_NAME}</string>
${icon_plist_block}
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

assemble_app_bundle() {
    local app_version="$1"

    write_step "Assembling temporary ${APP_BUNDLE_NAME}"
    rm -rf "${APP_BUNDLE_PATH}"
    ensure_dir "${MACOS_DIR}"
    ensure_dir "${RESOURCES_DIR}"

    cp "${ARTIFACT_PATH}" "${MACOS_DIR}/${ARTIFACT_NAME}"
    chmod +x "${MACOS_DIR}/${ARTIFACT_NAME}"

    rm -rf "${MACOS_RESOURCES_BUILD_DIR}"
    build_icns
    create_dmg_background
    write_info_plist "${app_version}"
}

stage_dmg_volume_icon() {
    if [[ -f "${APP_ICNS_PATH}" ]]; then
        cp "${APP_ICNS_PATH}" "${DMG_VOLUME_ICON_PATH}"
        chflags hidden "${DMG_VOLUME_ICON_PATH}" >/dev/null 2>&1 || true
    fi
}

stage_dmg_background_asset() {
    require_file "${DMG_BACKGROUND_PATH}" "generated DMG background image"
    ensure_dir "${DMG_BACKGROUND_STAGING_DIR}"
    cp "${DMG_BACKGROUND_PATH}" "${DMG_STAGED_BACKGROUND_PATH}"
}

assert_dmg_staging_root_allowed() {
    local entry
    local name

    while IFS= read -r entry; do
        name="$(basename "${entry}")"
        case "${name}" in
            "${APP_BUNDLE_NAME}"|"${APPLICATIONS_LINK_NAME}"|"${DMG_VOLUME_ICON_NAME}"|"${DMG_BACKGROUND_DIR_NAME}")
                ;;
            *)
                fail "Unexpected DMG root entry: ${name}. The DMG root may contain only ${APP_BUNDLE_NAME}, ${APPLICATIONS_LINK_NAME}, ${DMG_VOLUME_ICON_NAME}, and ${DMG_BACKGROUND_DIR_NAME}."
                ;;
        esac
    done < <(find "${DMG_STAGING_DIR}" -mindepth 1 -maxdepth 1 -print)
}

resolve_setfile() {
    if [[ -n "${SETFILE_BIN}" ]]; then
        return 0
    fi

    if command -v SetFile >/dev/null 2>&1; then
        SETFILE_BIN="$(command -v SetFile)"
        return 0
    fi

    if xcrun --find SetFile >/dev/null 2>&1; then
        SETFILE_BIN="$(xcrun --find SetFile)"
        return 0
    fi

    return 1
}

apply_volume_branding() {
    local mounted_volume_icon_path="${DMG_LAYOUT_MOUNT_POINT}/${DMG_VOLUME_ICON_NAME}"

    if [[ ! -f "${mounted_volume_icon_path}" ]]; then
        return 0
    fi

    chflags hidden "${mounted_volume_icon_path}" >/dev/null 2>&1 || true

    if ! resolve_setfile; then
        write_warn "SetFile is unavailable; skipping the custom DMG volume icon"
        return 0
    fi

    if ! "${SETFILE_BIN}" -a C "${DMG_LAYOUT_MOUNT_POINT}" >/dev/null 2>&1; then
        write_warn "Unable to mark the mounted DMG volume with a custom icon"
    fi

    if ! "${SETFILE_BIN}" -a V "${mounted_volume_icon_path}" >/dev/null 2>&1; then
        write_warn "Unable to hide the mounted DMG volume icon asset"
    fi
}

hide_mounted_metadata_path() {
    local path="$1"

    if [[ ! -e "${path}" ]]; then
        return 0
    fi

    chflags -R hidden "${path}" >/dev/null 2>&1 || true

    if resolve_setfile; then
        "${SETFILE_BIN}" -a V "${path}" >/dev/null 2>&1 || true
    fi
}

remove_mounted_metadata_path() {
    local path="$1"

    if [[ -e "${path}" ]]; then
        rm -rf "${path}" >/dev/null 2>&1 || true
    fi

    hide_mounted_metadata_path "${path}"
}

assert_mounted_dmg_root_allowed() {
    local root="$1"
    local entry
    local name

    while IFS= read -r entry; do
        name="$(basename "${entry}")"
        case "${name}" in
            "${APP_BUNDLE_NAME}"|"${APPLICATIONS_LINK_NAME}"|"${DMG_VOLUME_ICON_NAME}"|"${DMG_BACKGROUND_DIR_NAME}"|".DS_Store"|".Trashes"|".fseventsd")
                ;;
            *)
                fail "Unexpected mounted DMG root entry: ${name}. Only ${APP_BUNDLE_NAME}, ${APPLICATIONS_LINK_NAME}, ${DMG_VOLUME_ICON_NAME}, ${DMG_BACKGROUND_DIR_NAME}, .DS_Store, .Trashes, and .fseventsd are allowed."
                ;;
        esac
    done < <(find "${root}" -mindepth 1 -maxdepth 1 -print)
}

assert_dmg_layout_persisted() {
    local root="$1"
    local marker_path="$2"
    local ds_store_path="${root}/.DS_Store"

    if [[ ! -f "${ds_store_path}" ]]; then
        fail "Finder did not persist the DMG layout because ${ds_store_path} was not created."
    fi

    if [[ ! -s "${ds_store_path}" ]]; then
        fail "Finder created ${ds_store_path}, but it is empty; the DMG layout/background were not persisted."
    fi

    if [[ ! -f "${marker_path}" ]]; then
        fail "Missing DMG layout persistence marker: ${marker_path}"
    fi

    if [[ "${ds_store_path}" != "${marker_path}" && ! "${ds_store_path}" -nt "${marker_path}" ]]; then
        fail "Finder did not update ${ds_store_path} after the layout pass started, so the persisted DMG layout may be stale."
    fi
}

assert_dmg_background_binding_persisted() {
    local root="$1"
    local ds_store_path="${root}/.DS_Store"
    local ds_store_strings=""

    require_file "${ds_store_path}" "mounted DMG .DS_Store metadata"
    ds_store_strings="$(strings -a "${ds_store_path}" 2>/dev/null || true)"

    if [[ "${ds_store_strings}" != *"${DMG_BACKGROUND_DIR_NAME}"* || "${ds_store_strings}" != *"${DMG_BACKGROUND_NAME}"* ]]; then
        fail "The final DMG .DS_Store did not persist a reference to ${DMG_BACKGROUND_RELATIVE_PATH}."
    fi

    if [[ "${ds_store_strings}" == *"${DMG_DS_STORE_TEMP_NAME}"* ]]; then
        write_warn "The final DMG .DS_Store still mentions ${DMG_DS_STORE_TEMP_NAME}; Finder aliases can retain source-image breadcrumbs, so verification relies on the persisted in-volume background binding."
    fi

    if [[ "${ds_store_strings}" == *"${BUILD_ROOT}"* ]]; then
        write_warn "The final DMG .DS_Store still mentions the writable build root ${BUILD_ROOT}; Finder aliases can retain source-image breadcrumbs, so verification relies on the persisted in-volume background binding."
    fi
}

hide_mounted_dmg_metadata() {
    local root="$1"

    hide_mounted_metadata_path "${root}/.fseventsd"
    hide_mounted_metadata_path "${root}/.DS_Store"
    hide_mounted_metadata_path "${root}/.Trashes"
    hide_mounted_metadata_path "${root}/${DMG_VOLUME_ICON_NAME}"
}

disable_mounted_dmg_fsevents() {
    local root="$1"
    local fsevents_dir="${root}/.fseventsd"

    mkdir -p "${fsevents_dir}" >/dev/null 2>&1 || true
    touch "${fsevents_dir}/no_log" >/dev/null 2>&1 || true
    hide_mounted_metadata_path "${fsevents_dir}"
}

sanitize_mounted_dmg_metadata() {
    local root="$1"

    remove_mounted_metadata_path "${root}/.fseventsd"
    remove_mounted_metadata_path "${root}/.DS_Store"
    remove_mounted_metadata_path "${root}/.Trashes"
    hide_mounted_metadata_path "${root}/${DMG_VOLUME_ICON_NAME}"
}

keep_mounted_dmg_metadata_out_of_view() {
    run_osascript - \
        "${DMG_LAYOUT_MOUNT_POINT}" \
        "${DMG_HELPER_SHELF_START_X}" \
        "${DMG_HELPER_SHELF_STEP_X}" \
        "${DMG_HELPER_SHELF_Y}" <<'APPLESCRIPT'
on run argv
    set mountPath to item 1 of argv
    set helperShelfStartX to (item 2 of argv) as integer
    set helperShelfStepX to (item 3 of argv) as integer
    set helperShelfY to (item 4 of argv) as integer

    tell application "Finder"
        set targetContainer to (POSIX file mountPath as alias)
        my keep_helper_item_out_of_view(targetContainer, ".fseventsd", 0, helperShelfStartX, helperShelfStepX, helperShelfY)
        my keep_helper_item_out_of_view(targetContainer, ".DS_Store", 1, helperShelfStartX, helperShelfStepX, helperShelfY)
        my keep_helper_item_out_of_view(targetContainer, ".Trashes", 2, helperShelfStartX, helperShelfStepX, helperShelfY)
        my keep_helper_item_out_of_view(targetContainer, ".VolumeIcon.icns", 3, helperShelfStartX, helperShelfStepX, helperShelfY)
        my keep_helper_item_out_of_view(targetContainer, ".background", 4, helperShelfStartX, helperShelfStepX, helperShelfY)
    end tell
end run

on keep_helper_item_out_of_view(targetContainer, entryName, helperIndex, helperShelfStartX, helperShelfStepX, helperShelfY)
    tell application "Finder"
        try
            set helperItem to item entryName of targetContainer
            set helperX to helperShelfStartX + (helperIndex * helperShelfStepX)
            try
                set position of helperItem to {helperX, helperShelfY}
            end try
            try
                set extension hidden of helperItem to true
            end try
        end try
    end tell
end keep_helper_item_out_of_view
APPLESCRIPT
}

apply_custom_icon() {
    local icon_path="$1"
    local target_path="$2"
    local target_label="$3"
    local mode="$4"

    if [[ ! -f "${icon_path}" ]]; then
        if [[ "${mode}" == "required" ]]; then
            fail "Missing macOS packaging icon required for ${target_label}: ${icon_path}"
        fi
        write_warn "Skipping custom icon for ${target_label}; icon asset is missing"
        return 0
    fi

    if ! osascript - "${icon_path}" "${target_path}" <<'APPLESCRIPT' >/dev/null; then
use framework "Foundation"
use framework "AppKit"

on run argv
    set iconPath to item 1 of argv
    set targetPath to item 2 of argv
    set iconImage to current application's NSImage's alloc()'s initWithContentsOfFile:iconPath
    if iconImage is missing value then error "Unable to load icon at " & iconPath
    set didApply to (current application's NSWorkspace's sharedWorkspace()'s setIcon:iconImage forFile:targetPath options:0) as boolean
    if didApply is false then error "Unable to apply icon to " & targetPath
end run
APPLESCRIPT
        if [[ "${mode}" == "required" ]]; then
            fail "Unable to apply the custom icon to ${target_label}"
        fi
        write_warn "Unable to apply the custom icon to ${target_label}"
    fi
}

configure_dmg_finder_layout() {
    run_osascript - \
        "${DMG_VOLUME_NAME}" \
        "${APP_BUNDLE_NAME}" \
        "${APPLICATIONS_LINK_NAME}" \
        "${DMG_MOUNTED_BACKGROUND_PATH}" \
        "${DMG_BACKGROUND_HFS_PATH}" \
        "${DMG_BACKGROUND_RELATIVE_PATH}" \
        "${DMG_APP_X}" \
        "${DMG_APPLICATIONS_X}" \
        "${DMG_ITEM_Y}" \
        "${DMG_WINDOW_WIDTH}" \
        "${DMG_WINDOW_HEIGHT}" \
        "${DMG_ICON_SIZE}" \
        "${DMG_HELPER_SHELF_START_X}" \
        "${DMG_HELPER_SHELF_STEP_X}" \
        "${DMG_HELPER_SHELF_Y}" <<'APPLESCRIPT'
on run argv
    set volumeName to item 1 of argv
    set appName to item 2 of argv
    set applicationsName to item 3 of argv
    set backgroundPath to item 4 of argv
    set backgroundHfsPath to item 5 of argv
    set backgroundRelativePath to item 6 of argv
    set appX to (item 7 of argv) as integer
    set applicationsX to (item 8 of argv) as integer
    set itemY to (item 9 of argv) as integer
    set windowWidth to (item 10 of argv) as integer
    set windowHeight to (item 11 of argv) as integer
    set iconSize to (item 12 of argv) as integer
    set helperShelfStartX to (item 13 of argv) as integer
    set helperShelfStepX to (item 14 of argv) as integer
    set helperShelfY to (item 15 of argv) as integer
    set expectedAppPosition to {appX + (iconSize / 2) as integer, itemY + (iconSize / 2) as integer}
    set expectedApplicationsPosition to {applicationsX + (iconSize / 2) as integer, itemY + (iconSize / 2) as integer}

    tell application "Finder"
        activate
        delay 1
        tell disk (volumeName as string)
            open
        end tell

        set targetDisk to disk (volumeName as string)
        set targetWindow to my wait_for_container_window(targetDisk)

        set didApplyBackground to false
        set lastBackgroundErrorMessage to ""
        repeat 5 times
            try
                tell targetDisk
                    open
                    tell container window
                        set current view to icon view
                        delay 0.5
                        try
                            set toolbar visible to false
                        end try
                        try
                            set statusbar visible to false
                        end try
                        try
                            set pathbar visible to false
                        end try
                        try
                            set sidebar width to 0
                        end try
                        set bounds to {140, 120, 140 + windowWidth, 120 + windowHeight}
                    end tell

                    set targetViewOptions to the icon view options of container window
                    set arrangement of targetViewOptions to not arranged
                    set icon size of targetViewOptions to iconSize
                    set background picture of targetViewOptions to file backgroundHfsPath
                end tell
                update targetDisk
                delay 1
                set didApplyBackground to true
                exit repeat
            on error errMessage number errNumber
                set lastBackgroundErrorMessage to errMessage & " (" & errNumber & ")"
                delay 1
            end try
        end repeat

        if didApplyBackground is false then
            error "Finder failed to persist the DMG background from " & backgroundPath & ". Last error: " & lastBackgroundErrorMessage
        end if
        
        -- Crucial: give Finder time to settle after background change
        delay 1
        
        my position_dmg_items(targetDisk, appName, applicationsName, expectedAppPosition, expectedApplicationsPosition)
        my keep_helper_items_out_of_view(targetDisk, helperShelfStartX, helperShelfStepX, helperShelfY)
        my assert_helper_items_out_of_view(targetDisk, helperShelfStartX, helperShelfStepX, helperShelfY)
        my assert_item_layout(targetDisk, appName, applicationsName, expectedAppPosition, expectedApplicationsPosition)
        
        -- One final update to the container to ensure DS_Store is written
        update targetDisk
        delay 1
        my assert_helper_items_out_of_view(targetDisk, helperShelfStartX, helperShelfStepX, helperShelfY)
        my assert_item_layout(targetDisk, appName, applicationsName, expectedAppPosition, expectedApplicationsPosition)
    end tell
end run

on wait_for_container_window(targetDisk)
    tell application "Finder"
        repeat 25 times
            try
                return container window of targetDisk
            on error
                delay 0.2
            end try
        end repeat
    end tell

    error "Finder did not expose the DMG window in time for layout configuration."
end wait_for_container_window

on position_dmg_items(targetDisk, appName, applicationsName, expectedAppPosition, expectedApplicationsPosition)
    tell application "Finder"
        set appItem to my wait_for_entry_item(targetDisk, appName)
        set applicationsItem to my wait_for_entry_item(targetDisk, applicationsName)

        try
            set extension hidden of appItem to true
        end try

        set position of appItem to expectedAppPosition
        set position of applicationsItem to expectedApplicationsPosition
    end tell
end position_dmg_items

on assert_item_layout(targetDisk, appName, applicationsName, expectedAppPosition, expectedApplicationsPosition)
    tell application "Finder"
        set appItem to my wait_for_entry_item(targetDisk, appName)
        set applicationsItem to my wait_for_entry_item(targetDisk, applicationsName)
        my assert_item_position(appItem, appName, expectedAppPosition)
        my assert_item_position(applicationsItem, applicationsName, expectedApplicationsPosition)
    end tell
end assert_item_layout

on assert_item_position(targetItem, itemName, expectedPosition)
    tell application "Finder"
        set actualPosition to position of targetItem
        if actualPosition is not expectedPosition then
            error "Finder placed " & itemName & " at " & (actualPosition as text) & " instead of " & (expectedPosition as text)
        end if
    end tell
end assert_item_position

on keep_helper_items_out_of_view(targetDisk, helperShelfStartX, helperShelfStepX, helperShelfY)
    my keep_helper_item_out_of_view(targetDisk, ".fseventsd", 0, helperShelfStartX, helperShelfStepX, helperShelfY)
    my keep_helper_item_out_of_view(targetDisk, ".DS_Store", 1, helperShelfStartX, helperShelfStepX, helperShelfY)
    my keep_helper_item_out_of_view(targetDisk, ".Trashes", 2, helperShelfStartX, helperShelfStepX, helperShelfY)
    my keep_helper_item_out_of_view(targetDisk, ".VolumeIcon.icns", 3, helperShelfStartX, helperShelfStepX, helperShelfY)
    my keep_helper_item_out_of_view(targetDisk, ".background", 4, helperShelfStartX, helperShelfStepX, helperShelfY)
end keep_helper_items_out_of_view

on keep_helper_item_out_of_view(targetDisk, entryName, helperIndex, helperShelfStartX, helperShelfStepX, helperShelfY)
    tell application "Finder"
        try
            set helperItem to item entryName of targetDisk
            set helperX to helperShelfStartX + (helperIndex * helperShelfStepX)
            try
                set position of helperItem to {helperX, helperShelfY}
            end try
            try
                set extension hidden of helperItem to true
            end try
        end try
    end tell
end keep_helper_item_out_of_view

on assert_helper_items_out_of_view(targetDisk, helperShelfStartX, helperShelfStepX, helperShelfY)
    my assert_helper_item_out_of_view(targetDisk, ".fseventsd", 0, helperShelfStartX, helperShelfStepX, helperShelfY)
    my assert_helper_item_out_of_view(targetDisk, ".DS_Store", 1, helperShelfStartX, helperShelfStepX, helperShelfY)
    my assert_helper_item_out_of_view(targetDisk, ".Trashes", 2, helperShelfStartX, helperShelfStepX, helperShelfY)
    my assert_helper_item_out_of_view(targetDisk, ".VolumeIcon.icns", 3, helperShelfStartX, helperShelfStepX, helperShelfY)
    my assert_helper_item_out_of_view(targetDisk, ".background", 4, helperShelfStartX, helperShelfStepX, helperShelfY)
end assert_helper_items_out_of_view

on assert_helper_item_out_of_view(targetDisk, entryName, helperIndex, helperShelfStartX, helperShelfStepX, helperShelfY)
    tell application "Finder"
        try
            set helperItem to item entryName of targetDisk
            set actualPosition to position of helperItem
            set expectedX to helperShelfStartX + (helperIndex * helperShelfStepX)
            if item 2 of actualPosition is less than helperShelfY then
                error "Finder left helper metadata " & entryName & " inside the visible DMG window at " & (actualPosition as text)
            end if
            if item 1 of actualPosition is not expectedX then
                error "Finder moved helper metadata " & entryName & " to " & (actualPosition as text) & " instead of the helper shelf"
            end if
        on error errMessage number errNumber
            if errNumber is not -1728 then
                error errMessage number errNumber
            end if
        end try
    end tell
end assert_helper_item_out_of_view

on wait_for_entry_item(targetDisk, entryName)
    tell application "Finder"
        repeat 25 times
            try
                return item entryName of targetDisk
            on error
                delay 0.2
            end try
        end repeat
    end tell

    error "Finder did not expose the DMG item in time: " & entryName
end wait_for_entry_item

on posix_paths_match(leftPath, rightPath)
    if leftPath is rightPath or leftPath is (rightPath & "/") then
        return true
    end if

    if rightPath ends with "/" then
        return leftPath is text 1 thru -2 of rightPath
    end if

    return leftPath is (rightPath & "/")
end posix_paths_match

on path_has_expected_suffix(candidatePath, relativePath)
    set normalizedRelativePath to relativePath
    if normalizedRelativePath starts with "/" then
        set normalizedRelativePath to text 2 thru -1 of normalizedRelativePath
    end if

    set requiredSuffix to "/" & normalizedRelativePath
    set suffixLength to length of requiredSuffix
    if (length of candidatePath) < suffixLength then
        return false
    end if

    return text (-suffixLength) thru -1 of candidatePath is requiredSuffix
end path_has_expected_suffix
APPLESCRIPT
}

verify_final_dmg_layout() {
    write_step "Verifying the final DMG Finder presentation"
    local verify_output=""
    local background_status=""

    if [[ -d "${DMG_VERIFY_MOUNT_POINT}" ]]; then
        hdiutil detach "${DMG_VERIFY_MOUNT_POINT}" -quiet >/dev/null 2>&1 || true
        rmdir "${DMG_VERIFY_MOUNT_POINT}" >/dev/null 2>&1 || true
    fi

    detach_stale_dmg_mounts

    DMG_VERIFY_DEVICE="$(
        hdiutil attach \
            -readonly \
            -noverify \
            -noautoopen \
            -mountpoint "${DMG_VERIFY_MOUNT_POINT}" \
            "${RELEASE_DMG_PATH}" | awk '$1 ~ /^\/dev\/disk/ && $2 == "Apple_HFS" {print $1; exit}'
    )"

    if [[ -z "${DMG_VERIFY_DEVICE}" ]]; then
        fail "Unable to mount the final DMG for verification."
    fi

    trap 'cleanup_verify_dmg_mount' EXIT

    require_file "${DMG_VERIFY_BACKGROUND_PATH}" "mounted DMG background image inside the final DMG"
    assert_mounted_dmg_root_allowed "${DMG_VERIFY_MOUNT_POINT}"
    assert_dmg_background_binding_persisted "${DMG_VERIFY_MOUNT_POINT}"

    verify_output="$(
        run_osascript - \
        "${DMG_VERIFY_MOUNT_POINT}" \
        "${APP_BUNDLE_NAME}" \
        "${APPLICATIONS_LINK_NAME}" \
        "${DMG_VERIFY_BACKGROUND_PATH}" \
        "${DMG_BACKGROUND_RELATIVE_PATH}" \
        "${DMG_APP_X}" \
        "${DMG_APPLICATIONS_X}" \
        "${DMG_ITEM_Y}" \
        "${DMG_ICON_SIZE}" \
        "${DMG_HELPER_SHELF_START_X}" \
        "${DMG_HELPER_SHELF_STEP_X}" \
        "${DMG_HELPER_SHELF_Y}" <<'APPLESCRIPT'
on run argv
    set mountPath to item 1 of argv
    set appName to item 2 of argv
    set applicationsName to item 3 of argv
    set backgroundPath to item 4 of argv
    set backgroundRelativePath to item 5 of argv
    set appX to (item 6 of argv) as integer
    set applicationsX to (item 7 of argv) as integer
    set itemY to (item 8 of argv) as integer
    set iconSize to (item 9 of argv) as integer
    set helperShelfStartX to (item 10 of argv) as integer
    set helperShelfStepX to (item 11 of argv) as integer
    set helperShelfY to (item 12 of argv) as integer
    set expectedAppPosition to {appX + (iconSize / 2) as integer, itemY + (iconSize / 2) as integer}
    set expectedApplicationsPosition to {applicationsX + (iconSize / 2) as integer, itemY + (iconSize / 2) as integer}
    tell application "Finder"
        activate
        delay 1
        set targetContainer to (POSIX file mountPath as alias)
        open targetContainer
        set targetWindow to my wait_for_container_window(targetContainer)

        if current view of targetWindow is not icon view then
            error "The final DMG did not reopen in Finder icon view."
        end if

        my assert_helper_items_out_of_view(targetContainer, helperShelfStartX, helperShelfStepX, helperShelfY)
        set backgroundStatus to my background_status(targetContainer, backgroundPath, backgroundRelativePath)
        my assert_item_layout(targetContainer, appName, applicationsName, expectedAppPosition, expectedApplicationsPosition)
        return "BACKGROUND_STATUS=" & backgroundStatus
    end tell
end run

on wait_for_container_window(targetContainer)
    tell application "Finder"
        repeat 25 times
            try
                return container window of targetContainer
            on error
                delay 0.2
            end try
        end repeat
    end tell

    error "Finder did not reopen the final DMG window in time for verification."
end wait_for_container_window

on background_status(targetContainer, backgroundPath, backgroundRelativePath)
    tell application "Finder"
        try
            set currentBackground to background picture of icon view options of container window of targetContainer
            if currentBackground is missing value then
                return "missing"
            end if

            set resolvedBackgroundPath to POSIX path of (currentBackground as alias)
            if my posix_paths_match(resolvedBackgroundPath, backgroundPath) then
                return "match"
            end if
            if my path_has_expected_suffix(resolvedBackgroundPath, backgroundRelativePath) then
                return "match"
            end if
            return "mismatch:" & resolvedBackgroundPath
        on error errMessage number errNumber
            return "read_error:" & errNumber & ":" & errMessage
        end try
    end tell
end background_status

on assert_item_layout(targetContainer, appName, applicationsName, expectedAppPosition, expectedApplicationsPosition)
    tell application "Finder"
        set appItem to my wait_for_entry_item(targetContainer, appName)
        set applicationsItem to my wait_for_entry_item(targetContainer, applicationsName)

        my assert_item_position(appItem, appName, expectedAppPosition)
        my assert_item_position(applicationsItem, applicationsName, expectedApplicationsPosition)
    end tell
end assert_item_layout

on wait_for_entry_item(targetContainer, entryName)
    tell application "Finder"
        repeat 25 times
            try
                return item entryName of targetContainer
            on error
                delay 0.2
            end try
        end repeat
    end tell

    error "Finder did not expose the final DMG item in time: " & entryName
end wait_for_entry_item

on assert_item_position(targetItem, itemName, expectedPosition)
    tell application "Finder"
        set actualPosition to position of targetItem
        if actualPosition is not expectedPosition then
            error "The final DMG placed " & itemName & " at " & (actualPosition as text) & " instead of " & (expectedPosition as text)
        end if
    end tell
end assert_item_position

on assert_helper_items_out_of_view(targetContainer, helperShelfStartX, helperShelfStepX, helperShelfY)
    my assert_helper_item_out_of_view(targetContainer, ".fseventsd", 0, helperShelfStartX, helperShelfStepX, helperShelfY)
    my assert_helper_item_out_of_view(targetContainer, ".DS_Store", 1, helperShelfStartX, helperShelfStepX, helperShelfY)
    my assert_helper_item_out_of_view(targetContainer, ".Trashes", 2, helperShelfStartX, helperShelfStepX, helperShelfY)
    my assert_helper_item_out_of_view(targetContainer, ".VolumeIcon.icns", 3, helperShelfStartX, helperShelfStepX, helperShelfY)
    my assert_helper_item_out_of_view(targetContainer, ".background", 4, helperShelfStartX, helperShelfStepX, helperShelfY)
end assert_helper_items_out_of_view

on assert_helper_item_out_of_view(targetContainer, entryName, helperIndex, helperShelfStartX, helperShelfStepX, helperShelfY)
    tell application "Finder"
        try
            set helperItem to item entryName of targetContainer
            set actualPosition to position of helperItem
            set expectedX to helperShelfStartX + (helperIndex * helperShelfStepX)
            if item 2 of actualPosition is less than helperShelfY then
                error "The final DMG left helper metadata " & entryName & " inside the visible window at " & (actualPosition as text)
            end if
            if item 1 of actualPosition is not expectedX then
                error "The final DMG moved helper metadata " & entryName & " to " & (actualPosition as text) & " instead of the helper shelf"
            end if
        on error errMessage number errNumber
            if errNumber is not -1728 then
                error errMessage number errNumber
            end if
        end try
    end tell
end assert_helper_item_out_of_view

on posix_paths_match(leftPath, rightPath)
    if leftPath is rightPath or leftPath is (rightPath & "/") then
        return true
    end if

    if rightPath ends with "/" then
        return leftPath is text 1 thru -2 of rightPath
    end if

    return leftPath is (rightPath & "/")
end posix_paths_match

on path_has_expected_suffix(candidatePath, relativePath)
    set normalizedRelativePath to relativePath
    if normalizedRelativePath starts with "/" then
        set normalizedRelativePath to text 2 thru -1 of normalizedRelativePath
    end if

    set requiredSuffix to "/" & normalizedRelativePath
    set suffixLength to length of requiredSuffix
    if (length of candidatePath) < suffixLength then
        return false
    end if

    return text (-suffixLength) thru -1 of candidatePath is requiredSuffix
end path_has_expected_suffix
APPLESCRIPT
    )"

    background_status="$(printf '%s\n' "${verify_output}" | sed -n 's/^BACKGROUND_STATUS=//p' | tail -n 1)"

    case "${background_status}" in
        match|"")
            ;;
        read_error:-10000:*)
            write_warn "Finder did not read back the final DMG background via AppleScript (${background_status}); accepted because .DS_Store persisted the in-volume background binding."
            ;;
        *)
            fail "The final DMG did not reopen with the expected Finder background image at ${DMG_VERIFY_BACKGROUND_PATH}. Finder status: ${background_status}"
            ;;
    esac

    cleanup_verify_dmg_mount
    trap - EXIT
}

package_dmg() {
    write_step "Refreshing release/macos with ${DMG_NAME}"
    rm -rf "${RELEASE_DIR}" "${DMG_STAGING_DIR}"
    rm -f "${DMG_RW_PATH}"
    ensure_dir "${RELEASE_DIR}"
    ensure_dir "${DMG_STAGING_DIR}"

    cp -R "${APP_BUNDLE_PATH}" "${DMG_STAGING_DIR}/${APP_BUNDLE_NAME}"
    ln -s /Applications "${DMG_STAGING_DIR}/${APPLICATIONS_LINK_NAME}"
    stage_dmg_volume_icon
    stage_dmg_background_asset
    assert_dmg_staging_root_allowed

    write_step "Preparing writable DMG layout"
    hdiutil create \
        -srcfolder "${DMG_STAGING_DIR}" \
        -volname "${DMG_VOLUME_NAME}" \
        -fs HFS+ \
        -fsargs "-c c=64,a=16,e=16" \
        -format UDRW \
        -ov \
        "${DMG_RW_PATH}" >/dev/null

    if [[ -d "${DMG_LAYOUT_MOUNT_POINT}" ]]; then
        hdiutil detach "${DMG_LAYOUT_MOUNT_POINT}" -quiet >/dev/null 2>&1 || true
        rmdir "${DMG_LAYOUT_MOUNT_POINT}" >/dev/null 2>&1 || true
    fi

    detach_stale_dmg_mounts

    DMG_DEVICE="$(
        hdiutil attach \
            -readwrite \
            -noverify \
            -noautoopen \
            -mountpoint "${DMG_LAYOUT_MOUNT_POINT}" \
            "${DMG_RW_PATH}" | awk '$1 ~ /^\/dev\/disk/ && $2 == "Apple_HFS" {print $1; exit}'
    )"

    if [[ -z "${DMG_DEVICE}" ]]; then
        fail "Unable to attach the temporary DMG for Finder layout configuration."
    fi

    trap cleanup_dmg_mount EXIT

    apply_volume_branding
    sanitize_mounted_dmg_metadata "${DMG_LAYOUT_MOUNT_POINT}"
    disable_mounted_dmg_fsevents "${DMG_LAYOUT_MOUNT_POINT}"
    hide_mounted_dmg_metadata "${DMG_LAYOUT_MOUNT_POINT}"
    require_file "${DMG_MOUNTED_BACKGROUND_PATH}" "mounted DMG background image inside the writable layout volume"

    sync
    sleep 1

    write_step "Configuring Finder window layout"
    rm -f "${DMG_LAYOUT_PERSISTENCE_MARKER_PATH}"
    touch "${DMG_LAYOUT_PERSISTENCE_MARKER_PATH}"
    configure_dmg_finder_layout
    # macOS can recreate .fseventsd after deletion; keep no_log hidden before convert.
    hide_mounted_metadata_path "${DMG_LAYOUT_MOUNT_POINT}/${DMG_BACKGROUND_DIR_NAME}"
    disable_mounted_dmg_fsevents "${DMG_LAYOUT_MOUNT_POINT}"
    hide_mounted_dmg_metadata "${DMG_LAYOUT_MOUNT_POINT}"
    keep_mounted_dmg_metadata_out_of_view
    assert_mounted_dmg_root_allowed "${DMG_LAYOUT_MOUNT_POINT}"
    assert_dmg_layout_persisted "${DMG_LAYOUT_MOUNT_POINT}" "${DMG_LAYOUT_PERSISTENCE_MARKER_PATH}"

    if [[ -d "${DMG_LAYOUT_MOUNT_POINT}" ]]; then
        chmod -Rf go-w "${DMG_LAYOUT_MOUNT_POINT}" || true
    fi
    sync
    cleanup_dmg_mount
    trap - EXIT

    write_step "Building ${RELEASE_DMG_PATH}"
    hdiutil convert \
        "${DMG_RW_PATH}" \
        -ov \
        -format UDZO \
        -tgtimagekey zlib-level=9 \
        -o "${RELEASE_DMG_PATH}" >/dev/null

    rm -f "${DMG_RW_PATH}"

    apply_custom_icon \
        "${APP_ICNS_PATH}" \
        "${RELEASE_DMG_PATH}" \
        "the final DMG file" \
        best-effort

    verify_final_dmg_layout
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

for tool in sips iconutil plutil hdiutil osascript; do
    if ! command -v "${tool}" >/dev/null 2>&1; then
        fail "${tool} is required for macOS DMG packaging."
    fi
done

determine_target_triple

ensure_dir "${BUILD_TOOLS_ROOT}"
ensure_dir "${DOWNLOADS_DIR}"
ensure_dir "${BUILD_CARGO_HOME}"
ensure_dir "${BUILD_ROOT}"
ensure_dir "${RELEASE_DIR}"
require_file "${ICON_SVG_SOURCE_PATH}" "macOS SVG app icon"
require_file "${ICON_ICO_SOURCE_PATH}" "fallback ICO app icon"

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

assemble_app_bundle "${APP_VERSION}"
package_dmg

write_step "Build completed"
printf 'DMG: %s\n' "${RELEASE_DMG_PATH}"
printf 'Note: the generated macOS disk image contains an unsigned app bundle and is not notarized.\n'
