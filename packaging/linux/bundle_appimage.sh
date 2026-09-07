#!/usr/bin/env bash
set -e

# Change directory to project root
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "${SCRIPT_DIR}/../.."

APP_NAME="zenvi"
TARGET_DIR="target"
ARCH="$(uname -m)"
PACKAGE_NAME="${APP_NAME}-linux-${ARCH}"
APPDIR="${TARGET_DIR}/${APP_NAME}.AppDir"
APPIMAGE_OUTPUT="${TARGET_DIR}/${PACKAGE_NAME}.AppImage"

echo "=== [1/5] Building Zenvi Release Binary ==="
cargo build --release

echo "=== [2/5] Ensuring Desktop Icons Are Generated ==="
cargo run --bin generate_icon

echo "=== [3/5] Assembling AppDir (${APPDIR}) ==="
rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin"
mkdir -p "${APPDIR}/usr/share/applications"
mkdir -p "${APPDIR}/usr/share/icons/hicolor"

# 1. Copy release binary
cp "target/release/zenvi" "${APPDIR}/usr/bin/zenvi"
chmod +x "${APPDIR}/usr/bin/zenvi"

# 2. Copy AppRun entrypoint
cp "packaging/linux/AppRun" "${APPDIR}/AppRun"
chmod +x "${APPDIR}/AppRun"

# 3. Copy desktop entry to root and standard share location
cp "packaging/linux/zenvi.desktop" "${APPDIR}/zenvi.desktop"
cp "packaging/linux/zenvi.desktop" "${APPDIR}/usr/share/applications/zenvi.desktop"

# 4. Copy root icon (standard 256x256 png for launcher)
if [ -f "packaging/linux/icons/zenvi_256x256.png" ]; then
    cp "packaging/linux/icons/zenvi_256x256.png" "${APPDIR}/zenvi.png"
elif [ -f "packaging/linux/icons/zenvi.svg" ]; then
    cp "packaging/linux/icons/zenvi.svg" "${APPDIR}/zenvi.svg"
fi

# 5. Copy hicolor icon sets
ICON_SIZES=(16 24 32 48 64 128 256 512)
for size in "${ICON_SIZES[@]}"; do
    dest_dir="${APPDIR}/usr/share/icons/hicolor/${size}x${size}/apps"
    mkdir -p "${dest_dir}"
    src_icon="packaging/linux/icons/zenvi_${size}x${size}.png"
    if [ -f "${src_icon}" ]; then
        cp "${src_icon}" "${dest_dir}/zenvi.png"
    fi
done

mkdir -p "${APPDIR}/usr/share/icons/hicolor/scalable/apps"
if [ -f "packaging/linux/icons/zenvi.svg" ]; then
    cp "packaging/linux/icons/zenvi.svg" "${APPDIR}/usr/share/icons/hicolor/scalable/apps/zenvi.svg"
fi

echo "=== [4/5] Packaging AppImage (${APPIMAGE_OUTPUT}) ==="
APPIMAGETOOL_BIN=""

if command -v appimagetool >/dev/null 2>&1; then
    APPIMAGETOOL_BIN="appimagetool"
else
    TOOLS_DIR="${TARGET_DIR}/tools"
    mkdir -p "${TOOLS_DIR}"
    CACHED_TOOL="${TOOLS_DIR}/appimagetool-${ARCH}.AppImage"

    if [ ! -f "${CACHED_TOOL}" ]; then
        echo "appimagetool not found in PATH. Downloading official appimagetool..."
        DOWNLOAD_URL="https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${ARCH}.AppImage"
        curl -sL -o "${CACHED_TOOL}" "${DOWNLOAD_URL}" || {
            echo "Error: Failed to download appimagetool from ${DOWNLOAD_URL}" >&2
            exit 1
        }
        chmod +x "${CACHED_TOOL}"
    fi
    APPIMAGETOOL_BIN="${CACHED_TOOL}"
fi

export ARCH="${ARCH}"
rm -f "${APPIMAGE_OUTPUT}"

# Execute appimagetool (using --appimage-extract-and-run fallback for environments without FUSE)
if "${APPIMAGETOOL_BIN}" --appimage-extract-and-run --version >/dev/null 2>&1; then
    "${APPIMAGETOOL_BIN}" --appimage-extract-and-run "${APPDIR}" "${APPIMAGE_OUTPUT}"
else
    "${APPIMAGETOOL_BIN}" "${APPDIR}" "${APPIMAGE_OUTPUT}"
fi

chmod +x "${APPIMAGE_OUTPUT}"

echo "=== [5/5] Generating SHA256 Checksum ==="
APPIMAGE_FILENAME="$(basename "${APPIMAGE_OUTPUT}")"
(cd "${TARGET_DIR}" && sha256sum "${APPIMAGE_FILENAME}" > "${APPIMAGE_FILENAME}.sha256")

echo "=== AppImage Packaging Finished Successfully ==="
echo "AppImage : $(pwd)/${APPIMAGE_OUTPUT}"
echo "SHA256   : $(cat "${APPIMAGE_OUTPUT}.sha256" | cut -d' ' -f1)"
