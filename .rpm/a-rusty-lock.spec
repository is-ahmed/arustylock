%define __spec_install_post %{nil}
%define __os_install_post %{_dbpath}/brp-compress
%define debug_package %{nil}

Name: a-rusty-lock
Summary: A CLI Password manager written in Rust
Version: @@VERSION@@
Release: @@RELEASE@@%{?dist}
License: APACHE 2.0
Group: Applications/System
Source0: %{name}-%{version}.tar.gz

BuildRoot: %{_tmppath}/%{name}-%{version}-%{release}-root

%description
%{summary}

%prep
%setup -q

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a * %{buildroot}

%clean
rm -rf %{buildroot}

%files
%defattr(-,root,root,-)
%{_bindir}/*
