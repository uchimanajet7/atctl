#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: scripts/maintenance/generate-third-party-notices.sh OUTPUT LIBUSB_VERSION

Generate the third-party notice file for the Apple Silicon macOS release.
OUTPUT must not already exist. LIBUSB_VERSION is the version used by the build.
EOF
}

if (($# != 2)); then
  usage
  exit 64
fi

output_path="$1"
libusb_version="$2"

if [[ -z "${output_path}" || -z "${libusb_version}" ]]; then
  usage
  exit 64
fi

if [[ ! "${libusb_version}" =~ ^[0-9]+([.][0-9]+)+([-.+][0-9A-Za-z.-]+)?$ ]]; then
  echo "libusb version is not valid: ${libusb_version}" >&2
  exit 64
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
# shellcheck source=scripts/maintenance/tool-versions.env
source scripts/maintenance/tool-versions.env

readonly required_cargo_about_version="cargo-about ${CARGO_ABOUT_VERSION}"

if ! command -v cargo-about >/dev/null 2>&1; then
  echo "cargo-about ${CARGO_ABOUT_VERSION} is required." >&2
  echo "Install it with: cargo install --locked cargo-about --version ${CARGO_ABOUT_VERSION} --features cli" >&2
  exit 127
fi

actual_cargo_about_version="$(cargo about --version)"
if [[ "${actual_cargo_about_version}" != "${required_cargo_about_version}" ]]; then
  echo "Expected ${required_cargo_about_version}, found ${actual_cargo_about_version}." >&2
  exit 65
fi

for required_file in about.toml about.hbs licenses/third-party/libusb-LGPL-2.1-or-later.txt; do
  if [[ ! -f "${required_file}" ]]; then
    echo "Required notice input is missing: ${required_file}" >&2
    exit 66
  fi
done

if [[ -e "${output_path}" ]]; then
  echo "Refusing to overwrite existing notice file: ${output_path}" >&2
  exit 73
fi

output_parent="$(dirname "${output_path}")"
mkdir -p "${output_parent}"
output_parent="$(cd "${output_parent}" && pwd -P)"
output_path="${output_parent}/$(basename "${output_path}")"

temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/atctl-notices.XXXXXX")"
trap 'rm -rf "${temp_dir}"' EXIT

rust_notices="${temp_dir}/rust-runtime-dependencies.txt"
combined_notices="${temp_dir}/THIRD-PARTY-NOTICES.txt"

cargo about generate \
  --config about.toml \
  --locked \
  --offline \
  --fail \
  --target aarch64-apple-darwin \
  --output-file "${rust_notices}" \
  about.hbs

if [[ ! -s "${rust_notices}" ]]; then
  echo "cargo-about generated an empty notice file." >&2
  exit 65
fi

if grep -Eiq 'NOASSERTION|UNKNOWN LICENSE|UNLICENSED' "${rust_notices}"; then
  echo "Generated Rust notices contain an unidentified license." >&2
  exit 65
fi

if grep -Eq '&quot;|&#x27;|&amp;|&lt;|&gt;' "${rust_notices}"; then
  echo "Generated Rust notices contain HTML-escaped license text." >&2
  exit 65
fi

if grep -Eq '^- atctl[[:space:]]' "${rust_notices}"; then
  echo "Generated third-party components unexpectedly include the atctl package." >&2
  exit 65
fi

if ! grep -Eq '^- [A-Za-z0-9_-]+[[:space:]][0-9]' "${rust_notices}"; then
  echo "Generated Rust notices contain no third-party components." >&2
  exit 65
fi

{
  cat <<EOF
THIRD-PARTY NOTICES FOR atctl

This file lists third-party software used by the atctl Apple Silicon macOS
release. The atctl project license is provided separately in LICENSE.

NATIVE RUNTIME DEPENDENCY

Component: libusb ${libusb_version}
Source: https://github.com/libusb/libusb
License: LGPL-2.1-or-later
Linkage: dynamically linked; libusb is not included in this archive.

GNU LESSER GENERAL PUBLIC LICENSE VERSION 2.1

EOF
  cat licenses/third-party/libusb-LGPL-2.1-or-later.txt
  printf '\n\n'
  cat "${rust_notices}"
} > "${combined_notices}"

if [[ ! -s "${combined_notices}" ]]; then
  echo "Combined third-party notices are empty." >&2
  exit 65
fi

mv "${combined_notices}" "${output_path}"
printf 'Generated %s\n' "${output_path}"
