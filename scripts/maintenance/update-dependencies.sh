#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

# shellcheck source=scripts/maintenance/tool-versions.env
source scripts/maintenance/tool-versions.env

usage() {
  cat <<'EOF'
Usage:
  scripts/maintenance/update-dependencies.sh

Updates the selected stable Rust toolchain, repository-pinned maintenance
tools, and every Cargo.lock dependency allowed by Cargo.toml, then runs the
complete local dependency and Rust verification gates.

The command does not commit, push, merge, tag, publish, or create a release.
EOF
}

if (($# != 0)); then
  case "${1:-}" in
    -h|--help|help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 64
      ;;
  esac
fi

for command_name in awk cargo curl git; do
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "Required dependency-update command is missing: ${command_name}" >&2
    exit 127
  fi
done

valid_complete_version() {
  [[ "$1" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]]
}

version_not_older_than() {
  local candidate_major
  local candidate_minor
  local candidate_patch
  local current_major
  local current_minor
  local current_patch

  IFS=. read -r candidate_major candidate_minor candidate_patch <<<"$1"
  IFS=. read -r current_major current_minor current_patch <<<"$2"

  ((candidate_major > current_major)) ||
    ((candidate_major == current_major && candidate_minor > current_minor)) ||
    ((candidate_major == current_major &&
      candidate_minor == current_minor &&
      candidate_patch >= current_patch))
}

replace_quoted_assignment() {
  local file_path="$1"
  local key="$2"
  local value="$3"
  local temp_path

  temp_path="$(mktemp "${TMPDIR:-/tmp}/atctl-version-file.XXXXXX")"
  if [[ "$(grep -Ec "^${key}[[:space:]]*=" "${file_path}")" != "1" ]]; then
    rm -f "${temp_path}"
    echo "Expected exactly one ${key} assignment in ${file_path}." >&2
    exit 65
  fi

  awk -v key="${key}" -v value="${value}" '
    $0 ~ "^" key "[[:space:]]*=" {
      print key " = \"" value "\""
      next
    }
    { print }
  ' "${file_path}" > "${temp_path}"
  chmod 0644 "${temp_path}"
  mv "${temp_path}" "${file_path}"
}

replace_env_assignment() {
  local key="$1"
  local value="$2"
  local file_path="scripts/maintenance/tool-versions.env"
  local temp_path

  temp_path="$(mktemp "${TMPDIR:-/tmp}/atctl-tool-versions.XXXXXX")"
  if [[ "$(grep -Ec "^${key}=" "${file_path}")" != "1" ]]; then
    rm -f "${temp_path}"
    echo "Expected exactly one ${key} assignment in ${file_path}." >&2
    exit 65
  fi

  awk -F= -v key="${key}" -v value="${value}" '
    $1 == key {
      print key "=" value
      next
    }
    { print }
  ' "${file_path}" > "${temp_path}"
  chmod 0644 "${temp_path}"
  mv "${temp_path}" "${file_path}"
}

update_temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/atctl-dependency-update.XXXXXX")"
trap 'rm -rf "${update_temp_dir}"' EXIT

current_rust_version="$(
  awk -F '"' '/^channel = / { print $2; exit }' rust-toolchain.toml
)"
rust_manifest="${update_temp_dir}/channel-rust-stable.toml"
curl -fsSL --retry 3 \
  https://static.rust-lang.org/dist/channel-rust-stable.toml \
  -o "${rust_manifest}"
latest_rust_version="$(
  awk -F '"' '
    /^\[pkg[.]rust\]$/ {
      in_rust = 1
      next
    }
    in_rust && /^version = / {
      split($2, parts, " ")
      print parts[1]
      exit
    }
  ' "${rust_manifest}"
)"

cargo_search_error="${update_temp_dir}/cargo-search-error.txt"
if ! cargo_search_output="$(
  cargo search cargo-about --limit 1 2>"${cargo_search_error}"
)"; then
  cat "${cargo_search_error}" >&2
  exit 69
fi
latest_cargo_about_version="$(
  awk -F '"' '$1 ~ /^cargo-about = / { print $2; exit }' \
    <<<"${cargo_search_output}"
)"

latest_actionlint_tag="$(
  git ls-remote \
    --tags \
    --refs \
    --sort=version:refname \
    https://github.com/rhysd/actionlint.git \
    'refs/tags/v*' |
    awk -F/ '
      $NF ~ /^v[0-9]+[.][0-9]+[.][0-9]+$/ {
        latest = $NF
      }
      END {
        sub(/^v/, "", latest)
        print latest
      }
    '
)"

for version_value in \
  "${current_rust_version}" \
  "${CARGO_ABOUT_VERSION}" \
  "${ACTIONLINT_VERSION}" \
  "${latest_rust_version}" \
  "${latest_cargo_about_version}" \
  "${latest_actionlint_tag}"; do
  if ! valid_complete_version "${version_value}"; then
    echo "Could not resolve a complete stable version: ${version_value:-<empty>}" >&2
    exit 69
  fi
done

if ! version_not_older_than "${latest_rust_version}" "${current_rust_version}" ||
  ! version_not_older_than "${latest_cargo_about_version}" "${CARGO_ABOUT_VERSION}" ||
  ! version_not_older_than "${latest_actionlint_tag}" "${ACTIONLINT_VERSION}"; then
  echo "An authoritative upstream returned a version older than the repository selection." >&2
  exit 69
fi

echo "== Dependency update plan =="
printf 'Rust:        %s -> %s\n' "${current_rust_version}" "${latest_rust_version}"
printf 'cargo-about: %s -> %s\n' "${CARGO_ABOUT_VERSION}" "${latest_cargo_about_version}"
printf 'actionlint:  %s -> %s\n' "${ACTIONLINT_VERSION}" "${latest_actionlint_tag}"
echo "Cargo:       update every version allowed by Cargo.toml"

replace_quoted_assignment rust-toolchain.toml channel "${latest_rust_version}"
replace_env_assignment CARGO_ABOUT_VERSION "${latest_cargo_about_version}"
replace_env_assignment ACTIONLINT_VERSION "${latest_actionlint_tag}"

installed_cargo_about_version=""
if command -v cargo-about >/dev/null 2>&1; then
  installed_cargo_about_version="$(cargo about --version)"
fi
if [[ "${installed_cargo_about_version}" != "cargo-about ${latest_cargo_about_version}" ]]; then
  cargo install \
    --locked \
    --force \
    cargo-about \
    --version "${latest_cargo_about_version}" \
    --features cli
fi

cargo update

scripts/maintenance/check-deps.sh
cargo fmt --check
cargo check --all-targets --all-features --locked
cargo test --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
git diff --check

echo "== Local changes for maintainer review =="
git status --short
git diff --stat

echo "Dependencies updated and verified."
