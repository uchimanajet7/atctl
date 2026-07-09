# Installation

This document defines the end-user installation and runtime prerequisites for
`atctl`.

## Validated Environment

Validated environments:

```text
Mac: Apple Silicon Mac
USB modem: SORACOM Onyx LTE USB Dongle
Internal modem: Quectel EG25-G
USB ID: 0x2c7c:0x0125
```

This installation guide documents the validated environment above. Other
operating systems, Intel Macs, universal macOS binaries, and broad modem
coverage are not presented here as validated install environments until they
have matching validation and documentation.

## Normal End-User Install Flow

End users should install `atctl` through Homebrew:

```sh
brew install uchimanajet7/atctl/atctl
```

This fully qualified formula name installs `Formula/atctl.rb` from the
`uchimanajet7/atctl` tap. The tap name uses Homebrew's GitHub tap naming
convention: `uchimanajet7/atctl` maps to the GitHub repository
`https://github.com/uchimanajet7/homebrew-atctl`; the source repository remains
`https://github.com/uchimanajet7/atctl`.

The equivalent two-step form is:

```sh
brew tap uchimanajet7/atctl
brew install atctl
```

References:

- https://docs.brew.sh/Taps
- https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap

The Homebrew formula must declare `libusb` as a dependency. Homebrew should
therefore install `libusb` automatically for end users.

The intended Homebrew path is a formula that can use a bottle when available
and falls back to a source build when no matching bottle is available or when
source verification is needed. `libusb` remains a runtime dependency in both
cases. Rust and `pkgconf` are source-build dependencies, not normal runtime
dependencies for bottled installs.

GitHub Releases archives are release artifacts and manual artifacts. They are
not the normal end-user install path.

## Manual libusb Fallback

Use this only when troubleshooting Homebrew dependency installation or when
building from source:

```sh
brew install libusb
```

`libusb` is the native USB access library used by `atctl` through the Rust
`rusb` crate. It allows `atctl` to communicate with USB interfaces and endpoints
without relying on `/dev/cu.*` serial devices.

## USB Device Visibility Check

Check whether macOS sees the modem as a USB device:

```sh
system_profiler SPUSBHostDataType | grep -Ei -A 12 'EG25|Quectel|2c7c|0125'
```

If this command does not show the modem, try another USB port, avoid unpowered
hubs, and confirm the modem is physically connected.

## First Commands After Installation

```sh
atctl devices
atctl inspect
atctl send AT
```

By default, `atctl devices` shows plausible AT operation targets. If it cannot
show the expected USB modem, run `atctl devices --all-usb` to inspect all USB
devices visible through `libusb`, then see
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).
