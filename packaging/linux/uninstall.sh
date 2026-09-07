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
