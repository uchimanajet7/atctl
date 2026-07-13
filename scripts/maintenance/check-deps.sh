#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
# shellcheck source=scripts/maintenance/tool-versions.env
source scripts/maintenance/tool-versions.env

missing_tools=()

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    missing_tools+=("${command_name}")
  fi
}

require_command cargo-audit
require_command cargo-about
require_command cargo-outdated
require_command pkg-config

if ! command -v actionlint >/dev/null 2>&1 && ! command -v docker >/dev/null 2>&1; then
  missing_tools+=("actionlint-or-docker")
fi

if ((${#missing_tools[@]} > 0)); then
  printf 'Missing maintenance tool(s): %s\n' "${missing_tools[*]}" >&2
  cat >&2 <<EOF

Install the required tools, then rerun this script:

  cargo install cargo-audit
  cargo install --locked cargo-about --version ${CARGO_ABOUT_VERSION} --features cli
  cargo install --locked cargo-outdated
  brew install libusb pkgconf
  brew install actionlint

Alternatively, install Docker and let the script run the pinned actionlint
container image.
EOF
  exit 127
fi

actual_cargo_about_version="$(cargo about --version)"
if [[ "${actual_cargo_about_version}" != "cargo-about ${CARGO_ABOUT_VERSION}" ]]; then
  echo "Expected cargo-about ${CARGO_ABOUT_VERSION}, found ${actual_cargo_about_version}." >&2
  exit 65
fi

if command -v actionlint >/dev/null 2>&1; then
  actual_actionlint_version="$(actionlint -version)"
  actual_actionlint_version="${actual_actionlint_version%%$'\n'*}"
  if [[ "${actual_actionlint_version}" != "${ACTIONLINT_VERSION}" ]]; then
    echo "Expected actionlint ${ACTIONLINT_VERSION}, found ${actual_actionlint_version}." >&2
    exit 65
  fi
fi

echo "== Repository shell syntax =="
bash -n scripts/maintenance/*.sh scripts/release/*.sh

echo "== Rust and fixed-tool version drift =="
scripts/maintenance/check-version-drift.sh

echo "== Cargo lockfile metadata =="
cargo metadata --locked --format-version 1 >/dev/null

echo "== RustSec advisory audit =="
cargo audit

echo "== Direct dependency version report =="
outdated_exit_code="${ATCTL_OUTDATED_EXIT_CODE:-0}"
cargo outdated --workspace --root-deps-only --exit-code "${outdated_exit_code}"

echo "== Duplicate dependency tree review signal =="
cargo tree --duplicates

echo "== Third-party notice generation =="
notice_temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/atctl-maintenance.XXXXXX")"
trap 'rm -rf "${notice_temp_dir}"' EXIT
libusb_version="$(pkg-config --modversion libusb-1.0)"
cargo fetch --locked --target aarch64-apple-darwin
scripts/maintenance/generate-third-party-notices.sh \
  "${notice_temp_dir}/notices-1.txt" \
  "${libusb_version}"
scripts/maintenance/generate-third-party-notices.sh \
  "${notice_temp_dir}/notices-2.txt" \
  "${libusb_version}"
if ! cmp -s "${notice_temp_dir}/notices-1.txt" "${notice_temp_dir}/notices-2.txt"; then
  echo "Third-party notice generation is not deterministic." >&2
  exit 65
fi

echo "== GitHub Actions workflow lint =="
if command -v actionlint >/dev/null 2>&1; then
  actionlint -color
else
  actionlint_image="rhysd/actionlint:${ACTIONLINT_VERSION}"
  docker run --rm -v "${repo_root}:/repo" --workdir /repo "${actionlint_image}" -color
fi
