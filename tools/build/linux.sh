#!/usr/bin/env bash

set -euo pipefail

TARGET_TRIPLE="x86_64-unknown-linux-gnu"
ARTIFACT_NAME="LANScanner"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

BUILD_TOOLS_ROOT="${PROJECT_ROOT}/.build-tools/linux-gnu"
LOCAL_TOOLS_ROOT="${BUILD_TOOLS_ROOT}/local-tools"
DOWNLOADS_DIR="${LOCAL_TOOLS_ROOT}/downloads"
LOCAL_CARGO_HOME="${LOCAL_TOOLS_ROOT}/cargo"
LOCAL_RUSTUP_HOME="${LOCAL_TOOLS_ROOT}/rustup"
BUILD_ROOT="${BUILD_TOOLS_ROOT}/build"
CARGO_TARGET_DIR="${BUILD_ROOT}/target"
RELEASE_DIR="${PROJECT_ROOT}/release/linux"
ARTIFACT_PATH="${CARGO_TARGET_DIR}/${TARGET_TRIPLE}/release/${ARTIFACT_NAME}"
RELEASE_ARTIFACT_PATH="${RELEASE_DIR}/${ARTIFACT_NAME}"
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

    fail "Neither curl nor wget is available. Install one of them so the script can bootstrap Rust when cargo is missing."
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

write_step "Refreshing release/linux with only ${ARTIFACT_NAME}"
rm -rf "${RELEASE_DIR}"
ensure_dir "${RELEASE_DIR}"
cp "${ARTIFACT_PATH}" "${RELEASE_ARTIFACT_PATH}"
chmod +x "${RELEASE_ARTIFACT_PATH}"

write_step "Build completed"
printf 'Executable: %s\n' "${RELEASE_ARTIFACT_PATH}"
printf 'Note: %s is a Linux executable and should be run inside Linux/WSL2, not as a native Windows .exe.\n' "${RELEASE_ARTIFACT_PATH}"
