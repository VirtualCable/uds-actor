#!/usr/bin/make -f
# -*- makefile -*-

.PHONY: all install-udsactor install-udsactor-unmanaged install-udsactor-common clean

VERSION := $(shell [ -f ../../../VERSION ] && cat ../../../VERSION || echo "devel")
RELEASE := 1

# Distro to build for (Debian12, Fedora41, openSUSE15)
DISTRO ?= Debian12
DESTDIR ?= ./package/$(DISTRO)

# Output directory from rustbuilder
OUTPUTDIR := builders/$(DISTRO)/output

# Install targets
BINDIR := $(DESTDIR)/usr/bin
SBINDIR := $(DESTDIR)/usr/sbin
APPSDIR := $(DESTDIR)/usr/share/applications
POLKITDIR := $(DESTDIR)/usr/share/polkit-1/actions
SYSTEMDDIR := $(DESTDIR)/etc/systemd/system
AUTOSTARTDIR := $(DESTDIR)/etc/xdg/autostart
ICONDIR := $(DESTDIR)/usr/share/udsactor

# No all rule, because the build depends on the variant (managed/unmanaged), etc..

help:
	@echo "Usage:"
	@echo "  make install-udsactor DISTRO=Debian12"
	@echo "  make install-udsactor-unmanaged DISTRO=Debian12"
	@echo "  make clean DISTRO=Debian12"


# === Build binaries using rustbuilder.py ===
$(OUTPUTDIR)/udsactor-client: rustbuilder.py
	python3 rustbuilder.py $(DISTRO)
	@test -f $(OUTPUTDIR)/udsactor-client || (echo "Missing udsactor-client binary" && exit 1)


# === Common installation logic ===
install-udsactor-common: $(OUTPUTDIR)/udsactor-client
	mkdir -p $(BINDIR) $(SBINDIR) $(APPSDIR) $(POLKITDIR) $(AUTOSTARTDIR) $(ICONDIR)

	# Install shared Rust binaries
	cp $(OUTPUTDIR)/udsactor-client $(BINDIR)/
	cp $(OUTPUTDIR)/udsactor-service $(SBINDIR)/
	cp $(OUTPUTDIR)/gui-helper $(BINDIR)/

	# Install icon
	cp ../../assets/img/uds-icon.png $(ICONDIR)/

	# Install PolicyKit
	cp policy/org.openuds.pkexec.udsactor_config.policy $(POLKITDIR)/

	# Install systemd service for RH-based distros
ifneq (,$(filter $(DISTRO),Fedora41 openSUSE15))
	mkdir -p $(SYSTEMDDIR)
	cp debian/udsactor.service $(SYSTEMDDIR)/
endif

	# Permissions
	chmod 755 $(BINDIR)/udsactor-client $(BINDIR)/gui-helper
	chmod 755 $(SBINDIR)/udsactor-service
	chmod 644 $(POLKITDIR)/org.openuds.pkexec.udsactor_config.policy
	chmod 644 $(ICONDIR)/uds-icon.png

# === Install managed variant ===
install-udsactor: install-udsactor-common
	# Install managed config
	cp $(OUTPUTDIR)/udsactor-config $(SBINDIR)/udsactor-config

	# Install managed desktop entry
	cp desktop/udsactor_config.desktop $(APPSDIR)/
	cp desktop/udsactor_client.desktop $(AUTOSTARTDIR)/

# === Install unmanaged variant ===
install-udsactor-unmanaged: install-udsactor-common
	# Install unmanaged config
	cp $(OUTPUTDIR)/udsactor-unmanaged-config $(SBINDIR)/udsactor-config

	# Install unmanaged desktop entry
	cp desktop/udsactor_config-unmanaged.desktop $(APPSDIR)/udsactor_config.desktop
	cp desktop/udsactor_client.desktop $(AUTOSTARTDIR)/

package-rpm:
	@echo "→ Generating RPM spec files for version $(VERSION)"
	cat udsactor-template.spec | \
		sed -e s/"version 0.0.0"/"version $(VERSION)"/g | \
		sed -e s/"release 1"/"release $(RELEASE)"/g > udsactor-$(VERSION).spec

	cat udsactor-unmanaged-template.spec | \
		sed -e s/"version 0.0.0"/"version $(VERSION)"/g | \
		sed -e s/"release 1"/"release $(RELEASE)"/g > udsactor-unmanaged-$(VERSION).spec

	@echo "→ Building RPM packages"
	rm -rf rpm
	for folder in SOURCES BUILD RPMS SPECS SRPMS; do \
		mkdir -p rpm/$$folder; \
	done

	for pkg in udsactor-*$(VERSION).spec; do \
		rpmbuild -v -bb --clean --target noarch $$pkg; \
	done

	rpm --addsign ../*.rpm

package-deb:
	dpkg-buildpackage -b


# === Clean build artifacts ===
clean:
	rm -rf $(DESTDIR)
