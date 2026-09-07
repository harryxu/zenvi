.PHONY: \
	macapp \
	macapp-arm \
	macapp-intel \
	dmg \
	dmg-arm \
	dmg-intel \
	dmg-all \
	linux

# macOS App Bundle (.app) targets
macapp:
	./packaging/macos/bundle_macos.sh

macapp-arm:
	./packaging/macos/bundle_macos.sh aarch64

macapp-intel:
	./packaging/macos/bundle_macos.sh x86_64

# macOS Disk Image (.dmg) targets
dmg:
	./packaging/macos/bundle_dmg.sh

dmg-arm:
	./packaging/macos/bundle_dmg.sh arm

dmg-intel:
	./packaging/macos/bundle_dmg.sh intel

dmg-all:
	./packaging/macos/bundle_dmg.sh all

# Linux bundle target
linux:
	./packaging/linux/bundle_linux.sh