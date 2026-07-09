#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

missing_tools=()

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    missing_tools+=("${command_name}")
  fi
}

require_command cargo-audit
require_command cargo-outdated

if ! command -v actionlint >/dev/null 2>&1 && ! command -v docker >/dev/null 2>&1; then
  missing_tools+=("actionlint-or-docker")
fi

if ((${#missing_tools[@]} > 0)); then
  printf 'Missing maintenance tool(s): %s\n' "${missing_tools[*]}" >&2
  cat >&2 <<'EOF'

Install the required tools, then rerun this script:

  cargo install cargo-audit
  cargo install --locked cargo-outdated
  brew install actionlint

Alternatively, install Docker and let the script run the pinned actionlint
container image.
EOF
  exit 127
fi

echo "== Cargo lockfile metadata =="
cargo metadata --locked --format-version 1 >/dev/null

echo "== RustSec advisory audit =="
cargo audit

echo "== Direct dependency version report =="
outdated_exit_code="${ATCTL_OUTDATED_EXIT_CODE:-0}"
cargo outdated --workspace --root-deps-only --exit-code "${outdated_exit_code}"

echo "== Duplicate dependency tree review signal =="
cargo tree --duplicates

echo "== GitHub Actions workflow lint =="
if command -v actionlint >/dev/null 2>&1; then
  actionlint -color
else
  actionlint_image="${ACTIONLINT_DOCKER_IMAGE:-rhysd/actionlint:1.7.12}"
  docker run --rm -v "${repo_root}:/repo" --workdir /repo "${actionlint_image}" -color
fi
