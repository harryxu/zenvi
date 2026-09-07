#!/usr/bin/env bash
set -e

# Change directory to project root
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "${SCRIPT_DIR}/../.."

APP_NAME="zenvi"
TARGET_DIR="target"
ARCH="$(uname -m)"
PACKAGE_NAME="${APP_NAME}-linux-${ARCH}"
BUNDLE_DIR="${TARGET_DIR}/${PACKAGE_NAME}"

echo "=== [1/6] Building Zenvi Release Binary ==="
cargo build --release

echo "=== [2/6] Ensuring Desktop Icons Are Generated ==="
cargo run --bin generate_icon

echo "=== [3/6] Assembling Linux Distribution Bundle (${PACKAGE_NAME}) ==="
rm -rf "${BUNDLE_DIR}"

BIN_DIR="${BUNDLE_DIR}/bin"
APPS_DIR="${BUNDLE_DIR}/share/applications"
ICONS_BASE_DIR="${BUNDLE_DIR}/share/icons/hicolor"

mkdir -p "${BIN_DIR}"
mkdir -p "${APPS_DIR}"

# 1. Copy release binary
cp "target/release/zenvi" "${BIN_DIR}/zenvi"
chmod +x "${BIN_DIR}/zenvi"

# 2. Copy .desktop entry
cp "packaging/linux/zenvi.desktop" "${APPS_DIR}/zenvi.desktop"

# 3. Copy hicolor icons (freedesktop specification)
ICON_SIZES=(16 24 32 48 64 128 256 512)
for size in "${ICON_SIZES[@]}"; do
    dest_dir="${ICONS_BASE_DIR}/${size}x${size}/apps"
    mkdir -p "${dest_dir}"
    src_icon="packaging/linux/icons/zenvi_${size}x${size}.png"
    if [ -f "${src_icon}" ]; then
        cp "${src_icon}" "${dest_dir}/zenvi.png"
    fi
done

# Scalable SVG icon
mkdir -p "${ICONS_BASE_DIR}/scalable/apps"
if [ -f "packaging/linux/icons/zenvi.svg" ]; then
    cp "packaging/linux/icons/zenvi.svg" "${ICONS_BASE_DIR}/scalable/apps/zenvi.svg"
fi

# 4. Copy modular install and uninstall scripts
cp "packaging/linux/install.sh" "${BUNDLE_DIR}/install.sh"
chmod +x "${BUNDLE_DIR}/install.sh"

cp "packaging/linux/uninstall.sh" "${BUNDLE_DIR}/uninstall.sh"
chmod +x "${BUNDLE_DIR}/uninstall.sh"

echo "=== [4/6] Creating Compressed Archive (.tar.gz) ==="
TAR_OUTPUT="${TARGET_DIR}/${PACKAGE_NAME}.tar.gz"
tar -czf "${TAR_OUTPUT}" -C "${TARGET_DIR}" "${PACKAGE_NAME}"
(cd "${TARGET_DIR}" && sha256sum "${PACKAGE_NAME}.tar.gz" > "${PACKAGE_NAME}.tar.gz.sha256")

echo "=== [5/6] Packaging AppImage Format ==="
"${SCRIPT_DIR}/bundle_appimage.sh"

echo "=== [6/6] Linux Packaging Finished Successfully ==="
echo "Bundle Directory: $(pwd)/${BUNDLE_DIR}"
echo "Release Tarball : $(pwd)/${TAR_OUTPUT}"
echo "AppImage Output : $(pwd)/${TARGET_DIR}/${PACKAGE_NAME}.AppImage"
echo "AppImage SHA256 : $(cat "${TARGET_DIR}/${PACKAGE_NAME}.AppImage.sha256" | cut -d' ' -f1)"
