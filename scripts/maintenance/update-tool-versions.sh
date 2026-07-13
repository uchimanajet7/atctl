#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

usage() {
  cat >&2 <<'EOF'
Usage:
  scripts/maintenance/update-tool-versions.sh rust <version>
  scripts/maintenance/update-tool-versions.sh cargo-about <version>
  scripts/maintenance/update-tool-versions.sh actionlint <version>

The version must be a complete stable release such as 1.97.0 or 0.9.1.
Run scripts/maintenance/check-deps.sh after updating a pin.
EOF
}

if (($# != 2)); then
  usage
  exit 64
fi

version_name="$1"
new_version="$2"

if [[ ! "${new_version}" =~ ^[0-9]+[.][0-9]+[.][0-9]+$ ]]; then
  echo "Version must be a complete stable release: ${new_version}" >&2
  exit 64
fi

replace_quoted_assignment() {
  local file_path="$1"
  local key="$2"
  local value="$3"
  local temp_path="${file_path}.tmp.$$"

  if [[ "$(grep -Ec "^${key}[[:space:]]*=" "${file_path}")" != "1" ]]; then
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
  local temp_path="${file_path}.tmp.$$"

  if [[ "$(grep -Ec "^${key}=" "${file_path}")" != "1" ]]; then
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

case "${version_name}" in
  rust)
    if [[ "$(grep -Ec '^channel[[:space:]]*=' rust-toolchain.toml)" != "1" ||
      "$(grep -Ec '^rust-version[[:space:]]*=' Cargo.toml)" != "1" ]]; then
      echo "Rust version sources do not have the expected single assignments." >&2
      exit 65
    fi
    replace_quoted_assignment rust-toolchain.toml channel "${new_version}"
    replace_quoted_assignment Cargo.toml rust-version "${new_version%.*}"
    ;;
  cargo-about)
    replace_env_assignment CARGO_ABOUT_VERSION "${new_version}"
    ;;
  actionlint)
    replace_env_assignment ACTIONLINT_VERSION "${new_version}"
    ;;
  *)
    usage
    exit 64
    ;;
esac

printf 'Updated %s to %s.\n' "${version_name}" "${new_version}"
echo "Run scripts/maintenance/check-deps.sh before committing the update."
