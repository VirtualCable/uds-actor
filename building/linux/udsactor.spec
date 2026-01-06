Name: udsactor
Version: %{version}
Release: %{release}
Summary: Actor for Universal Desktop Services (UDS) Broker
License: BSD-3-Clause
URL: https://www.udsenterprise.com

AutoReq: yes
AutoProv: yes

# No debuginfo package
%global debug_package %{nil}

# Avoid RPM trying to use SOURCES/BUILD incorrectly
%global _builddir %{_topdir}
%global _sourcedir %{_topdir}

# Runtime dependencies
Requires: libXScrnSaver, xset

%changelog
* Fri Dec 19 2025 Adolfo <info@udsenterprise.com> - %{version}-%{release}
- Initial release

%description
Actor for UDS Broker environments.

%prep
# Nothing

%build
# Nothing (built externally)

%install
cp -a %{DESTDIR}/* %{buildroot}/

%post
if [ -x /usr/bin/systemctl ]; then
    systemctl enable udsactor.service >/dev/null 2>&1 || true
fi

%preun
if [ -x /usr/bin/systemctl ]; then
    systemctl disable udsactor.service >/dev/null 2>&1 || true
    systemctl stop udsactor.service >/dev/null 2>&1 || true
fi

%postun
if [ $1 -eq 0 ]; then
    rm -rf /etc/udsactor
    rm -f /var/log/udsactor.log
fi

%files
/usr
/etc