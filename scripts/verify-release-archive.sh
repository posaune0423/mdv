#!/bin/sh

set -eu

die() {
  printf 'release verify: %s\n' "$1" >&2
  exit 1
}

archive="${1:-}"
checksum_file="${2:-}"

[ -n "$archive" ] || die "usage: scripts/verify-release-archive.sh <archive> <checksum-file>"
[ -n "$checksum_file" ] || die "usage: scripts/verify-release-archive.sh <archive> <checksum-file>"
[ -f "$archive" ] || die "archive not found: $archive"
[ -f "$checksum_file" ] || die "checksum file not found: $checksum_file"

archive_name=$(basename "$archive")
expected_hash=$(awk -v name="$archive_name" '$2 == name { print $1 }' "$checksum_file")
[ -n "$expected_hash" ] || die "checksum entry for ${archive_name} not found in ${checksum_file}"

if command -v sha256sum >/dev/null 2>&1; then
  actual_hash=$(sha256sum "$archive" | awk '{ print $1 }')
else
  actual_hash=$(shasum -a 256 "$archive" | awk '{ print $1 }')
fi

[ "$expected_hash" = "$actual_hash" ] || die "checksum mismatch for ${archive_name}"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

tar xzf "$archive" -C "$tmp_dir"

bin="${tmp_dir}/mdv"
[ -f "$bin" ] || die "archive did not contain a top-level mdv binary"
chmod +x "$bin"
"$bin" --help >/dev/null

printf 'verified %s\n' "$archive"
