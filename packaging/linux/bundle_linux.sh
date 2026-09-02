#!/usr/bin/env bash
set -e

# Change directory to project root
cd "$(dirname "$0")/../.."

APP_NAME="zenvi"
TARGET_DIR="target"
ARCH="$(uname -m)"
PACKAGE_NAME="${APP_NAME}-linux-${ARCH}"
BUNDLE_DIR="${TARGET_DIR}/${PACKAGE_NAME}"

echo "=== [1/5] Building Zenvi Release Binary ==="
cargo build --release

echo "=== [2/5] Ensuring Desktop Icons Are Generated ==="
cargo run --bin generate_icon

echo "=== [3/5] Assembling Linux Distribution Bundle (${PACKAGE_NAME}) ==="
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

# 4. Generate user-friendly installation script
cat << 'EOF' > "${BUNDLE_DIR}/install.sh"
#!/usr/bin/env bash
set -e

# Support user install (~/.local) or system install (/usr/local)
if [ "$EUID" -ne 0 ]; then
    PREFIX="${HOME}/.local"
    echo "Installing Zenvi to user directory (${PREFIX})..."
else
    PREFIX="/usr/local"
    echo "Installing Zenvi to system directory (${PREFIX})..."
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

mkdir -p "${PREFIX}/bin"
mkdir -p "${PREFIX}/share/applications"
mkdir -p "${PREFIX}/share/icons/hicolor"

# Copy binary
cp "${SCRIPT_DIR}/bin/zenvi" "${PREFIX}/bin/zenvi"
chmod +x "${PREFIX}/bin/zenvi"

# Copy desktop entry
cp "${SCRIPT_DIR}/share/applications/zenvi.desktop" "${PREFIX}/share/applications/zenvi.desktop"

# Copy icons
cp -r "${SCRIPT_DIR}/share/icons/hicolor/"* "${PREFIX}/share/icons/hicolor/"

# Update desktop & icon caches if tools exist
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${PREFIX}/share/applications" || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${PREFIX}/share/icons/hicolor" || true
fi

echo "Zenvi successfully installed!"
echo "You can now run 'zenvi' from terminal or launch it from your application menu."
EOF
chmod +x "${BUNDLE_DIR}/install.sh"

# 5. Generate uninstall script
cat << 'EOF' > "${BUNDLE_DIR}/uninstall.sh"
#!/usr/bin/env bash
set -e

if [ "$EUID" -ne 0 ]; then
    PREFIX="${HOME}/.local"
    echo "Uninstalling Zenvi from user directory (${PREFIX})..."
else
    PREFIX="/usr/local"
    echo "Uninstalling Zenvi from system directory (${PREFIX})..."
fi

rm -f "${PREFIX}/bin/zenvi"
rm -f "${PREFIX}/share/applications/zenvi.desktop"
find "${PREFIX}/share/icons/hicolor" -name "zenvi.png" -delete 2>/dev/null || true
find "${PREFIX}/share/icons/hicolor" -name "zenvi.svg" -delete 2>/dev/null || true

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "${PREFIX}/share/applications" || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "${PREFIX}/share/icons/hicolor" || true
fi

echo "Zenvi has been uninstalled."
EOF
chmod +x "${BUNDLE_DIR}/uninstall.sh"

echo "=== [4/5] Creating Compressed Archive (.tar.gz) ==="
TAR_OUTPUT="${TARGET_DIR}/${PACKAGE_NAME}.tar.gz"
tar -czf "${TAR_OUTPUT}" -C "${TARGET_DIR}" "${PACKAGE_NAME}"

echo "=== [5/5] Packaging Finished Successfully ==="
echo "Bundle Directory: $(pwd)/${BUNDLE_DIR}"
echo "Release Tarball : $(pwd)/${TAR_OUTPUT}"
