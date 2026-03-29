#!/bin/sh

set -eu

die() {
  printf 'release package: %s\n' "$1" >&2
  exit 1
}

target="${1:-}"
out_dir="${2:-dist}"

[ -n "$target" ] || die "usage: scripts/package-release.sh <target-triple> [output-dir]"

mkdir -p "$out_dir"

cargo build --release --locked --target "$target"

bin="target/${target}/release/mdv"
[ -x "$bin" ] || die "missing built binary at ${bin}"

case "$target" in
*-apple-darwin)
  if command -v codesign >/dev/null 2>&1; then
    codesign --force --sign - "$bin"
  fi
  ;;
esac

asset="mdv-${target}.tar.gz"
asset_path="${out_dir}/${asset}"
checksum_path="${out_dir}/SHA256SUMS.part"

tar czf "$asset_path" -C "target/${target}/release" mdv

(
  cd "$out_dir"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$asset" > SHA256SUMS.part
  else
    shasum -a 256 "$asset" > SHA256SUMS.part
  fi
)

printf 'packaged %s\n' "$asset_path"
printf 'wrote %s\n' "$checksum_path"
