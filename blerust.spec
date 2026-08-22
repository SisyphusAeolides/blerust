%global debug_package %{nil}
Name:           blerust
Version:        0.1.12
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

%changelog
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
