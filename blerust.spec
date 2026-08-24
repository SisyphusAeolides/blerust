%global debug_package %{nil}
Name:           blerust
Version:        0.1.14
Release:        1%{?dist}
Summary:        Blazing fast and robust line editor in Rust

License:        MIT
URL:            https://github.com/SisyphusAeolides/blerust
Source0:        %{url}/archive/main/blerust-main.tar.gz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  git

%description
Blazing fast and robust line editor in Rust (replacement for blesh).

%prep
%autosetup -n %{name}-main

%build
cargo build --release --locked

%install
mkdir -p %{buildroot}%{_bindir}
install -pm 755 target/release/%{name} %{buildroot}%{_bindir}/%{name}

%files
%doc README.md
%{_bindir}/%{name}

%post
if [ $1 -eq 1 ]; then
    %{_bindir}/%{name} --install || true
fi

%preun
if [ $1 -eq 0 ]; then
    HOME_DIR=$(eval echo ~$(logname 2>/dev/null || echo $SUDO_USER || echo $USER))
    BASHRC="$HOME_DIR/.bashrc"
    if [ -f "$BASHRC" ]; then
        sed -i '/# blerust initialization/,/exec blerust/d' "$BASHRC"
    fi
fi

%changelog
* Sun Aug 23 2026 Kenny Glowner <SisyphusAeolides@pm.me> - 0.1.14-1
- refactor: use native bash wrapper loop to support state persistence, aliases, and job control
- fix: share ~/.bash_history instead of using isolated history file

* Sun Aug 23 2026 Kenny Glowner <SisyphusAeolides@pm.me> - 0.1.13-1
- fix: restore raw terminal carriage returns and correct prompt layout

* Sat Aug 22 2026 Kenny Glowner <SisyphusAeolides@pm.me> - 0.1.12-1
- Keep multiline paste editable until the user explicitly presses Enter

* Sat Aug 22 2026 Kenny Glowner <SisyphusAeolides@pm.me> - 0.1.11-1
- Suppress history suggestions while literal paste input is rendered

* Sat Aug 22 2026 Kenny Glowner <SisyphusAeolides@pm.me> - 0.1.10-1
- Keep completion disabled while bracketed paste input is handled

* Sat Aug 22 2026 Kenny Glowner <SisyphusAeolides@pm.me> - 0.1.9-1
- Preserve and execute pasted multi-command shell blocks atomically

* Sat Aug 22 2026 Kenny Glowner <SisyphusAeolides@pm.me> - 0.1.0-1
- Initial release
