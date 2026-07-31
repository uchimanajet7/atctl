#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
# shellcheck source=scripts/maintenance/tool-versions.env
source scripts/maintenance/tool-versions.env

missing_tools=()
actionlint_runner=""

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
  if [[ "${actual_actionlint_version}" == "${ACTIONLINT_VERSION}" ]]; then
    actionlint_runner="local"
  elif command -v docker >/dev/null 2>&1; then
    actionlint_runner="docker"
  else
    echo "Expected actionlint ${ACTIONLINT_VERSION}, found ${actual_actionlint_version}." >&2
    echo "Upgrade actionlint or install Docker, then rerun this script." >&2
    exit 65
  fi
else
  actionlint_runner="docker"
fi

echo "== Repository shell syntax =="
bash -n scripts/maintenance/*.sh scripts/release/*.sh

echo "== Selected Rust, maintenance-tool, and GitHub Actions state =="

rust_toolchain_version="$(
  awk -F '"' '/^channel = / { print $2; exit }' rust-toolchain.toml
)"
cargo_rust_version="$(
  awk -F '"' '/^rust-version = / { print $2; exit }' Cargo.toml
)"
active_rust_version="$(rustc --version | awk '{ print $2 }')"

if [[ ! "${rust_toolchain_version}" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]]; then
  echo "rust-toolchain.toml must contain one complete stable Rust version." >&2
  exit 65
fi
if [[ ! "${cargo_rust_version}" =~ ^[0-9]+[.][0-9]+$ ]]; then
  echo "Cargo.toml rust-version must contain a major.minor MSRV." >&2
  exit 65
fi
if [[ ! "${CARGO_ABOUT_VERSION}" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ||
  ! "${ACTIONLINT_VERSION}" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]]; then
  echo "Fixed maintenance-tool pins must be complete stable versions." >&2
  exit 65
fi

IFS=. read -r selected_major selected_minor _ <<<"${rust_toolchain_version}"
IFS=. read -r msrv_major msrv_minor <<<"${cargo_rust_version}"
if ((selected_major < msrv_major)) ||
  ((selected_major == msrv_major && selected_minor < msrv_minor)); then
  echo "Selected Rust ${rust_toolchain_version} is older than MSRV ${cargo_rust_version}." >&2
  exit 65
fi
if [[ "${active_rust_version}" != "${rust_toolchain_version}" ]]; then
  echo "Expected active Rust ${rust_toolchain_version}, found ${active_rust_version}." >&2
  exit 65
fi

if ! awk '
  function valid_tag(tag,    version, parts, count, number) {
    if (substr(tag, 1, 1) != "v") {
      return 0
    }
    version = substr(tag, 2)
    count = split(version, parts, ".")
    if (count < 1 || count > 3) {
      return 0
    }
    for (number = 1; number <= count; number++) {
      if (parts[number] !~ /^[0-9]+$/) {
        return 0
      }
    }
    return 1
  }

  {
    marker_position = index($0, "uses: ")
    if (marker_position == 0) {
      next
    }

    use_value = substr($0, marker_position + length("uses: "))
    at_position = index(use_value, "@")
    if (at_position == 0) {
      next
    }

    repository = substr(use_value, 1, at_position - 1)
    if (repository ~ /^\./ || repository ~ /^docker:\/\//) {
      next
    }

    reference_and_comment = substr(use_value, at_position + 1)
    split(reference_and_comment, fields, /[[:space:]]+/)
    reference = fields[1]
    comment_position = index(reference_and_comment, "#")

    if (length(reference) != 40 ||
        reference ~ /[^0-9a-f]/ ||
        comment_position == 0) {
      invalid++
      next
    }

    tag = substr(reference_and_comment, comment_position + 1)
    sub(/^[[:space:]]+/, "", tag)
    sub(/[[:space:]]+$/, "", tag)
    if (!valid_tag(tag)) {
      invalid++
    }
  }

  END {
    if (invalid != 0) {
      exit 65
    }
  }
' .github/workflows/*.yml; then
  echo "Remote GitHub Actions must use a full commit SHA and version-tag comment." >&2
  exit 65
fi

printf 'Selected versions are internally consistent: MSRV %s, Rust %s.\n' \
  "${cargo_rust_version}" \
  "${rust_toolchain_version}"

echo "== Cargo lockfile metadata =="
cargo metadata --locked --format-version 1 >/dev/null

echo "== RustSec advisory audit =="
cargo audit

echo "== Direct dependency version report =="
cargo outdated --workspace --root-deps-only --exit-code 0

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
if [[ "${actionlint_runner}" == "local" ]]; then
  actionlint -color
else
  actionlint_image="rhysd/actionlint:${ACTIONLINT_VERSION}"
  docker run --rm -v "${repo_root}:/repo" --workdir /repo "${actionlint_image}" -color
fi
