#!/usr/bin/env bash
set -e

# Change directory to project root
cd "$(dirname "$0")/.."

echo "=== Building Zenvi Release Binary ==="
cargo build --release

echo "=== Ensuring App Icons Are Generated ==="
cargo run --bin generate_icon

APP_NAME="Zenvi.app"
TARGET_DIR="target"
BUNDLE_DIR="${TARGET_DIR}/${APP_NAME}"
CONTENTS_DIR="${BUNDLE_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

echo "=== Creating macOS App Bundle: ${BUNDLE_DIR} ==="
rm -rf "${BUNDLE_DIR}"
mkdir -p "${MACOS_DIR}"
mkdir -p "${RESOURCES_DIR}"

# Copy binary
cp "target/release/zenvi" "${MACOS_DIR}/zenvi"
chmod +x "${MACOS_DIR}/zenvi"

# Copy Info.plist
cp "assets/Info.plist" "${CONTENTS_DIR}/Info.plist"

# Copy AppIcon.icns
cp "assets/AppIcon.icns" "${RESOURCES_DIR}/AppIcon.icns"

echo "=== Ad-hoc Code Signing for macOS LaunchServices ==="
# Remove any quarantine flags from locally built artifacts
xattr -cr "${BUNDLE_DIR}" || true
# Sign ad-hoc so Finder double-click and Gatekeeper permit launch
codesign --force --deep --sign - "${BUNDLE_DIR}"

echo "=== Successfully Packaged ${APP_NAME} ==="
echo "Path: $(pwd)/${BUNDLE_DIR}"
