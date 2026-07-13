#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

# shellcheck source=scripts/maintenance/tool-versions.env
source scripts/maintenance/tool-versions.env

for command_name in awk cargo curl git rustc; do
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "Required version-check command is missing: ${command_name}" >&2
    exit 127
  fi
done

for version_value in "${CARGO_ABOUT_VERSION}" "${ACTIONLINT_VERSION}"; do
  if [[ ! "${version_value}" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]]; then
    echo "Invalid pinned maintenance-tool version: ${version_value}" >&2
    exit 65
  fi
done

rust_toolchain_version="$(
  awk -F '"' '/^channel = / { print $2; exit }' rust-toolchain.toml
)"
cargo_rust_version="$(
  awk -F '"' '/^rust-version = / { print $2; exit }' Cargo.toml
)"
active_rust_version="$(rustc --version | awk '{ print $2 }')"

if [[ ! "${rust_toolchain_version}" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]]; then
  echo "rust-toolchain.toml does not contain one complete stable version." >&2
  exit 65
fi

if [[ "${cargo_rust_version}" != "${rust_toolchain_version%.*}" ]]; then
  echo "Cargo.toml rust-version ${cargo_rust_version} does not match Rust ${rust_toolchain_version}." >&2
  echo "Run: scripts/maintenance/update-tool-versions.sh rust ${rust_toolchain_version}" >&2
  exit 65
fi

if [[ "${active_rust_version}" != "${rust_toolchain_version}" ]]; then
  echo "Expected active Rust ${rust_toolchain_version}, found ${active_rust_version}." >&2
  exit 65
fi

version_temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/atctl-version-check.XXXXXX")"
trap 'rm -rf "${version_temp_dir}"' EXIT

rust_manifest="${version_temp_dir}/channel-rust-stable.toml"
curl -fsSL --retry 3 \
  https://static.rust-lang.org/dist/channel-rust-stable.toml \
  -o "${rust_manifest}"
latest_rust_version="$(
  awk -F '"' '
    /^\[pkg[.]rust\]$/ { in_rust = 1; next }
    in_rust && /^version = / {
      split($2, parts, " ")
      print parts[1]
      exit
    }
  ' "${rust_manifest}"
)"

latest_cargo_about_version="$(
  cargo search cargo-about --limit 1 |
    awk -F '"' '$1 ~ /^cargo-about = / { print $2; exit }'
)"

latest_actionlint_version="$(
  git ls-remote --tags --refs --sort=version:refname \
    https://github.com/rhysd/actionlint.git 'v*' |
    awk -F/ 'END { sub(/^v/, "", $NF); print $NF }'
)"

for latest_version in \
  "${latest_rust_version}" \
  "${latest_cargo_about_version}" \
  "${latest_actionlint_version}"; do
  if [[ ! "${latest_version}" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]]; then
    echo "Could not resolve a valid latest version from an authoritative upstream." >&2
    exit 69
  fi
done

drift_found=0

report_drift() {
  local name="$1"
  local pinned="$2"
  local latest="$3"
  local update_name="$4"

  if [[ "${pinned}" == "${latest}" ]]; then
    printf '%s: %s (current)\n' "${name}" "${pinned}"
    return
  fi

  printf '%s: pinned %s, latest %s\n' "${name}" "${pinned}" "${latest}" >&2
  printf 'Run: scripts/maintenance/update-tool-versions.sh %s %s\n' \
    "${update_name}" "${latest}" >&2
  drift_found=1
}

report_drift Rust "${rust_toolchain_version}" "${latest_rust_version}" rust
report_drift cargo-about "${CARGO_ABOUT_VERSION}" "${latest_cargo_about_version}" cargo-about
report_drift actionlint "${ACTIONLINT_VERSION}" "${latest_actionlint_version}" actionlint

if ((drift_found != 0)); then
  exit 1
fi
