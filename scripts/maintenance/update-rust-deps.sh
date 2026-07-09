#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

usage() {
  cat <<'EOF'
Usage:
  scripts/maintenance/update-rust-deps.sh all
  scripts/maintenance/update-rust-deps.sh package <crate>
  scripts/maintenance/update-rust-deps.sh package <crate> <version>

Modes:
  all                  Refresh Cargo.lock for compatible dependency updates.
  package <crate>      Refresh one package in Cargo.lock.
  package <crate> <version>
                       Refresh one package to an exact version allowed by
                       Cargo.toml.

For a direct dependency baseline change, edit Cargo.toml so the intended exact
version is visible in the manifest, then run this script to refresh Cargo.lock
and execute the maintenance and Rust verification checks.
EOF
}

mode="${1:-}"

case "${mode}" in
  all)
    cargo update
    ;;
  package)
    crate_name="${2:-}"
    precise_version="${3:-}"
    if [[ -z "${crate_name}" ]]; then
      usage >&2
      exit 2
    fi
    if [[ -n "${precise_version}" ]]; then
      cargo update -p "${crate_name}" --precise "${precise_version}"
    else
      cargo update -p "${crate_name}"
    fi
    ;;
  -h|--help|help)
    usage
    exit 0
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

scripts/maintenance/check-deps.sh

cargo fmt --check
cargo check --all-targets --all-features --locked
cargo test --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
