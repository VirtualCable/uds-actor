#!/usr/bin/make -f
# -*- makefile -*-

.PHONY: all install-udsactor install-udsactor-unmanaged clean

# Distro to build for (Debian12, Fedora41, openSUSE15)
DISTRO ?= Debian12

# Output directory from rustbuilder
OUTPUTDIR := builders/$(DISTRO)/output

# Install targets
BINDIR := $(DESTDIR)/usr/bin
SBINDIR := $(DESTDIR)/usr/sbin
APPSDIR := $(DESTDIR)/usr/share/applications
POLKITDIR := $(DESTDIR)/usr/share/polkit-1/actions
SYSTEMDDIR := $(DESTDIR)/etc/systemd/system
AUTOSTARTDIR := $(DESTDIR)/etc/xdg/autostart

# === Entry point ===
all: install-udsactor install-udsactor-unmanaged

# === Build binaries using rustbuilder.py ===
$(OUTPUTDIR)/udsactor-client: rustbuilder.py
    python3 rustbuilder.py $(DISTRO)

# === Install managed variant ===
install-udsactor: $(OUTPUTDIR)/udsactor-client
    mkdir -p $(BINDIR) $(SBINDIR) $(APPSDIR) $(POLKITDIR) $(AUTOSTARTDIR)

    # Install Rust binaries
    cp $(OUTPUTDIR)/udsactor-client $(BINDIR)/
    cp $(OUTPUTDIR)/udsactor-service $(SBINDIR)/
    cp $(OUTPUTDIR)/udsactor-config $(SBINDIR)/
    cp $(OUTPUTDIR)/gui-helper $(BINDIR)/

    # Install desktop entries
    cp desktop/udsactor_client.desktop $(APPSDIR)/
    cp desktop/udsactor_config.desktop $(AUTOSTARTDIR)/

    # Install PolicyKit
    cp policy/org.openuds.pkexec.udsactor_config.policy $(POLKITDIR)/

    # Install systemd service for RH-based distros
ifneq (,$(filter $(DISTRO),Fedora41 openSUSE15))
    mkdir -p $(SYSTEMDDIR)
    cp debian/udsactor.service $(SYSTEMDDIR)/
endif

    # Permissions
    chmod 755 $(BINDIR)/udsactor-client $(BINDIR)/gui-helper
    chmod 755 $(SBINDIR)/udsactor-service $(SBINDIR)/udsactor-config
    chmod 644 $(POLKITDIR)/org.openuds.pkexec.udsactor_config.policy

# === Install unmanaged variant ===
install-udsactor-unmanaged: $(OUTPUTDIR)/udsactor-unmanaged-config
    mkdir -p $(SBINDIR) $(AUTOSTARTDIR)

    # Replace config binary
    cp $(OUTPUTDIR)/udsactor-unmanaged-config $(SBINDIR)/udsactor-config

    # Replace desktop entry
    cp desktop/udsactor_config-unmanaged.desktop $(AUTOSTARTDIR)/udsactor_config.desktop

# === Clean build artifacts ===
clean:
    rm -rf $(DESTDIR)
