#!/usr/bin/env bash
set -e

# Change directory to project root
cd "$(dirname "$0")/../.."

ARCH_INPUT="${1:-}"
TARGET_TRIPLE=""

case "${ARCH_INPUT}" in
    arm|arm64|aarch64|aarch64-apple-darwin)
        TARGET_TRIPLE="aarch64-apple-darwin"
        ;;
    intel|x86_64|x64|amd64|x86_64-apple-darwin)
        TARGET_TRIPLE="x86_64-apple-darwin"
        ;;
    ""|host|default)
        TARGET_TRIPLE=""
        ;;
    *)
        TARGET_TRIPLE="${ARCH_INPUT}"
        ;;
esac

echo "=== Ensuring App Icons Are Generated ==="
if [ ! -f "packaging/macos/AppIcon.icns" ]; then
    cargo run --bin generate_icon
fi

APP_NAME="Zenvi.app"

if [ -n "${TARGET_TRIPLE}" ]; then
    if command -v rustup >/dev/null 2>&1; then
        if ! rustup target list --installed | grep -q "^${TARGET_TRIPLE}\$"; then
            echo "Rust target '${TARGET_TRIPLE}' is not installed. Attempting to install via rustup..."
            rustup target add "${TARGET_TRIPLE}" || {
                echo "Error: Target '${TARGET_TRIPLE}' could not be installed automatically." >&2
                echo "Please check your network connection or run: rustup target add ${TARGET_TRIPLE}" >&2
                exit 1
            }
        fi
    fi

    echo "=== Building Zenvi Release Binary (${TARGET_TRIPLE}) ==="
    cargo build --release --target "${TARGET_TRIPLE}"
    BIN_PATH="target/${TARGET_TRIPLE}/release/zenvi"
    BUNDLE_DIR="target/${TARGET_TRIPLE}/${APP_NAME}"
else
    echo "=== Building Zenvi Release Binary (Host) ==="
    cargo build --release
    BIN_PATH="target/release/zenvi"
    BUNDLE_DIR="target/${APP_NAME}"
fi

CONTENTS_DIR="${BUNDLE_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "=== Creating macOS App Bundle: ${BUNDLE_DIR} ==="
rm -rf "${BUNDLE_DIR}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

# Copy binary
cp "${BIN_PATH}" "${MACOS_DIR}/zenvi"
chmod +x "${MACOS_DIR}/zenvi"

# Copy Info.plist
cp "packaging/macos/Info.plist" "${CONTENTS_DIR}/Info.plist"

# Copy AppIcon.icns
cp "packaging/macos/AppIcon.icns" "${RESOURCES_DIR}/AppIcon.icns"

echo "=== Ad-hoc Code Signing for macOS LaunchServices ==="
# Remove any quarantine flags from locally built artifacts
xattr -cr "${BUNDLE_DIR}" || true
# Sign ad-hoc so Finder double-click and Gatekeeper permit launch
codesign --force --deep --sign - "${BUNDLE_DIR}"

# Also mirror to target/Zenvi.app if building for host architecture
HOST_ARCH="$(uname -m)"
if [ -z "${TARGET_TRIPLE}" ] || \
   { [ "${HOST_ARCH}" = "arm64" ] && [ "${TARGET_TRIPLE}" = "aarch64-apple-darwin" ]; } || \
   { [ "${HOST_ARCH}" = "x86_64" ] && [ "${TARGET_TRIPLE}" = "x86_64-apple-darwin" ]; }; then
    if [ "${BUNDLE_DIR}" != "target/${APP_NAME}" ]; then
        rm -rf "target/${APP_NAME}"
        cp -R "${BUNDLE_DIR}" "target/${APP_NAME}"
    fi
fi

echo "=== Successfully Packaged ${APP_NAME} ==="
echo "Path: $(pwd)/${BUNDLE_DIR}"
