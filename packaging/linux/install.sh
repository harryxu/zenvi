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
