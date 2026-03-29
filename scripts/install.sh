#!/bin/sh
# Install a prebuilt mdv binary from a published mdv channel.
# Usage:
#   curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/posaune0423/mdv/main/scripts/install.sh | sh
# Optional env:
#   MDV_INSTALL_DIR  directory for the binary (default: $HOME/.local/bin)
#   MDV_CHANNEL      release tag or rolling channel to install (default: main)

set -eu

REPO="posaune0423/mdv"
DEFAULT_INSTALL_DIR="${MDV_INSTALL_DIR:-$HOME/.local/bin}"
DEFAULT_CHANNEL="${MDV_CHANNEL:-main}"

die() {
  printf 'mdv install: %s\n' "$1" >&2
  exit 1
}

command -v curl >/dev/null 2>&1 || die "curl is required"
command -v tar >/dev/null 2>&1 || die "tar is required"

uname_s=$(uname -s)
uname_m=$(uname -m)

case "$uname_s" in
Linux) os=linux ;;
Darwin) os=darwin ;;
*) die "unsupported OS: $uname_s (expected Linux or Darwin)" ;;
esac

case "$uname_m" in
x86_64 | amd64) arch=x86_64 ;;
arm64 | aarch64) arch=aarch64 ;;
*) die "unsupported CPU: $uname_m" ;;
esac

if [ "$os" = "linux" ]; then
  target="${arch}-unknown-linux-gnu"
elif [ "$os" = "darwin" ]; then
  target="${arch}-apple-darwin"
else
  die "internal error"
fi

asset="mdv-${target}.tar.gz"
channel=$(printf '%s' "$DEFAULT_CHANNEL" | tr -d '[:space:]')
[ -n "$channel" ] || die "MDV_CHANNEL must not be empty"
url="https://github.com/${REPO}/releases/download/${channel}/${asset}"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT INT TERM

printf 'Downloading %s from channel %s\n' "$asset" "$channel"
if ! curl -fL --proto '=https' --tlsv1.2 --retry 3 --retry-delay 1 -o "$tmp_dir/$asset" "$url"; then
  die "download failed (is there a published ${channel} build with ${asset}?)"
fi

(
  cd "$tmp_dir" && tar xzf "$asset"
)

if [ ! -f "$tmp_dir/mdv" ]; then
  die "archive did not contain a top-level mdv binary"
fi

install_dir="$DEFAULT_INSTALL_DIR"
case "$install_dir" in
/*) ;;
*) die "MDV_INSTALL_DIR must be an absolute path" ;;
esac

mkdir -p "$install_dir"
chmod +x "$tmp_dir/mdv"
mv "$tmp_dir/mdv" "$install_dir/mdv"

printf '\nInstalled mdv to %s/mdv\n' "$install_dir"
printf 'Ensure this directory is on your PATH (e.g. export PATH="%s:$PATH").\n' "$install_dir"
