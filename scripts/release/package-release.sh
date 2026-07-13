#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: scripts/release/package-release.sh VERSION TARGET BINARY OUTPUT_DIR

Build the verified Apple Silicon macOS release archive and checksum.
The destination assets must not already exist.
EOF
}

if (($# != 4)); then
  usage
  exit 64
fi

version="$1"
target="$2"
binary_path="$3"
output_dir="$4"

if [[ ! "${version}" =~ ^[0-9]+[.][0-9]+[.][0-9]+([-.+][0-9A-Za-z.-]+)?$ ]]; then
  echo "Version is not a semantic version: ${version}" >&2
  exit 64
fi

if [[ "${target}" != "aarch64-apple-darwin" ]]; then
  echo "Unsupported release target: ${target}" >&2
  exit 64
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"

if [[ ! -f "${binary_path}" || ! -x "${binary_path}" ]]; then
  echo "Release binary is missing or not executable: ${binary_path}" >&2
  exit 66
fi

binary_path="$(cd "$(dirname "${binary_path}")" && pwd -P)/$(basename "${binary_path}")"

missing_tools=()
for command_name in brew file lipo otool pkg-config shasum stat tar; do
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    missing_tools+=("${command_name}")
  fi
done

if ((${#missing_tools[@]} > 0)); then
  printf 'Missing release packaging tool(s): %s\n' "${missing_tools[*]}" >&2
  exit 127
fi

libusb_prefix="$(brew --prefix libusb)"
libusb_version="$(pkg-config --modversion libusb-1.0)"
expected_libusb="${libusb_prefix}/lib/libusb-1.0.0.dylib"

if [[ ! -f "${expected_libusb}" ]]; then
  echo "Homebrew libusb runtime library was not found: ${expected_libusb}" >&2
  exit 66
fi

file_description="$(file "${binary_path}")"
if [[ "${file_description}" != *"Mach-O 64-bit executable arm64"* ]]; then
  echo "Release binary is not an arm64 Mach-O executable: ${file_description}" >&2
  exit 65
fi

binary_arches="$(lipo -archs "${binary_path}")"
if [[ "${binary_arches}" != "arm64" ]]; then
  echo "Release binary must contain only arm64, found: ${binary_arches}" >&2
  exit 65
fi

linkage_file="$(mktemp "${TMPDIR:-/tmp}/atctl-linkage.XXXXXX")"
trap 'rm -f "${linkage_file}"' EXIT
otool -L "${binary_path}" > "${linkage_file}"

if ! grep -Fq "${expected_libusb}" "${linkage_file}"; then
  echo "Release binary does not dynamically link the expected Homebrew libusb:" >&2
  cat "${linkage_file}" >&2
  exit 65
fi

if tail -n +2 "${linkage_file}" | grep -E '/target/|/registry/src/' >/dev/null 2>&1; then
  echo "Release binary contains a build-directory dynamic library path:" >&2
  cat "${linkage_file}" >&2
  exit 65
fi

mkdir -p "${output_dir}"
output_dir="$(cd "${output_dir}" && pwd -P)"

root_name="atctl-v${version}-${target}"
asset_name="${root_name}.tar.gz"
checksum_name="${asset_name}.sha256"
asset_path="${output_dir}/${asset_name}"
checksum_path="${output_dir}/${checksum_name}"

if [[ -e "${asset_path}" || -e "${checksum_path}" ]]; then
  echo "Refusing to overwrite existing release assets in ${output_dir}." >&2
  exit 73
fi

temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/atctl-release.XXXXXX")"
trap 'rm -f "${linkage_file}"; rm -rf "${temp_dir}"' EXIT

package_root="${temp_dir}/package/${root_name}"
extract_root="${temp_dir}/extract"
temp_asset="${temp_dir}/${asset_name}"
temp_checksum="${temp_dir}/${checksum_name}"

mkdir -p "${package_root}" "${extract_root}"
install -m 0755 "${binary_path}" "${package_root}/atctl"
install -m 0644 LICENSE "${package_root}/LICENSE"
scripts/maintenance/generate-third-party-notices.sh \
  "${package_root}/THIRD-PARTY-NOTICES.txt" \
  "${libusb_version}"
chmod 0644 "${package_root}/THIRD-PARTY-NOTICES.txt"

tar -C "${temp_dir}/package" -czf "${temp_asset}" "${root_name}"

expected_manifest="${temp_dir}/expected-manifest.txt"
actual_manifest="${temp_dir}/actual-manifest.txt"
printf '%s\n' \
  "${root_name}/" \
  "${root_name}/atctl" \
  "${root_name}/LICENSE" \
  "${root_name}/THIRD-PARTY-NOTICES.txt" > "${expected_manifest}"
tar -tzf "${temp_asset}" > "${actual_manifest}"
LC_ALL=C sort -o "${expected_manifest}" "${expected_manifest}"
LC_ALL=C sort -o "${actual_manifest}" "${actual_manifest}"

if ! diff -u "${expected_manifest}" "${actual_manifest}"; then
  echo "Release archive contents do not match the required manifest." >&2
  exit 65
fi

tar -xzf "${temp_asset}" -C "${extract_root}"
extracted_root="${extract_root}/${root_name}"

for regular_file in atctl LICENSE THIRD-PARTY-NOTICES.txt; do
  if [[ ! -f "${extracted_root}/${regular_file}" || -L "${extracted_root}/${regular_file}" ]]; then
    echo "Archive member is not a regular file: ${regular_file}" >&2
    exit 65
  fi
done

if [[ "$(stat -f '%Lp' "${extracted_root}/atctl")" != "755" ]]; then
  echo "Packaged atctl mode is not 0755." >&2
  exit 65
fi

for notice_file in LICENSE THIRD-PARTY-NOTICES.txt; do
  if [[ "$(stat -f '%Lp' "${extracted_root}/${notice_file}")" != "644" ]]; then
    echo "Packaged ${notice_file} mode is not 0644." >&2
    exit 65
  fi
done

if ! cmp -s LICENSE "${extracted_root}/LICENSE"; then
  echo "Packaged LICENSE differs from the repository LICENSE." >&2
  exit 65
fi

if ! cmp -s "${package_root}/THIRD-PARTY-NOTICES.txt" "${extracted_root}/THIRD-PARTY-NOTICES.txt"; then
  echo "Packaged third-party notices changed during archive creation." >&2
  exit 65
fi

(
  cd "${temp_dir}"
  shasum -a 256 "${asset_name}" > "${checksum_name}"
  shasum -a 256 -c "${checksum_name}"
)

mv "${temp_asset}" "${asset_path}"
mv "${temp_checksum}" "${checksum_path}"

printf 'asset=%s\n' "${asset_name}"
printf 'checksum=%s\n' "${checksum_name}"
printf 'libusb=%s\n' "${libusb_version}"
