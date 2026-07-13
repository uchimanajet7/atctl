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

## Manual GitHub Release Archive

Use this path only when you need to inspect or run a source-repository release
artifact without installing the Homebrew formula. The normal install path
remains `brew install uchimanajet7/atctl/atctl`.

From the [atctl Releases page](https://github.com/uchimanajet7/atctl/releases),
download both files for the same version:

```text
atctl-v0.2.0-aarch64-apple-darwin.tar.gz
atctl-v0.2.0-aarch64-apple-darwin.tar.gz.sha256
```

In the directory containing those downloads, verify the archive before
extracting it:

```sh
shasum -a 256 -c atctl-v0.2.0-aarch64-apple-darwin.tar.gz.sha256
tar -xzf atctl-v0.2.0-aarch64-apple-darwin.tar.gz
```

The extracted directory contains:

```text
atctl-v0.2.0-aarch64-apple-darwin/
  atctl
  LICENSE
  THIRD-PARTY-NOTICES.txt
```

The executable dynamically links to Homebrew `libusb`. Install that runtime
dependency, then verify the executable:

```sh
brew install libusb
./atctl-v0.2.0-aarch64-apple-darwin/atctl --version
```

`LICENSE` contains the MIT license for `atctl`.
`THIRD-PARTY-NOTICES.txt` identifies target-specific Rust dependencies and the
dynamically linked `libusb` dependency with their applicable license texts.

The direct-download binary is not Developer ID signed or Apple notarized.
Depending on how it was downloaded and on local macOS policy, Gatekeeper or a
quarantine warning may appear. The Homebrew formula remains the supported
normal installation path.

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
