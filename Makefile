.PHONY: \
	macapp \
	linux

macapp:
	./packaging/macos/bundle_macos.sh

linux:
	./packaging/linux/bundle_linux.sh