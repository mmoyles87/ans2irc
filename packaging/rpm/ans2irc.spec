%global debug_package %{nil}

Name:           ans2irc
Version:        0.1.0
Release:        1%{?dist}
Summary:        Convert ANSI art to mIRC-coloured text for IRC
License:        GPL-3.0-or-later
URL:            https://github.com/mmoyles87/ans2irc
Source0:        %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires:  cargo >= 1.70
BuildRequires:  rust >= 1.70

%description
ans2irc reads ANSI art (CP437 text with escape codes, SAUCE optional) and
writes it as UTF-8 text with mIRC colour codes, one IRC message per line, in
as few bytes as the picture allows. Given a byte budget, it merges colours
until each line fits before it cuts any.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release --locked --offline

%check
cargo test --release --locked --offline

%install
install -Dm755 target/release/ans2irc %{buildroot}%{_bindir}/ans2irc
install -Dm644 doc/ans2irc.1 %{buildroot}%{_mandir}/man1/ans2irc.1

%files
%license LICENSE
%doc README.md
%{_bindir}/ans2irc
%{_mandir}/man1/ans2irc.1*

%changelog
* Fri Oct 16 2026 Matt Moyles <matt@mattmoyles.com> - 0.1.0-1
- Initial release
