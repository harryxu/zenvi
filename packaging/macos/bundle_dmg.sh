#!/usr/bin/env bash
set -e

# Change directory to project root
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "${SCRIPT_DIR}/../.."

if ! command -v hdiutil >/dev/null 2>&1; then
    echo "Error: hdiutil not found. DMG packaging is only supported on macOS." >&2
    exit 1
fi

# Extract version from Cargo.toml
VERSION=$(grep -m1 '^version' Cargo.toml | sed -E 's/version[[:space:]]*=[[:space:]]*"([^"]+)"/\1/')
if [ -z "${VERSION}" ]; then
    VERSION="0.1.0"
fi

build_dmg() {
    local ARCH_TAG="$1"
    local TARGET_TRIPLE="$2"

    echo ""
    echo "=================================================="
    echo "  Packaging Zenvi DMG for ${ARCH_TAG} (${TARGET_TRIPLE})"
    echo "=================================================="

    # 1. Build .app bundle
    "${SCRIPT_DIR}/bundle_macos.sh" "${TARGET_TRIPLE}"

    local APP_SOURCE="target/${TARGET_TRIPLE}/Zenvi.app"
    if [ ! -d "${APP_SOURCE}" ]; then
        APP_SOURCE="target/Zenvi.app"
    fi

    if [ ! -d "${APP_SOURCE}" ]; then
        echo "Error: Application bundle not found at ${APP_SOURCE}" >&2
        exit 1
    fi

    # 2. Prepare staging directory
    local STAGING_DIR="target/dmg-staging-${ARCH_TAG}"
    rm -rf "${STAGING_DIR}"
    mkdir -p "${STAGING_DIR}"

    echo "=== Copying Zenvi.app to staging area ==="
    cp -R "${APP_SOURCE}" "${STAGING_DIR}/Zenvi.app"

    echo "=== Creating /Applications symlink ==="
    ln -s /Applications "${STAGING_DIR}/Applications"

    # 3. Create DMG
    local DMG_NAME="Zenvi-${VERSION}-${ARCH_TAG}.dmg"
    local DMG_PATH="target/${DMG_NAME}"
    rm -f "${DMG_PATH}"

    echo "=== Creating compressed disk image: ${DMG_PATH} ==="
    hdiutil create \
        -volname "Zenvi" \
        -srcfolder "${STAGING_DIR}" \
        -ov \
        -format UDZO \
        "${DMG_PATH}"

    # 4. Clean up staging directory
    rm -rf "${STAGING_DIR}"

    # 5. Generate SHA256 checksum
    echo "=== Generating SHA256 Checksum ==="
    (cd target && shasum -a 256 "${DMG_NAME}" > "${DMG_NAME}.sha256")

    echo "=== Successfully Created DMG: ${DMG_NAME} ==="
    echo "File: $(pwd)/${DMG_PATH}"
    echo "Size: $(du -h "${DMG_PATH}" | cut -f1)"
    echo "SHA256: $(cat "target/${DMG_NAME}.sha256")"
}

ARCH_MODE="${1:-}"

case "${ARCH_MODE}" in
    arm|arm64|aarch64)
        build_dmg "aarch64" "aarch64-apple-darwin"
        ;;
    intel|x86_64|x64|amd64)
        build_dmg "x86_64" "x86_64-apple-darwin"
        ;;
    all)
        build_dmg "aarch64" "aarch64-apple-darwin"
        build_dmg "x86_64" "x86_64-apple-darwin"
        ;;
    ""|host|default)
        HOST_ARCH="$(uname -m)"
        if [ "${HOST_ARCH}" = "arm64" ]; then
            build_dmg "aarch64" "aarch64-apple-darwin"
        else
            build_dmg "x86_64" "x86_64-apple-darwin"
        fi
        ;;
    *)
        echo "Usage: $0 [arm|intel|all]"
        exit 1
        ;;
esac
